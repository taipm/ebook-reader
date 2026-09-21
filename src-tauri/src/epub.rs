//! EPUB adapter — MVP 1 chưa implement, chỉ stub + test format detection.
//!
//! PR #2 sẽ implement dùng `epub` crate (Rust). Stub ở đây để verify:
//! - Adapter trait compile được cho cả 3 format
//! - detect_format phân biệt đúng `.epub` / `.md` / `.pdf`
//! - Tauri build pass (không thiếu symbol)

use crate::adapter::{DocumentAdapter, LoadedDocument};
use crate::model::DocumentFormat;
use anyhow::{anyhow, Result};
use std::path::Path;

pub struct EpubAdapter;

impl DocumentAdapter for EpubAdapter {
    fn format(&self) -> DocumentFormat {
        DocumentFormat::Epub
    }

    fn load(&self, _path: &Path) -> Result<LoadedDocument> {
        Err(anyhow!("EpubAdapter::load not implemented yet (PR #2)"))
    }

    fn content_hash(&self, _path: &Path) -> Result<String> {
        Err(anyhow!(
            "EpubAdapter::content_hash not implemented yet (PR #2)"
        ))
    }
}
