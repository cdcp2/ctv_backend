use axum::{extract::{Json, State}, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::query_as;
use uuid::Uuid;
use crate::{db::DbPool, models::live_stream::{LiveStreamConfig, UpdateLiveStreamConfig}};

#[derive(Serialize)]
struct SyncPayload {
    sha256: String,
}

#[derive(Serialize)]
struct SyncResponse {
    synced: bool,
}

async fn sync_stream_key(stream_key: &str) -> Result<bool, String> {
    let sync_url = std::env::var("STREAM_SYNC_URL").unwrap_or_default();
    if sync_url.trim().is_empty() {
        return Ok(false);
    }
    let sync_token = std::env::var("STREAM_SYNC_TOKEN").unwrap_or_default();
    let hash = format!("{:x}", Sha256::digest(stream_key.as_bytes()));

    let client = reqwest::Client::new();
    let mut req = client.post(sync_url).json(&SyncPayload { sha256: hash });
    if !sync_token.trim().is_empty() {
        req = req
            .header("X-Stream-Sync-Token", sync_token.clone())
            .header("Authorization", format!("Bearer {sync_token}"));
    }
    let res = req.send().await.map_err(|e| format!("Error de red: {e}"))?;
    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Sync falló ({}): {}", status, body));
    }
    Ok(true)
}

async fn get_current_stream_key(pool: &DbPool) -> Option<String> {
    sqlx::query_scalar::<_, Option<String>>("SELECT stream_key FROM live_stream_config WHERE id = 1")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .flatten()
}

// GET /api/admin/live-stream (admin o sub-admin)
pub async fn get_live_stream_config_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let result = query_as::<_, LiveStreamConfig>(r#"
        SELECT
            id,
            ingest_url,
            stream_key,
            server_main_url,
            server_backup_url,
            playback_url,
            is_active,
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
    "#)
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
    let current_key = get_current_stream_key(&pool).await;
    let should_sync = match (&current_key, &body.stream_key) {
        (Some(old_key), Some(new_key)) => old_key != new_key,
        (None, Some(_)) => true,
        _ => false,
    };
    if let Some(new_key) = body.stream_key.as_deref() {
        if should_sync {
            if let Err(err) = sync_stream_key(new_key).await {
                return (StatusCode::BAD_GATEWAY, err).into_response();
            }
        }
    }

    let result = query_as::<_, LiveStreamConfig>(r#"
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
            is_active,
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
    "#)
    .bind(body.ingest_url)
    .bind(body.stream_key)
    .bind(body.server_main_url)
    .bind(body.server_backup_url)
    .bind(body.playback_url)
    .bind(body.is_active)
    .bind(body.notes)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(cfg) => (StatusCode::OK, Json(cfg)).into_response(),
        Err(e) => {
            if should_sync {
                if let Some(old_key) = current_key.as_deref() {
                    let _ = sync_stream_key(old_key).await;
                }
            }
            tracing::error!("Error actualizando live_stream_config: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

// POST /api/admin/live-stream/rotate-key (admin o sub-admin)
pub async fn rotate_stream_key_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let current_key = get_current_stream_key(&pool).await;
    let new_key = format!("ctv-{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let sync_attempt = sync_stream_key(&new_key).await;
    if let Err(err) = sync_attempt {
        return (StatusCode::BAD_GATEWAY, err).into_response();
    }

    let result = query_as::<_, LiveStreamConfig>(r#"
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
            is_active,
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
    "#)
    .bind(new_key)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(cfg) => (StatusCode::OK, Json(cfg)).into_response(),
        Err(e) => {
            if let Some(old_key) = current_key.as_deref() {
                let _ = sync_stream_key(old_key).await;
            }
            tracing::error!("Error rotando stream_key: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

// POST /api/admin/live-stream/sync-key (admin o sub-admin)
pub async fn sync_stream_key_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let current_key = get_current_stream_key(&pool).await;
    let Some(stream_key) = current_key else {
        return (StatusCode::NOT_FOUND, "No hay clave configurada").into_response();
    };

    match sync_stream_key(&stream_key).await {
        Ok(true) => (StatusCode::OK, Json(SyncResponse { synced: true })).into_response(),
        Ok(false) => (StatusCode::PRECONDITION_FAILED, "STREAM_SYNC_URL no configurado").into_response(),
        Err(err) => (StatusCode::BAD_GATEWAY, err).into_response(),
    }
}
