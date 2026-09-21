use ebook_reader_lib::adapter::adapter_for;
use ebook_reader_lib::model::DocumentFormat;
use std::path::Path;

fn main() {
    let path = std::env::args().nth(1).expect("usage: load_epub <path>");
    let path_buf = Path::new(&path);
    let format = ebook_reader_lib::adapter::detect_format(path_buf)
        .expect("Unsupported format");
    let adapter = adapter_for(format);
    let loaded = adapter.load(path_buf).expect("Load failed");
    println!("✓ Loaded: {}", loaded.meta.title);
    println!("  Author: {:?}", loaded.meta.author);
    println!("  Format: {:?}", loaded.meta.format);
    println!("  Chapters: {}", loaded.chapters.len());
    println!("  Blocks: {}", loaded.blocks.len());
    println!("  Total chars: {}", loaded.blocks.iter().map(|b| b.text.len()).sum::<usize>());
    println!("  Hash: {}", loaded.meta.content_hash);
    println!();
    println!("First 5 chapters:");
    for ch in loaded.chapters.iter().take(5) {
        println!("  [L{}] {}", ch.level, ch.title);
    }
}
