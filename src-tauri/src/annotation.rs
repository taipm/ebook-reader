//! Annotation — highlight + note + tags
//!
//! Mỗi annotation bám vào một `CanonicalLocation`. Text highlight + note
//! riêng nhau để:
//! - Highlight có thể tồn tại mà không cần note (1-click highlight)
//! - Note có thể dài, có thể chứa markdown, có thể không có highlight gốc
//!   (vd note "abstract" cho cả chapter)
//!
//! Tags dùng cho knowledge graph sau này. MVP 1 chỉ cần tag name (string),
//! sau này nâng cấp lên Tag entity riêng nếu cần hierarchy.

use crate::model::{AnnotationId, CanonicalLocation, DocumentId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationKind {
    /// Chỉ bôi vàng, không có note.
    Highlight,
    /// Highlight + note (ghi chú ngắn).
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Annotation {
    pub id: AnnotationId,
    pub document_id: DocumentId,
    pub location: CanonicalLocation,
    pub kind: AnnotationKind,
    /// Text đã highlight (snapshot tại thời điểm tạo).
    /// Lưu lại để:
    /// 1. Restore UI không cần lookup lại document
    /// 2. Search full-text trong SQLite FTS5
    /// 3. Detect drift nếu document gốc đổi (hash so sánh)
    pub highlighted_text: String,
    /// Note (Markdown). Empty nếu chỉ highlight.
    pub note: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Màu highlight (hex). Default = vàng #FFEB3B.
    #[serde(default = "default_color")]
    pub color: String,
}

fn default_color() -> String {
    "#FFEB3B".to_string()
}

impl Annotation {
    /// Tạo mới với timestamps hiện tại.
    pub fn new(
        document_id: DocumentId,
        location: CanonicalLocation,
        kind: AnnotationKind,
        highlighted_text: String,
        note: String,
        tags: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: AnnotationId::new(),
            document_id,
            location,
            kind,
            highlighted_text,
            note,
            tags,
            created_at: now,
            updated_at: now,
            color: default_color(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BlockId, ChapterId};

    fn fixture_location() -> CanonicalLocation {
        CanonicalLocation {
            document_id: DocumentId::new(),
            chapter_id: ChapterId::new(),
            block_id: BlockId::new(),
            char_start: 5,
            char_end: 25,
        }
    }

    #[test]
    fn new_annotation_has_timestamps_and_default_color() {
        let before = Utc::now();
        let ann = Annotation::new(
            DocumentId::new(),
            fixture_location(),
            AnnotationKind::Note,
            "highlighted text".into(),
            "my note".into(),
            vec!["agent".into(), "memory".into()],
        );
        let after = Utc::now();
        assert!(ann.created_at >= before && ann.created_at <= after);
        assert_eq!(ann.updated_at, ann.created_at);
        assert_eq!(ann.color, "#FFEB3B");
        assert_eq!(ann.tags.len(), 2);
    }

    #[test]
    fn highlight_only_kind() {
        let ann = Annotation::new(
            DocumentId::new(),
            fixture_location(),
            AnnotationKind::Highlight,
            "x".into(),
            String::new(),
            vec![],
        );
        assert_eq!(ann.kind, AnnotationKind::Highlight);
        assert!(ann.note.is_empty());
    }

    #[test]
    fn serde_round_trip() {
        let original = Annotation::new(
            DocumentId::new(),
            fixture_location(),
            AnnotationKind::Note,
            "An agent should...".into(),
            "Liên quan đến Agent Architecture".into(),
            vec!["agent".into()],
        );
        let json = serde_json::to_string(&original).unwrap();
        let back: Annotation = serde_json::from_str(&json).unwrap();
        assert_eq!(original, back);
        // Verify IDs survive
        assert_eq!(original.id, back.id);
        assert_eq!(original.document_id, back.document_id);
        assert_eq!(original.location, back.location);
    }

    #[test]
    fn serde_color_uses_default_when_missing() {
        // Test rằng một JSON không có field "color" vẫn deserialize được
        let json = r#"{
            "id": "00000000-0000-0000-0000-000000000001",
            "document_id": "00000000-0000-0000-0000-000000000002",
            "location": {
                "document_id": "00000000-0000-0000-0000-000000000002",
                "chapter_id": "00000000-0000-0000-0000-000000000003",
                "block_id": "00000000-0000-0000-0000-000000000004",
                "char_start": 0,
                "char_end": 10
            },
            "kind": "highlight",
            "highlighted_text": "x",
            "note": "",
            "tags": [],
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        }"#;
        let ann: Annotation = serde_json::from_str(json).unwrap();
        assert_eq!(ann.color, "#FFEB3B");
    }
}
