use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;

use lopdf::Document as PdfDoc;
use lopdf::{Object, ObjectId};

use crate::ocean_fs::parser::helpers::{parse_pdf_date, validate_file_size};
use crate::ocean_fs::parser::{
    Document, DocumentElement, DocumentError, DocumentFormat, DocumentMetadata, HeadingElement,
    ImageElement, ImageFormat, ImagePosition, Margins, Match, PageLayout,
    PageOrientation, ParagraphElement, ReadResult, Run, Selector,
};

#[derive(Debug, Default, Clone)]
struct PdfMetadataExtended {
    title: Option<String>,
    author: Option<String>,
    subject: Option<String>,
    keywords: Option<String>,
    creator: Option<String>,
    producer: Option<String>,
    created: Option<String>,
    modified: Option<String>,
    trapped: Option<bool>,
}

#[derive(Debug, Clone)]
struct FontInfo {
    base_font: String,
    bold: bool,
    italic: bool,
    mono: bool,
}

#[derive(Debug, Clone, Default)]
struct ParsedRun {
    text: String,
    bold: bool,
    italic: bool,
    code: bool,
    font_size: f32,
    font_family: Option<String>,
    hyperlink_url: Option<String>,
}

pub struct PdfDocument {
    path: PathBuf,
    size: u64,
    doc: Arc<PdfDoc>,
    page_layouts: Vec<PageLayout>,
    page_elements: Vec<OnceLock<Vec<DocumentElement>>>,
    page_ids: Vec<(u32, ObjectId)>,
    metadata: PdfMetadataExtended,
    page_map: HashMap<u32, usize>,
}

fn parse_page_dimensions(doc: &PdfDoc, page_id: ObjectId) -> (f32, f32, PageOrientation) {
    if let Ok(page) = doc.get_dictionary(page_id) {
        if let Ok(media_box) = page.get(b"MediaBox").and_then(|o| o.as_array()) {
            if media_box.len() >= 4 {
                let x0 = media_box[0].as_float().unwrap_or(0.0);
                let y0 = media_box[1].as_float().unwrap_or(0.0);
                let x1 = media_box[2].as_float().unwrap_or(612.0);
                let y1 = media_box[3].as_float().unwrap_or(792.0);
                let w = (x1 - x0).abs() / 72.0;
                let h = (y1 - y0).abs() / 72.0;
                let orientation = if w > h {
                    PageOrientation::Landscape
                } else {
                    PageOrientation::Portrait
                };
                return (w, h, orientation);
            }
        }
    }
    (8.5, 11.0, PageOrientation::Portrait)
}

fn analyze_font(font_dict: &lopdf::Dictionary) -> FontInfo {
    let base_font = font_dict
        .get(b"BaseFont")
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(|n| String::from_utf8_lossy(n).to_string())
        .unwrap_or_default();

    let mut bold = false;
    let mut italic = false;
    let mut mono = false;

    // Check font name for bold/italic/mono hints
    let name_upper = base_font.to_uppercase();
    if name_upper.contains("BOLD") {
        bold = true;
    }
    if name_upper.contains("ITALIC") || name_upper.contains("OBLIQUE") {
        italic = true;
    }
    if name_upper.contains("COURIER") || name_upper.contains("MONO") {
        mono = true;
    }

    // Check FontDescriptor flags if available
    if let Ok(_desc_ref) = font_dict
        .get(b"FontDescriptor")
        .and_then(|o| o.as_reference())
    {
        // FontDescriptor not directly accessible from here without doc reference
        // We'll handle this via the flags from the descriptor
        // For now name-based detection is sufficient
    }

    // Check /Flags in font dict directly (some fonts put it here)
    if let Ok(flags) = font_dict.get(b"Flags").and_then(|o| o.as_i64()) {
        // PDF Flag bits: 1=FixedPitch, 2=Serif, 4=Script, 6=Italic, 7=AllCap, 18=Bold
        if flags & (1 << 6) != 0 {
            italic = true;
        }
        if flags & (1 << 18) != 0 {
            bold = true;
        }
        if flags & 1 != 0 {
            mono = true;
        }
    }

    FontInfo {
        base_font,
        bold,
        italic,
        mono,
    }
}

