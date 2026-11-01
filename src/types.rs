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
