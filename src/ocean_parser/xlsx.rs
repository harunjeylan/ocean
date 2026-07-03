use std::collections::HashMap;
use std::path::PathBuf;

use calamine::{Data, Reader, Xlsx, open_workbook};

use crate::ocean_parser::helpers::validate_file_size;
use crate::ocean_parser::{
    Document, DocumentError, DocumentFormat, DocumentMetadata, Match, ReadResult, Selector,
};

/// Cell reference parser for converting between A1 notation and indices
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CellRef {
    pub(crate) col: usize,
    pub(crate) row: usize,
}

impl CellRef {
    /// Parse cell reference like "A1", "AA42", etc.
    pub(crate) fn parse(cell_ref: &str) -> Result<Self, DocumentError> {
        let cell_upper = cell_ref.to_uppercase();

        if cell_upper.is_empty() {
            return Err(DocumentError::InvalidSelector(
                "empty cell reference".to_string(),
            ));
        }

        let (col_part, row_part): (String, String) =
            cell_upper.chars().partition(|c| c.is_ascii_alphabetic());

        if col_part.is_empty() || row_part.is_empty() {
            return Err(DocumentError::InvalidSelector(format!(
                "invalid cell reference format: {}",
                cell_ref
            )));
        }

        // Convert column letters to index (A=0, Z=25, AA=26, etc.)
        let col = col_part
            .chars()
            .fold(0usize, |acc, c| acc * 26 + (c as usize - 'A' as usize + 1));

        if col == 0 || col > 16384 {
            // Excel max columns
            return Err(DocumentError::InvalidSelector(format!(
                "column out of range: {}",
                col_part
            )));
        }

        let row: usize = row_part.parse().map_err(|_| {
            DocumentError::InvalidSelector(format!("invalid row number: {}", row_part))
        })?;

        if row == 0 || row > 1_048_576 {
            // Excel max rows
            return Err(DocumentError::InvalidSelector(format!(
                "row out of range: {}",
                row
            )));
        }

        Ok(CellRef {
            col: col - 1, // Convert to 0-based
            row: row - 1, // Convert to 0-based
        })
    }

    /// Convert indices back to A1 notation
    pub(crate) fn to_notation(&self) -> String {
        let mut col_str = String::new();
        let mut col = self.col + 1;

        while col > 0 {
            col -= 1;
            col_str.insert(0, (b'A' + (col % 26) as u8) as char);
            col /= 26;
        }

        format!("{}{}", col_str, self.row + 1)
    }
}

/// Range parser for converting ranges like "A1:B10"
#[derive(Debug, Clone)]
pub(crate) struct RangeRef {
    start: CellRef,
    end: CellRef,
}

impl RangeRef {
    pub(crate) fn parse(range_str: &str) -> Result<Self, DocumentError> {
        let parts: Vec<&str> = range_str.split(':').collect();
        if parts.len() != 2 {
            return Err(DocumentError::InvalidSelector(format!(
                "invalid range format: {}",
                range_str
            )));
        }

        let start = CellRef::parse(parts[0])?;
        let end = CellRef::parse(parts[1])?;

        Ok(RangeRef { start, end })
    }

    pub(crate) fn cells(&self) -> Vec<CellRef> {
        let mut cells = Vec::new();
        let min_col = self.start.col.min(self.end.col);
        let max_col = self.start.col.max(self.end.col);
        let min_row = self.start.row.min(self.end.row);
        let max_row = self.start.row.max(self.end.row);

        for row in min_row..=max_row {
            for col in min_col..=max_col {
                cells.push(CellRef { col, row });
            }
        }

        cells
    }
}

pub struct XlsxDocument {
    path: PathBuf,
    size: u64,
    sheet_names: Vec<String>,
    sheets: Vec<Vec<Vec<String>>>,
    // Cache for frequently accessed data
    sheet_index_cache: HashMap<String, usize>,
    // Metadata cache
    row_counts: Vec<usize>,
    col_counts: Vec<usize>,
}

