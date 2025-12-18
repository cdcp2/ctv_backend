use axum::{extract::{Json, State}, http::StatusCode, response::IntoResponse};
use crate::{db::DbPool, models::tag::{Tag, CreateTagSchema}};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TagAssignment {
    pub tag_ids: Vec<i32>,
}

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

// GET /api/articles/:slug/tags
pub async fn list_article_tags_handler(
    axum::extract::Path(slug): axum::extract::Path<String>,
    State(pool): State<DbPool>,
) -> impl IntoResponse {
    let result = sqlx::query_as!(
        Tag,
        r#"
        SELECT t.id, t.name, t.slug
        FROM tags t
        JOIN article_tags at ON at.tag_id = t.id
        JOIN articles a ON a.id = at.article_id
        WHERE a.slug = $1
        ORDER BY t.name ASC
        "#,
        slug
    )
    .fetch_all(&pool)
    .await;

    match result {
        Ok(tags) => (StatusCode::OK, axum::Json(tags)).into_response(),
        Err(e) => {
            tracing::error!("Error listando tags de artículo: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

// POST /api/admin/articles/:id/tags (admin) - reemplaza set completo
pub async fn set_article_tags_handler(
    axum::extract::Path(article_id): axum::extract::Path<i64>,
    State(pool): State<DbPool>,
    Json(body): Json<TagAssignment>,
) -> impl IntoResponse {
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Error iniciando transacción: {:?}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Limpiar tags existentes
    if let Err(e) = sqlx::query!("DELETE FROM article_tags WHERE article_id = $1", article_id)
        .execute(&mut *tx)
        .await
    {
        tracing::error!("Error limpiando tags: {:?}", e);
        let _ = tx.rollback().await;
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    // Insertar nuevos
    for tag_id in body.tag_ids.iter() {
        if let Err(e) = sqlx::query!(
            "INSERT INTO article_tags (article_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            article_id,
            tag_id
        )
        .execute(&mut *tx)
        .await
        {
            tracing::error!("Error insertando tag {}: {:?}", tag_id, e);
            let _ = tx.rollback().await;
            return StatusCode::BAD_REQUEST.into_response();
        }
    }

    if let Err(e) = tx.commit().await {
        tracing::error!("Error commit tags: {:?}", e);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    tracing::info!("tags_set article_id={} tags={:?}", article_id, body.tag_ids);
    StatusCode::NO_CONTENT.into_response()
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
