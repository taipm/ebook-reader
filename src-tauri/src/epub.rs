//! EPUB adapter — load EPUB file → LoadedDocument.
//!
//! Dùng `epub` crate 2.x để parse EPUB 2.0/3.0.
//!
//! ## Pipeline
//!
//! 1. `EpubDoc::new(path)` → mở file
//! 2. `get_title()` → metadata.title
//! 3. Walk `doc.toc` (NavPoint tree) → build `Chapter` list recursive; navpoint
//!    đầu tiên trỏ tới một resource là chủ sở hữu spine item đó
//! 4. Duyệt spine theo thứ tự → strip HTML → `Block`; spine item không có trong
//!    TOC gán vào chapter gần nhất phía trước (chưa có → "Front matter")
//! 5. SHA256 toàn bộ file → `content_hash`
//!
//! ## Caveats
//!
//! - Strip HTML bằng parser đơn giản: bỏ tag/comment/script/style, decode
//!   entity thông dụng + numeric. Đủ cho MVP 1 (highlight text). PR sau sẽ dùng
//!   `html5ever` nếu cần render phức tạp.
//! - TOC rỗng → mọi spine item vào một chapter "Chapter 1" (flat).
//! - Nhiều navpoint cùng resource (khác #anchor) → chỉ navpoint đầu có nội dung.

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

