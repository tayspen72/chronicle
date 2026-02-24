use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Task {
    pub title: String,
    pub assigned_to: Option<String>,
    pub status: Option<String>, // e.g., "todo|doing|done|blocked"
    pub details: Option<String>,
    pub created_at: DateTime<Utc>,
    pub due: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}
