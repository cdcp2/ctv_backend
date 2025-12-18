use axum::{extract::{State, Json}, http::StatusCode, response::IntoResponse};
use crate::{db::DbPool, models::site_config::{SiteConfig, UpdateSiteConfigSchema}};

// GET /api/site-config (público)
pub async fn get_site_config_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let result = sqlx::query_as!(
        SiteConfig,
        r#"SELECT id, live_stream_url, is_live_active as "is_live_active!: bool", breaking_news_banner FROM site_config WHERE id = 1"#
    )
    .fetch_optional(&pool)
    .await;

    match result {
        Ok(Some(cfg)) => (StatusCode::OK, Json(cfg)).into_response(),
        Ok(None) => {
            // Si no existe, devolvemos defaults
            let cfg = SiteConfig {
                id: 1,
                live_stream_url: None,
                is_live_active: true,
                breaking_news_banner: None,
            };
            (StatusCode::OK, Json(cfg)).into_response()
        }
        Err(e) => {
            tracing::error!("Error leyendo site_config: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

// PUT /api/admin/site-config (admin)
pub async fn update_site_config_handler(
    State(pool): State<DbPool>,
    Json(body): Json<UpdateSiteConfigSchema>,
) -> impl IntoResponse {
    // Upsert sencillo
    let result = sqlx::query_as!(
        SiteConfig,
        r#"
        INSERT INTO site_config (id, live_stream_url, is_live_active, breaking_news_banner)
        VALUES (1, $1, COALESCE($2, true), $3)
        ON CONFLICT (id) DO UPDATE SET
            live_stream_url = COALESCE(EXCLUDED.live_stream_url, site_config.live_stream_url),
            is_live_active = COALESCE(EXCLUDED.is_live_active, site_config.is_live_active),
            breaking_news_banner = COALESCE(EXCLUDED.breaking_news_banner, site_config.breaking_news_banner)
        RETURNING id, live_stream_url, is_live_active as "is_live_active!: bool", breaking_news_banner
        "#,
        body.live_stream_url,
        body.is_live_active,
        body.breaking_news_banner
    )
    .fetch_one(&pool)
    .await;

    match result {
        Ok(cfg) => (StatusCode::OK, Json(cfg)).into_response(),
        Err(e) => {
            tracing::error!("Error actualizando site_config: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}
