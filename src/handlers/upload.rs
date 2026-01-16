use axum::{
    extract::Multipart,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde_json::json;
use std::{env, path::{Path, PathBuf}};
use tokio::fs;
use uuid::Uuid;
use mime::Mime;

const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024; // 5MB

fn upload_dir() -> PathBuf {
    // En Render pon UPLOAD_DIR=/var/data/uploads
    env::var("UPLOAD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/var/data/uploads"))
}

pub async fn upload_image_handler(mut multipart: Multipart) -> impl IntoResponse {
    let dir = upload_dir();

    // Crear carpeta si no existe (en el DISK)
    if let Err(e) = fs::create_dir_all(&dir).await {
        tracing::error!("No se pudo crear dir {:?}: {:?}", dir, e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "No se pudo preparar el directorio").into_response();
    }

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        if field.name().unwrap_or("") != "image" {
            continue;
        }

        let file_name = field.file_name().unwrap_or("unknown.jpg").to_string();
        let content_type: Option<Mime> = field.content_type().and_then(|ct| ct.parse::<Mime>().ok());

        let extension = Path::new(&file_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("jpg");

        let new_filename = format!("{}.{}", Uuid::new_v4(), extension);
        let filepath = dir.join(&new_filename);

        let data = match field.bytes().await {
            Ok(bytes) => bytes,
            Err(_) => return (StatusCode::BAD_REQUEST, "Error al leer el archivo").into_response(),
        };

        if data.len() > MAX_IMAGE_BYTES {
            return (StatusCode::BAD_REQUEST, "La imagen excede el tamaño máximo de 5MB").into_response();
        }

        if let Some(ct) = content_type {
            let allowed = matches!(
                (ct.type_().as_str(), ct.subtype().as_str()),
                ("image", "jpeg") | ("image", "png") | ("image", "webp") | ("image", "gif")
            );
            if !allowed {
                return (StatusCode::BAD_REQUEST, "Solo se permiten imágenes (jpg, png, webp, gif)").into_response();
            }
        }

        if let Err(e) = fs::write(&filepath, data).await {
            tracing::error!("Error guardando imagen: {:?} => {:?}", filepath, e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "No se pudo guardar la imagen").into_response();
        }

        // URL pública (route), NO la ruta del disco
        let public_url = format!("/uploads/{}", new_filename);

        return (StatusCode::OK, Json(json!({
            "url": public_url,
            "original_name": file_name
        }))).into_response();
    }

    (StatusCode::BAD_REQUEST, "No se envió ningún campo 'image'").into_response()
}
