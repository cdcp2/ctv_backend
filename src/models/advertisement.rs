use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow, Clone)]
pub struct Advertisement {
    pub id: i64,
    pub title: String,
    pub image_url: Option<String>,
    pub target_url: Option<String>,
    pub position: String,
    pub html_snippet: Option<String>,
    pub is_active: bool,
    pub weight: i32,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAdSchema {
    pub title: String,
    pub image_url: Option<String>,
    pub target_url: Option<String>,
    pub position: String,
    pub html_snippet: Option<String>,
    pub is_active: Option<bool>,
    pub weight: Option<i32>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAdSchema {
    pub title: Option<String>,
    pub image_url: Option<String>,
    pub target_url: Option<String>,
    pub position: Option<String>,
    pub html_snippet: Option<String>,
    pub is_active: Option<bool>,
    pub weight: Option<i32>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}
