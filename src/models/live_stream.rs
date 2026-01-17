use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow, Clone)]
pub struct LiveStreamConfig {
    pub id: i32,
    pub ingest_url: Option<String>,
    pub stream_key: Option<String>,
    pub server_main_url: Option<String>,
    pub server_backup_url: Option<String>,
    pub playback_url: Option<String>,
    pub is_active: bool,
    pub notes: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    // Campos derivados para mostrar listo para copiar
    pub rtmp_url_primary: Option<String>,
    pub rtmp_url_backup: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLiveStreamConfig {
    pub ingest_url: Option<String>,
    pub stream_key: Option<String>,
    pub server_main_url: Option<String>,
    pub server_backup_url: Option<String>,
    pub playback_url: Option<String>,
    pub is_active: Option<bool>,
    pub notes: Option<String>,
}