/// Extract text from content stream operands (Tj / TJ / ' / " operators).
fn extract_operand_text(operands: &[Object]) -> String {
    let mut text = String::new();
    for op in operands {
        match op {
            Object::String(bytes, _) => {
                if let Ok(s) = std::str::from_utf8(bytes) {
                    text.push_str(s);
                } else {
                    text.push_str(&String::from_utf8_lossy(bytes));
                }
            }
            Object::Array(arr) => {
                for item in arr {
                    if let Object::String(bytes, _) = item {
                        if let Ok(s) = std::str::from_utf8(bytes) {
                            text.push_str(s);
                        } else {
                            text.push_str(&String::from_utf8_lossy(bytes));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    text
}

/// Parse page content stream into structured document elements.
fn parse_page_content(
    doc: &PdfDoc,
    page_id: ObjectId,
    page_number: u32,
) -> Result<(Vec<DocumentElement>, Vec<ImageElement>), DocumentError> {
    let content = doc
        .get_and_decode_page_content(page_id)
        .map_err(|e| DocumentError::ParseFailed(format!("failed to decode page content: {}", e)))?;

    let fonts = doc
        .get_page_fonts(page_id)
        .map_err(|e| DocumentError::ParseFailed(format!("failed to get page fonts: {}", e)))?;

    let images = doc
        .get_page_images(page_id)
        .map_err(|e| DocumentError::ParseFailed(format!("failed to get page images: {}", e)))?;

    // Build font info map
    let mut font_info_map: HashMap<Vec<u8>, FontInfo> = HashMap::new();
    for (name, dict) in &fonts {
        let info = analyze_font(dict);
        font_info_map.insert(name.clone(), info);
    }

    // Parse operations into raw text runs
    let mut current_font_name: Option<Vec<u8>> = None;
    let mut current_font_size: f32 = 12.0;
    let mut raw_runs: Vec<ParsedRun> = Vec::new();

    for op in &content.operations {
        match op.operator.as_str() {
            "Tf" => {
                if op.operands.len() >= 2 {
                    current_font_name = op.operands[0].as_name().ok().map(|n| n.to_vec());
                    current_font_size = op.operands[1].as_float().unwrap_or(12.0);
                }
            }
            "Tj" => {
                let text = extract_operand_text(&op.operands);
                if !text.is_empty() {
                    let style = build_style(&current_font_name, &font_info_map);
                    raw_runs.push(ParsedRun {
                        text,
                        font_size: current_font_size,
                        font_family: style.font_family,
                        bold: style.bold,
                        italic: style.italic,
                        code: style.code,
                        ..Default::default()
                    });
                }
            }
            "TJ" => {
                let text = extract_operand_text(&op.operands);
                if !text.is_empty() {
                    let style = build_style(&current_font_name, &font_info_map);
                    raw_runs.push(ParsedRun {
                        text,
                        font_size: current_font_size,
                        font_family: style.font_family,
                        bold: style.bold,
                        italic: style.italic,
                        code: style.code,
                        ..Default::default()
                    });
                }
            }
            "'" => {
                let text = extract_operand_text(&op.operands);
                if !text.is_empty() {
                    let style = build_style(&current_font_name, &font_info_map);
                    raw_runs.push(ParsedRun {
                        text,
                        font_size: current_font_size,
                        font_family: style.font_family,
                        bold: style.bold,
                        italic: style.italic,
                        code: style.code,
                        ..Default::default()
                    });
                }
            }
            "\"" => {
                if op.operands.len() >= 3 {
                    let text = extract_operand_text(&op.operands[2..]);
                    if !text.is_empty() {
                        let style = build_style(&current_font_name, &font_info_map);
                        raw_runs.push(ParsedRun {
                            text,
                            font_size: current_font_size,
                            font_family: style.font_family,
                            bold: style.bold,
                            italic: style.italic,
                            code: style.code,
                            ..Default::default()
                        });
                    }
                }
            }
            _ => {}
        }
    }

    // Group runs into paragraphs
    let mut elements: Vec<DocumentElement> = Vec::new();
    if raw_runs.is_empty() {
        return Ok((elements, vec![]));
    }

    // Simple heuristic: group runs by font size changes or large gaps
    // In PDF, each Tj/TJ typically represents one "text snippet"
    // We merge consecutive runs with similar font size into paragraphs
    let mut merged_runs: Vec<Vec<ParsedRun>> = Vec::new();
    if !raw_runs.is_empty() {
        merged_runs.push(vec![raw_runs[0].clone()]);
        for run in raw_runs.iter().skip(1) {
            let last = merged_runs.last().unwrap().last().unwrap();
            let size_diff = (run.font_size - last.font_size).abs();
            if size_diff > 2.0 || run.bold != last.bold || run.italic != last.italic {
                merged_runs.push(vec![run.clone()]);
            } else {
                merged_runs.last_mut().unwrap().push(run.clone());
            }
        }
    }

    // Convert merged runs to elements with heading detection
    let max_size = merged_runs
        .iter()
        .filter_map(|g| g.first().map(|r| r.font_size))
        .fold(0.0f32, f32::max);

    for group in &merged_runs {
        let font_size = group.first().map(|r| r.font_size).unwrap_or(12.0);
        let bold = group.first().map(|r| r.bold).unwrap_or(false);
        let text: String = group.iter().map(|r| r.text.as_str()).collect();

        let runs: Vec<Run> = group
            .iter()
            .map(|pr| Run {
                text: pr.text.clone(),
                bold: pr.bold,
                italic: pr.italic,
                underline: false,
                strike: false,
                subscript: false,
                superscript: false,
                code: pr.code,
                hyperlink_url: pr.hyperlink_url.clone(),
                font_size: Some(pr.font_size),
                font_family: pr.font_family.clone(),
                font_color: None,
                highlight_color: None,
            })
            .collect();

        // Classify: headings if large font or if bold + short text
        let is_large = font_size >= max_size * 0.75 && font_size >= 14.0;
        let is_bold_short = bold && text.split_whitespace().count() <= 8 && font_size >= 12.0;
        if is_large || is_bold_short {
            let level = if font_size >= 20.0 {
                1
            } else if font_size >= 16.0 {
                2
            } else {
                3
            };
            elements.push(DocumentElement::Heading(HeadingElement { level, runs }));
        } else {
            elements.push(DocumentElement::Paragraph(ParagraphElement {
                runs,
                alignment: None,
                indent_left: None,
                indent_right: None,
                indent_first_line: None,
                spacing_before: None,
                spacing_after: None,
                line_spacing: None,
                border_top: false,
                border_bottom: false,
                background_color: None,
            }));
        }
    }

    // Add image elements from XObject references
    let mut image_elements: Vec<ImageElement> = Vec::new();
    for img in &images {
        let format = match img
            .filters
            .as_ref()
            .and_then(|f| f.first().map(|s| s.as_str()))
        {
            Some("DCTDecode") => ImageFormat::Jpeg,
            Some("JPXDecode") => ImageFormat::Jpeg,
            _ => ImageFormat::Png,
        };
        image_elements.push(ImageElement {
            path: format!(
                "page{}.img{}.{}",
                page_number,
                img.id.0,
                match format {
                    ImageFormat::Jpeg => "jpg",
                    ImageFormat::Png => "png",
                    _ => "bin",
                }
            ),
            bytes: img.content.to_vec(),
            format,
            caption: None,
            alt_text: None,
            width: Some(img.width as f32 / 72.0),
            height: Some(img.height as f32 / 72.0),
            position: ImagePosition::Inline,
        });
    }

    Ok((elements, image_elements))
}

struct RunStyle {
    font_family: Option<String>,
    bold: bool,
    italic: bool,
    code: bool,
}

fn build_style(
    font_name: &Option<Vec<u8>>,
    font_info_map: &HashMap<Vec<u8>, FontInfo>,
) -> RunStyle {
    match font_name {
        Some(name) => {
            if let Some(info) = font_info_map.get(name) {
                RunStyle {
                    font_family: Some(info.base_font.clone()),
                    bold: info.bold,
                    italic: info.italic,
                    code: info.mono,
                }
            } else {
                let name_str = String::from_utf8_lossy(name);
                let upper = name_str.to_uppercase();
                RunStyle {
                    font_family: Some(name_str.to_string()),
                    bold: upper.contains("BOLD"),
                    italic: upper.contains("ITALIC") || upper.contains("OBLIQUE"),
                    code: upper.contains("COURIER") || upper.contains("MONO"),
                }
            }
        }
        None => RunStyle {
            font_family: None,
            bold: false,
            italic: false,
            code: false,
        },
    }
}

impl PdfDocument {
    pub fn open(path: &str) -> Result<Self, DocumentError> {
        let p = PathBuf::from(path);
        let meta = std::fs::metadata(&p)
            .map_err(|e| DocumentError::PermissionDenied(format!("{}: {}", path, e)))?;
        validate_file_size(path, meta.len())?;

        let inner = PdfDoc::load(&p)
            .map_err(|e| DocumentError::CorruptedFile(format!("invalid pdf: {}", e)))?;

        let metadata = Self::extract_metadata(&inner);
        let (page_layouts, page_ids) = Self::parse_page_headers(&inner)?;

        let doc = Arc::new(inner);
        let num_pages = page_ids.len();
        let mut page_elements: Vec<OnceLock<Vec<DocumentElement>>> = Vec::with_capacity(num_pages);
        for _ in 0..num_pages {
            page_elements.push(OnceLock::new());
        }

        let mut page_map = HashMap::new();
        for (idx, &(page_num, _)) in page_ids.iter().enumerate() {
            page_map.insert(page_num, idx);
        }

        Ok(Self {
            path: p,
            size: meta.len(),
            doc,
            page_layouts,
            page_elements,
            page_ids,
            metadata,
            page_map,
        })
    }

    fn ensure_page_content(&self, idx: usize) -> &[DocumentElement] {
        let (page_num, page_id) = self.page_ids[idx];
        self.page_elements[idx].get_or_init(|| {
            let (elements, image_elements) =
                parse_page_content(&self.doc, page_id, page_num).unwrap_or_default();
            let mut all = elements;
            for img in image_elements {
                all.push(DocumentElement::Image(img));
            }
            all
        })
    }

    fn parse_page_headers(doc: &PdfDoc) -> Result<(Vec<PageLayout>, Vec<(u32, ObjectId)>), DocumentError> {
        let mut page_layouts = Vec::new();
        let mut page_ids = Vec::new();
        let pages_obj = doc.get_pages();

        for (page_num, page_id) in &pages_obj {
            let (width, height, orientation) = parse_page_dimensions(doc, *page_id);

            page_layouts.push(PageLayout {
                number: *page_num as u32,
                elements: vec![],
                columns: 1,
                column_widths: vec![],
                page_width: Some(width),
                page_height: Some(height),
                orientation,
                margins: Margins {
                    top: 1.0,
                    bottom: 1.0,
                    left: 1.0,
                    right: 1.0,
                },
                header: None,
                footer: None,
            });
            page_ids.push((*page_num as u32, *page_id));
        }

        Ok((page_layouts, page_ids))
    }
    
    fn extract_metadata(doc: &PdfDoc) -> PdfMetadataExtended {
        let mut result = PdfMetadataExtended::default();

        let trailer = doc.trailer.clone();
        if let Ok(lopdf::Object::Reference(info_id)) = trailer.get(b"Info") {
            if let Ok(info_obj) = doc.get_object(*info_id) {
                if let Ok(info_dict) = info_obj.as_dict() {
                    result.title = Self::extract_string(info_dict, b"Title");
                    result.author = Self::extract_string(info_dict, b"Author");
                    result.subject = Self::extract_string(info_dict, b"Subject");
                    result.keywords = Self::extract_string(info_dict, b"Keywords");
                    result.creator = Self::extract_string(info_dict, b"Creator");
                    result.producer = Self::extract_string(info_dict, b"Producer");
                    result.created = Self::extract_date_string(info_dict, b"CreationDate");
                    result.modified = Self::extract_date_string(info_dict, b"ModDate");

                    if let Ok(trapped) = info_dict.get(b"Trapped") {
                        result.trapped = trapped.as_bool().ok();
                    }
                }
            }
        }

        result
    }

    fn extract_string(dict: &lopdf::Dictionary, key: &[u8]) -> Option<String> {
        dict.get(key).ok().and_then(|obj| {
            if let Ok(bytes) = obj.as_str() {
                std::str::from_utf8(bytes).ok().map(|s| s.to_string())
            } else if let Ok(name) = obj.as_name() {
                Some(String::from_utf8_lossy(name).to_string())
            } else {
                None
            }
        })
    }

    fn extract_date_string(dict: &lopdf::Dictionary, key: &[u8]) -> Option<String> {
        dict.get(key)
            .ok()
            .and_then(|obj| obj.as_str().ok())
            .map(|b| String::from_utf8_lossy(b).to_string())
    }
}

impl Document for PdfDocument {
    fn metadata(&self) -> DocumentMetadata {
        DocumentMetadata {
            path: self.path.clone(),
            format: DocumentFormat::Pdf,
            title: self.metadata.title.clone(),
            author: self.metadata.author.clone(),
            created: self
                .metadata
                .created
                .as_ref()
                .and_then(|s| parse_pdf_date(s)),
            modified: self
                .metadata
                .modified
                .as_ref()
                .and_then(|s| parse_pdf_date(s)),
            page_count: Some(self.page_layouts.len() as u32),
            size: self.size,
        }
    }



    fn search(&self, query: &str) -> Vec<Match> {
        let q = query.to_lowercase();
        let mut results = vec![];

        for (idx, layout) in self.page_layouts.iter().enumerate() {
            let elements = self.ensure_page_content(idx);
            let page_text = render_pdf_page_to_markdown(layout, elements);
            if page_text.to_lowercase().contains(&q) {
                results.push(Match {
                    selector: Selector::Page(layout.number),
                    text: page_text,
                    context: String::new(),
                    score: 1.0,
                });
            }
        }

        results
    }

    fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError> {
        match selector {
            Selector::Page(n) => {
                if let Some(&idx) = self.page_map.get(n) {
                    let elements = self.ensure_page_content(idx);
                    let md = render_pdf_page_to_markdown(&self.page_layouts[idx], elements);
                    Ok(ReadResult::Text(md))
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "page {} not found (document has {} pages)",
                        n,
                        self.page_layouts.len()
                    )))
                }
            }
            Selector::Pages(pages) => {
                let mut parts = Vec::new();
                for p in pages {
                    if let Some(&idx) = self.page_map.get(p) {
                        let elements = self.ensure_page_content(idx);
                        parts.push(render_pdf_page_to_markdown(&self.page_layouts[idx], elements));
                    }
                }
                if parts.is_empty() {
                    Err(DocumentError::InvalidSelector(
                        "none of the requested pages exist".into(),
                    ))
                } else {
                    Ok(ReadResult::Text(parts.join("\n\n---\n\n")))
                }
            }
            Selector::Slice { skip, take } => {
                let s = *skip as usize;
                let t = *take as usize;
                if s >= self.page_layouts.len() {
                    return Err(DocumentError::InvalidSelector(format!(
                        "skip {} beyond page count {}",
                        skip,
                        self.page_layouts.len()
                    )));
                }
                let end = std::cmp::min(s + t, self.page_layouts.len());
                let mut parts = Vec::new();
                for i in s..end {
                    let elements = self.ensure_page_content(i);
                    parts.push(render_pdf_page_to_markdown(&self.page_layouts[i], elements));
                }
                Ok(ReadResult::Text(parts.join("\n\n---\n\n")))
            }
            _ => Err(DocumentError::InvalidSelector(format!(
                "selector {:?} not supported for pdf documents (use Page, Pages, or Slice)",
                selector
            ))),
        }
    }
}

fn render_pdf_page_to_markdown(page: &PageLayout, elements: &[DocumentElement]) -> String {
    let mut md = String::new();
    md.push_str(&format!("<!-- Page {} -->\n\n", page.number));

    if let Some(w) = page.page_width {
        if let Some(h) = page.page_height {
            md.push_str(&format!(
                "> **Page:** {:.1}in × {:.1}in ({:?})\n\n",
                w, h, page.orientation
            ));
        }
    }

    for element in elements {
        match element {
            DocumentElement::Heading(h) => {
                let text = render_runs(&h.runs);
                md.push_str(&format!("{} {}\n\n", "#".repeat(h.level as usize), text));
            }
            DocumentElement::Paragraph(p) => {
                let text = render_runs(&p.runs);
                if !text.is_empty() {
                    md.push_str(&text);
                    md.push_str("\n\n");
                }
            }
            DocumentElement::Image(img) => {
                let label = img
                    .caption
                    .as_deref()
                    .or(img.alt_text.as_deref())
                    .unwrap_or("image");
                md.push_str(&format!("![{}]({})", label, img.path));
                if let Some(w) = img.width {
                    md.push_str(&format!(" width={:.1}in", w));
                }
                if let Some(h) = img.height {
                    md.push_str(&format!(" height={:.1}in", h));
                }
                md.push_str("\n\n");
            }
            _ => {}
        }
    }

    md
}

fn render_runs(runs: &[Run]) -> String {
    runs.iter().map(render_run).collect()
}

fn render_run(run: &Run) -> String {
    let mut text = escape_md_chars(&run.text);
    if run.code {
        text = format!("`{}`", text);
    }
    if run.strike {
        text = format!("~~{}~~", text);
    }
    if run.superscript {
        text = format!("<sup>{}</sup>", text);
    }
    if run.subscript {
        text = format!("<sub>{}</sub>", text);
    }
    if run.bold && run.italic {
        text = format!("***{}***", text);
    } else if run.bold {
        text = format!("**{}**", text);
    } else if run.italic {
        text = format!("*{}*", text);
    }
    if let Some(ref url) = run.hyperlink_url {
        text = format!("[{}]({})", text, url.replace(' ', "%20"));
    }
    text
}

fn escape_md_chars(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '*' | '_' | '`' | '[' | ']' | '(' | ')' | '#' | '~' | '|' | '!' | '>' | '-' | '+'
            | '.' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}
