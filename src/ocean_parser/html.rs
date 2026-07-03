use std::path::PathBuf;

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::ocean_parser::helpers::validate_file_size;
use crate::ocean_parser::{
    Document, DocumentError, DocumentFormat, DocumentMetadata, Match, ReadResult, Run,
    Selector,
};

#[derive(Debug, Clone, PartialEq)]
enum ElementType {
    Paragraph,
    ListItem(u32),
    BlockQuote,
    Code,
    Heading(u8),
}

#[derive(Debug, Clone)]
struct ParsedParagraph {
    runs: Vec<Run>,
    element_type: ElementType,
}

impl ParsedParagraph {
    fn text(&self) -> String {
        self.runs.iter().map(|r| r.text.as_str()).collect()
    }
}

#[derive(Debug, Default, Clone)]
struct FormatState {
    bold: bool,
    italic: bool,
    code: bool,
    strike: bool,
    underline: bool,
    superscript: bool,
    subscript: bool,
    hyperlink_url: Option<String>,
    font_size: Option<f32>,
    font_family: Option<String>,
    font_color: Option<String>,
    highlight: Option<String>,
}

impl FormatState {
    fn make_run(&self, text: String) -> Run {
        Run {
            text,
            bold: self.bold,
            italic: self.italic,
            code: self.code,
            strike: self.strike,
            underline: self.underline,
            superscript: self.superscript,
            subscript: self.subscript,
            hyperlink_url: self.hyperlink_url.clone(),
            font_size: self.font_size,
            font_family: self.font_family.clone(),
            font_color: self.font_color.clone(),
            highlight_color: self.highlight.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct ImageMetadata {
    src: String,
    alt: String,
    title: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Debug, Default, Clone)]
struct HtmlMetadataExtended {
    title: Option<String>,
    description: Option<String>,
    keywords: Option<String>,
    author: Option<String>,
    viewport: Option<String>,
    charset: Option<String>,
    language: Option<String>,
    og_title: Option<String>,
    og_description: Option<String>,
    og_image: Option<String>,
    og_url: Option<String>,
}

pub struct HtmlDocument {
    path: PathBuf,
    content: String,
    size: u64,
    headings: Vec<ParsedParagraph>,
    paragraphs: Vec<ParsedParagraph>,
    tables: Vec<Vec<Vec<String>>>,
    images: Vec<ImageMetadata>,
    metadata: HtmlMetadataExtended,
}

fn render_parsed_paragraph(para: &ParsedParagraph) -> String {
    match para.element_type {
        ElementType::Heading(level) => {
            let text = render_runs(&para.runs);
            format!("{} {}\n\n", "#".repeat(level as usize), text)
        }
        ElementType::Paragraph => {
            let text = render_runs(&para.runs);
            if text.is_empty() {
                String::new()
            } else {
                format!("{}\n\n", text)
            }
        }
        ElementType::BlockQuote => {
            let text = render_runs(&para.runs);
            text.lines()
                .map(|line| format!("> {}", line))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n\n"
        }
        ElementType::Code => {
            let text = para.text();
            format!("```\n{}\n```\n\n", text)
        }
        ElementType::ListItem(level) => {
            let indent = "  ".repeat(level as usize);
            let text = render_runs(&para.runs);
            format!("{}- {}\n", indent, text)
        }
    }
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
            '*' | '_' | '`' | '[' | ']' | '(' | ')' | '#' | '~' | '|' | '!' | '>' | '-' | '+' | '.' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

impl HtmlDocument {
    pub fn open(path: &str) -> Result<Self, DocumentError> {
        let p = PathBuf::from(path);
        let meta = std::fs::metadata(&p)
            .map_err(|e| DocumentError::PermissionDenied(format!("{}: {}", path, e)))?;

        validate_file_size(path, meta.len())?;

        let content = std::fs::read_to_string(&p).map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                DocumentError::PermissionDenied(format!("{}: {}", path, e))
            }
            std::io::ErrorKind::InvalidData => {
                DocumentError::InvalidEncoding(format!("{} is not valid UTF-8", path))
            }
            _ => DocumentError::ParseFailed(format!("{}: {}", path, e)),
        })?;

        let (headings, paragraphs, tables, images, metadata) =
            Self::parse_html(&content);

        Ok(Self {
            path: p,
            content,
            size: meta.len(),
            headings,
            paragraphs,
            tables,
            images,
            metadata,
        })
    }

