use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

use quick_xml::Reader;
use quick_xml::events::Event;
use zip::ZipArchive;

use crate::ocean_parser::helpers::{validate_file_size, parse_iso8601_date, build_outline_tree_from_selectors};
use crate::ocean_parser::{
    Document, DocumentError, DocumentFormat, DocumentMetadata, Match, Outline,
    ReadResult, Selector, DocumentElement, ParagraphElement, Run, TableElement, TableCell,
    ImageElement, ImagePosition, ImageFormat, HeadingElement, ListElement, ListItem,
    PageLayout, PageOrientation, Margins, TextAlignment,
};

struct Paragraph {
    text: String,
    style: Option<String>,
    #[allow(dead_code)]
    page_break: bool, // kept for backward compat
}

pub struct DocxDocument {
    path: PathBuf,
    size: u64,
    paragraphs: Vec<Paragraph>,
    pages: Vec<PageLayout>,
    para_to_page: Vec<usize>,
    metadata: DocumentMetadataExtended,
}

#[derive(Debug, Default, Clone)]
struct DocumentMetadataExtended {
    title: Option<String>,
    author: Option<String>,
    subject: Option<String>,
    keywords: Option<String>,
    created: Option<String>,
    modified: Option<String>,
    content_status: Option<String>,
}

impl DocxDocument {
    pub fn open(path: &str) -> Result<Self, DocumentError> {
        let p = PathBuf::from(path);
        let meta = std::fs::metadata(&p)
            .map_err(|e| DocumentError::PermissionDenied(format!("{}: {}", path, e)))?;

        validate_file_size(path, meta.len())?;

        let file = std::fs::File::open(&p)
            .map_err(|e| DocumentError::PermissionDenied(format!("{}: {}", path, e)))?;

        let mut archive = ZipArchive::new(file)
            .map_err(|e| DocumentError::CorruptedFile(format!("invalid docx: {}", e)))?;

        let mut metadata = DocumentMetadataExtended::default();
        if let Ok(mut core_xml) = archive.by_name("docProps/core.xml") {
            let mut xml_content = String::new();
            if core_xml.read_to_string(&mut xml_content).is_ok() {
                metadata = Self::parse_core_properties(&xml_content);
            }
        }

        let rels_map = Self::parse_rels(&mut archive);
        let num_format_map = Self::parse_numbering(&mut archive);
        let images_map = Self::extract_images(&mut archive);

        let mut paragraphs = vec![];
        let mut pages = vec![];
        let mut para_to_page = vec![];

        if let Ok(mut doc_xml) = archive.by_name("word/document.xml") {
            let mut xml_content = String::new();
            if doc_xml.read_to_string(&mut xml_content).is_ok() {
                (pages, paragraphs, para_to_page) = Self::parse_document_xml(
                    &xml_content,
                    &rels_map,
                    &num_format_map,
                    &images_map,
                );
            }
        }

        Ok(Self {
            path: p,
            size: meta.len(),
            paragraphs,
            pages,
            para_to_page,
            metadata,
        })
    }

