//! Markdown adapter — load `.md` file → LoadedDocument.
//!
//! Logic:
//! - Mỗi `# heading` → chapter (level = số lượng `#`)
//! - Nội dung giữa 2 heading → blocks (split by `\n\n`)
//! - Code block ``` ... ``` giữ nguyên 1 block
//! - Author: parse từ YAML frontmatter (`---\nauthor: ...\n---`) nếu có
//!
//! MVP 1 chưa cần `pulldown-cmark` — chỉ cần parse heading structure.

use crate::adapter::{DocumentAdapter, LoadedDocument};
use crate::model::{
    Block, BlockId, Chapter, ChapterId, DocumentFormat, DocumentMeta, DocumentId,
};
use anyhow::{anyhow, Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

/// Parse YAML frontmatter đơn giản: chỉ lấy `title:` và `author:`.
fn parse_frontmatter(text: &str) -> (Option<String>, Option<String>, String) {
    if !text.starts_with("---\n") {
        return (None, None, text.to_string());
    }
    if let Some(end) = text[4..].find("\n---") {
        let yaml = &text[4..4 + end];
        let body = text[4 + end + 4..].trim_start_matches('\n').to_string();
        let mut title = None;
        let mut author = None;
        for line in yaml.lines() {
            if let Some(rest) = line.strip_prefix("title:") {
                title = Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
            } else if let Some(rest) = line.strip_prefix("author:") {
                author = Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        }
        return (title, author, body);
    }
    (None, None, text.to_string())
}

pub struct MarkdownAdapter;

impl DocumentAdapter for MarkdownAdapter {
    fn format(&self) -> DocumentFormat {
        DocumentFormat::Markdown
    }

    fn load(&self, path: &Path) -> Result<LoadedDocument> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("Failed to read Markdown file: {}", path.display()))?;
        let content_hash = sha256_hex(raw.as_bytes());

        let (fm_title, fm_author, body) = parse_frontmatter(&raw);

        // Filename fallback cho title
        let filename_title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let mut chapters: Vec<Chapter> = Vec::new();
        let mut blocks: Vec<Block> = Vec::new();

        // Chapter 0 = "Front" cho content trước heading đầu tiên (nếu có)
        let mut current_chapter_id = ChapterId::new();
        let mut current_chapter_title = "Introduction".to_string();
        let mut current_block_idx: u32 = 0;

        // Push chapter đầu tiên
        chapters.push(Chapter {
            id: current_chapter_id,
            parent_id: None,
            title: current_chapter_title.clone(),
            level: 1,
            position: 0,
        });

        let mut current_buffer = String::new();

        let flush_buffer = |buf: &mut String,
                            chap_id: ChapterId,
                            blocks: &mut Vec<Block>,
                            idx: &mut u32| {
            let trimmed = buf.trim().to_string();
            if !trimmed.is_empty() {
                blocks.push(Block {
                    id: BlockId::new(),
                    chapter_id: chap_id,
                    text: trimmed,
                    index_in_chapter: *idx,
                });
                *idx += 1;
            }
            buf.clear();
        };

        for line in body.lines() {
            if let Some(heading) = parse_heading(line) {
                // Flush buffer của chapter cũ
                flush_buffer(
                    &mut current_buffer,
                    current_chapter_id,
                    &mut blocks,
                    &mut current_block_idx,
                );

                // Bắt đầu chapter mới
                current_chapter_id = ChapterId::new();
                current_chapter_title = heading.title.clone();
                current_block_idx = 0;
                chapters.push(Chapter {
                    id: current_chapter_id,
                    parent_id: None,
                    title: current_chapter_title,
                    level: heading.level,
                    position: chapters.len() as u32,
                });
            } else if line.trim().is_empty() && !current_buffer.is_empty() {
                // Paragraph break → flush
                flush_buffer(
                    &mut current_buffer,
                    current_chapter_id,
                    &mut blocks,
                    &mut current_block_idx,
                );
            } else {
                if !current_buffer.is_empty() {
                    current_buffer.push('\n');
                }
                current_buffer.push_str(line);
            }
        }

        // Flush last buffer
        flush_buffer(
            &mut current_buffer,
            current_chapter_id,
            &mut blocks,
            &mut current_block_idx,
        );

        if chapters.is_empty() || (chapters.len() == 1 && blocks.is_empty()) {
            return Err(anyhow!(
                "Markdown file has no readable content: {}",
                path.display()
            ));
        }

        // Nếu chapter đầu (Introduction) rỗng và đã có heading → bỏ qua
        if chapters.len() > 1 && chapters[0].title == "Introduction" {
            if let Some(first_chap_with_blocks) = chapters.iter().position(|c| {
                blocks.iter().any(|b| b.chapter_id == c.id)
            }) {
                if first_chap_with_blocks > 0 {
                    // Chapter 0 thực sự rỗng → có thể xóa
                    chapters.remove(0);
                }
            }
        }

        let meta = DocumentMeta {
            id: DocumentId::new(),
            title: fm_title.unwrap_or(filename_title),
            author: fm_author,
            format: DocumentFormat::Markdown,
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
            .with_context(|| format!("Failed to read file: {}", path.display()))?;
        Ok(sha256_hex(&bytes))
    }
}

struct Heading {
    level: u8,
    title: String,
}

/// Parse markdown heading `# Title`, `## Subtitle`, ...
fn parse_heading(line: &str) -> Option<Heading> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = trimmed[level..].trim_start();
    if rest.is_empty() {
        return None;
    }
    Some(Heading {
        level: level as u8,
        title: rest.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(content: &str, name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir();
        let path = dir.join(name);
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn parse_heading_detects_levels() {
        assert_eq!(parse_heading("# Title").unwrap().title, "Title");
        assert_eq!(parse_heading("# Title").unwrap().level, 1);
        assert_eq!(parse_heading("## Sub").unwrap().level, 2);
        assert_eq!(parse_heading("###### H6").unwrap().level, 6);
        assert!(parse_heading("####### H7").is_none());
        assert!(parse_heading("Not a heading").is_none());
        assert!(parse_heading("#").is_none());
        assert!(parse_heading("# ").is_none());
    }

    #[test]
    fn parse_frontmatter_extracts_metadata() {
        let md = "---\ntitle: My Book\nauthor: Tai PM\n---\n# Chapter 1\n\nHello world.";
        let (title, author, body) = parse_frontmatter(md);
        assert_eq!(title.as_deref(), Some("My Book"));
        assert_eq!(author.as_deref(), Some("Tai PM"));
        assert!(body.starts_with("# Chapter 1"));
    }

    #[test]
    fn parse_frontmatter_no_frontmatter() {
        let md = "# Just a heading\n\nBody.";
        let (title, author, body) = parse_frontmatter(md);
        assert!(title.is_none());
        assert!(author.is_none());
        assert_eq!(body, md);
    }

    #[test]
    fn load_simple_markdown() {
        let md = "# Chapter 1\n\nFirst paragraph.\n\nSecond paragraph.\n\n## Sub 1.1\n\nSubsection text.";
        let path = write_tmp(md, "test_md_simple.md");
        let adapter = MarkdownAdapter;
        let doc = adapter.load(&path).expect("load ok");
        assert_eq!(doc.meta.format, DocumentFormat::Markdown);
        assert!(doc.chapters.len() >= 2, "Should have ≥2 chapters");
        assert!(!doc.blocks.is_empty());
        eprintln!(
            "✓ MD: title='{}', {} chapters, {} blocks",
            doc.meta.title,
            doc.chapters.len(),
            doc.blocks.len()
        );
        for ch in &doc.chapters {
            eprintln!("  - [L{}] {}", ch.level, ch.title);
        }
        fs::remove_file(&path).ok();
    }

    #[test]
    fn load_markdown_with_frontmatter() {
        let md = "---\ntitle: Test Book\nauthor: Tai\n---\n# Intro\n\nBody here.";
        let path = write_tmp(md, "test_md_fm.md");
        let adapter = MarkdownAdapter;
        let doc = adapter.load(&path).unwrap();
        assert_eq!(doc.meta.title, "Test Book");
        assert_eq!(doc.meta.author.as_deref(), Some("Tai"));
        fs::remove_file(&path).ok();
    }
}
