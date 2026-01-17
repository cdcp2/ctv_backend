use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::{query, query_as};

use crate::{
    db::DbPool,
    models::{
        advertisement::{Advertisement, CreateAdSchema, UpdateAdSchema},
        user::Claims,
    },
};

#[derive(Debug, Deserialize)]
pub struct AdsQuery {
    pub position: Option<String>,
    pub limit: Option<i64>,
}

// GET /api/ads (público)
pub async fn list_ads_handler(
    Query(q): Query<AdsQuery>,
    State(pool): State<DbPool>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(5).clamp(1, 20);
    let position = q.position;
    let now = Utc::now();

    let result = query_as::<_, Advertisement>(r#"
        SELECT id, title, image_url, target_url, position, html_snippet,
               is_active, weight, starts_at, ends_at, created_at, updated_at
        FROM advertisements
        WHERE is_active = TRUE
          AND (starts_at IS NULL OR starts_at <= $1)
          AND (ends_at IS NULL OR ends_at >= $1)
          AND ($2::text IS NULL OR position = $2)
        ORDER BY weight DESC, created_at DESC
        LIMIT $3
    "#)
    .bind(now)
    .bind(position)
    .bind(limit)
    .fetch_all(&pool)
    .await;

    match result {
        Ok(ads) => (StatusCode::OK, Json(ads)).into_response(),
        Err(e) => {
            tracing::error!("Error listando anuncios: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

// GET /api/admin/ads
pub async fn list_admin_ads_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let result = query_as::<_, Advertisement>(r#"
        SELECT id, title, image_url, target_url, position, html_snippet,
               is_active, weight, starts_at, ends_at, created_at, updated_at
        FROM advertisements
        ORDER BY created_at DESC
    "#)
    .fetch_all(&pool)
    .await;

    match result {
        Ok(ads) => (StatusCode::OK, Json(ads)).into_response(),
        Err(e) => {
            tracing::error!("Error listando anuncios (admin): {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

// POST /api/admin/ads
pub async fn create_ad_handler(
    State(pool): State<DbPool>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateAdSchema>,
) -> impl IntoResponse {
    if claims.role != "admin" {
        return (StatusCode::FORBIDDEN, "Solo admin puede crear anuncios").into_response();
    }

    let is_active = body.is_active.unwrap_or(true);
    let weight = body.weight.unwrap_or(1);

    let result = query_as::<_, Advertisement>(r#"
        INSERT INTO advertisements (title, image_url, target_url, position, html_snippet, is_active, weight, starts_at, ends_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, title, image_url, target_url, position, html_snippet,
                  is_active, weight, starts_at, ends_at, created_at, updated_at
    "#)
    .bind(body.title)
    .bind(body.image_url)
    .bind(body.target_url)
    .bind(body.position)
    .bind(body.html_snippet)
    .bind(is_active)
    .bind(weight)
    .bind(body.starts_at)
    .bind(body.ends_at)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(ad) => (StatusCode::CREATED, Json(ad)).into_response(),
        Err(e) => {
            tracing::error!("Error creando anuncio: {:?}", e);
            (StatusCode::BAD_REQUEST, "No se pudo crear el anuncio").into_response()
        }
    }
}

// PUT /api/admin/ads/:id
pub async fn update_ad_handler(
    Path(id): Path<i64>,
    State(pool): State<DbPool>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateAdSchema>,
) -> impl IntoResponse {
    if claims.role != "admin" {
        return (StatusCode::FORBIDDEN, "Solo admin puede editar anuncios").into_response();
    }

    let result = query_as::<_, Advertisement>(r#"
        UPDATE advertisements SET
            title = COALESCE($1, title),
            image_url = COALESCE($2, image_url),
            target_url = COALESCE($3, target_url),
            position = COALESCE($4, position),
            html_snippet = COALESCE($5, html_snippet),
            is_active = COALESCE($6, is_active),
            weight = COALESCE($7, weight),
            starts_at = COALESCE($8, starts_at),
            ends_at = COALESCE($9, ends_at),
            updated_at = NOW()
        WHERE id = $10
        RETURNING id, title, image_url, target_url, position, html_snippet,
                  is_active, weight, starts_at, ends_at, created_at, updated_at
    "#)
    .bind(body.title)
    .bind(body.image_url)
    .bind(body.target_url)
    .bind(body.position)
    .bind(body.html_snippet)
    .bind(body.is_active)
    .bind(body.weight)
    .bind(body.starts_at)
    .bind(body.ends_at)
    .bind(id)
    .fetch_optional(&pool)
    .await;

    match result {
        Ok(Some(ad)) => (StatusCode::OK, Json(ad)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Anuncio no encontrado").into_response(),
        Err(e) => {
            tracing::error!("Error actualizando anuncio {}: {:?}", id, e);
            (StatusCode::BAD_REQUEST, "No se pudo actualizar").into_response()
        }
    }
}

// DELETE /api/admin/ads/:id
pub async fn delete_ad_handler(
    Path(id): Path<i64>,
    State(pool): State<DbPool>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    if claims.role != "admin" {
        return (StatusCode::FORBIDDEN, "Solo admin puede borrar anuncios").into_response();
    }

    let result = query("DELETE FROM advertisements WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await;

    match result {
        Ok(res) if res.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, "Anuncio no encontrado").into_response(),
        Err(e) => {
            tracing::error!("Error borrando anuncio {}: {:?}", id, e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}