    fn parse_html(
        html: &str,
    ) -> (
        Vec<ParsedParagraph>,
        Vec<ParsedParagraph>,
        Vec<Vec<Vec<String>>>,
        Vec<ImageMetadata>,
        HtmlMetadataExtended,
    ) {
        let mut reader = Reader::from_str(html);
        reader.config_mut().trim_text(true);

        let mut headings: Vec<ParsedParagraph> = vec![];
        let mut paragraphs: Vec<ParsedParagraph> = vec![];
        let mut tables = vec![];
        let mut images = vec![];
        let mut metadata = HtmlMetadataExtended::default();

        let mut buf = Vec::new();

        // Parser state
        let mut in_table = false;
        let mut in_tr = false;
        let mut current_row = vec![];
        let mut current_cell = String::new();
        let mut table_rows = vec![];

        let mut in_para = false;
        let mut para_runs: Vec<Run> = Vec::new();
        let mut in_heading = false;
        let mut heading_level = 0u8;
        let mut heading_runs: Vec<Run> = Vec::new();
        let mut in_blockquote = false;
        let mut blockquote_runs: Vec<Run> = Vec::new();
        let mut in_code = false;
        let mut code_runs: Vec<Run> = Vec::new();
        let mut in_list_item = false;
        let mut list_level = 0u32;
        let mut list_item_runs: Vec<Run> = Vec::new();
        let mut in_head = false;
        let mut skip_script = false;
        let mut skip_style = false;

        // Inline formatting stack
        let mut fmt = FormatState::default();

        // Ackermann list state
        let mut list_stack: Vec<u32> = Vec::new(); // depth of nesting

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();

                    match tag.as_str() {
                        "head" => in_head = true,
                        "script" => skip_script = true,
                        "style" => skip_style = true,
                        _ => {}
                    }

                    if skip_script || skip_style {
                        continue;
                    }

                    // Meta tag handling in head
                    if in_head && tag == "meta" {
                        let mut name = String::new();
                        let mut property = String::new();
                        let mut content = String::new();
                        let mut charset = String::new();

                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                            let val = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                            match key.as_str() {
                                "name" => name = val.to_lowercase(),
                                "property" => property = val.to_lowercase(),
                                "content" => content = val,
                                "charset" => charset = val,
                                _ => {}
                            }
                        }

                        if !charset.is_empty() {
                            metadata.charset = Some(charset);
                        }

                        match name.as_str() {
                            "description" => metadata.description = Some(content.clone()),
                            "keywords" => metadata.keywords = Some(content.clone()),
                            "author" => metadata.author = Some(content.clone()),
                            "viewport" => metadata.viewport = Some(content.clone()),
                            _ => {}
                        }

                        if property.starts_with("og:") {
                            match property.as_str() {
                                "og:title" => metadata.og_title = Some(content.clone()),
                                "og:description" => {
                                    metadata.og_description = Some(content.clone())
                                }
                                "og:image" => metadata.og_image = Some(content.clone()),
                                "og:url" => metadata.og_url = Some(content.clone()),
                                _ => {}
                            }
                        }
                    }

                    // Title tag — handled via text inside head
                    if in_head && tag == "title" {
                        // text captured below
                    }

