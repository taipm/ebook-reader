# ebook-reader

Personal knowledge workspace — ebook reader + annotation + notes.

## MVP 1 (hiện tại)

Open EPUB / Markdown / PDF → TOC → reader → select → highlight → note → SQLite → restore position.

Chưa có AI / search / knowledge graph (đó là Phase 2).

## Stack

- **Backend**: Rust 1.95 + Tauri 2.11
- **Frontend**: Svelte 5 + TypeScript + Vite 5
- **Storage**: SQLite (PR #5)

## Quick start

```bash
cd src-tauri
cargo test           # chạy unit tests

cd ..
npm install
npm run dev          # Vite dev server
npm run tauri dev    # chạy Tauri app (mở cửa sổ native)
```

## Roadmap

Xem `docs/ARCHITECTURE.md` để biết chi tiết canonical Document Model, Location, Annotation system.

| PR  | Scope                                                | Status      |
|-----|------------------------------------------------------|-------------|
| #1  | Scaffold + Architecture + Core types                 | ✅ merged   |
| #2  | Rust core + EPUB adapter                             | 📋 planned  |
| #3  | Markdown adapter + Svelte 3-column UI + highlight    | 📋 planned  |
| #4  | PDF adapter + note ↔ reader sync                     | 📋 planned  |
| #5  | SQLite persist + restore position + 4 Focus Mode     | 📋 planned  |

## License

TBD
