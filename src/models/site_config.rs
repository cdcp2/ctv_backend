use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct SiteConfig {
    pub id: i32,
    pub live_stream_url: Option<String>,
    pub is_live_active: bool,
    pub breaking_news_banner: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSiteConfigSchema {
    pub live_stream_url: Option<String>,
    pub is_live_active: Option<bool>,
    pub breaking_news_banner: Option<String>,
}
