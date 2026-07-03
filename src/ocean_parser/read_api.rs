use crate::ocean_parser::{Document, DocumentError, DocumentFormat, ReadResult, Selector};

pub fn read(document: &dyn Document, selector: &Selector) -> Result<ReadResult, DocumentError> {
    document.read(selector)
}

pub fn read_page(document: &dyn Document, page: u32) -> Result<ReadResult, DocumentError> {
    document.read(&Selector::Page(page))
}

pub fn read_pages(
    document: &dyn Document,
    pages: Vec<u32>,
) -> Result<ReadResult, DocumentError> {
    document.read(&Selector::Pages(pages))
}

pub fn read_sheet(document: &dyn Document, sheet: &str) -> Result<ReadResult, DocumentError> {
    document.read(&Selector::Sheet(sheet.to_string()))
}

pub fn read_slide(document: &dyn Document, slide: u32) -> Result<ReadResult, DocumentError> {
    document.read(&Selector::Slide(slide))
}

pub fn read_cell(document: &dyn Document, cell: &str) -> Result<ReadResult, DocumentError> {
    document.read(&Selector::Cell(cell.to_string()))
}

pub fn read_all_blocks(doc: &dyn Document) -> Result<Vec<ReadResult>, DocumentError> {
    let meta = doc.metadata();
    let mut blocks = Vec::new();

    match meta.format {
        DocumentFormat::Pdf | DocumentFormat::Docx => {
            let n = meta.page_count.unwrap_or(0);
            for i in 1..=n {
                if let Ok(page) = doc.read(&Selector::Page(i)) {
                    blocks.push(page);
                }
            }
        }
        DocumentFormat::Pptx => {
            let n = meta.page_count.unwrap_or(0);
            for i in 1..=n {
                if let Ok(slide) = doc.read(&Selector::Slide(i)) {
                    blocks.push(slide);
                }
            }
        }
        DocumentFormat::Xlsx
        | DocumentFormat::Html
        | DocumentFormat::Text
        | DocumentFormat::Markdown => {
            if let Ok(text) = doc.read(&Selector::Slice { skip: 0, take: u32::MAX }) {
                blocks.push(text);
            }
        }
    }

    if blocks.is_empty() {
        if let Ok(result) = doc.read(&Selector::Slice { skip: 0, take: u32::MAX }) {
            blocks.push(result);
        }
    }

    Ok(blocks)
}
