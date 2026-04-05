use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

pub const DEFAULT_ROTATION_INTERVAL_SECONDS: i64 = 30;
pub const MIN_ROTATION_INTERVAL_SECONDS: i64 = 5;
pub const MAX_ROTATION_INTERVAL_SECONDS: i64 = 3600;

#[derive(Debug, Serialize, Clone, Copy)]
pub struct AdvertisementPosition {
    pub key: &'static str,
    pub label: &'static str,
}

pub const AD_POSITIONS: [AdvertisementPosition; 4] = [
    AdvertisementPosition {
        key: "home_top",
        label: "Home superior",
    },
    AdvertisementPosition {
        key: "home_sidebar",
        label: "Home lateral",
    },
    AdvertisementPosition {
        key: "article_inline",
        label: "Articulo intermedio",
    },
    AdvertisementPosition {
        key: "article_footer",
        label: "Articulo inferior",
    },
];

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

pub fn sanitize_ad_position(position: &str) -> Option<String> {
    let trimmed = position.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub fn clamp_rotation_interval(interval_seconds: Option<i64>) -> i64 {
    interval_seconds
        .unwrap_or(DEFAULT_ROTATION_INTERVAL_SECONDS)
        .clamp(MIN_ROTATION_INTERVAL_SECONDS, MAX_ROTATION_INTERVAL_SECONDS)
}
