use crate::ocean_fs::types::{FileCategory, FileMeta, NormalizedFile};

const MIME_MAPPINGS: &[(&str, &str, &str)] = &[
    ("pdf", "application/pdf", "Document"),
    ("docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "Document"),
    ("xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "Spreadsheet"),
    ("pptx", "application/vnd.openxmlformats-officedocument.presentationml.presentation", "Presentation"),
    ("png", "image/png", "Image"),
    ("jpg", "image/jpeg", "Image"),
    ("jpeg", "image/jpeg", "Image"),
    ("txt", "text/plain", "Text"),
    ("md", "text/markdown", "Text"),
    ("html", "text/html", "Text"),
    ("htm", "text/html", "Text"),
];

pub fn normalize(meta: FileMeta) -> NormalizedFile {
    let ext = meta.extension.to_lowercase();
    let mime_type = MIME_MAPPINGS
        .iter()
        .find(|(e, _, _)| *e == ext)
        .map(|(_, m, _)| m.to_string())
        .unwrap_or_else(|| {
            mime_guess::from_ext(&ext)
                .first_or_octet_stream()
                .to_string()
        });

    let category = FileCategory::from_extension(&ext);

    NormalizedFile {
        id: meta.id.clone(),
        meta,
        mime_type,
        category,
    }
}
