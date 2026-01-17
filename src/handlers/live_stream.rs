use axum::{extract::{Json, State}, http::StatusCode, response::IntoResponse};
use uuid::Uuid;
use crate::{db::DbPool, models::live_stream::{LiveStreamConfig, UpdateLiveStreamConfig}};

// GET /api/admin/live-stream (admin o sub-admin)
pub async fn get_live_stream_config_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let result = sqlx::query_as!(
        LiveStreamConfig,
        r#"
        SELECT
            id,
            ingest_url,
            stream_key,
            server_main_url,
            server_backup_url,
            playback_url,
            is_active as "is_active!: bool",
            notes,
            updated_at,
            CASE
                WHEN server_main_url IS NOT NULL AND stream_key IS NOT NULL THEN concat_ws('/', rtrim(server_main_url, '/'), stream_key)
                ELSE NULL
            END as rtmp_url_primary,
            CASE
                WHEN server_backup_url IS NOT NULL AND stream_key IS NOT NULL THEN concat_ws('/', rtrim(server_backup_url, '/'), stream_key)
                ELSE NULL
            END as rtmp_url_backup
        FROM live_stream_config
        WHERE id = 1
        "#
    )
    .fetch_optional(&pool)
    .await;

    match result {
        Ok(Some(cfg)) => (StatusCode::OK, Json(cfg)).into_response(),
        Ok(None) => {
            let cfg = LiveStreamConfig {
                id: 1,
                ingest_url: None,
                stream_key: None,
                server_main_url: None,
                server_backup_url: None,
                playback_url: None,
                is_active: false,
                notes: None,
                updated_at: None,
                rtmp_url_primary: None,
                rtmp_url_backup: None,
            };
            (StatusCode::OK, Json(cfg)).into_response()
        }
        Err(e) => {
            tracing::error!("Error leyendo live_stream_config: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

// PUT /api/admin/live-stream (admin o sub-admin)
pub async fn upsert_live_stream_config_handler(
    State(pool): State<DbPool>,
    Json(body): Json<UpdateLiveStreamConfig>,
) -> impl IntoResponse {
    let result = sqlx::query_as!(
        LiveStreamConfig,
        r#"
        INSERT INTO live_stream_config (id, ingest_url, stream_key, server_main_url, server_backup_url, playback_url, is_active, notes, updated_at)
        VALUES (1, $1, $2, $3, $4, $5, COALESCE($6, true), $7, NOW())
        ON CONFLICT (id) DO UPDATE SET
            ingest_url = COALESCE(EXCLUDED.ingest_url, live_stream_config.ingest_url),
            stream_key = COALESCE(EXCLUDED.stream_key, live_stream_config.stream_key),
            server_main_url = COALESCE(EXCLUDED.server_main_url, live_stream_config.server_main_url),
            server_backup_url = COALESCE(EXCLUDED.server_backup_url, live_stream_config.server_backup_url),
            playback_url = COALESCE(EXCLUDED.playback_url, live_stream_config.playback_url),
            is_active = COALESCE(EXCLUDED.is_active, live_stream_config.is_active),
            notes = COALESCE(EXCLUDED.notes, live_stream_config.notes),
            updated_at = NOW()
        RETURNING
            id,
            ingest_url,
            stream_key,
            server_main_url,
            server_backup_url,
            playback_url,
            is_active as "is_active!: bool",
            notes,
            updated_at,
            CASE
                WHEN server_main_url IS NOT NULL AND stream_key IS NOT NULL THEN concat_ws('/', rtrim(server_main_url, '/'), stream_key)
                ELSE NULL
            END as rtmp_url_primary,
            CASE
                WHEN server_backup_url IS NOT NULL AND stream_key IS NOT NULL THEN concat_ws('/', rtrim(server_backup_url, '/'), stream_key)
                ELSE NULL
            END as rtmp_url_backup
        "#,
        body.ingest_url,
        body.stream_key,
        body.server_main_url,
        body.server_backup_url,
        body.playback_url,
        body.is_active,
        body.notes
    )
    .fetch_one(&pool)
    .await;

    match result {
        Ok(cfg) => (StatusCode::OK, Json(cfg)).into_response(),
        Err(e) => {
            tracing::error!("Error actualizando live_stream_config: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

// POST /api/admin/live-stream/rotate-key (admin o sub-admin)
pub async fn rotate_stream_key_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let new_key = format!("ctv-{}", Uuid::new_v4().simple());

    let result = sqlx::query_as!(
        LiveStreamConfig,
        r#"
        INSERT INTO live_stream_config (id, stream_key, updated_at)
        VALUES (1, $1, NOW())
        ON CONFLICT (id) DO UPDATE SET
            stream_key = EXCLUDED.stream_key,
            updated_at = NOW()
        RETURNING
            id,
            ingest_url,
            stream_key,
            server_main_url,
            server_backup_url,
            playback_url,
            is_active as "is_active!: bool",
            notes,
            updated_at,
            CASE
                WHEN server_main_url IS NOT NULL AND stream_key IS NOT NULL THEN concat_ws('/', rtrim(server_main_url, '/'), stream_key)
                ELSE NULL
            END as rtmp_url_primary,
            CASE
                WHEN server_backup_url IS NOT NULL AND stream_key IS NOT NULL THEN concat_ws('/', rtrim(server_backup_url, '/'), stream_key)
                ELSE NULL
            END as rtmp_url_backup
        "#,
        new_key
    )
    .fetch_one(&pool)
    .await;

    match result {
        Ok(cfg) => (StatusCode::OK, Json(cfg)).into_response(),
        Err(e) => {
            tracing::error!("Error rotando stream_key: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}