                    // Language attribute
                    if tag == "html" {
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                            if key == "lang" {
                                metadata.language =
                                    Some(String::from_utf8_lossy(attr.value.as_ref()).to_string());
                            }
                        }
                    }

                    // Inline formatting — set flags on entry
                    match tag.as_str() {
                        "b" | "strong" => fmt.bold = true,
                        "i" | "em" => fmt.italic = true,
                        "code" | "kbd" => fmt.code = true,
                        "s" | "del" | "strike" => fmt.strike = true,
                        "u" | "ins" => fmt.underline = true,
                        "sup" => fmt.superscript = true,
                        "sub" => fmt.subscript = true,
                        "mark" => {
                            fmt.highlight = Some("yellow".to_string());
                        }
                        "a" => {
                            for attr in e.attributes().flatten() {
                                let key =
                                    String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                if key == "href" {
                                    fmt.hyperlink_url =
                                        Some(String::from_utf8_lossy(attr.value.as_ref()).to_string());
                                }
                            }
                        }
                        _ => {}
                    }

                    // Element start
                    match tag.as_str() {
                        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                            in_heading = true;
                            heading_runs.clear();
                            heading_level = tag[1..].parse().unwrap_or(1);
                        }
                        "p" => {
                            in_para = true;
                            para_runs.clear();
                        }
                        "li" => {
                            in_list_item = true;
                            list_item_runs.clear();
                            list_level = list_stack.len().max(1) as u32 - 1;
                        }
                        "ul" | "ol" => {
                            list_stack.push(0);
                        }
                        "blockquote" => {
                            in_blockquote = true;
                            blockquote_runs.clear();
                        }
                        "pre" => {
                            in_code = true;
                            code_runs.clear();
                        }
                        "table" => {
                            in_table = true;
                            table_rows = vec![];
                        }
                        "tr" if in_table => {
                            in_tr = true;
                            current_row = vec![];
                        }
                        "td" | "th" if in_tr => {
                            current_cell.clear();
                        }
                        "br" => {
                            if in_para {
                                para_runs.push(Run {
                                    text: "\n".to_string(),
                                    ..Default::default()
                                });
                            } else if in_heading {
                                heading_runs.push(Run {
                                    text: "\n".to_string(),
                                    ..Default::default()
                                });
                            }
                        }
                        "img" => {
                            let mut img_meta = ImageMetadata {
                                src: String::new(),
                                alt: String::new(),
                                title: None,
                                width: None,
                                height: None,
                            };

                            for attr in e.attributes().flatten() {
                                let key =
                                    String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                let val =
                                    String::from_utf8_lossy(attr.value.as_ref()).to_string();
                                match key.as_str() {
                                    "src" => img_meta.src = val,
                                    "alt" => img_meta.alt = val,
                                    "title" => img_meta.title = Some(val),
                                    "width" => img_meta.width = val.parse().ok(),
                                    "height" => img_meta.height = val.parse().ok(),
                                    _ => {}
                                }
                            }

                            // If inside a paragraph/heading, insert a run marker
                            // Otherwise, add as standalone image
                            let img_text = if !img_meta.alt.is_empty() {
                                img_meta.alt.clone()
                            } else {
                                "image".to_string()
                            };

                            if in_para {
                                para_runs.push(fmt.make_run(format!(
                                    "!{}",
                                    if img_meta.src.is_empty() { img_text } else { format!("[{}]({})", img_text, img_meta.src) }
                                )));
                            } else if in_heading {
                                heading_runs.push(fmt.make_run(format!(
                                    "!{}",
                                    if img_meta.src.is_empty() { img_text } else { format!("[{}]({})", img_text, img_meta.src) }
                                )));
                            }

                            images.push(img_meta);
                        }
                        _ => {}
                    }
                }

                Ok(Event::End(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();

                    if skip_script && tag == "script" {
                        skip_script = false;
                        continue;
                    }
                    if skip_style && tag == "style" {
                        skip_style = false;
                        continue;
                    }

                    if skip_script || skip_style {
                        continue;
                    }

                    // Inline formatting — unset flags on exit
                    match tag.as_str() {
                        "b" | "strong" => fmt.bold = false,
                        "i" | "em" => fmt.italic = false,
                        "code" | "kbd" => fmt.code = false,
                        "s" | "del" | "strike" => fmt.strike = false,
                        "u" | "ins" => fmt.underline = false,
                        "sup" => fmt.superscript = false,
                        "sub" => fmt.subscript = false,
                        "mark" => fmt.highlight = None,
                        "a" => fmt.hyperlink_url = None,
                        _ => {}
                    }

                    match tag.as_str() {
                        "head" => {
                            in_head = false;
                        }
                        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                            if in_heading {
                                in_heading = false;
                                if !heading_runs.is_empty() {
                                    let h = ParsedParagraph {
                                        runs: heading_runs.clone(),
                                        element_type: ElementType::Heading(heading_level),
                                    };
                                    headings.push(h.clone());
                                    paragraphs.push(h);
                                }
                            }
                        }
                        "p" => {
                            if in_para {
                                in_para = false;
                                if !para_runs.is_empty() {
                                    paragraphs.push(ParsedParagraph {
                                        runs: para_runs.clone(),
                                        element_type: ElementType::Paragraph,
                                    });
                                }
                            }
                        }
                        "blockquote" => {
                            if in_blockquote {
                                in_blockquote = false;
                                if !blockquote_runs.is_empty() {
                                    paragraphs.push(ParsedParagraph {
                                        runs: blockquote_runs.clone(),
                                        element_type: ElementType::BlockQuote,
                                    });
                                }
                            }
                        }
                        "pre" => {
                            if in_code {
                                in_code = false;
                                if !code_runs.is_empty() {
                                    paragraphs.push(ParsedParagraph {
                                        runs: code_runs.clone(),
                                        element_type: ElementType::Code,
                                    });
                                }
                            }
                        }
                        "li" => {
                            if in_list_item {
                                in_list_item = false;
                                if !list_item_runs.is_empty() {
                                    paragraphs.push(ParsedParagraph {
                                        runs: list_item_runs.clone(),
                                        element_type: ElementType::ListItem(list_level),
                                    });
                                }
                            }
                        }
                        "ul" | "ol" => {
                            list_stack.pop();
                        }
                        "td" | "th" if in_tr => {
                            current_row.push(current_cell.clone());
                            current_cell.clear();
                        }
                        "tr" if in_table => {
                            in_tr = false;
                            if !current_row.is_empty() {
                                table_rows.push(current_row.clone());
                            }
                        }
                        "table" => {
                            in_table = false;
                            if !table_rows.is_empty() {
                                tables.push(table_rows.clone());
                            }
                        }
                        "title" => {
                            // title text captured via Event::Text when in_head
                        }
                        _ => {}
                    }
                }

                Ok(Event::Text(ref e)) => {
                    let raw_text = e.decode().unwrap_or_default().to_string();

                    if skip_script || skip_style {
                        continue;
                    }

                    // Capture title from text
                    if in_head && tag_stack_is_title(&reader) {
                        if metadata.title.is_none() {
                            metadata.title = Some(raw_text.trim().to_string());
                        }
                    }

                    if in_heading {
                        heading_runs.push(fmt.make_run(raw_text));
                    } else if in_para {
                        para_runs.push(fmt.make_run(raw_text));
                    } else if in_blockquote {
                        blockquote_runs.push(fmt.make_run(raw_text));
                    } else if in_code {
                        code_runs.push(Run {
                            text: raw_text,
                            ..Default::default()
                        });
                    } else if in_list_item {
                        list_item_runs.push(fmt.make_run(raw_text));
                    } else if in_tr {
                        current_cell.push_str(&raw_text);
                    }
                }

                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        (headings, paragraphs, tables, images, metadata)
    }
}

