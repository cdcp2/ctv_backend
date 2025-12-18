use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

// 1. Estructura que representa una fila completa en la Base de Datos
#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct Article {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub excerpt: Option<String>,
    pub main_image_url: Option<String>,
    pub video_embed_url: Option<String>,
    pub author_id: Option<i64>,
    pub category_id: Option<i32>,
    pub status: String,
    pub is_featured: bool,
    pub is_breaking: bool,
    pub views_count: i64,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

// 2. Estructura para recibir los datos del Frontend (JSON) al crear una noticia
#[derive(Debug, Deserialize)]
pub struct CreateArticleSchema {
    pub title: String,
    pub content: String,
    pub excerpt: Option<String>,
    pub category_id: Option<i32>,
    pub main_image_url: Option<String>,
    pub video_embed_url: Option<String>,
    pub status: Option<String>,       // draft | published | archived
    pub is_featured: Option<bool>,
    pub is_breaking: Option<bool>,
    pub published_at: Option<DateTime<Utc>>,
}
