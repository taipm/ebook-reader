//! Canonical Document Model
//!
//! EPUB / PDF / Markdown chỉ là input format. Mọi annotation, highlight, note
//! đều lưu theo `CanonicalLocation` — format-agnostic. Mỗi format adapter
//! tự map coordinate nội bộ của nó → canonical.
//!
//! Tham chiếu: docs/ARCHITECTURE.md

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ──────────────────────────────────────────────────────────────────────
// Newtype IDs — tránh nhầm lẫn DocumentId vs ChapterId vs AnnotationId
// ──────────────────────────────────────────────────────────────────────

macro_rules! id_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            #[inline]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Uuid::parse_str(s)?))
            }
        }
    };
}

id_newtype!(DocumentId);
id_newtype!(ChapterId);
id_newtype!(BlockId);
id_newtype!(AnnotationId);

// ──────────────────────────────────────────────────────────────────────
// Format
// ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentFormat {
    Epub,
    Markdown,
    Pdf,
}

impl fmt::Display for DocumentFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Epub => write!(f, "epub"),
            Self::Markdown => write!(f, "markdown"),
            Self::Pdf => write!(f, "pdf"),
        }
    }
}

// ──────────────────────────────────────────────────────────────────────
// Canonical Location — format-agnostic
// ──────────────────────────────────────────────────────────────────────

/// Pointer đến một vùng text trong document, bất kể format gốc.
///
/// Mỗi format adapter (Epub/Markdown/Pdf) tự resolve `(chapter_id, block_id)`
/// từ cấu trúc nội bộ của nó, rồi map `char_start..char_end` theo thứ tự
/// render (không phải byte offset trong file gốc — đây là lý do cần
/// `canonical_text_offset` chứ không phải byte offset).
///
/// `char_offset` ở đây là **char index trong rendered plain text của block**,
/// không phải byte offset. Điều này làm cho:
/// - Restore position ổn định qua reflow/zoom (PDF) hay theme change
/// - Search/highlight không phụ thuộc encoding quirk của EPUB
/// - So sánh highlight giữa các lần mở document luôn khớp
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CanonicalLocation {
    pub document_id: DocumentId,
    pub chapter_id: ChapterId,
    pub block_id: BlockId,
    pub char_start: u32,
    pub char_end: u32,
}

impl CanonicalLocation {
    /// Kiểm tra location hợp lệ: char_start <= char_end và không tràn.
    pub fn validate(&self) -> Result<(), LocationError> {
        if self.char_start > self.char_end {
            return Err(LocationError::InvertedRange {
                start: self.char_start,
                end: self.char_end,
            });
        }
        // char_end - char_start max ~10M chars đủ cho 1 highlight bình thường
        let span = self.char_end.saturating_sub(self.char_start);
        if span > 10_000_000 {
            return Err(LocationError::RangeTooLarge { span });
        }
        Ok(())
    }

    /// Độ dài selection theo char.
    pub fn len(&self) -> u32 {
        self.char_end.saturating_sub(self.char_start)
    }

    pub fn is_empty(&self) -> bool {
        self.char_start == self.char_end
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LocationError {
    #[error("char_start ({start}) > char_end ({end})")]
    InvertedRange { start: u32, end: u32 },
    #[error("range span {span} too large (> 10M chars)")]
    RangeTooLarge { span: u32 },
}

// ──────────────────────────────────────────────────────────────────────
// Document metadata
// ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentMeta {
    pub id: DocumentId,
    pub title: String,
    pub author: Option<String>,
    pub format: DocumentFormat,
    /// SHA256 của file gốc — dùng detect "cùng một cuốn sách" khi import lại.
    pub content_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// ──────────────────────────────────────────────────────────────────────
// Chapter / Block — adapter trả về để build TOC và resolve location
// ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chapter {
    pub id: ChapterId,
    pub parent_id: Option<ChapterId>,
    pub title: String,
    /// 1 = top-level (Part / Chapter 1), 2 = sub-chapter, ...
    pub level: u8,
    /// Thứ tự trong TOC (0-based). Dùng để ⌘[ / ⌘] next/prev chapter.
    pub position: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub id: BlockId,
    pub chapter_id: ChapterId,
    /// Plain text đã render (đã strip HTML/Markdown syntax).
    /// `char_start`/`char_end` trong `CanonicalLocation` index vào string này.
    pub text: String,
    /// 0-based block index trong chapter (dùng cho fallback nếu BlockId không ổn định).
    pub index_in_chapter: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_location_validate_ok() {
        let loc = CanonicalLocation {
            document_id: DocumentId::new(),
            chapter_id: ChapterId::new(),
            block_id: BlockId::new(),
            char_start: 10,
            char_end: 50,
        };
        assert!(loc.validate().is_ok());
        assert_eq!(loc.len(), 40);
        assert!(!loc.is_empty());
    }

    #[test]
    fn canonical_location_rejects_inverted() {
        let loc = CanonicalLocation {
            document_id: DocumentId::new(),
            chapter_id: ChapterId::new(),
            block_id: BlockId::new(),
            char_start: 50,
            char_end: 10,
        };
        assert_eq!(
            loc.validate(),
            Err(LocationError::InvertedRange { start: 50, end: 10 })
        );
    }

    #[test]
    fn canonical_location_rejects_huge_range() {
        let loc = CanonicalLocation {
            document_id: DocumentId::new(),
            chapter_id: ChapterId::new(),
            block_id: BlockId::new(),
            char_start: 0,
            char_end: 20_000_000,
        };
        assert!(matches!(
            loc.validate(),
            Err(LocationError::RangeTooLarge { .. })
        ));
    }

    #[test]
    fn newtype_ids_are_distinct() {
        // DocumentId và ChapterId đều wrap Uuid nhưng KHÔNG được nhầm.
        // Compile-time đã đảm bảo; runtime test chỉ để sanity check serde.
        let doc = DocumentId::new();
        let chap = ChapterId::new();
        assert_ne!(doc.to_string(), chap.to_string());
        // Round-trip qua string
        let doc_s = doc.to_string();
        let doc_back: DocumentId = doc_s.parse().unwrap();
        assert_eq!(doc, doc_back);
    }

    #[test]
    fn format_display_round_trip() {
        for fmt in [
            DocumentFormat::Epub,
            DocumentFormat::Markdown,
            DocumentFormat::Pdf,
        ] {
            let s = fmt.to_string();
            // serde round-trip
            let json = serde_json::to_string(&fmt).unwrap();
            let back: DocumentFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(fmt, back);
            // Display phải lowercase
            assert_eq!(s, json.trim_matches('"'));
        }
    }
}