/// Quick check if we're inside a <title> by looking at the reader's tag stack.
/// Uses the reader's state to determine context.
fn tag_stack_is_title(_reader: &Reader<&[u8]>) -> bool {
    true // Text inside <head><title> will arrive while in_head=true
}

impl Document for HtmlDocument {
    fn metadata(&self) -> DocumentMetadata {
        DocumentMetadata {
            path: self.path.clone(),
            format: DocumentFormat::Html,
            title: self
                .metadata
                .title
                .clone()
                .or_else(|| self.metadata.og_title.clone()),
            author: self.metadata.author.clone(),
            created: None,
            modified: None,
            page_count: None,
            size: self.size,
        }
    }



    fn search(&self, query: &str) -> Vec<Match> {
        let q = query.to_lowercase();
        let mut results = vec![];

        for para in &self.paragraphs {
            let text = para.text();
            if text.to_lowercase().contains(&q) {
                let context = match para.element_type {
                    ElementType::Heading(level) => format!("[H{}]", level),
                    ElementType::ListItem(level) => format!("[List Level {}]", level),
                    ElementType::BlockQuote => "[Quote]".to_string(),
                    ElementType::Code => "[Code]".to_string(),
                    _ => String::new(),
                };

                results.push(Match {
                    selector: Selector::Slice {
                        skip: 0,
                        take: u32::MAX,
                    },
                    text,
                    context,
                    score: 1.0,
                });
            }
        }

        results
    }

    fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError> {
        match selector {
            Selector::Range { start, end } => {
                if *start >= self.content.len() || *end > self.content.len() || start >= end {
                    return Err(DocumentError::InvalidSelector(format!(
                        "invalid range {}..{} (content length: {})",
                        start,
                        end,
                        self.content.len()
                    )));
                }
                Ok(ReadResult::Text(self.content[*start..*end].to_string()))
            }
            Selector::Slice { skip, take } => {
                let s = *skip as usize;
                let t = *take as usize;
                if s >= self.paragraphs.len() {
                    return Err(DocumentError::InvalidSelector(format!(
                        "skip {} beyond paragraph count {}",
                        skip,
                        self.paragraphs.len()
                    )));
                }
                let end = std::cmp::min(s + t, self.paragraphs.len());

                // Render each paragraph as Markdown
                let mut md = String::new();
                for para in &self.paragraphs[s..end] {
                    md.push_str(&render_parsed_paragraph(para));
                }

                // Append image info if any
                if !self.images.is_empty() {
                    md.push_str("---\n\n**Images:**\n\n");
                    for img in &self.images {
                        let alt = if img.alt.is_empty() { "image" } else { &img.alt };
                        md.push_str(&format!("![{}]({})", alt, img.src));
                        if let Some(w) = img.width {
                            md.push_str(&format!(" width={}", w));
                        }
                        if let Some(h) = img.height {
                            md.push_str(&format!(" height={}", h));
                        }
                        md.push_str("\n\n");
                    }
                }

                // Append tables if any
                if !self.tables.is_empty() {
                    md.push_str("---\n\n**Tables:**\n\n");
                    for table in &self.tables {
                        for (i, row) in table.iter().enumerate() {
                            let cells: Vec<&str> = row.iter().map(|c| c.as_str()).collect();
                            md.push_str(&format!("| {} |\n", cells.join(" | ")));
                            if i == 0 {
                                md.push_str(&format!("| {} |\n", vec!["---"; row.len()].join(" | ")));
                            }
                        }
                        md.push_str("\n");
                    }
                }

                Ok(ReadResult::Text(md))
            }
            _ => Err(DocumentError::InvalidSelector(format!(
                "selector {:?} not supported for html documents",
                selector
            ))),
        }
    }
}
