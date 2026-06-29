use std::io::Read;
use std::path::PathBuf;

use quick_xml::events::Event;
use quick_xml::Reader;
use zip::ZipArchive;

use crate::ocean_parser::{
    Document, DocumentError, DocumentFactory, DocumentFormat, DocumentMetadata, ImageFormat, Match,
    Outline, OutlineEntry, ReadResult, Selector,
};

pub struct PptxDocument {
    path: PathBuf,
    size: u64,
    slides: Vec<SlideInfo>,
    images: Vec<(String, Vec<u8>)>,
}

struct SlideInfo {
    number: u32,
    title: Option<String>,
    content: String,
}

impl PptxDocument {
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

        let file = std::fs::File::open(&p)
            .map_err(|e| DocumentError::PermissionDenied(format!("{}: {}", path, e)))?;

        let mut archive = ZipArchive::new(file)
            .map_err(|e| DocumentError::CorruptedFile(format!("invalid pptx: {}", e)))?;

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

        let mut slides = vec![];
        for (num, xml) in &slide_files {
            let (title, text) = Self::parse_slide_xml(xml);
            slides.push(SlideInfo {
                number: *num,
                title,
                content: text,
            });
        }

        let mut images = vec![];
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).ok();
            if let Some(ref mut entry) = entry {
                let name = entry.name().to_string();
                if name.starts_with("ppt/media/") {
                    let mut buf = vec![];
                    if entry.read_to_end(&mut buf).is_ok() {
                        let file_name = std::path::Path::new(&name)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        images.push((file_name, buf));
                    }
                }
            }
        }

        Ok(Self {
            path: p,
            size: meta.len(),
            slides,
            images,
        })
    }

    fn parse_slide_xml(xml: &str) -> (Option<String>, String) {
        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut title: Option<String> = None;
        let mut all_text = String::new();
        let mut in_text = false;
        let mut current_text = String::new();
        let mut is_title = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();
                    match tag.as_str() {
                        "a:t" => {
                            in_text = true;
                            current_text.clear();
                        }
                        "p:ph" | "p:placeholder" => {
                            for attr in e.attributes().flatten() {
                                let key =
                                    String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                let val =
                                    String::from_utf8_lossy(attr.value.as_ref()).to_string();
                                if key == "type" && val.to_lowercase() == "title" {
                                    is_title = true;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_lowercase();
                    match tag.as_str() {
                        "a:t" => {
                            in_text = false;
                            if !current_text.is_empty() {
                                if is_title && title.is_none() {
                                    title = Some(current_text.clone());
                                }
                                if !all_text.is_empty() {
                                    all_text.push(' ');
                                }
                                all_text.push_str(&current_text);
                            }
                            is_title = false;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(ref e)) => {
                    if in_text {
                        current_text.push_str(&e.unescape().unwrap_or_default());
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        (title, all_text)
    }
}

impl Document for PptxDocument {
    fn metadata(&self) -> DocumentMetadata {
        DocumentMetadata {
            path: self.path.clone(),
            format: DocumentFormat::Pptx,
            title: None,
            author: None,
            created: None,
            modified: None,
            page_count: Some(self.slides.len() as u32),
            size: self.size,
        }
    }

    fn outline(&self) -> Outline {
        let entries: Vec<OutlineEntry> = self
            .slides
            .iter()
            .map(|s| OutlineEntry {
                label: s.title.clone().unwrap_or_else(|| format!("Slide {}", s.number)),
                level: 1,
                selector: Selector::Slide(s.number),
                children: vec![],
            })
            .collect();

        Outline { entries }
    }

    fn page_count(&self) -> Option<u32> {
        Some(self.slides.len() as u32)
    }

    fn search(&self, query: &str) -> Vec<Match> {
        let q = query.to_lowercase();
        let mut results = vec![];

        for slide in &self.slides {
            if slide.content.to_lowercase().contains(&q)
                || slide
                    .title
                    .as_deref()
                    .map(|t| t.to_lowercase().contains(&q))
                    .unwrap_or(false)
            {
                results.push(Match {
                    selector: Selector::Slide(slide.number),
                    text: slide.content.clone(),
                    context: slide.title.clone().unwrap_or_default(),
                    score: 1.0,
                });
            }
        }

        results
    }

    fn read(&self, selector: &Selector) -> Result<ReadResult, DocumentError> {
        match selector {
            Selector::Slide(n) => {
                if let Some(slide) = self.slides.iter().find(|s| s.number == *n) {
                    Ok(ReadResult::Slide {
                        number: slide.number,
                        title: slide.title.clone(),
                        content: slide.content.clone(),
                    })
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "slide {} not found",
                        n
                    )))
                }
            }
            Selector::Image(n) => {
                let idx = *n as usize;
                if idx < self.images.len() {
                    let (name, bytes) = &self.images[idx];
                    let fmt = if name.ends_with(".png") {
                        ImageFormat::Png
                    } else if name.ends_with(".jpg") || name.ends_with(".jpeg") {
                        ImageFormat::Jpeg
                    } else {
                        ImageFormat::Unknown
                    };
                    Ok(ReadResult::Image {
                        bytes: bytes.clone(),
                        format: fmt,
                        caption: Some(name.clone()),
                    })
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "image {} not found",
                        n
                    )))
                }
            }
            Selector::Paragraph(n) => {
                let idx = *n as usize;
                if idx < self.slides.len() {
                    Ok(ReadResult::Text(self.slides[idx].content.clone()))
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "paragraph {} not found",
                        n
                    )))
                }
            }
            Selector::Note(n) => {
                let idx = *n as usize;
                if idx < self.slides.len() {
                    Ok(ReadResult::Text(self.slides[idx].content.clone()))
                } else {
                    Err(DocumentError::InvalidSelector(format!(
                        "note {} not found",
                        n
                    )))
                }
            }
            Selector::Slice { skip, take } => {
                let s = *skip as usize;
                let t = *take as usize;
                if s >= self.slides.len() {
                    return Err(DocumentError::InvalidSelector(format!(
                        "skip {} beyond slide count {}",
                        skip,
                        self.slides.len()
                    )));
                }
                let end = std::cmp::min(s + t, self.slides.len());
                let texts: Vec<&str> = self.slides[s..end]
                    .iter()
                    .map(|sl| sl.content.as_str())
                    .collect();
                Ok(ReadResult::Text(texts.join("\n\n---\n\n")))
            }
            _ => Err(DocumentError::InvalidSelector(format!(
                "selector {:?} not supported for pptx documents",
                selector
            ))),
        }
    }
}

pub struct PptxFactory;

impl DocumentFactory for PptxFactory {
    fn open(&self, path: &str) -> Result<Box<dyn Document>, DocumentError> {
        PptxDocument::open(path).map(|d| Box::new(d) as Box<dyn Document>)
    }
}
