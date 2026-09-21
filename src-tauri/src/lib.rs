//! ebook-reader — Personal knowledge workspace
//!
//! MVP 1 scope: open EPUB/Markdown/PDF → TOC → reader → select → highlight
//!   → note → SQLite → restore position. CHƯA có AI/search/KG.
//!
//! Architecture: docs/ARCHITECTURE.md

pub mod adapter;
pub mod annotation;
pub mod epub;
pub mod markdown;
pub mod model;
pub mod pdf;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use crate::adapter::detect_format;
    use crate::model::DocumentFormat;
    use std::path::Path;

    #[test]
    fn detect_format_from_extension() {
        assert_eq!(
            detect_format(Path::new("book.epub")),
            Some(DocumentFormat::Epub)
        );
        assert_eq!(
            detect_format(Path::new("notes.md")),
            Some(DocumentFormat::Markdown)
        );
        assert_eq!(
            detect_format(Path::new("README.markdown")),
            Some(DocumentFormat::Markdown)
        );
        assert_eq!(
            detect_format(Path::new("book.pdf")),
            Some(DocumentFormat::Pdf)
        );
        assert_eq!(detect_format(Path::new("book.txt")), None);
        assert_eq!(detect_format(Path::new("noext")), None);
    }

    #[test]
    fn detect_format_is_case_insensitive() {
        assert_eq!(
            detect_format(Path::new("BOOK.EPUB")),
            Some(DocumentFormat::Epub)
        );
        assert_eq!(
            detect_format(Path::new("Book.PDF")),
            Some(DocumentFormat::Pdf)
        );
    }
}
