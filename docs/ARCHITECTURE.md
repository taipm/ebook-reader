# ebook-reader — Architecture

> Personal knowledge workspace. Ebook reader + annotation + notes.
> MVP 1: open EPUB/Markdown/PDF → TOC → reader → select → highlight → note → SQLite → restore position.
> Chưa có AI / search / knowledge graph (Phase 2).

## Nguyên tắc thiết kế

1. **Selection là primitive trung tâm.** Mọi action (highlight, note, AI, copy) đều bắt đầu từ selection trong reader.
2. **Format-agnostic core.** EPUB/PDF/Markdown chỉ là input. Bên dưới là canonical Document Model.
3. **Restore-stable location.** Annotation phải restore đúng vị trí qua reflow/zoom/theme change.
4. **Single source of truth cho ID.** Mỗi entity (document/chapter/block/annotation) có 1 ID type riêng — không thể nhầm.

## Canonical Document Model

```
DocumentId  ──┬──► ChapterId ──► Block ──► text (canonical plain)
              │                              ↑
              │                              │ char_start..char_end
              │                              │
              └──► Annotation ──────────────►│
                       │                     │
                       ├─ highlighted_text   │
                       ├─ note (Markdown)    │
                       ├─ tags               │
                       └─ color              │
```

### `CanonicalLocation`

```rust
pub struct CanonicalLocation {
    pub document_id: DocumentId,
    pub chapter_id: ChapterId,
    pub block_id: BlockId,
    pub char_start: u32,   // char index trong rendered plain text của block
    pub char_end: u32,
}
```

**Tại sao `char_start..char_end` chứ không phải byte offset hay DOM Range?**
- Byte offset phụ thuộc encoding (UTF-8 vs UTF-16) — restore qua theme/font change không ổn định.
- DOM Range phụ thuộc markup — thay đổi HTML structure = mất.
- Char offset trong **rendered plain text** ổn định nhất: thay đổi font/theme/reflow không ảnh hưởng.

### `BlockId` stability

- **EPUB**: `BlockId = SHA256(canonical_text)` của block đó. EPUB có id nội bộ (`epub:id`) nhưng không ổn định giữa các lần re-export → dùng content hash.
- **Markdown**: tương tự EPUB — `SHA256(text)` của mỗi block.
- **PDF**: hybrid — có text → `SHA256(text)`; scan (no text) → `SHA256(page_image_bytes) + bbox`. PR #4 sẽ implement chi tiết.

### Newtype IDs

```rust
DocumentId(Uuid)   ChapterId(Uuid)   BlockId(Uuid)   AnnotationId(Uuid)
```

Mỗi ID là 1 type riêng, không thể gán nhầm. Tất cả implement `Serialize/Deserialize` qua `serde(transparent)`.

## Adapter pattern

```rust
pub trait DocumentAdapter: Send + Sync {
    fn format(&self) -> DocumentFormat;
    fn load(&self, path: &Path) -> Result<LoadedDocument>;
    fn content_hash(&self, path: &Path) -> Result<String>;
}
```

3 adapter: `EpubAdapter` (PR #2) · `MarkdownAdapter` (PR #3) · `PdfAdapter` (PR #4).
Mỗi adapter tự map coordinate nội bộ → `CanonicalLocation`.

## Storage

SQLite schema (PR #5):

```sql
documents          (id, title, author, format, path, content_hash, created_at, updated_at)
chapters           (id, document_id, parent_id, title, level, position)
annotations        (id, document_id, chapter_id, block_id, char_start, char_end,
                    kind, highlighted_text, note, color, created_at, updated_at)
annotation_tags    (annotation_id, tag)
reading_progress   (document_id, chapter_id, block_id, char_offset, updated_at)
```

`annotations.highlighted_text` + `annotations.note` indexed bằng FTS5 cho full-text search (Phase 2).

## UI layout

3 cột resize được, mỗi cột 20% / 60% / 20% mặc định. Drag handle giữa các cột để resize.

```
┌─────────┬─────────────────────────────┬─────────┐
│Contents │           Reader            │  Notes  │
│         │                             │         │
│ TOC     │  ┌─Chapter 3─────────────┐  │ Linked  │
│ ▾ Part1 │  │  paragraph ...        │  │ notes   │
│   ▾ Ch1 │  │                       │  │         │
│     1.1 │  │  [highlight] text...  │  │ + Note  │
│     1.2 │  │                       │  │         │
│   ▸ Ch2 │  └───────────────────────┘  │         │
│ ▾ Part2 │                             │         │
└─────────┴─────────────────────────────┴─────────┘
```

### Focus Mode (4 mode)

| Mode      | Contents | Reader | Notes | Shortcut |
|-----------|----------|--------|-------|----------|
| Normal    | 20%      | 60%    | 20%   | default  |
| Reader    | 0        | 100%   | 0     | ⌘0       |
| Research  | 25%      | 50%    | 25%   | ⌘1+⌘3   |
| Notes     | 0        | 40%    | 60%   | ⌘3 x2    |

## MVP 1 roadmap (4 PR)

| PR  | Scope                                    | Status   |
|-----|------------------------------------------|----------|
| #1  | Scaffold + Architecture doc + Core types | ← bạn đang ở đây |
| #2  | Rust core + EPUB adapter (load + TOC + canonical blocks) | planned |
| #3  | Markdown adapter + Svelte 3-column UI + highlight | planned |
| #4  | PDF adapter + note ↔ reader sync | planned |
| #5  | SQLite persist + restore position + 4 Focus Mode | planned |

## Phase 2 (sau khi Ba dùng thực tế 1-2 tuần)

- FTS5 search
- AI panel (Explain / Summarize / Question / Connect / Challenge)
- Knowledge graph
- Semantic search (vector DB)
- Cross-document linking

## Tech stack

- **Rust 1.95** + **Tauri 2.11** (desktop)
- **Svelte 5** + **TypeScript** + **Vite 5**
- **SQLite** (via `rusqlite` Phase 2)
- **EPUB**: `epub` crate (PR #2)
- **PDF**: `pdfium-render` (PR #4 — cần binary `pdfium.dylib`)
- **Markdown**: `pulldown-cmark` (PR #3)
