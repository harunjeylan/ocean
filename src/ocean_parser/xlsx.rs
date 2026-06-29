use std::path::PathBuf;

use calamine::{open_workbook, Data, Reader, Xlsx};

use crate::ocean_parser::{
    Document, DocumentError, DocumentFactory, DocumentFormat, DocumentMetadata, Match, Outline,
    OutlineEntry, ReadResult, Selector,
};

pub struct XlsxDocument {
    path: PathBuf,
    size: u64,
    sheet_names: Vec<String>,
    sheets: Vec<Vec<Vec<String>>>,
}

impl XlsxDocument {
    pub fn open(path: &str) -> Result<Self, DocumentError> {
        let p = PathBuf::from(path);
        let meta = std::fs::metadata(&p)
            .map_err(|e| DocumentError::PermissionDenied(format!("{}: {}", path, e)))?;

        if meta.len() > 500 * 1024 * 1024 {
            return Err(DocumentError::ParseFailed(format!(
                "file too large ({} bytes): {}",
                meta.len(),
                path
            )));
        }

        let mut workbook: Xlsx<_> = open_workbook(&p)
            .map_err(|e| DocumentError::CorruptedFile(format!("invalid xlsx: {}", e)))?;

        let sheet_names: Vec<String> = workbook.sheet_names().to_vec();
        let mut sheets = vec![];

        for name in &sheet_names {
            if let Ok(range) = workbook.worksheet_range(name) {
                let rows: Vec<Vec<String>> = range
                    .rows()
                    .map(|row| {
                        row.iter()
                            .map(|cell| match cell {
                                Data::String(s) => s.clone(),
                                Data::Float(f) => f.to_string(),
                                Data::Int(i) => i.to_string(),
                                Data::Bool(b) => b.to_string(),
                                Data::DateTime(d) => d.to_string(),
                                Data::Error(e) => format!("ERROR:{}", e),
                                _ => String::new(),
                            })
                            .collect()
                    })
                    .collect();
                sheets.push(rows);
            } else {
                sheets.push(vec![]);
            }
        }

        Ok(Self {
            path: p,
            size: meta.len(),
            sheet_names,
            sheets,
        })
    }

    fn find_sheet(&self, name: &str) -> Option<usize> {
        self.sheet_names
            .iter()
            .position(|s| s.to_lowercase() == name.to_lowercase())
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
            page_count: None,
            size: self.size,
        }
    }

    fn outline(&self) -> Outline {
        let entries: Vec<OutlineEntry> = self
            .sheet_names
            .iter()
            .map(|name| OutlineEntry {
                label: name.clone(),
                level: 1,
                selector: Selector::Sheet(name.clone()),
                children: vec![],
            })
            .collect();

        Outline { entries }
    }

    fn page_count(&self) -> Option<u32> {
        None
    }

    fn search(&self, query: &str) -> Vec<Match> {
        let q = query.to_lowercase();
        let mut results = vec![];

        for (sheet_idx, rows) in self.sheets.iter().enumerate() {
            for (row_idx, row) in rows.iter().enumerate() {
                for cell in row {
                    if cell.to_lowercase().contains(&q) {
                        let col = (row_idx % 26) as u8;
                        let col_letter = std::char::from_u32(b'A' as u32 + col as u32)
                            .unwrap_or('A');
                        let cell_ref = format!("{}{}", col_letter, row_idx + 1);
                        results.push(Match {
                            selector: Selector::Cell(cell_ref),
                            text: cell.clone(),
                            context: format!("Sheet: {}", self.sheet_names[sheet_idx]),
                            score: 1.0,
                        });
                    }
                }
            }
        }

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
                let cell_ref_upper = cell_ref.to_uppercase();
                let col_letter: String = cell_ref_upper.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
                let row_num: usize = cell_ref_upper
                    .chars()
                    .skip_while(|c| c.is_ascii_alphabetic())
                    .collect::<String>()
                    .parse()
                    .map_err(|_| {
                        DocumentError::InvalidSelector(format!("invalid cell reference: {}", cell_ref))
                    })?;

                let col_idx = col_letter
                    .chars()
                    .fold(0usize, |acc, c| acc * 26 + (c as usize - 'A' as usize));
                let row_idx = row_num.saturating_sub(1);

                // Use first sheet by default
                if let Some(first_sheet) = self.sheets.first() {
                    if row_idx < first_sheet.len() && col_idx < first_sheet[row_idx].len() {
                        Ok(ReadResult::CellValue(first_sheet[row_idx][col_idx].clone()))
                    } else {
                        Ok(ReadResult::CellValue(String::new()))
                    }
                } else {
                    Err(DocumentError::InvalidSelector("no sheets in document".to_string()))
                }
            }
            Selector::Table(n) => {
                let idx = *n as usize;
                if idx < self.sheets.len() {
                    let rows = self.sheets[idx].clone();
                    let headers = if rows.is_empty() { vec![] } else { rows[0].clone() };
                    let data = if rows.len() > 1 {
                        rows[1..].to_vec()
                    } else {
                        vec![]
                    };
                    Ok(ReadResult::Table { headers, rows: data })
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "table {} not found",
                        n
                    )))
                }
            }
            Selector::Slice { skip, take } => {
                let s = *skip as usize;
                let t = *take as usize;
                if s >= self.sheets.len() {
                    return Err(DocumentError::InvalidSelector(format!(
                        "skip {} beyond sheet count {}",
                        skip,
                        self.sheets.len()
                    )));
                }
                let end = std::cmp::min(s + t, self.sheets.len());
                let mut parts = Vec::new();
                for i in s..end {
                    parts.push(format!("--- Sheet: {} ---", self.sheet_names[i]));
                    for row in &self.sheets[i] {
                        parts.push(row.join(" | "));
                    }
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

pub struct XlsxFactory;

impl DocumentFactory for XlsxFactory {
    fn open(&self, path: &str) -> Result<Box<dyn Document>, DocumentError> {
        XlsxDocument::open(path).map(|d| Box::new(d) as Box<dyn Document>)
    }
}
