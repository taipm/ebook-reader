//! PDF adapter — MVP 1 chưa implement, stub tương tự EPUB/Markdown.
//!
//! PR #4 sẽ implement dùng `pdfium-render` (nặng nhất trong 3 adapter vì
//! phải xử lý cả text-based và scan-based PDF).

use crate::adapter::{DocumentAdapter, LoadedDocument};
use crate::model::DocumentFormat;
use anyhow::{anyhow, Result};
use std::path::Path;

pub struct PdfAdapter;

impl DocumentAdapter for PdfAdapter {
    fn format(&self) -> DocumentFormat {
        DocumentFormat::Pdf
    }

    fn load(&self, _path: &Path) -> Result<LoadedDocument> {
        Err(anyhow!("PdfAdapter::load not implemented yet (PR #4)"))
    }

    fn content_hash(&self, _path: &Path) -> Result<String> {
        Err(anyhow!(
            "PdfAdapter::content_hash not implemented yet (PR #4)"
        ))
    }
}
