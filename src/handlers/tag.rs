use axum::{extract::{Json, State}, http::StatusCode, response::IntoResponse};
use crate::{db::DbPool, models::tag::{Tag, CreateTagSchema}};

// GET /api/tags (público)
pub async fn list_tags_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let result = sqlx::query_as!(
        Tag,
        r#"SELECT id, name, slug FROM tags ORDER BY name ASC"#
    )
    .fetch_all(&pool)
    .await;

    match result {
        Ok(tags) => (StatusCode::OK, axum::Json(tags)).into_response(),
        Err(e) => {
            tracing::error!("Error listando tags: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

// POST /api/admin/tags (admin)
pub async fn create_tag_handler(
    State(pool): State<DbPool>,
    Json(body): Json<CreateTagSchema>,
) -> impl IntoResponse {
    let slug = body.slug.clone().unwrap_or_else(|| slugify(&body.name));

    let result = sqlx::query_as!(
        Tag,
        r#"INSERT INTO tags (name, slug) VALUES ($1, $2) RETURNING id, name, slug"#,
        body.name,
        slug
    )
    .fetch_one(&pool)
    .await;

    match result {
        Ok(tag) => (StatusCode::CREATED, axum::Json(tag)).into_response(),
        Err(e) => {
            tracing::error!("Error creando tag: {:?}", e);
            (StatusCode::BAD_REQUEST, "No se pudo crear el tag").into_response()
        }
    }
}

fn slugify(input: &str) -> String {
    let mut slug = String::with_capacity(input.len());
    let mut prev_hyphen = false;

    for ch in input.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            prev_hyphen = false;
        } else if !prev_hyphen {
            slug.push('-');
            prev_hyphen = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }
    while slug.starts_with('-') {
        slug.remove(0);
    }

    slug
}
