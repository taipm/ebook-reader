//! DocumentAdapter trait — interface chung cho mọi format.
//!
//! Mỗi format (Epub/Pdf/Markdown) implement trait này. Reader engine chỉ
//! biết đến `Box<dyn DocumentAdapter>` — không cần biết format gốc.

use crate::model::{Block, Chapter, DocumentFormat, DocumentMeta};
use anyhow::Result;
use std::path::Path;

/// Output của việc load document: metadata + cây chapter + danh sách block.
///
/// `blocks` ở đây là **toàn bộ** blocks của document (đã render thành plain
/// text). Với EPUB/Markdown thường vài nghìn blocks — OK in-memory. Với PDF
/// lớn (1000 trang = ~50K blocks) sẽ cần lazy load sau — Phase 2.
#[derive(Debug, Clone)]
pub struct LoadedDocument {
    pub meta: DocumentMeta,
    pub chapters: Vec<Chapter>,
    pub blocks: Vec<Block>,
}

/// Mỗi adapter phải:
///
/// 1. Parse file gốc → `LoadedDocument`
/// 2. Hash content (SHA256) — dùng detect "cùng một cuốn" khi import lại
/// 3. Map text selection trong UI (char offset trong rendered text)
///    → `CanonicalLocation` qua helper `resolve_location`
///
/// MVP 1 chỉ yêu cầu (1) và (2). (3) sẽ implement chi tiết khi có UI thật.
pub trait DocumentAdapter: Send + Sync {
    /// Format mà adapter này xử lý.
    fn format(&self) -> DocumentFormat;

    /// Load document từ file path.
    fn load(&self, path: &Path) -> Result<LoadedDocument>;

    /// Hash toàn bộ text content (dùng cho content_hash trong metadata).
    /// Implement SHA256 để ổn định qua re-import cùng file.
    fn content_hash(&self, path: &Path) -> Result<String>;
}

/// Helper: chọn adapter theo format.
pub fn adapter_for(format: DocumentFormat) -> Box<dyn DocumentAdapter> {
    match format {
        DocumentFormat::Epub => Box::new(crate::epub::EpubAdapter),
        DocumentFormat::Markdown => Box::new(crate::markdown::MarkdownAdapter),
        DocumentFormat::Pdf => Box::new(crate::pdf::PdfAdapter),
    }
}

/// Detect format từ file extension.
pub fn detect_format(path: &Path) -> Option<DocumentFormat> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        "epub" => Some(DocumentFormat::Epub),
        "md" | "markdown" => Some(DocumentFormat::Markdown),
        "pdf" => Some(DocumentFormat::Pdf),
        _ => None,
    }
}