impl XlsxDocument {
    pub fn open(path: &str) -> Result<Self, DocumentError> {
        let p = PathBuf::from(path);
        let meta = std::fs::metadata(&p)
            .map_err(|e| DocumentError::PermissionDenied(format!("{}: {}", path, e)))?;

        if !meta.is_file() {
            return Err(DocumentError::ParseFailed(format!(
                "path is not a file: {}",
                path
            )));
        }

        validate_file_size(path, meta.len())?;

        let mut workbook: Xlsx<_> = open_workbook(&p)
            .map_err(|e| DocumentError::CorruptedFile(format!("invalid xlsx: {}", e)))?;

        let sheet_names: Vec<String> = workbook.sheet_names().to_vec();
        if sheet_names.is_empty() {
            return Err(DocumentError::CorruptedFile(
                "workbook contains no sheets".to_string(),
            ));
        }

        let mut sheets = Vec::with_capacity(sheet_names.len());
        let mut row_counts = Vec::with_capacity(sheet_names.len());
        let mut col_counts = Vec::with_capacity(sheet_names.len());

        for name in &sheet_names {
            if let Ok(range) = workbook.worksheet_range(name) {
                let rows: Vec<Vec<String>> = range
                    .rows()
                    .map(|row| row.iter().map(|cell| Self::cell_to_string(cell)).collect())
                    .collect();

                let row_count = rows.len();
                let col_count = rows.first().map(|r| r.len()).unwrap_or(0);

                row_counts.push(row_count);
                col_counts.push(col_count);
                sheets.push(rows);
            } else {
                row_counts.push(0);
                col_counts.push(0);
                sheets.push(vec![]);
            }
        }

        let mut sheet_index_cache = HashMap::new();
        for (idx, name) in sheet_names.iter().enumerate() {
            sheet_index_cache.insert(name.to_lowercase(), idx);
        }

        Ok(Self {
            path: p,
            size: meta.len(),
            sheet_names,
            sheets,
            sheet_index_cache,
            row_counts,
            col_counts,
        })
    }

    /// Convert cell data to string with comprehensive handling
    fn cell_to_string(cell: &Data) -> String {
        match cell {
            Data::String(s) => s.clone(),
            Data::Float(f) => {
                // Handle special float values
                if f.is_nan() {
                    "#N/A".to_string()
                } else if f.is_infinite() {
                    if *f > 0.0 {
                        "#INF".to_string()
                    } else {
                        "#-INF".to_string()
                    }
                } else if f.fract() == 0.0 && *f >= 0.0 && *f < 1e10 {
                    // Format as integer if it's a whole number
                    format!("{}", *f as i64)
                } else {
                    f.to_string()
                }
            }
            Data::Int(i) => i.to_string(),
            Data::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
            Data::DateTime(d) => d.to_string(),
            Data::Error(e) => format!("#ERR:{}", e),
            Data::DurationIso(d) => d.clone(),
            _ => String::new(),
        }
    }

    fn find_sheet(&self, name: &str) -> Option<usize> {
        self.sheet_index_cache.get(&name.to_lowercase()).copied()
    }

    /// Get cell value safely with bounds checking
    fn get_cell(&self, sheet_idx: usize, row: usize, col: usize) -> Option<String> {
        self.sheets
            .get(sheet_idx)
            .and_then(|sheet| sheet.get(row))
            .and_then(|row| row.get(col))
            .cloned()
    }

    /// Get surrounding context around a cell (up to 2 cells in each direction)
    fn get_cell_context(
        &self,
        sheet_idx: usize,
        row: usize,
        col: usize,
        max_context: usize,
    ) -> String {
        let min_row = row.saturating_sub(max_context);
        let max_row =
            (row + max_context + 1).min(self.row_counts.get(sheet_idx).copied().unwrap_or(0));
        let min_col = col.saturating_sub(max_context);
        let max_col =
            (col + max_context + 1).min(self.col_counts.get(sheet_idx).copied().unwrap_or(0));

        let context_parts: Vec<String> = (min_row..max_row)
            .flat_map(|r| {
                (min_col..max_col)
                    .filter_map(|c| {
                        self.get_cell(sheet_idx, r, c)
                            .filter(|cell| !cell.is_empty())
                    })
                    .collect::<Vec<_>>()
            })
            .take(5)
            .collect();

        context_parts.join(", ")
    }

    /// Calculate relevance score for search matches
    fn calculate_score(&self, query: &str, cell_content: &str, is_exact: bool) -> f32 {
        let query_lower = query.to_lowercase();
        let content_lower = cell_content.to_lowercase();

        if is_exact && content_lower == query_lower {
            return 1.0;
        }

        if is_exact && content_lower.starts_with(&query_lower) {
            return 0.9;
        }

        let matches = content_lower.matches(&query_lower).count();
        let position = content_lower.find(&query_lower).unwrap_or(usize::MAX) as f32;
        let position_score = 1.0 - (position / content_lower.len().max(1) as f32 * 0.1);

        (matches as f32 * 0.5 + position_score.max(0.1)).min(0.99)
    }
}

