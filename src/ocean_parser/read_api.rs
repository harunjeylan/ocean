use crate::ocean_parser::{Document, DocumentError, ReadResult, Selector};

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

pub fn read_heading(
    document: &dyn Document,
    heading: &str,
) -> Result<ReadResult, DocumentError> {
    document.read(&Selector::Heading(heading.to_string()))
}

pub fn read_paragraph(
    document: &dyn Document,
    paragraph: u32,
) -> Result<ReadResult, DocumentError> {
    document.read(&Selector::Paragraph(paragraph))
}

pub fn read_table(document: &dyn Document, table: u32) -> Result<ReadResult, DocumentError> {
    document.read(&Selector::Table(table))
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

pub fn read_image(document: &dyn Document, image: u32) -> Result<ReadResult, DocumentError> {
    document.read(&Selector::Image(image))
}

pub fn read_notes(document: &dyn Document) -> Result<ReadResult, DocumentError> {
    document.read(&Selector::Note(0))
}

pub fn read_range(
    document: &dyn Document,
    start: usize,
    end: usize,
) -> Result<ReadResult, DocumentError> {
    document.read(&Selector::Range { start, end })
}
