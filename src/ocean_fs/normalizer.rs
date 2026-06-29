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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meta(extension: &str) -> FileMeta {
        FileMeta {
            id: "test-id".to_string(),
            path: format!("/path/to/file.{}", extension),
            hash: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string(),
            size: 1024,
            modified: 1700000000000,
            extension: extension.to_string(),
        }
    }

    #[test]
    fn test_normalize_document_pdf() {
        let normalized = normalize(make_meta("pdf"));
        assert_eq!(normalized.category, FileCategory::Document);
        assert_eq!(normalized.mime_type, "application/pdf");
    }

    #[test]
    fn test_normalize_document_docx() {
        let normalized = normalize(make_meta("docx"));
        assert_eq!(normalized.category, FileCategory::Document);
        assert_eq!(
            normalized.mime_type,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        );
    }

    #[test]
    fn test_normalize_spreadsheet() {
        let normalized = normalize(make_meta("xlsx"));
        assert_eq!(normalized.category, FileCategory::Spreadsheet);
        assert_eq!(
            normalized.mime_type,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
    }

    #[test]
    fn test_normalize_presentation() {
        let normalized = normalize(make_meta("pptx"));
        assert_eq!(normalized.category, FileCategory::Presentation);
        assert_eq!(
            normalized.mime_type,
            "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        );
    }

    #[test]
    fn test_normalize_image_png() {
        let normalized = normalize(make_meta("png"));
        assert_eq!(normalized.category, FileCategory::Image);
        assert_eq!(normalized.mime_type, "image/png");
    }

    #[test]
    fn test_normalize_image_jpg() {
        let normalized = normalize(make_meta("jpg"));
        assert_eq!(normalized.category, FileCategory::Image);
        assert_eq!(normalized.mime_type, "image/jpeg");
    }

    #[test]
    fn test_normalize_image_jpeg() {
        let normalized = normalize(make_meta("jpeg"));
        assert_eq!(normalized.category, FileCategory::Image);
        assert_eq!(normalized.mime_type, "image/jpeg");
    }

    #[test]
    fn test_normalize_text_txt() {
        let normalized = normalize(make_meta("txt"));
        assert_eq!(normalized.category, FileCategory::Text);
        assert_eq!(normalized.mime_type, "text/plain");
    }

    #[test]
    fn test_normalize_text_md() {
        let normalized = normalize(make_meta("md"));
        assert_eq!(normalized.category, FileCategory::Text);
        assert_eq!(normalized.mime_type, "text/markdown");
    }

    #[test]
    fn test_normalize_text_html() {
        let normalized = normalize(make_meta("html"));
        assert_eq!(normalized.category, FileCategory::Text);
        assert_eq!(normalized.mime_type, "text/html");
    }

    #[test]
    fn test_normalize_unknown_extension() {
        let normalized = normalize(make_meta("xyz"));
        assert_eq!(normalized.category, FileCategory::Unknown);
    }

    #[test]
    fn test_normalize_preserves_id() {
        let meta = make_meta("pdf");
        let normalized = normalize(meta.clone());
        assert_eq!(normalized.id, meta.id);
    }

    #[test]
    fn test_normalize_preserves_meta_fields() {
        let meta = make_meta("txt");
        let normalized = normalize(meta.clone());
        assert_eq!(normalized.meta.hash, meta.hash);
        assert_eq!(normalized.meta.size, meta.size);
        assert_eq!(normalized.meta.path, meta.path);
    }

    #[test]
    fn test_normalize_case_insensitive() {
        let normalized = normalize(make_meta("PDF"));
        assert_eq!(normalized.category, FileCategory::Document);
        assert_eq!(normalized.mime_type, "application/pdf");
    }
}
