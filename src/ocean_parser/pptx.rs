use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

use quick_xml::Reader;
use quick_xml::events::Event;
use zip::ZipArchive;

use crate::ocean_parser::{
    Document, DocumentError, DocumentFormat, DocumentMetadata, ImageElement, ImageFormat,
    ImagePosition, ListElement, ListItem, Match, ParagraphElement, Run,
    ReadResult, Selector, TableCell, TableElement, DocumentElement, HeadingElement,
};
use crate::ocean_parser::helpers::{validate_file_size, parse_iso8601_date};

// ── Internal model ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct SlideNotes {
    text: String,
}

/// Enhanced slide content with structured elements.
struct SlideContent {
    number: u32,
    title: Option<String>,
    elements: Vec<SlideElement>,
    notes: Option<SlideNotes>,
}

#[derive(Debug, Clone)]
struct SlideElement {
    element: DocumentElement,
}

// ── Metadata ───────────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone)]
struct PptxMetadataExtended {
    title: Option<String>,
    author: Option<String>,
    subject: Option<String>,
    keywords: Option<String>,
    description: Option<String>,
    created: Option<String>,
    modified: Option<String>,
    last_modified_by: Option<String>,
    version: Option<String>,
    category: Option<String>,
    company: Option<String>,
    presentation_format: Option<String>,
    slide_size: Option<(f32, f32)>,
}

// EMBU to inches: 1 inch = 914400 EMUs
const EMU_PER_INCH: f64 = 914400.0;

fn emu_to_inches(emu: i64) -> f32 {
    emu as f32 / EMU_PER_INCH as f32
}

// ── PptxDocument ───────────────────────────────────────────────────────────

pub struct PptxDocument {
    path: PathBuf,
    size: u64,
    slide_contents: Vec<SlideContent>,
    metadata: PptxMetadataExtended,
    slide_count: u32,
}

impl PptxDocument {
    pub fn open(path: &str) -> Result<Self, DocumentError> {
        let p = PathBuf::from(path);
        let meta = std::fs::metadata(&p)
            .map_err(|e| DocumentError::PermissionDenied(format!("{}: {}", path, e)))?;

        validate_file_size(path, meta.len())?;

        let file = std::fs::File::open(&p)
            .map_err(|e| DocumentError::PermissionDenied(format!("{}: {}", path, e)))?;

        let mut archive = ZipArchive::new(file)
            .map_err(|e| DocumentError::CorruptedFile(format!("invalid pptx: {}", e)))?;

        // Extract metadata
        let metadata = Self::extract_metadata(&mut archive)?;

        // Parse slides
        let slide_contents = Self::parse_all_slides(&mut archive)?;

        let slide_count = slide_contents.len() as u32;

        Ok(Self {
            path: p,
            size: meta.len(),
            slide_contents,
            metadata,
            slide_count,
        })
    }

