use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::{query, query_as};

use crate::{
    db::DbPool,
    models::{
        advertisement::{
            AD_POSITIONS, Advertisement, CreateAdSchema, UpdateAdSchema, clamp_rotation_interval,
            sanitize_ad_position,
        },
        user::{Claims, is_admin_or_sub_admin_role},
    },
};

const MAX_ROTATION_CANDIDATES: i64 = 100;

#[derive(Debug, Deserialize)]
pub struct AdsQuery {
    pub position: Option<String>,
    pub limit: Option<i64>,
    pub rotate: Option<bool>,
    pub rotation_interval_seconds: Option<i64>,
}

// GET /api/ads (público)
pub async fn list_ads_handler(
    Query(q): Query<AdsQuery>,
    State(pool): State<DbPool>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(5).clamp(1, 20) as usize;
    let position = match q.position.as_deref() {
        Some(raw_position) => match sanitize_ad_position(raw_position) {
            Some(position) => Some(position),
            None => {
                return (StatusCode::BAD_REQUEST, "La posicion no puede estar vacia")
                    .into_response();
            }
        },
        None => None,
    };
    let now = Utc::now();
    let rotate = q.rotate.unwrap_or(false);
    let query_limit = if rotate {
        MAX_ROTATION_CANDIDATES
    } else {
        limit as i64
    };

    let result = query_as::<_, Advertisement>(
        r#"
        SELECT id, title, image_url, target_url, position, html_snippet,
               is_active, weight, starts_at, ends_at, created_at, updated_at
        FROM advertisements
        WHERE is_active = TRUE
          AND (starts_at IS NULL OR starts_at <= $1)
          AND (ends_at IS NULL OR ends_at >= $1)
          AND ($2::text IS NULL OR position = $2)
        ORDER BY weight DESC, created_at DESC
        LIMIT $3
    "#,
    )
    .bind(now)
    .bind(position.as_deref())
    .bind(query_limit)
    .fetch_all(&pool)
    .await;

    match result {
        Ok(ads) => {
            let mut ads = if rotate {
                rotate_ads_by_bucket(
                    ads,
                    rotation_bucket(now, clamp_rotation_interval(q.rotation_interval_seconds)),
                )
            } else {
                ads
            };
            ads.truncate(limit);

            (StatusCode::OK, Json(ads)).into_response()
        }
        Err(e) => {
            tracing::error!("Error listando anuncios: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

// GET /api/ads/positions
pub async fn list_ad_positions_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(AD_POSITIONS)).into_response()
}

// GET /api/admin/ads
pub async fn list_admin_ads_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let result = query_as::<_, Advertisement>(
        r#"
        SELECT id, title, image_url, target_url, position, html_snippet,
               is_active, weight, starts_at, ends_at, created_at, updated_at
        FROM advertisements
        ORDER BY created_at DESC
    "#,
    )
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
    if !can_manage_ads(&claims) {
        return (
            StatusCode::FORBIDDEN,
            "Solo admin o subadmin puede crear anuncios",
        )
            .into_response();
    }

    if let Err(message) = validate_ad_schedule(body.starts_at.as_ref(), body.ends_at.as_ref()) {
        return (StatusCode::BAD_REQUEST, message).into_response();
    }

    let is_active = body.is_active.unwrap_or(true);
    let weight = body.weight.unwrap_or(1);
    let position = match sanitize_ad_position(&body.position) {
        Some(position) => position,
        None => {
            return (StatusCode::BAD_REQUEST, "La posicion no puede estar vacia").into_response();
        }
    };

    let result = query_as::<_, Advertisement>(r#"
        INSERT INTO advertisements (title, image_url, target_url, position, html_snippet, is_active, weight, starts_at, ends_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, title, image_url, target_url, position, html_snippet,
                  is_active, weight, starts_at, ends_at, created_at, updated_at
    "#)
    .bind(body.title)
    .bind(body.image_url)
    .bind(body.target_url)
    .bind(position)
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
    if !can_manage_ads(&claims) {
        return (
            StatusCode::FORBIDDEN,
            "Solo admin o subadmin puede editar anuncios",
        )
            .into_response();
    }

    if let Err(message) = validate_ad_schedule(body.starts_at.as_ref(), body.ends_at.as_ref()) {
        return (StatusCode::BAD_REQUEST, message).into_response();
    }

    let position = match body.position.as_deref() {
        Some(raw_position) => match sanitize_ad_position(raw_position) {
            Some(position) => Some(position),
            None => {
                return (StatusCode::BAD_REQUEST, "La posicion no puede estar vacia")
                    .into_response();
            }
        },
        None => None,
    };

    let result = query_as::<_, Advertisement>(
        r#"
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
    "#,
    )
    .bind(body.title)
    .bind(body.image_url)
    .bind(body.target_url)
    .bind(position)
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
    if !can_manage_ads(&claims) {
        return (
            StatusCode::FORBIDDEN,
            "Solo admin o subadmin puede borrar anuncios",
        )
            .into_response();
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

fn can_manage_ads(claims: &Claims) -> bool {
    is_admin_or_sub_admin_role(&claims.role)
}

fn validate_ad_schedule(
    starts_at: Option<&DateTime<Utc>>,
    ends_at: Option<&DateTime<Utc>>,
) -> Result<(), &'static str> {
    if let (Some(start), Some(end)) = (starts_at, ends_at) {
        if end < start {
            return Err("La fecha final no puede ser menor que la inicial");
        }
    }

    Ok(())
}

fn rotation_bucket(now: DateTime<Utc>, interval_seconds: i64) -> i64 {
    now.timestamp().div_euclid(interval_seconds)
}

fn rotate_ads_by_bucket(ads: Vec<Advertisement>, bucket: i64) -> Vec<Advertisement> {
    let mut grouped_ads: Vec<(String, Vec<Advertisement>)> = Vec::new();

    for ad in ads {
        if let Some((_, items)) = grouped_ads
            .iter_mut()
            .find(|(position, _)| position == &ad.position)
        {
            items.push(ad);
        } else {
            grouped_ads.push((ad.position.clone(), vec![ad]));
        }
    }

    let mut rotated_ads = Vec::new();

    for (_, mut items) in grouped_ads {
        if items.len() > 1 {
            let offset = bucket.rem_euclid(items.len() as i64) as usize;
            items.rotate_left(offset);
        }

        rotated_ads.extend(items);
    }

    rotated_ads
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ad(id: i64, position: &str) -> Advertisement {
        let now = Utc::now();

        Advertisement {
            id,
            title: format!("Ad {id}"),
            image_url: None,
            target_url: None,
            position: position.to_string(),
            html_snippet: None,
            is_active: true,
            weight: 1,
            starts_at: None,
            ends_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn rotate_ads_keeps_single_entries_per_position() {
        let ads = vec![sample_ad(1, "home_top"), sample_ad(2, "article_footer")];

        let rotated = rotate_ads_by_bucket(ads.clone(), 2);

        assert_eq!(rotated[0].id, ads[0].id);
        assert_eq!(rotated[1].id, ads[1].id);
    }

    #[test]
    fn rotate_ads_cycles_within_the_same_position() {
        let ads = vec![
            sample_ad(1, "home_sidebar"),
            sample_ad(2, "home_sidebar"),
            sample_ad(3, "home_sidebar"),
        ];

        let rotated = rotate_ads_by_bucket(ads, 1);

        assert_eq!(
            rotated.iter().map(|ad| ad.id).collect::<Vec<_>>(),
            vec![2, 3, 1]
        );
    }

    #[test]
    fn rotate_ads_keeps_positions_isolated() {
        let ads = vec![
            sample_ad(1, "home_sidebar"),
            sample_ad(2, "home_sidebar"),
            sample_ad(3, "article_inline"),
            sample_ad(4, "article_inline"),
        ];

        let rotated = rotate_ads_by_bucket(ads, 1);

        assert_eq!(
            rotated.iter().map(|ad| ad.id).collect::<Vec<_>>(),
            vec![2, 1, 4, 3]
        );
    }

    #[test]
    fn rejects_invalid_schedule() {
        let start = Utc::now();
        let end = start - chrono::Duration::minutes(5);

        assert!(validate_ad_schedule(Some(&start), Some(&end)).is_err());
    }

    #[test]
    fn subadmin_can_manage_ads() {
        let claims = Claims {
            sub: "user@example.com".into(),
            exp: 0,
            iat: 0,
            user_id: 1,
            role: "subadmin".into(),
        };

        assert!(can_manage_ads(&claims));
    }
}
