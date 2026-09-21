//! EPUB adapter — load EPUB file → LoadedDocument.
//!
//! Dùng `epub` crate 2.x để parse EPUB 2.0/3.0.
//!
//! ## Pipeline
//!
//! 1. `EpubDoc::new(path)` → mở file
//! 2. `get_title()` → metadata.title
//! 3. Walk `doc.toc` (NavPoint tree) → build `Chapter` list recursive
//! 4. Với mỗi resource trong spine → `get_resource_str(idref)` → strip HTML → `Block`
//! 5. SHA256 toàn bộ file → `content_hash`
//!
//! ## Caveats
//!
//! - Chỉ strip HTML tags bằng parser đơn giản — KHÔNG xử lý entity phức tạp
//!   hay embedded CSS/JS. Đủ cho MVP 1 (highlight text). PR sau sẽ dùng
//!   `html5ever` nếu cần render phức tạp.
//! - NavPoint thiếu thì fallback sang spine (flat).

use crate::adapter::{DocumentAdapter, LoadedDocument};
use crate::model::{
    Block, BlockId, Chapter, ChapterId, DocumentFormat, DocumentMeta, DocumentId,
};
use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::path::Path;

/// Strip HTML tags thô, giữ text. Skip script/style. Block tags → newline.
fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());

    // Find positions of <script>...</script> and <style>...</style> blocks to skip
    let lower = html.to_ascii_lowercase();
    let mut skip_ranges: Vec<(usize, usize)> = Vec::new();
    for tag in ["<script", "<style"] {
        let end_tag = if tag == "<script" { "</script>" } else { "</style>" };
        let mut start = 0;
        while let Some(pos) = lower[start..].find(tag) {
            let abs = start + pos;
            if let Some(end_pos) = lower[abs..].find(end_tag) {
                skip_ranges.push((abs, abs + end_pos + end_tag.len()));
                start = abs + end_pos + end_tag.len();
            } else {
                break;
            }
        }
    }

    // Iterate by char (UTF-8 safe)
    let mut i = 0;
    while i < html.len() {
        // Skip ranges
        let in_skip = skip_ranges.iter().any(|(s, e)| i >= *s && i < *e);
        if in_skip {
            // Advance to end of skip range
            if let Some((_, e)) = skip_ranges.iter().find(|(s, e)| i >= *s && i < *e) {
                i = *e;
                continue;
            }
        }

        // Find next char boundary
        let ch = match html[i..].chars().next() {
            Some(c) => c,
            None => break,
        };
        let ch_len = ch.len_utf8();

        if ch == '<' {
            // Find end of tag
            if let Some(te) = html[i..].find('>') {
                let tag = &html[i + 1..i + te];
                let tag_lower = tag.to_ascii_lowercase();
                let first_word = tag_lower.split_whitespace().next().unwrap_or("");
                if matches!(
                    first_word,
                    "br" | "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li" | "tr"
                ) {
                    out.push('\n');
                }
                i += te + 1;
                continue;
            } else {
                break;
            }
        }
        out.push(ch);
        i += ch_len;
    }

    // Collapse whitespace (preserve newlines)
    let mut result = String::with_capacity(out.len());
    let mut prev_space = false;
    let mut prev_newline = false;
    for ch in result_collapsed(&out).chars() {
        if ch == '\n' {
            if !prev_newline {
                result.push('\n');
            }
            prev_newline = true;
            prev_space = false;
        } else if ch.is_whitespace() {
            if !prev_space && !prev_newline {
                result.push(' ');
            }
            prev_space = true;
        } else {
            result.push(ch);
            prev_space = false;
            prev_newline = false;
        }
    }
    result.trim().to_string()
}

fn result_collapsed(s: &str) -> String {
    s.to_string()
}

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

pub struct EpubAdapter;

impl DocumentAdapter for EpubAdapter {
    fn format(&self) -> DocumentFormat {
        DocumentFormat::Epub
    }