impl Document for XlsxDocument {
    fn metadata(&self) -> DocumentMetadata {
        DocumentMetadata {
            path: self.path.clone(),
            format: DocumentFormat::Xlsx,
            title: None,
            author: None,
            created: None,
            modified: None,
            page_count: Some(self.sheet_names.len() as u32),
            size: self.size,
        }
    }

    fn search(&self, query: &str) -> Vec<Match> {
        if query.is_empty() {
            return vec![];
        }

        let q = query.to_lowercase();
        let is_exact = query.len() > 0 && !query.contains(' ');
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for (sheet_idx, rows) in self.sheets.iter().enumerate() {
            for (row_idx, row) in rows.iter().enumerate() {
                for (col_idx, cell) in row.iter().enumerate() {
                    if !cell.is_empty() && cell.to_lowercase().contains(&q) {
                        let cell_ref = CellRef {
                            col: col_idx,
                            row: row_idx,
                        };
                        let cell_notation = cell_ref.to_notation();
                        let selector_key = format!("{}/{}", sheet_idx, cell_notation);

                        if seen.insert(selector_key) {
                            let score = self.calculate_score(&q, cell, is_exact);
                            let context = self.get_cell_context(sheet_idx, row_idx, col_idx, 2);

                            results.push(Match {
                                selector: Selector::Cell(cell_notation),
                                text: cell.clone(),
                                context: if context.is_empty() {
                                    format!("Sheet: {}", self.sheet_names[sheet_idx])
                                } else {
                                    format!(
                                        "Sheet: {} | Context: {}",
                                        self.sheet_names[sheet_idx], context
                                    )
                                },
                                score: score as f64,
                            });
                        }
                    }
                }
            }
        }

        // Sort by relevance score
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(1000); // Limit results

        results
    }

    fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError> {
        match selector {
            Selector::Sheet(name) => {
                if let Some(idx) = self.find_sheet(name) {
                    let rows = self.sheets[idx].clone();
                    Ok(ReadResult::Sheet {
                        name: name.clone(),
                        rows,
                    })
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "sheet '{}' not found",
                        name
                    )))
                }
            }
            Selector::Cell(cell_ref) => {
                if let Some(pos) = cell_ref.find('!') {
                    let name = &cell_ref[..pos];
                    let cell_part = &cell_ref[pos + 1..];
                    let idx = self.find_sheet(name).ok_or_else(|| {
                        DocumentError::InvalidSelector(format!("sheet '{}' not found", name))
                    })?;
                    let cell = CellRef::parse(cell_part)?;
                    if cell.row < self.sheets[idx].len()
                        && cell.col < self.sheets[idx][cell.row].len()
                    {
                        Ok(ReadResult::CellValue(
                            self.sheets[idx][cell.row][cell.col].clone(),
                        ))
                    } else {
                        Ok(ReadResult::CellValue(String::new()))
                    }
                } else {
                    let cell = CellRef::parse(cell_ref)?;
                    // Use first sheet by default
                    if let Some(first_sheet) = self.sheets.first() {
                        if cell.row < first_sheet.len() && cell.col < first_sheet[cell.row].len() {
                            Ok(ReadResult::CellValue(
                                first_sheet[cell.row][cell.col].clone(),
                            ))
                        } else {
                            Ok(ReadResult::CellValue(String::new()))
                        }
                    } else {
                        Err(DocumentError::InvalidSelector(
                            "no sheets in document".to_string(),
                        ))
                    }
                }
            }
            Selector::Range { start, end } => {
                // Support ranges like "A1:B10" or raw index ranges
                let range_str = format!("{}:{}", start, end);
                let range = RangeRef::parse(&range_str)?;
                let cells = range.cells();

                let mut values = Vec::new();
                if let Some(first_sheet) = self.sheets.first() {
                    for cell_ref in cells {
                        if cell_ref.row < first_sheet.len()
                            && cell_ref.col < first_sheet[cell_ref.row].len()
                        {
                            values.push(first_sheet[cell_ref.row][cell_ref.col].clone());
                        }
                    }
                }

                Ok(ReadResult::Text(values.join("\n")))
            }
            Selector::Slice { skip, take } => {
                let s = *skip as usize;
                let t = *take as usize;
                let mut parts = Vec::new();
                for (i, sheet) in self.sheets.iter().enumerate() {
                    parts.push(format!(
                        "--- Sheet: {} ({} rows) ---",
                        self.sheet_names[i], self.row_counts[i]
                    ));
                    // Skip s rows, take up to min(t, 100) rows per sheet
                    for row in sheet.iter().skip(s).take(t.min(100)) {
                        parts.push(row.join(" | "));
                    }
                    let displayed = t.min(100).min(sheet.len().saturating_sub(s));
                    let remaining = sheet.len().saturating_sub(s + displayed);
                    if remaining > 0 {
                        parts.push(format!("... ({} more rows)", remaining));
                    }
                }
                Ok(ReadResult::Text(parts.join("\n")))
            }
            Selector::Row(n) => {
                let idx = *n as usize;
                if let Some(first_sheet) = self.sheets.first() {
                    if idx < first_sheet.len() {
                        Ok(ReadResult::Text(first_sheet[idx].join(" | ")))
                    } else {
                        Err(DocumentError::InvalidSelector(format!(
                            "row {} not found (sheet has {} rows)",
                            n,
                            first_sheet.len()
                        )))
                    }
                } else {
                    Err(DocumentError::InvalidSelector(
                        "no sheets in document".to_string(),
                    ))
                }
            }
            Selector::Column(n) => {
                let idx = *n as usize;
                if let Some(first_sheet) = self.sheets.first() {
                    let mut col_values = Vec::new();
                    for row in first_sheet.iter() {
                        if idx < row.len() {
                            col_values.push(row[idx].clone());
                        }
                    }
                    if !col_values.is_empty() {
                        Ok(ReadResult::Text(col_values.join("\n")))
                    } else {
                        Err(DocumentError::InvalidSelector(format!(
                            "column {} not found",
                            n
                        )))
                    }
                } else {
                    Err(DocumentError::InvalidSelector(
                        "no sheets in document".to_string(),
                    ))
                }
            }
            Selector::Sheets(names) => {
                let mut parts = Vec::new();
                for name in names {
                    let idx = self.find_sheet(name).ok_or_else(|| {
                        DocumentError::InvalidSelector(format!("sheet '{}' not found", name))
                    })?;
                    let rows = &self.sheets[idx];
                    parts.push(format!("--- Sheet: {} ---", self.sheet_names[idx]));
                    for row in rows.iter().take(100) {
                        parts.push(row.join(" | "));
                    }
                    if rows.len() > 100 {
                        parts.push(format!("... ({} more rows)", rows.len() - 100));
                    }
                }
                Ok(ReadResult::Text(parts.join("\n")))
            }
            Selector::Slides(_) => Err(DocumentError::InvalidSelector(
                "slides selector not supported for xlsx documents".to_string(),
            )),
            Selector::Rows(rows) => {
                let mut parts = Vec::new();
                for row in rows {
                    let row_idx = *row as usize;
                    for (sheet_idx, sheet) in self.sheets.iter().enumerate() {
                        parts.push(format!(
                            "--- Sheet: {} Row: {} ---",
                            self.sheet_names[sheet_idx], row
                        ));
                        if row_idx < sheet.len() {
                            parts.push(sheet[row_idx].join(" | "));
                        }
                    }
                }
                if parts.is_empty() {
                    return Err(DocumentError::InvalidSelector(
                        "no rows matched".to_string(),
                    ));
                }
                Ok(ReadResult::Text(parts.join("\n")))
            }
            Selector::Columns(columns) => {
                let mut parts = Vec::new();
                for col in columns {
                    let col_idx = *col as usize;
                    for (sheet_idx, sheet) in self.sheets.iter().enumerate() {
                        parts.push(format!(
                            "--- Sheet: {} Column: {} ---",
                            self.sheet_names[sheet_idx], col
                        ));
                        let mut col_values = Vec::new();
                        for row in sheet.iter() {
                            if col_idx < row.len() {
                                col_values.push(row[col_idx].clone());
                            }
                        }
                        if col_values.is_empty() {
                            parts.push("(empty column)".to_string());
                        } else {
                            parts.push(col_values.join("\n"));
                        }
                    }
                }
                if parts.is_empty() {
                    return Err(DocumentError::InvalidSelector(
                        "no columns matched".to_string(),
                    ));
                }
                Ok(ReadResult::Text(parts.join("\n")))
            }
            _ => Err(DocumentError::InvalidSelector(format!(
                "selector {:?} not supported for xlsx documents",
                selector
            ))),
        }
    }
}