/// Strip HTML tags thô, giữ text. Skip script/style/comment. Block tags → newline.
/// Decode entity thông dụng + numeric. `<` chỉ là tag khi ký tự sau là [A-Za-z/!].
fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let lower = html.to_ascii_lowercase();
    let mut i = 0;
    while i < html.len() {
        let rest = &html[i..];
        let ch = match rest.chars().next() {
            Some(c) => c,
            None => break,
        };

        if ch == '<' {
            let next = rest[1..].chars().next();
            let is_tag = matches!(next, Some(c) if c.is_ascii_alphabetic() || c == '/' || c == '!' || c == '?');
            if !is_tag {
                out.push('<');
                i += 1;
                continue;
            }
            if rest.starts_with("<?") {
                i = match rest.find("?>") {
                    Some(e) => i + e + 2,
                    None => html.len(),
                };
                continue;
            }
            if rest.starts_with("<!--") {
                i = match rest.find("-->") {
                    Some(e) => i + e + 3,
                    None => html.len(),
                };
                continue;
            }
            if let Some((_, close)) = [("<script", "</script>"), ("<style", "</style>"), ("<head", "</head>")]
                .iter()
                .find(|(open, _)| {
                    lower[i..].starts_with(open)
                        && lower[i + open.len()..]
                            .chars()
                            .next()
                            .is_some_and(|c| c == '>' || c.is_whitespace())
                })
            {
                i = match lower[i..].find(close) {
                    Some(e) => i + e + close.len(),
                    None => html.len(),
                };
                continue;
            }
            let Some(te) = rest.find('>') else {
                // unclosed tag: drop it, keep nothing after (malformed tail)
                break;
            };
            let tag_lower = &lower[i + 1..i + te];
            let first_word = tag_lower
                .trim_start_matches('/')
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or("");
            if matches!(
                first_word,
                "br" | "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li" | "tr"
            ) {
                out.push('\n');
            }
            i += te + 1;
            continue;
        }

        if ch == '&' {
            if let Some((decoded, len)) = decode_entity(rest) {
                out.push(decoded);
                i += len;
                continue;
            }
        }

        out.push(ch);
        i += ch.len_utf8();
    }

    // Collapse whitespace (preserve newlines)
    let mut result = String::with_capacity(out.len());
    let mut prev_space = false;
    let mut prev_newline = false;
    for ch in out.chars() {
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

/// Decode 1 entity ở đầu `s` (bắt đầu bằng '&'). Trả (char, số byte đã ăn).
fn decode_entity(s: &str) -> Option<(char, usize)> {
    let semi = s[..s.len().min(12)].find(';')?;
    let body = &s[1..semi];
    let ch = match body {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => ' ',
        _ => {
            let code = body.strip_prefix('#')?;
            let n = match code.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => code.parse::<u32>().ok()?,
            };
            char::from_u32(n)?
        }
    };
    Some((ch, semi + 1))
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

        // 1. TOC → chapters (giữ cây), mỗi navpoint resolve về spine index.
        let mut chapters: Vec<Chapter> = Vec::new();
        let mut spine_owner: HashMap<usize, ChapterId> = HashMap::new();
        let toc_snapshot = doc.toc.clone();
        walk_navpoints(&mut doc, &toc_snapshot, None, 1, &mut chapters, &mut spine_owner);

        // 2. Spine theo thứ tự → blocks. Spine item không có trong TOC gán vào
        //    chapter gần nhất phía trước; chưa có chapter nào → "Front matter".
        //    TOC rỗng → mỗi spine item một chapter "Chapter N" (flat).
        //    Navpoint trùng resource (khác #anchor) → chapter rỗng (không cắt anchor).
        let mut blocks: Vec<Block> = Vec::new();
        let mut block_index: HashMap<ChapterId, u32> = HashMap::new();
        let mut current: Option<ChapterId> = None;
        for i in 0..doc.get_num_chapters() {
            if let Some(owner) = spine_owner.get(&i) {
                current = Some(*owner);
            }
            if !doc.set_current_chapter(i) {
                continue;
            }
            let Some((content, _mime)) = doc.get_current_str() else {
                continue;
            };
            let text = strip_html(&content);
            if text.is_empty() {
                continue;
            }
            let chapter_id = match current {
                Some(id) if !toc_snapshot.is_empty() => id,
                _ => {
                    let id = ChapterId::new();
                    let title = if toc_snapshot.is_empty() {
                        format!("Chapter {}", chapters.len() + 1)
                    } else {
                        "Front matter".to_string()
                    };
                    chapters.push(Chapter {
                        id,
                        parent_id: None,
                        title,
                        level: 1,
                        position: 0,
                    });
                    current = Some(id);
                    id
                }
            };
            // Mỗi dòng (strip_html chèn '\n' tại <p>/<h*>/<li>…) là một block riêng
            // để reader hiển thị đúng đoạn và offset highlight ngắn, ổn định.
            let idx = block_index.entry(chapter_id).or_insert(0);
            for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
                blocks.push(Block {
                    id: BlockId::new(),
                    chapter_id,
                    text: line.to_string(),
                    index_in_chapter: *idx,
                });
                *idx += 1;
            }
        }
        // "Front matter" được push sau TOC nhưng phải đứng đầu
        if let Some(fm) = chapters.iter().position(|c| c.title == "Front matter" && c.parent_id.is_none()) {
            let ch = chapters.remove(fm);
            chapters.insert(0, ch);
        }
        for (pos, ch) in chapters.iter_mut().enumerate() {
            ch.position = pos as u32;
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
    spine_owner: &mut HashMap<usize, ChapterId>,
) {
    for nav in navs {
        let chapter_id = ChapterId::new();
        chapters.push(Chapter {
            id: chapter_id,
            parent_id,
            title: nav.label.clone(),
            level,
            position: chapters.len() as u32,
        });

        // Strip fragment (#...) vì NavPoint content thường có "#anchor"
        let content_path = nav
            .content
            .to_str()
            .and_then(|s| s.split('#').next())
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| nav.content.clone());
        if let Some(ch_idx) = doc.resource_uri_to_chapter(&content_path) {
            // navpoint đầu tiên trỏ tới resource là chủ sở hữu
            spine_owner.entry(ch_idx).or_insert(chapter_id);
        }

        if !nav.children.is_empty() {
            walk_navpoints(
                doc,
                &nav.children,
                Some(chapter_id),
                level.saturating_add(1),
                chapters,
                spine_owner,
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
    fn strip_html_decodes_entities() {
        assert_eq!(strip_html("GROSSET &amp; DUNLAP"), "GROSSET & DUNLAP");
        assert_eq!(strip_html("a&lt;b&gt;c &quot;q&quot; &apos;s&apos;"), "a<b>c \"q\" 's'");
        assert_eq!(strip_html("x&nbsp;y"), "x y");
        assert_eq!(strip_html("it&#8217;s &#x41;"), "it’s A");
        // unknown / malformed stay literal
        assert_eq!(strip_html("&bogus; & &#zz;"), "&bogus; & &#zz;");
    }

    #[test]
    fn strip_html_literal_lt_and_comments() {
        assert_eq!(strip_html("<p>1 < 2 and 3 > 1</p>"), "1 < 2 and 3 > 1");
        assert_eq!(strip_html("<!-- a > b --><p>kept</p>"), "kept");
        assert_eq!(strip_html("<p>tail 1 < 2"), "tail 1 < 2");
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