    fn load(&self, path: &Path) -> Result<LoadedDocument> {
        let file = fs::File::open(path)
            .with_context(|| format!("Failed to open EPUB file: {}", path.display()))?;
        let reader = BufReader::new(file);
        let mut doc = epub::doc::EpubDoc::from_reader(reader)
            .with_context(|| format!("Failed to parse EPUB: {}", path.display()))?;

        let title = doc.get_title().unwrap_or_else(|| "Untitled".to_string());
        let content_hash = self.content_hash(path)?;

        let mut chapters: Vec<Chapter> = Vec::new();
        let mut blocks: Vec<Block> = Vec::new();
        let mut block_index: HashMap<ChapterId, u32> = HashMap::new();

        let toc_snapshot = doc.toc.clone();
        walk_navpoints(
            &mut doc,
            &toc_snapshot,
            None,
            1,
            &mut chapters,
            &mut blocks,
            &mut block_index,
        );

        // Fallback: nếu toc rỗng, build flat từ spine
        if chapters.is_empty() {
            let n = doc.get_num_chapters();
            for i in 0..n {
                doc.set_current_chapter(i);
                if let Some((content, _mime)) = doc.get_current_str() {
                    let text = strip_html(&content);
                    if text.is_empty() {
                        continue;
                    }
                    let chapter_id = ChapterId::new();
                    chapters.push(Chapter {
                        id: chapter_id,
                        parent_id: None,
                        title: format!("Chapter {}", i + 1),
                        level: 1,
                        position: chapters.len() as u32,
                    });
                    blocks.push(Block {
                        id: BlockId::new(),
                        chapter_id,
                        text,
                        index_in_chapter: 0,
                    });
                }
            }
        }

        if chapters.is_empty() {
            return Err(anyhow!(
                "EPUB has no readable chapters: {}",
                path.display()
            ));
        }

        let meta = DocumentMeta {
            id: DocumentId::new(),
            title,
            author: doc.mdata("creator").map(|m| m.value.clone()),
            format: DocumentFormat::Epub,
            content_hash,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        Ok(LoadedDocument {
            meta,
            chapters,
            blocks,
        })
    }

    fn content_hash(&self, path: &Path) -> Result<String> {
        let bytes = fs::read(path)
            .with_context(|| format!("Failed to read EPUB file: {}", path.display()))?;
        Ok(sha256_hex(&bytes))
    }
}

fn walk_navpoints(
    doc: &mut epub::doc::EpubDoc<BufReader<fs::File>>,
    navs: &[epub::doc::NavPoint],
    parent_id: Option<ChapterId>,
    level: u8,
    chapters: &mut Vec<Chapter>,
    blocks: &mut Vec<Block>,
    block_index: &mut HashMap<ChapterId, u32>,
) {
    for nav in navs {
        let chapter_id = ChapterId::new();
        let position = chapters.len() as u32;

        chapters.push(Chapter {
            id: chapter_id,
            parent_id,
            title: nav.label.clone(),
            level,
            position,
        });

        // Resolve content path → spine index
        // Strip fragment (#...) vì NavPoint content thường có "#anchor"
        let content_path = nav
            .content
            .to_str()
            .and_then(|s| s.split('#').next())
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| nav.content.clone());
        if let Some(ch_idx) = doc.resource_uri_to_chapter(&content_path) {
            if doc.set_current_chapter(ch_idx) {
                if let Some((content, _mime)) = doc.get_current_str() {
                    let text = strip_html(&content);
                    if !text.is_empty() {
                        let idx = block_index.entry(chapter_id).or_insert(0);
                        blocks.push(Block {
                            id: BlockId::new(),
                            chapter_id,
                            text,
                            index_in_chapter: *idx,
                        });
                        *idx += 1;
                    }
                }
            }
        }

        if !nav.children.is_empty() {
            walk_navpoints(
                doc,
                &nav.children,
                Some(chapter_id),
                level.saturating_add(1),
                chapters,
                blocks,
                block_index,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_removes_tags() {
        assert_eq!(strip_html("<p>Hello <b>world</b></p>").trim(), "Hello world");
        assert_eq!(
            strip_html("<h1>Title</h1><p>Para 1</p><p>Para 2</p>").trim(),
            "Title\nPara 1\nPara 2"
        );
    }

    #[test]
    fn strip_html_skips_script_and_style() {
        let html = "<style>body { color: red }</style><p>Visible</p>";
        let out = strip_html(html).trim().to_string();
        eprintln!("strip_html output: {:?}", out);
        assert!(out.contains("Visible"));
        assert!(!out.contains("color"));
    }

    #[test]
    fn strip_html_handles_unclosed_tag() {
        let html = "<p>broken<span>no end";
        let out = strip_html(html);
        assert!(out.contains("broken"));
    }

    #[test]
    fn sha256_known_value() {
        // SHA256("abc") = ba7816bf...
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn load_real_epub_dracula() {
        let test_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../test-data/dracula.epub");
        if !test_path.exists() {
            eprintln!("SKIP: test-data/dracula.epub not found");
            return;
        }
        let adapter = EpubAdapter;
        let doc = adapter
            .load(&test_path)
            .expect("Failed to load dracula.epub");
        assert!(!doc.meta.title.is_empty(), "Title should not be empty");
        assert!(
            !doc.chapters.is_empty(),
            "Dracula should have > 0 chapters"
        );
        assert!(
            !doc.blocks.is_empty(),
            "Dracula should have > 0 blocks (content extracted)"
        );
        assert_eq!(doc.meta.format, DocumentFormat::Epub);

        eprintln!(
            "✓ Dracula: title='{}', {} chapters, {} blocks",
            doc.meta.title,
            doc.chapters.len(),
            doc.blocks.len()
        );
        for ch in doc.chapters.iter().take(5) {
            eprintln!("  - [L{}] {}", ch.level, ch.title);
        }
        // Print first block text (truncated)
        if let Some(first_block) = doc.blocks.first() {
            let preview: String = first_block.text.chars().take(200).collect();
            eprintln!(
                "  - First block ({} chars): {}",
                first_block.text.len(),
                preview
            );
        }
    }

    #[test]
    fn content_hash_is_stable() {
        let test_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../test-data/dracula.epub");
        if !test_path.exists() {
            return;
        }
        let adapter = EpubAdapter;
        let h1 = adapter.content_hash(&test_path).unwrap();
        let h2 = adapter.content_hash(&test_path).unwrap();
        assert_eq!(h1, h2, "Content hash must be deterministic");
        assert_eq!(h1.len(), 64, "SHA256 hex = 64 chars");
    }
}

