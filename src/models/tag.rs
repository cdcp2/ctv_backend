use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct Tag {
    pub id: i32,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTagSchema {
    pub name: String,
    pub slug: Option<String>,
}