    fn parse_core_properties(xml: &str) -> DocumentMetadataExtended {
        let mut result = DocumentMetadataExtended::default();
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();
        let mut current_tag = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    current_tag =
                        String::from_utf8_lossy(e.name().as_ref()).to_lowercase();
                }
                Ok(Event::Text(ref e)) => {
                    let text = e.decode().unwrap_or_default().to_string();
                    match current_tag.as_str() {
                        "dc:title" => result.title = Some(text),
                        "dc:creator" => result.author = Some(text),
                        "dc:subject" => result.subject = Some(text),
                        "keywords" => result.keywords = Some(text),
                        "dcterms:created" => result.created = Some(text),
                        "dcterms:modified" => result.modified = Some(text),
                        "contentstatus" => result.content_status = Some(text),
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

    fn parse_rels(archive: &mut ZipArchive<std::fs::File>) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Ok(mut rels_xml) = archive.by_name("word/_rels/document.xml.rels") {
            let mut xml_content = String::new();
            if rels_xml.read_to_string(&mut xml_content).is_ok() {
                let mut reader = Reader::from_str(&xml_content);
                reader.config_mut().trim_text(true);
                let mut buf = Vec::new();
                loop {
                    match reader.read_event_into(&mut buf) {
                        Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) => {
                            let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                            if tag.contains("Relationship") || tag == "Relationship" {
                                let mut id = String::new();
                                let mut target = String::new();
                                for attr in e.attributes().flatten() {
                                    let key = String::from_utf8_lossy(attr.key.as_ref());
                                    if key.contains("Id") {
                                        id = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0).unwrap_or_default().to_string();
                                    } else if key.contains("Target") {
                                        target =
                                            attr.normalized_value(quick_xml::XmlVersion::Implicit1_0).unwrap_or_default().to_string();
                                    }
                                }
                                if !id.is_empty() {
                                    map.insert(id, target);
                                }
                            }
                        }
                        Ok(Event::Eof) => break,
                        Err(_) => break,
                        _ => {}
                    }
                    buf.clear();
                }
            }
        }
        map
    }

    fn parse_numbering(archive: &mut ZipArchive<std::fs::File>) -> HashMap<u32, bool> {
        let mut num_id_to_is_bullet: HashMap<u32, bool> = HashMap::new();
        let mut abs_num_id_to_is_bullet: HashMap<u32, bool> = HashMap::new();
        let mut abs_num_id_to_has_levels: HashMap<u32, bool> = HashMap::new();

        if let Ok(mut num_xml) = archive.by_name("word/numbering.xml") {
            let mut xml_content = String::new();
            if num_xml.read_to_string(&mut xml_content).is_ok() {
                let mut reader = Reader::from_str(&xml_content);
                reader.config_mut().trim_text(true);
                let mut buf = Vec::new();

                let mut in_abstract_num = false;
                let mut in_num = false;
                let mut in_level = false;
                let mut current_abs_num_id: Option<u32> = None;
                let mut current_num_id: Option<u32> = None;
                let mut level_num_fmt: Option<String> = None;

                loop {
                    match reader.read_event_into(&mut buf) {
                        Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                            let tag =
                                String::from_utf8_lossy(e.name().as_ref()).to_lowercase();

                            match tag.as_str() {
                                "w:abstractnum" => {
                                    in_abstract_num = true;
                                    current_abs_num_id = None;
                                    for attr in e.attributes().flatten() {
                                        let key =
                                            String::from_utf8_lossy(attr.key.as_ref());
                                        if key.contains("AbstractNumId") {
                                            current_abs_num_id = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                                .ok()
                                                .and_then(|v| v.parse::<u32>().ok());
                                        }
                                    }
                                }
                                "w:num" => {
                                    in_num = true;
                                    current_num_id = None;
                                    for attr in e.attributes().flatten() {
                                        let key =
                                            String::from_utf8_lossy(attr.key.as_ref());
                                        if key.contains("NumId") {
                                            current_num_id = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                                .ok()
                                                .and_then(|v| v.parse::<u32>().ok());
                                        }
                                    }
                                }
                                "w:level" => {
                                    in_level = true;
                                    level_num_fmt = None;
                                }
                                "w:numfmt" if in_level => {
                                    for attr in e.attributes().flatten() {
                                        let key =
                                            String::from_utf8_lossy(attr.key.as_ref());
                                        if key.contains("val") || key.ends_with("Val") {
                                            let val = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                                .unwrap_or_default()
                                                .to_string();
                                            level_num_fmt = Some(val);
                                        }
                                    }
                                }
                                "w:abstractnumid" if in_num => {
                                    for attr in e.attributes().flatten() {
                                        let key =
                                            String::from_utf8_lossy(attr.key.as_ref());
                                        if key.contains("val") || key.ends_with("Val") {
                                            let abs_id = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                                .ok()
                                                .and_then(|v| v.parse::<u32>().ok());
                                            if let (Some(num_id), Some(abs_id)) =
                                                (current_num_id, abs_id)
                                            {
                                                let is_bullet = abs_num_id_to_is_bullet
                                                    .get(&abs_id)
                                                    .copied()
                                                    .unwrap_or(false);
                                                num_id_to_is_bullet.insert(num_id, is_bullet);
                                                abs_num_id_to_has_levels
                                                    .entry(abs_id)
                                                    .or_insert(true);
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }

                        Ok(Event::End(ref e)) => {
                            let tag =
                                String::from_utf8_lossy(e.name().as_ref()).to_lowercase();
                            match tag.as_str() {
                                "w:abstractnum" => {
                                    in_abstract_num = false;
                                    if let Some(abs_id) = current_abs_num_id {
                                        let is_bullet = level_num_fmt
                                            .as_deref()
                                            == Some("bullet");
                                        abs_num_id_to_is_bullet.insert(abs_id, is_bullet);
                                    }
                                    current_abs_num_id = None;
                                    level_num_fmt = None;
                                }
                                "w:num" => {
                                    in_num = false;
                                    current_num_id = None;
                                }
                                "w:level" => {
                                    in_level = false;
                                    if in_abstract_num {
                                        let is_bullet = level_num_fmt
                                            .as_deref()
                                            == Some("bullet");
                                        if let Some(abs_id) = current_abs_num_id {
                                            if !abs_num_id_to_is_bullet.contains_key(&abs_id)
                                            {
                                                abs_num_id_to_is_bullet
                                                    .insert(abs_id, is_bullet);
                                            }
                                        }
                                    }
                                    level_num_fmt = None;
                                }
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
        }
        num_id_to_is_bullet
    }

    fn extract_images(archive: &mut ZipArchive<std::fs::File>) -> HashMap<String, Vec<u8>> {
        let mut map = HashMap::new();
        for i in 0..archive.len() {
            if let Ok(mut entry) = archive.by_index(i) {
                let name = entry.name().to_string();
                if name.starts_with("word/media/") {
                    let mut buf = vec![];
                    if entry.read_to_end(&mut buf).is_ok() {
                        let file_name = std::path::Path::new(&name)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        map.insert(file_name, buf);
                    }
                }
            }
        }
        map
    }

    #[allow(clippy::too_many_lines)]
    fn parse_document_xml(
        xml: &str,
        rels_map: &HashMap<String, String>,
        num_format_map: &HashMap<u32, bool>,
        images_map: &HashMap<String, Vec<u8>>,
    ) -> (Vec<PageLayout>, Vec<Paragraph>, Vec<usize>) {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut buf = Vec::new();

        let mut paragraphs: Vec<Paragraph> = vec![];
        let mut para_to_page: Vec<usize> = vec![];
        let mut pages: Vec<PageLayout> = vec![];
        let mut current_els: Vec<DocumentElement> = vec![];
        let mut current_page_num: u32 = 1;

        let default_margins = Margins {
            top: 1.0,
            bottom: 1.0,
            left: 1.0,
            right: 1.0,
        };
        let mut page_width: Option<f32> = None;
        let mut page_height: Option<f32> = None;
        let mut orientation = PageOrientation::Portrait;
        let mut margins = default_margins.clone();
        let mut columns: u32 = 1;
        let mut column_widths: Vec<f32> = vec![];

        // Paragraph state
        let mut in_paragraph = false;
        let mut paragraph_runs: Vec<Run> = vec![];
        let mut paragraph_style: Option<String> = None;
        let mut paragraph_align: Option<TextAlignment> = None;
        let mut has_page_break = false;
        let mut num_id: Option<u32> = None;
        let mut ilvl: i32 = 0;
        let mut in_ppr = false;
        let mut in_numpr = false;

        // Run state
        let mut in_run = false;
        let mut run_text = String::new();
        let mut run_bold = false;
        let mut run_italic = false;
        let mut run_underline = false;
        let mut run_strike = false;
        let mut run_subscript = false;
        let mut run_superscript = false;
        let mut run_font_size: Option<f32> = None;
        let mut run_font_family: Option<String> = None;
        let mut run_font_color: Option<String> = None;
        let mut in_rpr = false;
        let mut in_text = false;

        // Hyperlink state

        let mut hyperlink_url: Option<String> = None;

        // Table state
        let mut in_table = false;
        let mut table_headers: Vec<TableCell> = vec![];
        let mut table_rows: Vec<Vec<TableCell>> = vec![];
        let mut in_tr = false;
        let mut current_row: Vec<TableCell> = vec![];
        let mut in_tc = false;
        let mut current_cell = String::new();
        let mut current_cell_colspan: u32 = 1;
        let mut current_cell_rowspan: u32 = 1;
        let mut in_tc_pr = false;

        // Image state
        let mut in_drawing = false;
        let mut blip_rid: Option<String> = None;
        let mut image_cx: Option<f32> = None;
        let mut image_cy: Option<f32> = None;
        let mut image_alt: Option<String> = None;
        let mut image_is_anchor = false;
        let mut pending_images: Vec<ImageElement> = vec![];

        // List tracking
        let mut current_list_items: Vec<ListItem> = vec![];
        let mut current_list_ordered = false;
        let mut current_list_num_id: Option<u32> = None;

        // Section properties
        let mut in_sectpr = false;

        let finalize_page = |els: &mut Vec<DocumentElement>,
                                  page_num: &mut u32,
                                  stored_pages: &mut Vec<PageLayout>,
                                  stored_cols: u32,
                                  stored_col_widths: &[f32],
                                  stored_page_w: Option<f32>,
                                  stored_page_h: Option<f32>,
                                  stored_orient: PageOrientation,
                                  stored_margins: Margins|
         -> Vec<DocumentElement> {
            let new_els = if !els.is_empty() {
                stored_pages.push(PageLayout {
                    number: *page_num,
                    elements: std::mem::take(els),
                    columns: stored_cols,
                    column_widths: stored_col_widths.to_vec(),
                    page_width: stored_page_w,
                    page_height: stored_page_h,
                    orientation: stored_orient,
                    margins: stored_margins,
                    header: None,
                    footer: None,
                });
                *page_num += 1;
                vec![]
            } else {
                std::mem::take(els)
            };
            new_els
        };

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();

                    match tag.as_str() {
                        // Paragraph
                        "w:p" => {
                            in_paragraph = true;
                            paragraph_runs.clear();
                            paragraph_style = None;
                            paragraph_align = None;
                            has_page_break = false;
                            num_id = None;
                            ilvl = 0;
                        }

                        // Paragraph properties
                        "w:ppr" => {
                            in_ppr = true;
                        }
                        "w:pstyle" if in_ppr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                if key.contains("val") || key.ends_with("Val") {
                                    paragraph_style = Some(
                                        attr.normalized_value(quick_xml::XmlVersion::Implicit1_0).unwrap_or_default().to_string(),
                                    );
                                }
                            }
                        }
                        "w:jc" if in_ppr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                if key.contains("val") || key.ends_with("Val") {
                                    let val =
                                        attr.normalized_value(quick_xml::XmlVersion::Implicit1_0).unwrap_or_default().to_string();
                                    paragraph_align = Some(match val.as_str() {
                                        "center" => TextAlignment::Center,
                                        "right" => TextAlignment::Right,
                                        "both" => TextAlignment::Justified,
                                        _ => TextAlignment::Left,
                                    });
                                }
                            }
                        }
                        "w:numpr" if in_ppr => {
                            in_numpr = true;
                        }
                        "w:ilvl" if in_numpr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                if key.contains("val") || key.ends_with("Val") {
                                    ilvl = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .unwrap_or_default()
                                        .parse::<i32>()
                                        .unwrap_or(0);
                                }
                            }
                        }
                        "w:numid" if in_numpr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                if key.contains("val") || key.ends_with("Val") {
                                    num_id = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .unwrap_or_default()
                                        .parse::<u32>()
                                        .ok();
                                }
                            }
                        }

                        // Page breaks
                        "w:br" => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                let val = String::from_utf8_lossy(attr.value.as_ref());
                                if key == "w:type" && val == "page" {
                                    has_page_break = true;
                                }
                            }
                        }

                        // Run
                        "w:r" => {
                            in_run = true;
                            run_text.clear();
                            run_bold = false;
                            run_italic = false;
                            run_underline = false;
                            run_strike = false;
                            run_subscript = false;
                            run_superscript = false;
                            run_font_size = None;
                            run_font_family = None;
                            run_font_color = None;
                        }

                        // Run properties
                        "w:rpr" => {
                            in_rpr = true;
                        }
                        "w:b" if in_rpr => {
                            run_bold = true;
                        }
                        "w:i" if in_rpr => {
                            run_italic = true;
                        }
                        "w:u" if in_rpr => {
                            run_underline = true;
                        }
                        "w:strike" if in_rpr => {
                            run_strike = true;
                        }
                        "w:vertalign" if in_rpr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                if key.contains("val") || key.ends_with("Val") {
                                    let val =
                                        attr.normalized_value(quick_xml::XmlVersion::Implicit1_0).unwrap_or_default().to_string();
                                    run_superscript = val == "superscript";
                                    run_subscript = val == "subscript";
                                }
                            }
                        }
                        "w:sz" if in_rpr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                if key.contains("val") || key.ends_with("Val") {
                                    let half_pts = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .unwrap_or_default()
                                        .parse::<f32>()
                                        .unwrap_or(0.0);
                                    run_font_size = Some(half_pts / 2.0);
                                }
                            }
                        }
                        "w:rfonts" if in_rpr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                if key.contains("ascii") || key.contains("Ascii") {
                                    run_font_family = Some(
                                        attr.normalized_value(quick_xml::XmlVersion::Implicit1_0).unwrap_or_default().to_string(),
                                    );
                                }
                            }
                        }
                        "w:color" if in_rpr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                if key.contains("val") || key.ends_with("Val") {
                                    run_font_color = Some(
                                        attr.normalized_value(quick_xml::XmlVersion::Implicit1_0).unwrap_or_default().to_string(),
                                    );
                                }
                            }
                        }

                        // Text
                        "w:t" => {
                            in_text = true;
                        }

                        // Hyperlink
                        "w:hyperlink" => {
                            hyperlink_url = None;
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                if key.contains("id") || key.ends_with("Id")
                                    || key.contains("Id")
                                {
                                    let rid =
                                        attr.normalized_value(quick_xml::XmlVersion::Implicit1_0).unwrap_or_default().to_string();
                                    hyperlink_url = rels_map.get(&rid).cloned();
                                }
                            }
                        }

                        // Table
                        "w:tbl" => {
                            in_table = true;
                            table_headers = vec![];
                            table_rows = vec![];
                        }
                        "w:tr" if in_table => {
                            in_tr = true;
                            current_row = vec![];
                        }
                        "w:tc" if in_tr => {
                            in_tc = true;
                            current_cell.clear();
                            current_cell_colspan = 1;
                            current_cell_rowspan = 1;
                        }
                        "w:tcpr" if in_tc => {
                            in_tc_pr = true;
                        }
                        "w:gridspan" if in_tc_pr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                if key.contains("val") || key.ends_with("Val") {
                                    current_cell_colspan = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .unwrap_or_default()
                                        .parse::<u32>()
                                        .unwrap_or(1);
                                }
                            }
                        }
                        "w:vmerge" if in_tc_pr => {
                            let mut val = String::from("restart");
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                if key.contains("val") || key.ends_with("Val") {
                                    val =
                                        attr.normalized_value(quick_xml::XmlVersion::Implicit1_0).unwrap_or_default().to_string();
                                }
                            }
                            if val == "restart" {
                                current_cell_rowspan = 2;
                                // Real rowspan requires scanning ahead; 2 is a safe default
                            }
                        }

                        // Drawing / Image
                        "w:drawing" => {
                            in_drawing = true;
                            blip_rid = None;
                            image_cx = None;
                            image_cy = None;
                            image_alt = None;
                            image_is_anchor = false;
                        }
                        "wp:inline" | "wps:inline" if in_drawing => {
                            image_is_anchor = false;
                        }
                        "wp:anchor" | "wps:anchor" if in_drawing => {
                            image_is_anchor = true;
                        }
                        "wp:extent" | "wps:extent" if in_drawing => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                let val = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .unwrap_or_default()
                                    .parse::<f32>()
                                    .unwrap_or(0.0);
                                if key.contains("cx") || key.ends_with("Cx") {
                                    image_cx = Some(val / 914_400.0);
                                } else if key.contains("cy") || key.ends_with("Cy") {
                                    image_cy = Some(val / 914_400.0);
                                }
                            }
                        }
                        "wp:docpr" | "wps:docpr" if in_drawing => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                if key.contains("descr") || key.ends_with("Descr") {
                                    image_alt = Some(
                                        attr.normalized_value(quick_xml::XmlVersion::Implicit1_0).unwrap_or_default().to_string(),
                                    );
                                }
                            }
                        }
                        "a:blip" if in_drawing => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                if key.contains("embed") || key.ends_with("Embed") {
                                    let rid =
                                        attr.normalized_value(quick_xml::XmlVersion::Implicit1_0).unwrap_or_default().to_string();
                                    blip_rid = Some(rid);
                                }
                            }
                        }

                        // Section properties
                        "w:sectpr" => {
                            in_sectpr = true;
                            columns = 1;
                            column_widths.clear();
                        }
                        "w:pgsz" if in_sectpr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                let val = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .unwrap_or_default()
                                    .parse::<f32>()
                                    .unwrap_or(0.0);
                                if key.contains("w") && !key.contains("Code") {
                                    page_width = Some(val / 1440.0);
                                } else if key.contains("h") {
                                    page_height = Some(val / 1440.0);
                                }
                            }
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                if key.contains("orient") || key.ends_with("Orient") {
                                    let val =
                                        attr.normalized_value(quick_xml::XmlVersion::Implicit1_0).unwrap_or_default().to_string();
                                    if val.to_lowercase() == "landscape" {
                                        orientation = PageOrientation::Landscape;
                                    }
                                }
                            }
                        }
                        "w:pgmar" if in_sectpr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                let val = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                    .unwrap_or_default()
                                    .parse::<f32>()
                                    .unwrap_or(1440.0);
                                let inches = val / 1440.0;
                                if key.contains("top") || key.ends_with("Top") {
                                    margins.top = inches;
                                } else if key.contains("bottom") || key.ends_with("Bottom") {
                                    margins.bottom = inches;
                                } else if key.contains("left") || key.ends_with("Left") {
                                    margins.left = inches;
                                } else if key.contains("right") || key.ends_with("Right") {
                                    margins.right = inches;
                                }
                            }
                        }
                        "w:cols" if in_sectpr => {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref());
                                if key.contains("num") || key.ends_with("Num") {
                                    columns = attr.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .unwrap_or_default()
                                        .parse::<u32>()
                                        .unwrap_or(1);
                                }
                            }
                        }

                        _ => {}
                    }
                }

                Ok(Event::End(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();

                    match tag.as_str() {
                        // End paragraph
                        "w:p" => {
                            in_paragraph = false;

                            let has_text = paragraph_runs.iter().any(|r| !r.text.is_empty());
                            let flat_text: String =
                                paragraph_runs.iter().map(|r| r.text.as_str()).collect();
                            let is_heading =
                                Self::is_heading_style(&paragraph_style);
                            let is_list_item = num_id.is_some();

                            // Build backward-compat Paragraph
                            paragraphs.push(Paragraph {
                                text: flat_text,
                                style: paragraph_style.clone(),
                                page_break: has_page_break,
                            });
                            para_to_page.push(current_page_num as usize);

                            // Handle page break
                            if has_page_break && !current_els.is_empty() {
                                current_els = finalize_page(
                                    &mut current_els,
                                    &mut current_page_num,
                                    &mut pages,
                                    columns,
                                    &column_widths,
                                    page_width,
                                    page_height,
                                    orientation.clone(),
                                    margins.clone(),
                                );
                            }

                            // Determine element for the paragraph content
                            // Push any pending images from drawings in this paragraph
                            for img in pending_images.drain(..) {
                                current_els.push(DocumentElement::Image(img));
                            }

                            if has_text && is_list_item {
                                let is_ordered = num_id
                                    .and_then(|n| num_format_map.get(&n))
                                    .copied()
                                    .unwrap_or(false);
                                let item = ListItem {
                                    level: ilvl,
                                    runs: paragraph_runs.clone(),
                                    children: vec![],
                                };

                                // Check if list continues
                                if current_list_num_id == num_id || current_list_items.is_empty() {
                                    current_list_items.push(item);
                                    current_list_ordered = is_ordered;
                                    current_list_num_id = num_id;
                                } else {
                                    // Finalize previous list
                                    if !current_list_items.is_empty() {
                                        current_els.push(DocumentElement::List(ListElement {
                                            items: std::mem::take(&mut current_list_items),
                                            ordered: current_list_ordered,
                                            start_number: None,
                                        }));
                                    }
                                    current_list_items.push(item);
                                    current_list_ordered = is_ordered;
                                    current_list_num_id = num_id;
                                }
                            } else if has_text {
                                // Flush pending list if any
                                if !current_list_items.is_empty() {
                                    current_els.push(DocumentElement::List(ListElement {
                                        items: std::mem::take(&mut current_list_items),
                                        ordered: current_list_ordered,
                                        start_number: None,
                                    }));
                                    current_list_num_id = None;
                                }

                                let element = if let Some(level) = is_heading {
                                    DocumentElement::Heading(HeadingElement {
                                        level,
                                        runs: paragraph_runs.clone(),
                                    })
                                } else {
                                    DocumentElement::Paragraph(ParagraphElement {
                                        runs: paragraph_runs.clone(),
                                        alignment: paragraph_align.clone(),
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
                                current_els.push(element);
                            }
                        }

                        // End paragraph properties
                        "w:ppr" => {
                            in_ppr = false;
                            in_numpr = false;
                        }

                        // End run
                        "w:r" => {
                            in_run = false;
                            if !run_text.is_empty() {
                                paragraph_runs.push(Run {
                                    text: run_text.clone(),
                                    bold: run_bold,
                                    italic: run_italic,
                                    underline: run_underline,
                                    strike: run_strike,
                                    subscript: run_subscript,
                                    superscript: run_superscript,
                                    code: false,
                                    hyperlink_url: hyperlink_url.clone(),
                                    font_size: run_font_size,
                                    font_family: run_font_family.clone(),
                                    font_color: run_font_color.clone(),
                                    highlight_color: None,
                                });
                            }
                        }

                        // End run properties
                        "w:rpr" => {
                            in_rpr = false;
                        }

                        // End hyperlink
                        "w:hyperlink" => {
                            hyperlink_url = None;
                        }

                        // End text
                        "w:t" => {
                            in_text = false;
                        }

                        // End table
                        "w:tbl" => {
                            in_table = false;
                            // Flush pending list
                            if !current_list_items.is_empty() {
                                current_els.push(DocumentElement::List(ListElement {
                                    items: std::mem::take(&mut current_list_items),
                                    ordered: current_list_ordered,
                                    start_number: None,
                                }));
                                current_list_num_id = None;
                            }

                            let has_data = if table_headers.is_empty() {
                                table_rows.iter().any(|r| !r.is_empty())
                            } else {
                                true
                            };
                            if has_data {
                                current_els.push(DocumentElement::Table(TableElement {
                                    headers: std::mem::take(&mut table_headers),
                                    rows: std::mem::take(&mut table_rows),
                                    column_widths: vec![],
                                    caption: None,
                                    borders: true,
                                    width: None,
                                    alignment: None,
                                }));
                            }
                        }

                        // End table row
                        "w:tr" if in_table => {
                            in_tr = false;
                            if current_row.is_empty() && !table_headers.is_empty() {
                                // Skip empty rows after headers
                            } else if table_headers.is_empty() {
                                table_headers = std::mem::take(&mut current_row);
                            } else {
                                table_rows.push(std::mem::take(&mut current_row));
                            }
                        }

                        // End table cell
                        "w:tc" if in_tr => {
                            in_tc = false;
                            current_row.push(TableCell {
                                text: std::mem::take(&mut current_cell),
                                rowspan: current_cell_rowspan,
                                colspan: current_cell_colspan,
                                horizontal_alignment: None,
                                background_color: None,
                                width: None,
                            });
                        }

                        // End cell properties
                        "w:tcpr" if in_tc => {
                            in_tc_pr = false;
                        }

                        // End drawing
                        // End drawing — resolve image if blip was found
                        "w:drawing" => {
                            in_drawing = false;
                            if let Some(rid) = blip_rid.take() {
                                let img_target = rels_map.get(&rid);
                                let (img_path, img_bytes, img_fmt) = if let Some(target) = img_target {
                                    let fname = std::path::Path::new(target)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("image")
                                        .to_string();
                                    let bytes = images_map
                                        .get(&fname)
                                        .cloned()
                                        .unwrap_or_default();
                                    let fmt = if fname.ends_with(".png") {
                                        ImageFormat::Png
                                    } else if fname.ends_with(".jpg") || fname.ends_with(".jpeg") {
                                        ImageFormat::Jpeg
                                    } else if fname.ends_with(".gif") {
                                        ImageFormat::Gif
                                    } else if fname.ends_with(".bmp") {
                                        ImageFormat::Bmp
                                    } else if fname.ends_with(".svg") {
                                        ImageFormat::Svg
                                    } else {
                                        ImageFormat::Unknown
                                    };
                                    (target.clone(), bytes, fmt)
                                } else {
                                    ("unknown".to_string(), vec![], ImageFormat::Unknown)
                                };

                                let position = if image_is_anchor {
                                    ImagePosition::FloatRight
                                } else {
                                    ImagePosition::Inline
                                };

                                pending_images.push(ImageElement {
                                    path: img_path,
                                    bytes: img_bytes,
                                    format: img_fmt,
                                    caption: None,
                                    alt_text: image_alt.take(),
                                    width: image_cx.take(),
                                    height: image_cy.take(),
                                    position,
                                });
                            }
                            blip_rid = None;
                            image_cx = None;
                            image_cy = None;
                            image_alt = None;
                            image_is_anchor = false;
                        }

                        // End section properties
                        "w:sectpr" => {
                            in_sectpr = false;
                            if !current_els.is_empty() {
                                current_els = finalize_page(
                                    &mut current_els,
                                    &mut current_page_num,
                                    &mut pages,
                                    columns,
                                    &column_widths,
                                    page_width,
                                    page_height,
                                    orientation.clone(),
                                    margins.clone(),
                                );
                            }
                        }

                        _ => {}
                    }
                }

                Ok(Event::Text(ref e)) => {
                    let text = e.decode().unwrap_or_default().to_string();

                    if in_tc {
                        current_cell.push_str(&text);
                    } else if in_text && in_run && in_paragraph {
                        run_text.push_str(&text);
                    }
                }

                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        // Finalize remaining list
        if !current_list_items.is_empty() {
            current_els.push(DocumentElement::List(ListElement {
                items: std::mem::take(&mut current_list_items),
                ordered: current_list_ordered,
                start_number: None,
            }));
        }

        // Push last page
        if !current_els.is_empty() {
            pages.push(PageLayout {
                number: current_page_num,
                elements: std::mem::take(&mut current_els),
                columns,
                column_widths: column_widths.clone(),
                page_width,
                page_height,
                orientation,
                margins,
                header: None,
                footer: None,
            });
        }

        // If no pages but has paragraphs, create a single page
        if pages.is_empty() && !paragraphs.is_empty() {
            pages.push(PageLayout {
                number: 1,
                elements: vec![],
                columns: 1,
                column_widths: vec![],
                page_width: None,
                page_height: None,
                orientation: PageOrientation::Portrait,
                margins: default_margins,
                header: None,
                footer: None,
            });
        }

        // Virtual page splitting: if single page has many elements, split into pages of 20
        if pages.len() == 1 && pages[0].elements.len() > 20 {
            let single = pages.remove(0);
            let count = single.elements.len();
            let chunk_size = (count / 20).max(1);
            for (i, chunk) in single.elements.chunks(chunk_size).enumerate() {
                pages.push(PageLayout {
                    number: (i + 1) as u32,
                    elements: chunk.to_vec(),
                    columns: single.columns,
                    column_widths: single.column_widths.clone(),
                    page_width: single.page_width,
                    page_height: single.page_height,
                    orientation: single.orientation.clone(),
                    margins: single.margins.clone(),
                    header: single.header.clone(),
                    footer: single.footer.clone(),
                });
            }
        }

        (pages, paragraphs, para_to_page)
    }

    fn is_heading_style(style: &Option<String>) -> Option<u8> {
        match style.as_deref() {
            Some(s) if s.to_lowercase().starts_with("heading") => {
                let level_str = s.to_lowercase().replace("heading", "").trim().to_string();
                level_str.parse::<u8>().ok().filter(|&l| l > 0 && l <= 9)
            }
            _ => None,
        }
    }
}

impl Document for DocxDocument {
    fn metadata(&self) -> DocumentMetadata {
        DocumentMetadata {
            path: self.path.clone(),
            format: DocumentFormat::Docx,
            title: self.metadata.title.clone(),
            author: self.metadata.author.clone(),
            created: self.metadata.created.as_deref().and_then(parse_iso8601_date),
            modified: self.metadata.modified.as_deref().and_then(parse_iso8601_date),
            page_count: if self.pages.is_empty() {
                if self.paragraphs.is_empty() {
                    None
                } else {
                    Some(1)
                }
            } else {
                Some(self.pages.len() as u32)
            },
            size: self.size,
        }
    }

    fn outline(&self) -> Outline {
        let headings: Vec<(u8, String, Selector)> = self
            .paragraphs
            .iter()
            .enumerate()
            .filter_map(|(i, para)| {
                Self::is_heading_style(&para.style).map(|level| {
                    let page_num = self.para_to_page.get(i).copied().unwrap_or(1);
                    (level, para.text.clone(), Selector::Page(page_num as u32))
                })
            })
            .collect();
        Outline { entries: build_outline_tree_from_selectors(&headings) }
    }

    fn page_count(&self) -> Option<u32> {
        if self.pages.is_empty() {
            if self.paragraphs.is_empty() {
                None
            } else {
                Some(1)
            }
        } else {
            Some(self.pages.len() as u32)
        }
    }

    fn search(&self, query: &str) -> Vec<Match> {
        let q = query.to_lowercase();
        let mut results = vec![];

        for (i, page) in self.pages.iter().enumerate() {
            let page_text = render_page_to_markdown(page);
            if page_text.to_lowercase().contains(&q) {
                results.push(Match {
                    selector: Selector::Page((i + 1) as u32),
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
                let idx = *n as usize;
                if idx > 0 && idx <= self.pages.len() {
                    let md = render_page_to_markdown(&self.pages[idx - 1]);
                    Ok(ReadResult::Text(md))
                } else if idx == 1 && self.pages.is_empty() && !self.paragraphs.is_empty() {
                    // Single page with no explicit page breaks
                    let md = render_page_to_markdown(&PageLayout {
                        number: 1,
                        elements: vec![],
                        columns: 1,
                        column_widths: vec![],
                        page_width: None,
                        page_height: None,
                        orientation: PageOrientation::Portrait,
                        margins: Margins {
                            top: 1.0,
                            bottom: 1.0,
                            left: 1.0,
                            right: 1.0,
                        },
                        header: None,
                        footer: None,
                    });
                    Ok(ReadResult::Text(md))
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "page {} not found (document has {} pages)",
                        n,
                        self.pages.len()
                    )))
                }
            }
            Selector::Pages(pages) => {
                let mut parts = Vec::new();
                for p in pages {
                    let idx = *p as usize;
                    if idx > 0 && idx <= self.pages.len() {
                        parts.push(render_page_to_markdown(&self.pages[idx - 1]));
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
                if s >= self.pages.len() {
                    return Err(DocumentError::InvalidSelector(format!(
                        "skip {} beyond page count {}",
                        skip,
                        self.pages.len()
                    )));
                }
                let end = std::cmp::min(s + t, self.pages.len());
                let mut parts = Vec::new();
                for i in s..end {
                    parts.push(render_page_to_markdown(&self.pages[i]));
                }
                Ok(ReadResult::Text(parts.join("\n\n---\n\n")))
            }
            _ => Err(DocumentError::InvalidSelector(format!(
                "selector {:?} not supported for docx documents (use Page, Pages, or Slice)",
                selector
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Markdown rendering helpers
// ---------------------------------------------------------------------------

fn render_page_to_markdown(page: &PageLayout) -> String {
    let mut md = String::new();

    md.push_str(&format!("<!-- Page {} -->\n\n", page.number));

    if page.columns > 1 {
        md.push_str(&format!("> **Layout:** {} columns", page.columns));
        if !page.column_widths.is_empty() {
            let widths: Vec<String> =
                page.column_widths.iter().map(|w| format!("{:.1}in", w)).collect();
            md.push_str(&format!(" ({})", widths.join(", ")));
        }
        md.push_str("\n\n");
    }
    if page.orientation == PageOrientation::Landscape {
        md.push_str("> **Orientation:** Landscape\n\n");
    }
    md.push_str(&format!(
        "> **Margins:** T={:.1}in B={:.1}in L={:.1}in R={:.1}in\n\n",
        page.margins.top, page.margins.bottom, page.margins.left, page.margins.right
    ));

    if let Some(ref hdr) = page.header {
        md.push_str(&format!("> *Header: {}*\n\n", hdr));
    }

    for element in &page.elements {
        match element {
            DocumentElement::Heading(h) => {
                let text = render_runs(&h.runs);
                md.push_str(&format!("{} {}\n\n", "#".repeat(h.level as usize), text));
            }
            DocumentElement::Paragraph(p) => {
                let text = render_runs(&p.runs);
                if !text.is_empty() {
                    md.push_str(&text);
                    match &p.alignment {
                        Some(TextAlignment::Center) => md.push_str("\n{:center}"),
                        Some(TextAlignment::Right) => md.push_str("\n{:right}"),
                        _ => {}
                    }
                    md.push_str("\n\n");
                }
            }
            DocumentElement::Image(img) => {
                md.push_str(&format!(
                    "![{}]({})",
                    img.alt_text.as_deref().unwrap_or(img.caption.as_deref().unwrap_or("image")),
                    img.path
                ));
                if let Some(ref cap) = img.caption {
                    md.push_str(&format!("\n*{}*", cap));
                }
                md.push(' ');
                match &img.position {
                    ImagePosition::FloatLeft => md.push_str("{:float-left} "),
                    ImagePosition::FloatRight => md.push_str("{:float-right} "),
                    ImagePosition::Center => md.push_str("{:center} "),
                    ImagePosition::BehindText => md.push_str("{:behind-text} "),
                    ImagePosition::InFrontOfText => md.push_str("{:in-front} "),
                    ImagePosition::Inline => {}
                }
                if let Some(w) = img.width {
                    md.push_str(&format!("width={:.1}in ", w));
                }
                if let Some(h) = img.height {
                    md.push_str(&format!("height={:.1}in ", h));
                }
                md.push_str("\n\n");
            }
            DocumentElement::Table(t) => {
                if let Some(ref cap) = t.caption {
                    md.push_str(&format!("**{}**\n\n", cap));
                }
                match &t.headers.is_empty() {
                    false => {
                        md.push('|');
                        for h in &t.headers {
                            md.push_str(&format!(" {} |", table_cell_text(h)));
                        }
                        md.push('\n');
                        md.push('|');
                        for _ in &t.headers {
                            md.push_str(" --- |");
                        }
                        md.push('\n');
                        for row in &t.rows {
                            md.push('|');
                            for cell in row {
                                md.push_str(&format!(" {} |", table_cell_text(cell)));
                            }
                            md.push('\n');
                        }
                    }
                    true => {
                        // No header row — use first row as header
                        if let Some(first) = t.rows.first() {
                            md.push('|');
                            for cell in first {
                                md.push_str(&format!(" {} |", table_cell_text(cell)));
                            }
                            md.push('\n');
                            md.push('|');
                            for _ in first {
                                md.push_str(" --- |");
                            }
                            md.push('\n');
                            for row in t.rows.iter().skip(1) {
                                md.push('|');
                                for cell in row {
                                    md.push_str(&format!(" {} |", table_cell_text(cell)));
                                }
                                md.push('\n');
                            }
                        }
                    }
                }
                let mut table_annotations = vec![];
                if let Some(w) = t.width {
                    table_annotations.push(format!("width={:.1}in", w));
                }
                if let Some(ref align) = t.alignment {
                    table_annotations.push(format!("align={:?}", align));
                }
                if t.borders {
                    table_annotations.push("borders".to_string());
                }
                if !table_annotations.is_empty() {
                    md.push_str(&format!("\n> *Table: {}*\n\n", table_annotations.join(", ")));
                } else {
                    md.push('\n');
                }
            }
            DocumentElement::List(list) => {
                render_list(list, 0, &mut md);
                md.push('\n');
            }
            DocumentElement::TextBox(tb) => {
                let text = render_runs(&tb.runs);
                md.push_str(&format!("> [{}]\n\n", text));
                match &tb.position {
                    ImagePosition::FloatLeft => md.push_str("> {:float-left}\n\n"),
                    ImagePosition::FloatRight => md.push_str("> {:float-right}\n\n"),
                    _ => {}
                }
            }
            DocumentElement::HorizontalRule => {
                md.push_str("---\n\n");
            }
            DocumentElement::PageBreak => {
                md.push_str("\n\\newpage\n\n");
            }
            DocumentElement::SectionBreak { columns: c } => {
                if let Some(c) = c {
                    md.push_str(&format!("> **Section:** {} columns\n\n", c));
                } else {
                    md.push_str("> **Section break**\n\n");
                }
            }
        }
    }

    if let Some(ref ftr) = page.footer {
        md.push_str(&format!("> *Footer: {}*\n\n", ftr));
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
            '*' | '_' | '`' | '[' | ']' | '(' | ')' | '#' | '~' | '|' | '!' | '>' | '-' | '+' | '.' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

fn render_list(list: &ListElement, depth: usize, md: &mut String) {
    for item in &list.items {
        let indent = "  ".repeat(depth + item.level as usize);
        let prefix = if list.ordered { "1." } else { "-" };
        md.push_str(&format!("{}{} {}\n", indent, prefix, render_runs(&item.runs)));
        for child in &item.children {
            render_list_item(child, depth + 1, md);
        }
    }
}

fn render_list_item(item: &ListItem, depth: usize, md: &mut String) {
    let indent = "  ".repeat(depth + item.level as usize);
    md.push_str(&format!("{}  - {}\n", indent, render_runs(&item.runs)));
    for child in &item.children {
        render_list_item(child, depth + 1, md);
    }
}

fn table_cell_text(cell: &TableCell) -> String {
    if cell.text.is_empty() {
        " ".to_string()
    } else {
        cell.text.clone()
    }
}
