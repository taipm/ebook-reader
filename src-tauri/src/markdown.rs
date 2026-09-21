//! Markdown adapter — MVP 1 chưa implement, stub tương tự EPUB.

use crate::adapter::{DocumentAdapter, LoadedDocument};
use crate::model::DocumentFormat;
use anyhow::{anyhow, Result};
use std::path::Path;

pub struct MarkdownAdapter;

impl DocumentAdapter for MarkdownAdapter {
    fn format(&self) -> DocumentFormat {
        DocumentFormat::Markdown
    }

    fn load(&self, _path: &Path) -> Result<LoadedDocument> {
        Err(anyhow!("MarkdownAdapter::load not implemented yet (PR #3)"))
    }

    fn content_hash(&self, _path: &Path) -> Result<String> {
        Err(anyhow!(
            "MarkdownAdapter::content_hash not implemented yet (PR #3)"
        ))
    }
}
