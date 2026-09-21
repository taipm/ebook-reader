/**
 * Shared types between Rust backend and Svelte frontend.
 * Mirror của Rust types trong src-tauri/src/model.rs
 */

export type DocumentFormat = "epub" | "markdown" | "pdf";

export type ChapterId = string;
export type BlockId = string;
export type DocumentId = string;
export type AnnotationId = string;

export interface Chapter {
  id: ChapterId;
  parent_id: ChapterId | null;
  title: string;
  level: number;
  position: number;
}

export interface Block {
  id: BlockId;
  chapter_id: ChapterId;
  text: string;
  index_in_chapter: number;
}

export interface DocumentMeta {
  id: DocumentId;
  title: string;
  author: string | null;
  format: DocumentFormat;
  content_hash: string;
  created_at: string;
  updated_at: string;
}

export interface CanonicalLocation {
  document_id: DocumentId;
  chapter_id: ChapterId;
  block_id: BlockId;
  char_start: number;
  char_end: number;
}

export type AnnotationKind = "highlight" | "note";

export interface Annotation {
  id: AnnotationId;
  document_id: DocumentId;
  location: CanonicalLocation;
  kind: AnnotationKind;
  highlighted_text: string;
  note: string;
  tags: string[];
  created_at: string;
  updated_at: string;
  color: string;
}

/** Response từ Rust `open_document` command */
export interface OpenDocumentResponse {
  meta: DocumentMeta;
  chapters: Chapter[];
  block_count: number;
  total_chars: number;
}

/** Selection đang active trong reader (chưa lưu thành annotation) */
export interface PendingSelection {
  block_id: BlockId;
  chapter_id: ChapterId;
  char_start: number;
  char_end: number;
  text: string;
  /** Toạ độ toolbar, tương đối với `.reader-body` (x = tâm selection, y = mép trên) */
  rect: { x: number; y: number };
  /** Mép dưới selection (cùng hệ toạ độ) — để lật toolbar xuống dưới khi thiếu chỗ trên */
  rect_bottom?: number;
}

export type FocusMode = "normal" | "reader" | "research" | "notes";

export interface ColumnWidths {
  contents: number; // percentage 0-100
  notes: number;
}

/** Metadata 1 cuốn sách trong bundled library */
export interface BundledBook {
  id: string;
  title: string;
  author: string | null;
  format: string;
  path: string;
  size_bytes: number;
}
