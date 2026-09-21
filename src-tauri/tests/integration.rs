//! Integration tests — verify end-to-end business logic không qua UI.
//!
//! Test các scenario:
//! 1. open_document() load EPUB thật qua adapter
//! 2. get_chapter_blocks() filter đúng chapter
//! 3. list_books_in_dir() scan bundled-books
//! 4. Edge cases: invalid path, empty directory
//!
//! Lưu ý: command `open_document` và `list_bundled_books` cần Tauri AppHandle,
//! nên ta test qua library API (pure functions) thay vì qua command wrapper.

use ebook_reader_lib::adapter::{adapter_for, detect_format, DocumentAdapter};
use ebook_reader_lib::epub::EpubAdapter;
use ebook_reader_lib::list_books_in_dir;
use ebook_reader_lib::markdown::MarkdownAdapter;
use ebook_reader_lib::model::{Block, DocumentFormat};
use std::path::PathBuf;

fn docs_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn bundled_books() -> PathBuf {
    docs_root().join("bundled-books")
}

// ──────────────────────────────────────────────────────────────
// 1. open_document flow (via adapter_for)
// ──────────────────────────────────────────────────────────────

#[test]
fn end_to_end_load_dracula_via_adapter() {
    let path = bundled_books().join("dracula.epub");
    assert!(path.exists(), "dracula.epub must exist in bundled-books");

    let format = detect_format(&path).expect("EPUB must be detected");
    assert_eq!(format, DocumentFormat::Epub);

    let doc = adapter_for(format)
        .load(&path)
        .expect("Dracula must load successfully");

    // Title + author
    assert_eq!(doc.meta.title, "Dracula", "Dracula title");
    assert_eq!(
        doc.meta.author.as_deref(),
        Some("Bram Stoker"),
        "Dracula author"
    );
    assert_eq!(doc.meta.format, DocumentFormat::Epub);

    // Chapters + blocks
    // 32 từ NavPoint tree; cover/title page chỉ có <head><title> → không có text
    // → không sinh "Front matter".
    assert_eq!(doc.chapters.len(), 32, "Dracula has 32 NavPoint chapters");
    assert_eq!(doc.chapters[0].title, "D R A C U L A");
    // Mỗi đoạn văn là một block → hàng nghìn block, không phải 32
    assert!(
        doc.blocks.len() > 1000,
        "Dracula should have > 1000 paragraph blocks (got {})",
        doc.blocks.len()
    );

    // Total chars phải lớn (sách dài)
    let total_chars: usize = doc.blocks.iter().map(|b| b.text.len()).sum();
    assert!(
        total_chars > 800_000,
        "Dracula > 800K chars (got {})",
        total_chars
    );

    println!(
        "✓ Dracula: {} chapters, {} blocks, {} chars, hash={}",
        doc.chapters.len(),
        doc.blocks.len(),
        total_chars,
        &doc.meta.content_hash[..16]
    );
}

#[test]
fn end_to_end_load_calculus_via_adapter() {
    let path = bundled_books().join("calculus.epub");
    assert!(path.exists(), "calculus.epub must exist in bundled-books");

    let format = detect_format(&path).expect("EPUB must be detected");
    let doc = adapter_for(format)
        .load(&path)
        .expect("Calculus must load");

    // Title non-empty
    assert!(!doc.meta.title.is_empty(), "Calculus has a title");
    // Has chapters
    assert!(!doc.chapters.is_empty(), "Calculus has chapters");
    // Has blocks
    assert!(!doc.blocks.is_empty(), "Calculus has blocks");

    let total_chars: usize = doc.blocks.iter().map(|b| b.text.len()).sum();
    println!(
        "✓ Calculus: title='{}', {} chapters, {} blocks, {} chars",
        doc.meta.title,
        doc.chapters.len(),
        doc.blocks.len(),
        total_chars
    );
}

