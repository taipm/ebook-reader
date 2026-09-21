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
use crate::model::{Block, DocumentFormat};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;

/// App state — single currently-loaded document (MVP 1 đơn giản).
#[derive(Default)]
pub struct AppState {
    pub current: Mutex<Option<LoadedDocument>>,
}

/// Response trả về frontend sau khi open document.
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
    let format =
        detect_format(&path_buf).ok_or_else(|| format!("Unsupported file format: {}", path))?;

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

    *state.current.lock().unwrap() = Some(loaded);

    Ok(response)
}

#[tauri::command]
fn get_chapter_blocks(
    chapter_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Block>, String> {
    let guard = state.current.lock().unwrap();
    let doc = guard
        .as_ref()
        .ok_or_else(|| "No document loaded".to_string())?;
    let target: uuid::Uuid = chapter_id
        .parse()
        .map_err(|e| format!("Invalid chapter id: {e}"))?;
    let blocks: Vec<Block> = doc
        .blocks
        .iter()
        .filter(|b| b.chapter_id.0 == target)
        .cloned()
        .collect();
    Ok(blocks)
}

/// Metadata cho 1 cuốn sách trong library.
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct BundledBook {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub format: String,
    pub path: String,
    pub size_bytes: u64,
}

/// Scan directory và trả về danh sách BundledBook (pure function, testable).
/// Skip files không detect được format (vd .DS_Store, README.md).
pub fn list_books_in_dir(dir: &std::path::Path) -> Result<Vec<BundledBook>, String> {
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut books = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read_dir failed: {e}"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(format) = detect_format(&path) else {
            continue;
        };
        let metadata = entry.metadata().ok();
        let size_bytes = metadata.map(|m| m.len()).unwrap_or(0);
        let id = entry.file_name().to_string_lossy().to_string();

        let (title, author) = match format {
            DocumentFormat::Epub => {
                let file = std::fs::File::open(&path).ok();
                let mut title = id.clone();
                let mut author = None;
                if let Some(file) = file {
                    let reader = std::io::BufReader::new(file);
                    if let Ok(doc) = ::epub::doc::EpubDoc::from_reader(reader) {
                        if let Some(t) = doc.get_title() {
                            title = t;
                        }
                        author = doc.mdata("creator").map(|m| m.value.clone());
                    }
                }
                (title, author)
            }
            DocumentFormat::Markdown => {
                let title = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string();
                (title, None)
            }
            DocumentFormat::Pdf => (id.clone(), None),
        };

        books.push(BundledBook {
            id,
            title,
            author,
            format: format.to_string(),
            path: path.to_string_lossy().to_string(),
            size_bytes,
        });
    }

    books.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(books)
}

/// Resolve path tới bundled-books directory (cho cả dev và prod).
pub fn bundled_books_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../bundled-books")
}

/// List tất cả file trong `bundled-books/` (dev) hoặc Tauri resource dir (prod).
#[tauri::command]
fn list_bundled_books(app: tauri::AppHandle) -> Result<Vec<BundledBook>, String> {
    use tauri::Manager;

    let dir = if let Ok(resource_dir) = app.path().resource_dir() {
        let candidate = resource_dir.join("bundled-books");
        if candidate.exists() {
            candidate
        } else {
            bundled_books_dir()
        }
    } else {
        bundled_books_dir()
    };

    list_books_in_dir(&dir)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            open_document,
            get_chapter_blocks,
            list_bundled_books
        ])
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
