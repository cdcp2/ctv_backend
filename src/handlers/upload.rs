use axum::{
    extract::Multipart,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde_json::json;
use std::path::Path;
use tokio::fs; // Usamos el sistema de archivos asíncrono
use uuid::Uuid;
use mime::Mime;

// Configuración: Carpeta donde se guardarán las fotos
const UPLOAD_DIR: &str = "uploads";
const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024; // 5MB

pub async fn upload_image_handler(mut multipart: Multipart) -> impl IntoResponse {
    // 1. Crear la carpeta 'uploads' si no existe
    if !Path::new(UPLOAD_DIR).exists() {
        let _ = fs::create_dir_all(UPLOAD_DIR).await;
    }

    // 2. Buscar el campo "image" en el formulario enviado
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "image" {
            let file_name = field.file_name().unwrap_or("unknown.jpg").to_string();
            let content_type: Option<Mime> = field
                .content_type()
                .and_then(|ct_str| ct_str.parse::<Mime>().ok());
            
            // Obtener extensión (jpg, png)
            let extension = Path::new(&file_name)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("jpg");

            // 3. Generar nombre único (Ej: 550e8400-e29b....jpg)
            let new_filename = format!("{}.{}", Uuid::new_v4(), extension);
            let filepath = format!("{}/{}", UPLOAD_DIR, new_filename);

            // 4. Leer los bytes del archivo
            let data = match field.bytes().await {
                Ok(bytes) => bytes,
                Err(_) => return (StatusCode::BAD_REQUEST, "Error al leer el archivo").into_response(),
            };

            // 4.1 Validar tamaño
            if data.len() > MAX_IMAGE_BYTES {
                return (StatusCode::BAD_REQUEST, "La imagen excede el tamaño máximo de 5MB").into_response();
            }

            // 4.2 Validar MIME (solo imágenes comunes)
            if let Some(ct) = content_type {
                let type_str = ct.type_().as_str();
                let sub_str = ct.subtype().as_str();
                let allowed = matches!(
                    (type_str, sub_str),
                    ("image", "jpeg") | ("image", "png") | ("image", "webp") | ("image", "gif")
                );
                if !allowed {
                    return (StatusCode::BAD_REQUEST, "Solo se permiten imágenes (jpg, png, webp, gif)").into_response();
                }
            }

            // 5. Guardar en el disco duro
            if let Err(e) = fs::write(&filepath, data).await {
                tracing::error!("Error guardando imagen: {:?}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "No se pudo guardar la imagen").into_response();
            }

            // 6. Devolver la URL pública
            // La URL será: http://localhost:3000/uploads/nombre-raro.jpg
            let public_url = format!("/uploads/{}", new_filename);
            
            return (StatusCode::OK, Json(json!({ 
                "url": public_url,
                "original_name": file_name 
            }))).into_response();
        }
    }

    (StatusCode::BAD_REQUEST, "No se envió ningún campo 'image'").into_response()
}