    fn parse_all_slides(
        archive: &mut ZipArchive<std::fs::File>,
    ) -> Result<Vec<SlideContent>, DocumentError> {
        // Collect slide xml files
        let mut slide_files: Vec<(u32, String)> = vec![];
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).ok();
            if let Some(ref mut entry) = entry {
                let name = entry.name().to_string();
                if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                    let num_str = name
                        .trim_start_matches("ppt/slides/slide")
                        .trim_end_matches(".xml");
                    if let Ok(num) = num_str.parse::<u32>() {
                        let mut content = String::new();
                        entry.read_to_string(&mut content).ok();
                        slide_files.push((num, content));
                    }
                }
            }
        }
        slide_files.sort_by_key(|(num, _)| *num);

        // Collect rels files and notes
        let mut rels_map: HashMap<u32, HashMap<String, String>> = HashMap::new();
        let mut notes_map: HashMap<u32, SlideNotes> = HashMap::new();

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).ok();
            if let Some(ref mut entry) = entry {
                let name = entry.name().to_string();

                // Slide rels
                if name.starts_with("ppt/slides/_rels/slide") && name.ends_with(".xml.rels") {
                    let num_str = name
                        .trim_start_matches("ppt/slides/_rels/slide")
                        .trim_end_matches(".xml.rels");
                    if let Ok(num) = num_str.parse::<u32>() {
                        let mut content = String::new();
                        if entry.read_to_string(&mut content).is_ok() {
                            rels_map.insert(num, Self::parse_rels_xml(&content));
                        }
                    }
                }

                // Notes
                if name.starts_with("ppt/notesSlides/notesSlide") && name.ends_with(".xml") {
                    let num_str = name
                        .trim_start_matches("ppt/notesSlides/notesSlide")
                        .trim_end_matches(".xml");
                    if let Ok(num) = num_str.parse::<u32>() {
                        let mut content = String::new();
                        if entry.read_to_string(&mut content).is_ok() {
                            if let Some(notes) = Self::parse_notes_xml(&content) {
                                notes_map.insert(num, notes);
                            }
                        }
                    }
                }
            }
        }

        // Build image registry (name → bytes + format)
        let mut image_registry: HashMap<String, (Vec<u8>, ImageFormat)> = HashMap::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).ok();
            if let Some(ref mut entry) = entry {
                let name = entry.name().to_string();
                if name.starts_with("ppt/media/") {
                    let file_name = std::path::Path::new(&name)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let mut buf = vec![];
                    if entry.read_to_end(&mut buf).is_ok() {
                        let format = if name.ends_with(".png") {
                            ImageFormat::Png
                        } else if name.ends_with(".jpg") || name.ends_with(".jpeg") {
                            ImageFormat::Jpeg
                        } else if name.ends_with(".gif") {
                            ImageFormat::Gif
                        } else {
                            ImageFormat::Unknown
                        };
                        image_registry.insert(file_name, (buf, format));
                    }
                }
            }
        }

        // Extract notes
        // Parse each slide with its rels
        let mut slides = vec![];
        for (num, xml) in &slide_files {
            let slide_rels = rels_map.get(num).cloned().unwrap_or_default();
            let notes = notes_map.get(num).cloned();
            let slide = Self::parse_slide_xml(xml, *num, &slide_rels, &image_registry, &notes);
            slides.push(slide);
        }

        Ok(slides)
    }

    fn parse_rels_xml(xml: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag == "Relationship" || tag.contains("Relationship") {
                        let mut id = String::new();
                        let mut target = String::new();
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            if key == "Id" || key.ends_with(":Id") {
                                id = String::from_utf8_lossy(&attr.value).to_string();
                            } else if key == "Target" || key.ends_with(":Target") {
                                target = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                        if !id.is_empty() && !target.is_empty() {
                            // target is relative to ppt/slides/ so ../media/x.png → ppt/media/x.png
                            let clean = target
                                .trim_start_matches("../")
                                .to_string();
                            map.insert(id, clean);
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        map
    }

    fn parse_notes_xml(xml: &str) -> Option<SlideNotes> {
        let mut text = String::new();
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut in_text = false;
        let mut current_text = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(_)) | Ok(Event::Empty(_)) => {
                    // We track a:t for text
                }
                Ok(Event::End(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();
                    match tag.as_str() {
                        "a:t" => {
                            in_text = false;
                            if !current_text.is_empty() {
                                if !text.is_empty() {
                                    text.push(' ');
                                }
                                text.push_str(&current_text);
                                current_text.clear();
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(ref e)) => {
                    if in_text {
                        current_text.push_str(&e.decode().unwrap_or_default().to_string());
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        if !text.is_empty() {
            Some(SlideNotes { text })
        } else {
            None
        }
    }

    /// Parse a single slide XML into structured SlideContent.
    fn parse_slide_xml(
        xml: &str,
        slide_num: u32,
        rels: &HashMap<String, String>,
        image_registry: &HashMap<String, (Vec<u8>, ImageFormat)>,
        notes: &Option<SlideNotes>,
    ) -> SlideContent {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        // Parser state
        let mut in_para = false;
        let mut in_run = false;
        let mut in_text_box = false;
        let mut in_table = false;
        let mut in_tr = false;
        let mut in_tc = false;
        let mut _in_tbl_grid = false;
        let mut in_pic = false;
        let mut _in_blip = false;
        let mut _in_ph = false;
        let mut in_r_pr = false;
        let mut in_end_para_r_pr = false;
        let mut in_solid_fill = false;
        let mut _in_scheme_clr = false;

        // Per-paragraph state
        let mut current_runs: Vec<Run> = Vec::new();
        let mut run_text = String::new();
        let mut is_title_placeholder = false;
        let mut is_list_item = false;
        let mut is_ordered = false;
        let mut list_indent_level: u32 = 0;
        let mut para_has_text = false;

        // Run-level formatting
        let mut run_bold = false;
        let mut run_italic = false;
        let mut run_underline = false;
        let mut run_strike = false;
        let mut run_font_size: Option<f32> = None;
        let mut run_font_family: Option<String> = None;
        let mut run_color: Option<String> = None;
        let mut run_hyperlink: Option<String> = None;

        // Current image ref being built
        let mut current_img_r_id: Option<String> = None;
        let mut current_img_alt: Option<String> = None;
        let mut current_img_name: Option<String> = None;
        let mut current_img_cx: Option<i64> = None;
        let mut current_img_cy: Option<i64> = None;
        let mut current_img_off_x: Option<i64> = None;
        let mut current_img_off_y: Option<i64> = None;

        // Table state
        let mut table_headers: Vec<TableCell> = vec![];
        let mut table_rows: Vec<Vec<TableCell>> = vec![];
        let mut current_table_cells: Vec<TableCell> = vec![];
        let mut current_cell_text = String::new();

        // Flattened elements
        let mut elements: Vec<SlideElement> = vec![];
        let mut title: Option<String> = None;

        let mut buf = Vec::new();

        macro_rules! reset_run_fmt {
            () => {
                run_bold = false;
                run_italic = false;
                run_underline = false;
                run_strike = false;
                run_font_size = None;
                run_font_family = None;
                run_color = None;
                run_hyperlink = None;
            };
        }

        macro_rules! flush_para {
            () => {
                if !current_runs.is_empty() || para_has_text {
                    // Determine element type
                    let is_big_text = run_font_size.map_or(false, |s| s >= 24.0);
                    let is_title_text = is_title_placeholder && is_big_text;

                    let element = if is_list_item {
                        DocumentElement::List(ListElement {
                            items: vec![ListItem {
                                level: list_indent_level as i32,
                                runs: current_runs.clone(),
                                children: vec![],
                            }],
                            ordered: is_ordered,
                            start_number: None,
                        })
                    } else if is_title_text
                        || (run_font_size.map_or(false, |s| s >= 28.0)
                            && !current_runs.is_empty()
                            && current_runs[0].text.split_whitespace().count() <= 12)
                    {
                        let level = if run_font_size.map_or(false, |s| s >= 36.0) {
                            1
                        } else if run_font_size.map_or(false, |s| s >= 28.0) {
                            2
                        } else {
                            3
                        };
                        DocumentElement::Heading(HeadingElement {
                            level,
                            runs: current_runs.clone(),
                        })
                    } else {
                        DocumentElement::Paragraph(ParagraphElement {
                            runs: current_runs.clone(),
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
                        })
                    };

                    if is_title_placeholder && title.is_none() {
                        title = Some(element_to_text(&element));
                    }

                    elements.push(SlideElement { element });
                }
                current_runs.clear();
                para_has_text = false;
                is_list_item = false;
                is_ordered = false;
                list_indent_level = 0;
                is_title_placeholder = false;
                run_text.clear();
            };
        }

        macro_rules! flush_current_run {
            () => {
                if !run_text.is_empty() {
                    let text = std::mem::take(&mut run_text);
                    let hyperlink = run_hyperlink.clone();
                    current_runs.push(Run {
                        text,
                        bold: run_bold,
                        italic: run_italic,
                        underline: run_underline,
                        strike: run_strike,
                        code: false,
                        subscript: false,
                        superscript: false,
                        hyperlink_url: hyperlink,
                        font_size: run_font_size,
                        font_family: run_font_family.clone(),
                        font_color: run_color.clone(),
                        highlight_color: None,
                    });
                    para_has_text = true;
                }
            };
        }

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let lower = tag.to_lowercase();

                    match lower.as_str() {
                        // Paragraphs
                        "a:p" => {
                            flush_para!();
                            in_para = true;
                            reset_run_fmt!();
                        }
                        // Runs
                        "a:r" => {
                            flush_current_run!();
                            in_run = true;
                            reset_run_fmt!();
                        }
                        // Run properties
                        "a:rpr" | "r:rpr" if in_run || in_end_para_r_pr => {
                            in_r_pr = true;
                        }
                        // End para run properties
                        "a:endpararp" => {
                            flush_current_run!();
                            in_end_para_r_pr = true;
                            reset_run_fmt!();
                        }
                        // Text
                        "a:t" if in_run || in_end_para_r_pr || in_text_box => {}

                        // Paragraph properties
                        "a:ppr" if in_para => {}

                        // Bold
                        "a:b" if in_r_pr || in_end_para_r_pr => {
                            run_bold = true;
                        }
                        // Italic
                        "a:i" if in_r_pr || in_end_para_r_pr => {
                            run_italic = true;
                        }
                        // Underline
                        "a:u" if in_r_pr || in_end_para_r_pr => {
                            run_underline = true;
                        }
                        // Strike
                        "a:strike" if in_r_pr || in_end_para_r_pr => {
                            run_strike = true;
                        }
                        // Font size (hundredths of a point, e.g. 1800 = 18pt)
                        "a:sz" if in_r_pr || in_end_para_r_pr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                if key == "val" {
                                    if let Ok(v) = String::from_utf8_lossy(&attr.value).parse::<f32>()
                                    {
                                        run_font_size = Some(v / 100.0);
                                    }
                                }
                            }
                        }
                        // Font family (Latin)
                        "a:latin" if in_r_pr || in_end_para_r_pr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                if key == "typeface" {
                                    run_font_family =
                                        Some(String::from_utf8_lossy(&attr.value).to_string());
                                }
                            }
                        }
                        // Font family (East Asian / others — same as Latin)
                        "a:ea" | "a:cs" if in_r_pr || in_end_para_r_pr => {
                            // Also capture typeface if Latin didn't
                            if run_font_family.is_none() {
                                for attr in e.attributes().flatten() {
                                    let key =
                                        String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                    if key == "typeface" {
                                        run_font_family =
                                            Some(String::from_utf8_lossy(&attr.value).to_string());
                                    }
                                }
                            }
                        }
                        // Color — solid fill
                        "a:solidfill" if in_r_pr || in_end_para_r_pr => {
                            in_solid_fill = true;
                        }
                        // sRGB color
                        "a:srgbclr" if in_solid_fill => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                if key == "val" {
                                    run_color =
                                        Some(String::from_utf8_lossy(&attr.value).to_string());
                                }
                            }
                        }
                        "a:schemecr" if in_solid_fill => {
                                        }

                        // List bullets
                        "a:buchar" if in_para => {
                            is_list_item = true;
                            is_ordered = false;
                        }
                        "a:buautonum" if in_para => {
                            is_list_item = true;
                            is_ordered = true;
                        }
                        "a:bunone" if in_para => {
                            // explicit "no bullet" -> not a list item
                            is_list_item = false;
                        }
                        // List level
                        "a:lvl" if in_para => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                if key == "val" || key == "ival" {
                                    if let Ok(v) =
                                        String::from_utf8_lossy(&attr.value).parse::<u32>()
                                    {
                                        list_indent_level = v;
                                    }
                                }
                            }
                        }

                        // Hyperlinks on runs
                        "a:hlinkclick" | "a:hlink" if in_r_pr || in_end_para_r_pr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                if key == "r:id" || key.ends_with(":id") {
                                    if let Some(target) = rels.get(
                                        &String::from_utf8_lossy(&attr.value).to_string(),
                                    ) {
                                        run_hyperlink = Some(target.clone());
                                    }
                                } else if key == "action" {
                                    let val = String::from_utf8_lossy(&attr.value).to_string();
                                    if val == "ppaction://hlinksldjump"
                                        || val.starts_with("ppaction://")
                                    {
                                        // Internal slide links — ignore for now
                                    }
                                }
                            }
                        }

                        // Images
                        "p:pic" => {
                            in_pic = true;
                            current_img_r_id = None;
                            current_img_alt = None;
                            current_img_name = None;
                            current_img_cx = None;
                            current_img_cy = None;
                            current_img_off_x = None;
                            current_img_off_y = None;
                        }
                        "a:blip" if in_pic => {
                                for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                if key == "r:embed" || key.ends_with(":embed") {
                                    current_img_r_id =
                                        Some(String::from_utf8_lossy(&attr.value).to_string());
                                }
                            }
                        }
                        "a:xfrm" if in_pic => {
                            // Read ext (cx/cy) and off (x/y)
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                let val = String::from_utf8_lossy(&attr.value).to_string();
                                match key.as_str() {
                                    "cx" => current_img_cx = val.parse().ok(),
                                    "cy" => current_img_cy = val.parse().ok(),
                                    _ => {}
                                }
                            }
                        }
                        "a:off" if in_pic => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                let val = String::from_utf8_lossy(&attr.value).to_string();
                                match key.as_str() {
                                    "x" => current_img_off_x = val.parse().ok(),
                                    "y" => current_img_off_y = val.parse().ok(),
                                    _ => {}
                                }
                            }
                        }
                        "p:cnvpr" | "p:nvpicpr" | "p:cNvPr" if in_pic => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                let val = String::from_utf8_lossy(&attr.value).to_string();
                                match key.as_str() {
                                    "name" => current_img_name = Some(val),
                                    "descr" => {
                                        if !val.is_empty() {
                                            current_img_alt = Some(val)
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // Placeholder detection
                        "p:ph" | "p:placeholder" => {
                                for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                let val = String::from_utf8_lossy(&attr.value).to_string();
                                if key == "type" && val.to_lowercase() == "title" {
                                    is_title_placeholder = true;
                                }
                            }
                        }

                        // Text boxes
                        "p:txbody" | "p:txSp" => {
                            in_text_box = true;
                        }

                        // Tables
                        "a:tbl" => {
                            in_table = true;
                            table_headers = vec![];
                            table_rows = vec![];
                            current_table_cells = vec![];
                            current_cell_text.clear();
                        }
                        "a:tblgrid" => {
                            }
                        "a:tr" if in_table => {
                            in_tr = true;
                            current_table_cells = vec![];
                        }
                        "a:tc" if in_table => {
                            in_tc = true;
                            current_cell_text.clear();
                        }

                        // Hyperlinks on text runs (new style)
                        "a:hlinkclick" | "a:hlink" if in_run || in_end_para_r_pr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                                if key == "r:id" || key.ends_with(":id") {
                                    if let Some(target) = rels.get(
                                        &String::from_utf8_lossy(&attr.value).to_string(),
                                    ) {
                                        run_hyperlink = Some(target.clone());
                                    }
                                }
                            }
                        }

                        _ => {}
                    }
                }

                Ok(Event::End(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let lower = tag.to_lowercase();

                    match lower.as_str() {
                        "a:p" if in_para => {
                            flush_current_run!();
                            flush_para!();
                            in_para = false;
                        }
                        "a:r" if in_run => {
                            flush_current_run!();
                            in_run = false;
                        }
                        "a:endpararp" => {
                            flush_current_run!();
                            in_end_para_r_pr = false;
                        }
                        "a:rpr" | "r:rpr" => {
                            in_r_pr = false;
                        }
                        "a:solidfill" => {
                            in_solid_fill = false;
                        }
                        "a:schemecr" => {
                                        }
                        "p:ph" | "p:placeholder" => {
                            }
                        "p:txbody" | "p:txsp" => {
                            in_text_box = false;
                        }
                        "a:blip" => {
                            }

                        // Image
                        "p:pic" => {
                            if let Some(r_id) = current_img_r_id.take() {
                                let path = rels.get(&r_id).cloned();
                                let alt = current_img_alt.take();
                                let name = current_img_name.take();
                                let cx = current_img_cx.take();
                                let cy = current_img_cy.take();
                                let _x = current_img_off_x.take();
                                let _y = current_img_off_y.take();

                                // Resolve image bytes if available
                                let resolved_alt = alt.clone().or(name);
                                let (bytes, format, resolved_path) = if let Some(ref p) = path {
                                    let fname = std::path::Path::new(p)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("")
                                        .to_string();
                                    if let Some((b, f)) = image_registry.get(&fname) {
                                        (b.clone(), f.clone(), Some(format!("ppt/{}", p)))
                                    } else {
                                        (vec![], ImageFormat::Unknown, None)
                                    }
                                } else {
                                    (vec![], ImageFormat::Unknown, None)
                                };

                                if !bytes.is_empty() || resolved_path.is_some() {
                                    let caption = resolved_alt.clone();
                                    elements.push(SlideElement {
                                        element: DocumentElement::Image(ImageElement {
                                            path: resolved_path.unwrap_or_default(),
                                            bytes,
                                            format,
                                            caption,
                                            alt_text: resolved_alt,
                                            width: cx.map(emu_to_inches),
                                            height: cy.map(emu_to_inches),
                                            position: ImagePosition::Center,
                                        }),
                                    });
                                }
                            }
                            in_pic = false;
                        }

                        // Table
                        "a:tc" if in_tc => {
                            let cell_text = std::mem::take(&mut current_cell_text);
                            current_table_cells.push(TableCell {
                                text: cell_text,
                                rowspan: 1,
                                colspan: 1,
                                horizontal_alignment: None,
                                background_color: None,
                                width: None,
                            });
                            in_tc = false;
                        }
                        "a:tr" if in_tr => {
                            if !current_table_cells.is_empty() {
                                if table_headers.is_empty() {
                                    table_headers = current_table_cells.clone();
                                } else {
                                    table_rows.push(current_table_cells.clone());
                                }
                            }
                            current_table_cells = vec![];
                            in_tr = false;
                        }
                        "a:tbl" => {
                            // Only push if we have real content
                            if !table_headers.is_empty() || !table_rows.is_empty() {
                                // If only one row, treat as header only
                                let (headers, rows) = if table_headers.is_empty()
                                    && !table_rows.is_empty()
                                {
                                    // First row becomes header
                                    let h = table_rows.remove(0);
                                    (h, table_rows.clone())
                                } else {
                                    (table_headers.clone(), table_rows.clone())
                                };
                                elements.push(SlideElement {
                                    element: DocumentElement::Table(TableElement {
                                        headers,
                                        rows,
                                        column_widths: vec![],
                                        caption: None,
                                        borders: true,
                                        width: None,
                                        alignment: None,
                                    }),
                                });
                            }
                            in_table = false;
                                table_headers = vec![];
                            table_rows = vec![];
                            current_table_cells = vec![];
                            current_cell_text.clear();
                        }

                        _ => {}
                    }
                }

                Ok(Event::Text(ref e)) => {
                    let text = e.decode().unwrap_or_default().to_string();

                    if in_run || in_end_para_r_pr {
                        run_text.push_str(&text);
                    } else if in_tc {
                        current_cell_text.push_str(&text);
                    }
                }

                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        SlideContent {
            number: slide_num,
            title,
            elements,
            notes: notes.clone(),
        }
    }

    // ── Metadata extraction ──────────────────────────────────────────────

    fn extract_metadata(
        archive: &mut ZipArchive<std::fs::File>,
    ) -> Result<PptxMetadataExtended, DocumentError> {
        let mut metadata = PptxMetadataExtended::default();

        if let Ok(mut core_xml) = archive.by_name("docProps/core.xml") {
            let mut content = String::new();
            if core_xml.read_to_string(&mut content).is_ok() {
                metadata = Self::parse_core_properties(&content);
            }
        }

        if let Ok(mut app_xml) = archive.by_name("docProps/app.xml") {
            let mut content = String::new();
            if app_xml.read_to_string(&mut content).is_ok() {
                Self::parse_app_properties(&mut metadata, &content);
            }
        }

        if let Ok(mut pres_xml) = archive.by_name("ppt/presentation.xml") {
            let mut content = String::new();
            if pres_xml.read_to_string(&mut content).is_ok() {
                metadata.slide_size = Self::extract_slide_size(&content);
            }
        }

        Ok(metadata)
    }

    fn parse_core_properties(xml: &str) -> PptxMetadataExtended {
        let mut result = PptxMetadataExtended::default();
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut current_tag = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    current_tag = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();
                }
                Ok(Event::Text(ref e)) => {
                    let text = e.decode().unwrap_or_default().to_string();
                    match current_tag.as_str() {
                        "dc:title" => result.title = Some(text),
                        "dc:creator" | "dc:author" => result.author = Some(text),
                        "dc:subject" => result.subject = Some(text),
                        "keywords" => result.keywords = Some(text),
                        "dc:description" => result.description = Some(text),
                        "dcterms:created" => result.created = Some(text),
                        "dcterms:modified" => result.modified = Some(text),
                        "lastmodifiedby" => result.last_modified_by = Some(text),
                        "category" => result.category = Some(text),
                        "company" => result.company = Some(text),
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        result
    }

    fn parse_app_properties(metadata: &mut PptxMetadataExtended, xml: &str) {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut current_tag = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    current_tag = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();
                }
                Ok(Event::Text(ref e)) => {
                    let text = e.decode().unwrap_or_default().to_string();
                    match current_tag.as_str() {
                        "appversion" => metadata.version = Some(text),
                        "presentationformat" => metadata.presentation_format = Some(text),
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }
    }

    fn extract_slide_size(xml: &str) -> Option<(f32, f32)> {
        let mut reader = Reader::from_str(xml);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let lower = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();
                    if lower == "p:sldsz" || lower == "sldsz" {
                        let mut cx: Option<f32> = None;
                        let mut cy: Option<f32> = None;
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                            let val = String::from_utf8_lossy(&attr.value).to_string();
                            match key.as_str() {
                                "cx" => cx = val.parse().ok(),
                                "cy" => cy = val.parse().ok(),
                                _ => {}
                            }
                        }
                        if let (Some(w), Some(h)) = (cx, cy) {
                            return Some((w, h));
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        None
    }
}

// ── Rendering helpers ──────────────────────────────────────────────────────

fn element_to_text(element: &DocumentElement) -> String {
    match element {
        DocumentElement::Paragraph(p) => render_runs(&p.runs),
        DocumentElement::Heading(h) => render_runs(&h.runs),
        DocumentElement::List(l) => {
            let mut text = String::new();
            for item in &l.items {
                text.push_str(&render_runs(&item.runs));
                text.push(' ');
            }
            text
        }
        DocumentElement::Image(img) => {
            img.caption.clone().unwrap_or_else(|| "image".to_string())
        }
        DocumentElement::Table(t) => {
            let mut text = String::new();
            for cell in &t.headers {
                text.push_str(&cell.text);
                text.push(' ');
            }
            for row in &t.rows {
                for cell in row {
                    text.push_str(&cell.text);
                    text.push(' ');
                }
            }
            text
        }
        _ => String::new(),
    }
}

fn render_slide_to_markdown(slide: &SlideContent) -> String {
    let mut md = String::new();

    // Slide header
    if let Some(ref title) = slide.title {
        md.push_str(&format!("# {}\n\n", title));
    } else {
        md.push_str(&format!("# Slide {}\n\n", slide.number));
    }

    // Elements
    for se in &slide.elements {
        md.push_str(&render_element(&se.element));
    }

    // Notes
    if let Some(ref notes) = slide.notes {
        if !notes.text.is_empty() {
            md.push_str("\n> **Notes:** ");
            md.push_str(&notes.text);
            md.push('\n');
        }
    }

    md.trim().to_string()
}

fn render_element(element: &DocumentElement) -> String {
    match element {
        DocumentElement::Paragraph(para) => {
            let text = render_runs(&para.runs);
            if text.is_empty() {
                String::new()
            } else {
                format!("{}\n\n", text)
            }
        }
        DocumentElement::Heading(heading) => {
            let level = heading.level.min(6).max(1);
            let text = render_runs(&heading.runs);
            if text.is_empty() {
                String::new()
            } else {
                format!("{} {}\n\n", "#".repeat(level as usize), text)
            }
        }
        DocumentElement::List(list) => {
            let mut md = String::new();
            for item in &list.items {
                let prefix = if list.ordered {
                    format!("{}. ", 1)
                } else {
                    "- ".to_string()
                };
                let indent = "  ".repeat(item.level.max(0) as usize);
                let text = render_runs(&item.runs);
                if !text.is_empty() {
                    md.push_str(&format!("{}{}{}\n", indent, prefix, text));
                }
                // render children (not commonly used in PPTX but supported)
                for child in &item.children {
                    let child_prefix = if list.ordered {
                        format!("  {}. ", 1)
                    } else {
                        "  - ".to_string()
                    };
                    let child_text = render_runs(&child.runs);
                    if !child_text.is_empty() {
                        md.push_str(&format!("{}  {}{}\n", indent, child_prefix, child_text));
                    }
                }
            }
            md.push('\n');
            md
        }
        DocumentElement::Image(img) => {
            let alt = img
                .alt_text
                .clone()
                .or_else(|| img.caption.clone())
                .unwrap_or_else(|| "image".to_string());
            let src = if img.path.is_empty() {
                if img.bytes.is_empty() {
                    "unknown".to_string()
                } else {
                    format!("inline-{}", alt.to_lowercase().replace(' ', "-"))
                }
            } else {
                img.path.clone()
            };
            format!("![{}]({})\n\n", alt, src)
        }
        DocumentElement::Table(table) => {
            let mut md = String::new();
            // Header row
            if !table.headers.is_empty() {
                let cells: Vec<&str> = table.headers.iter().map(|c| c.text.as_str()).collect();
                md.push_str(&format!("| {} |\n", cells.join(" | ")));
                md.push_str(&format!("| {} |\n", vec!["---"; table.headers.len()].join(" | ")));
            }
            // Data rows
            for row in &table.rows {
                let cells: Vec<&str> = row.iter().map(|c| c.text.as_str()).collect();
                md.push_str(&format!("| {} |\n", cells.join(" | ")));
            }
            md.push('\n');
            md
        }
        DocumentElement::TextBox(tb) => {
            let text = render_runs(&tb.runs);
            if text.is_empty() {
                String::new()
            } else {
                format!("> {}\n\n", text)
            }
        }
        DocumentElement::HorizontalRule => "---\n\n".to_string(),
        DocumentElement::PageBreak => "\n---\n\n".to_string(),
        DocumentElement::SectionBreak { .. } => "\n---\n\n".to_string(),
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

    // Apply bold+italic in combination
    if run.bold && run.italic {
        text = format!("***{}***", text);
    } else if run.bold {
        text = format!("**{}**", text);
    } else if run.italic {
        text = format!("*{}*", text);
    }

    // Underline — Markdown doesn't have native underline; use HTML
    if run.underline {
        text = format!("<u>{}</u>", text);
    }

    if let Some(ref url) = run.hyperlink_url {
        let display = text;
        text = format!("[{}]({})", display, url.replace(' ', "%20"));
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

// ── Document trait ─────────────────────────────────────────────────────────

impl Document for PptxDocument {
    fn metadata(&self) -> DocumentMetadata {
        DocumentMetadata {
            path: self.path.clone(),
            format: DocumentFormat::Pptx,
            title: self.metadata.title.clone(),
            author: self.metadata.author.clone(),
            created: self.metadata.created.as_deref().and_then(parse_iso8601_date),
            modified: self.metadata.modified.as_deref().and_then(parse_iso8601_date),
            page_count: Some(self.slide_count),
            size: self.size,
        }
    }

    fn search(&self, query: &str) -> Vec<Match> {
        let q = query.to_lowercase();
        let mut results = vec![];

        for slide in &self.slide_contents {
            // Build full text from elements
            let full_text: String = slide
                .elements
                .iter()
                .map(|e| element_to_text(&e.element))
                .collect::<Vec<_>>()
                .join(" ");
            let full_text_lower = full_text.to_lowercase();

            let mut matched = false;
            let mut context = String::new();

            if let Some(ref title) = slide.title {
                if title.to_lowercase().contains(&q) {
                    matched = true;
                    context.push_str("[Title]");
                }
            }

            if full_text_lower.contains(&q) {
                matched = true;
                if !context.is_empty() {
                    context.push_str(" | ");
                }
                context.push_str("[Content]");
            }

            if let Some(ref notes) = slide.notes {
                if notes.text.to_lowercase().contains(&q) {
                    matched = true;
                    if !context.is_empty() {
                        context.push_str(" | ");
                    }
                    context.push_str("[Notes]");
                }
            }

            if matched {
                results.push(Match {
                    selector: Selector::Slide(slide.number),
                    text: full_text,
                    context: if context.is_empty() {
                        slide.title.clone().unwrap_or_default()
                    } else {
                        context
                    },
                    score: 1.0,
                });
            }
        }

        results
    }

    fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError> {
        match selector {
            Selector::Slide(n) => {
                if let Some(slide) = self.slide_contents.iter().find(|s| s.number == *n) {
                    let md = render_slide_to_markdown(slide);
                    let title = slide.title.clone();
                    Ok(ReadResult::Slide {
                        number: slide.number,
                        title,
                        content: md,
                    })
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "slide {} not found",
                        n
                    )))
                }
            }
            Selector::Slides(slides) => {
                let mut parts = Vec::new();
                for n in slides {
                    if let Some(slide) = self.slide_contents.iter().find(|s| s.number == *n) {
                        parts.push(render_slide_to_markdown(slide));
                    } else {
                        return Err(DocumentError::InvalidSelector(format!(
                            "slide {} not found",
                            n
                        )));
                    }
                }
                Ok(ReadResult::Text(parts.join("\n\n---\n\n")))
            }
            Selector::Slice { skip, take } => {
                let s = *skip as usize;
                let t = *take as usize;
                if s >= self.slide_contents.len() {
                    return Err(DocumentError::InvalidSelector(format!(
                        "skip {} beyond slide count {}",
                        skip,
                        self.slide_contents.len()
                    )));
                }
                let end = std::cmp::min(s + t, self.slide_contents.len());
                let md_parts: Vec<String> = self.slide_contents[s..end]
                    .iter()
                    .map(render_slide_to_markdown)
                    .collect();
                Ok(ReadResult::Text(md_parts.join("\n\n---\n\n")))
            }
            _ => Err(DocumentError::InvalidSelector(format!(
                "selector {:?} not supported for pptx documents",
                selector
            ))),
        }
    }
}
