use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use axum_extra::headers::{Authorization, authorization::Bearer};
use axum_extra::TypedHeader;
use jsonwebtoken::{decode, DecodingKey, Validation};
use crate::{models::user::Claims};

// Esta función se ejecutará ANTES de llegar al handler de crear noticia
pub async fn auth_middleware(
    // Axum extrae automáticamente el header "Authorization: Bearer <token>"
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Obtener el token del header
    let token = auth.token();

    // 2. Obtener el secreto
    let secret = std::env::var("JWT_SECRET").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 3. Decodificar y verificar firma
    let validation = Validation::default();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    );

    match token_data {
        Ok(data) => {
            // Adjuntamos claims para que los handlers sepan quién es el usuario
            request.extensions_mut().insert(data.claims);
            Ok(next.run(request).await)
        }
        Err(_) => {
            // Token falso, expirado o manipulado
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

pub async fn admin_middleware(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Obtener y decodificar token (igual que el otro middleware)
    let token = auth.token();
    let secret = std::env::var("JWT_SECRET").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let validation = Validation::default();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    );

    match token_data {
        Ok(data) => {
            // 2. VERIFICACIÓN EXTRA: ¿Es Admin?
            if data.claims.role == "admin" {
                // Adjuntamos claims por si se necesitan aguas abajo
                request.extensions_mut().insert(data.claims);
                // Si es admin, pase señor
                Ok(next.run(request).await)
            } else {
                // Si es editor, error 403 (Prohibido)
                Err(StatusCode::FORBIDDEN)
            }
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}
