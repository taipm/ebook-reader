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

use crate::adapter::{adapter_for, detect_format, LoadedDocument};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;

/// App state — single currently-loaded document (MVP 1 đơn giản).
#[derive(Default)]
pub struct AppState {
    pub current: Mutex<Option<LoadedDocument>>,
}

/// Response trả về frontend sau khi open document.
/// Serialize LoadedDocument metadata + summary (chapters + block count)
/// thay vì full content (blocks to vài MB).
#[derive(Debug, Serialize)]
pub struct OpenDocumentResponse {
    pub meta: crate::model::DocumentMeta,
    pub chapters: Vec<crate::model::Chapter>,
    pub block_count: usize,
    pub total_chars: usize,
}

#[tauri::command]
fn open_document(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<OpenDocumentResponse, String> {
    let path_buf = PathBuf::from(&path);
    let format = detect_format(&path_buf).ok_or_else(|| {
        format!("Unsupported file format: {}", path)
    })?;

    let loaded = adapter_for(format)
        .load(&path_buf)
        .map_err(|e| format!("Failed to load document: {e}"))?;

    let block_count = loaded.blocks.len();
    let total_chars: usize = loaded.blocks.iter().map(|b| b.text.len()).sum();

    let response = OpenDocumentResponse {
        meta: loaded.meta.clone(),
        chapters: loaded.chapters.clone(),
        block_count,
        total_chars,
    };

    // Lưu vào state để dùng sau (cho highlight/note sau này)
    *state.current.lock().unwrap() = Some(loaded);

    Ok(response)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![open_document])
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