/// Spine item không có trong TOC vẫn phải vào blocks (calculus: 9/42 spine
/// item ngoài TOC), và navpoint trùng resource không được nhân bản text.
#[test]
fn epub_keeps_spine_items_outside_toc_without_duplicates() {
    use std::collections::HashSet;
    for (name, min_blocks) in [("calculus.epub", 42usize), ("dracula.epub", 32)] {
        let path = bundled_books().join(name);
        let doc = EpubAdapter.load(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(
            doc.blocks.len() >= min_blocks,
            "{name}: expected >= {min_blocks} blocks (one per non-empty spine item), got {}",
            doc.blocks.len()
        );
        // Block giờ là đoạn văn; đoạn ngắn ("Exercises", số trang) lặp là hợp lệ,
        // nhưng đoạn dài lặp = resource bị nạp hai lần.
        let long: Vec<&str> = doc.blocks.iter().map(|b| b.text.as_str()).filter(|t| t.len() > 200).collect();
        let unique: HashSet<&str> = long.iter().copied().collect();
        assert_eq!(unique.len(), long.len(), "{name}: duplicated long block text");
        assert!(
            !doc.blocks.iter().any(|b| b.text.contains("<?xml")),
            "{name}: xml prolog leaked into text"
        );
        // mọi block phải thuộc một chapter có thật, positions liên tục
        let ids: HashSet<_> = doc.chapters.iter().map(|c| c.id).collect();
        assert!(doc.blocks.iter().all(|b| ids.contains(&b.chapter_id)), "{name}: orphan block");
        let positions: Vec<u32> = doc.chapters.iter().map(|c| c.position).collect();
        assert_eq!(positions, (0..doc.chapters.len() as u32).collect::<Vec<_>>(), "{name}: positions");
        assert!(
            !doc.blocks.iter().any(|b| b.text.contains("&amp;") || b.text.contains("&nbsp;")),
            "{name}: undecoded entity"
        );
    }
}

#[test]
fn end_to_end_load_markdown_via_adapter() {
    use std::fs;
    use std::io::Write;

    // Tạo MD file tạm với frontmatter + headings
    let dir = std::env::temp_dir().join("ebook_reader_int_test");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sample.md");
    let mut f = fs::File::create(&path).unwrap();
    writeln!(
        f,
        "---\ntitle: Integration Test\nauthor: Tester\n---\n# Chapter 1\n\nFirst paragraph here.\n\n# Chapter 2\n\nSecond chapter content.\n\n## Sub 2.1\n\nSubsection text.\n"
    )
    .unwrap();
    drop(f);

    let format = detect_format(&path).expect("MD must be detected");
    let doc = adapter_for(format).load(&path).expect("MD load");

    assert_eq!(doc.meta.title, "Integration Test");
    assert_eq!(doc.meta.author.as_deref(), Some("Tester"));
    assert!(doc.chapters.len() >= 3, "At least 3 chapters (2 + 1 sub)");
    assert!(doc.blocks.len() >= 3, "At least 3 blocks");

    // All blocks non-empty
    for b in &doc.blocks {
        assert!(!b.text.is_empty(), "Block text non-empty");
        assert!(b.text.len() > 5, "Block text has substance");
    }

    println!(
        "✓ Markdown: {} chapters, {} blocks",
        doc.chapters.len(),
        doc.blocks.len()
    );

    fs::remove_file(&path).ok();
}

// ──────────────────────────────────────────────────────────────
// 2. get_chapter_blocks flow — simulate bằng filter thủ công
// ──────────────────────────────────────────────────────────────

#[test]
fn chapter_blocks_filter_is_correct() {
    let path = bundled_books().join("dracula.epub");
    let format = detect_format(&path).unwrap();
    let doc = adapter_for(format).load(&path).unwrap();

    // Lấy chapter đầu tiên (D R A C U L A cover)
    let first_chapter = &doc.chapters[0];
    let blocks: Vec<&Block> = doc
        .blocks
        .iter()
        .filter(|b| b.chapter_id == first_chapter.id)
        .collect();

    println!(
        "✓ First chapter '{}': {} blocks",
        first_chapter.title,
        blocks.len()
    );

    // Chapter đầu có thể cover/TOC — chỉ cần filter hoạt động đúng
    for b in &blocks {
        assert_eq!(b.chapter_id, first_chapter.id, "Block belongs to chapter");
    }

    // Tất cả blocks đều thuộc chapter nào đó trong doc
    for ch in &doc.chapters {
        let chap_blocks: Vec<&Block> =
            doc.blocks.iter().filter(|b| b.chapter_id == ch.id).collect();
        // Mỗi chapter có ít nhất 0 blocks (cover có thể rỗng)
        let _ = chap_blocks;
    }
    // Total blocks >= chapters with blocks
    let chapters_with_blocks: usize = doc
        .chapters
        .iter()
        .filter(|c| doc.blocks.iter().any(|b| b.chapter_id == c.id))
        .count();
    assert!(chapters_with_blocks > 0, "Some chapters have blocks");
}

// ──────────────────────────────────────────────────────────────
// 3. list_books_in_dir — scan bundled-books
// ──────────────────────────────────────────────────────────────

#[test]
fn list_bundled_books_finds_two_ebooks() {
    let dir = bundled_books();
    if !dir.exists() {
        eprintln!("SKIP: bundled-books dir not found at {:?}", dir);
        return;
    }

    let books = list_books_in_dir(&dir).expect("list should succeed");
    assert_eq!(books.len(), 2, "Should find exactly 2 books");

    // Cả 2 đều là EPUB
    for b in &books {
        assert_eq!(b.format, "epub", "Both are EPUB");
        assert!(b.size_bytes > 0, "Size > 0");
        assert!(!b.title.is_empty(), "Title parsed");
    }

    // Sort theo title
    assert!(books[0].title.starts_with("Calculus"), "Calculus first");
    assert!(books[1].title.starts_with("Dracula"), "Dracula second");

    println!(
        "✓ Library: {} books — '{}' ({}), '{}' ({})",
        books.len(),
        books[0].title,
        books[0].author.as_deref().unwrap_or("?"),
        books[1].title,
        books[1].author.as_deref().unwrap_or("?")
    );
}

#[test]
fn list_books_in_empty_dir_returns_empty() {
    let dir = std::env::temp_dir().join("ebook_reader_empty_test");
    std::fs::create_dir_all(&dir).unwrap();

    let books = list_books_in_dir(&dir).expect("list should succeed");
    assert_eq!(books.len(), 0);

    std::fs::remove_dir(&dir).ok();
}

#[test]
fn list_books_in_nonexistent_dir_returns_empty() {
    let dir = PathBuf::from("/tmp/this_dir_does_not_exist_12345");
    let books = list_books_in_dir(&dir).expect("list should succeed");
    assert_eq!(books.len(), 0);
}

#[test]
fn list_books_skips_unsupported_files() {
    use std::fs::File;
    let dir = std::env::temp_dir().join("ebook_reader_mixed_test");
    std::fs::create_dir_all(&dir).unwrap();

    // Create 1 EPUB (fake) + 1 .txt + 1 .DS_Store
    File::create(dir.join("book.epub")).unwrap();
    File::create(dir.join("notes.txt")).unwrap();
    File::create(dir.join(".DS_Store")).unwrap();

    let books = list_books_in_dir(&dir).expect("list should succeed");
    // EPUB load fail → vẫn được list (title fallback = filename)
    // txt + .DS_Store bị skip vì detect_format trả None
    assert_eq!(books.len(), 1, "Only EPUB listed");
    assert_eq!(books[0].id, "book.epub");

    std::fs::remove_dir_all(&dir).ok();
}

// ──────────────────────────────────────────────────────────────
// 4. Edge cases
// ──────────────────────────────────────────────────────────────

#[test]
fn invalid_epub_path_returns_error() {
    let path = PathBuf::from("/tmp/nonexistent.epub");
    let format = detect_format(&path);
    assert!(format.is_some(), "Extension detected");
    let result = adapter_for(format.unwrap()).load(&path);
    assert!(result.is_err(), "Load should fail");
}

#[test]
fn unsupported_format_returns_none() {
    assert_eq!(detect_format(&PathBuf::from("book.docx")), None);
    assert_eq!(detect_format(&PathBuf::from("image.png")), None);
    assert_eq!(detect_format(&PathBuf::from("archive.zip")), None);
}

#[test]
fn content_hash_is_deterministic_across_calls() {
    let path = bundled_books().join("dracula.epub");
    if !path.exists() {
        return;
    }
    let adapter = EpubAdapter;
    let h1 = adapter.content_hash(&path).unwrap();
    let h2 = adapter.content_hash(&path).unwrap();
    let h3 = adapter.content_hash(&path).unwrap();
    assert_eq!(h1, h2);
    assert_eq!(h2, h3);
    assert_eq!(h1.len(), 64, "SHA256 hex");
}

#[test]
fn different_files_have_different_hashes() {
    let dracula = bundled_books().join("dracula.epub");
    let calculus = bundled_books().join("calculus.epub");
    if !dracula.exists() || !calculus.exists() {
        return;
    }
    let adapter = EpubAdapter;
    let h1 = adapter.content_hash(&dracula).unwrap();
    let h2 = adapter.content_hash(&calculus).unwrap();
    assert_ne!(h1, h2, "Different files = different hashes");
}

#[test]
fn markdown_adapter_roundtrip_simple() {
    use std::fs;
    use std::io::Write;

    let dir = std::env::temp_dir();
    let path = dir.join("roundtrip.md");
    let mut f = fs::File::create(&path).unwrap();
    writeln!(
        f,
        "# Title\n\nPara 1 text.\n\n## Sub\n\nPara 2 text.\n"
    )
    .unwrap();
    drop(f);

    let adapter = MarkdownAdapter;
    let doc = adapter.load(&path).expect("load ok");

    // Frontmatter parse: 0, content parse: 2 chapters (Title + Sub)
    assert!(doc.chapters.len() >= 2);
    assert!(doc.blocks.iter().any(|b| b.text.contains("Para 1")));
    assert!(doc.blocks.iter().any(|b| b.text.contains("Para 2")));

    fs::remove_file(&path).ok();
}
