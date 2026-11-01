use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Note {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkedAccount {
    pub id: Uuid,
    pub provider: String,
    pub email: Option<String>,
}

/// Bounded read-only card data; opening an editor fetches the complete note.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct NotePreview {
    pub id: Uuid,
    pub title: String,
    pub excerpt: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceOverview {
    pub note_count: i64,
    pub updated_last_seven_days: i64,
    pub last_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    pub recent_notes: Vec<NotePreview>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NoteSearchResults {
    pub notes: Vec<NotePreview>,
    pub has_more: bool,
}
