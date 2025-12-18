use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::headers::{Authorization, authorization::Bearer};
use axum_extra::TypedHeader;
use jsonwebtoken::{encode, EncodingKey, Header};
use chrono::{Utc, Duration};
use crate::{
    db::DbPool,
    models::user::{User, LoginPayload, RegisterPayload, AuthResponse, Claims},
    utils::security::{hash_password, verify_password},
};

// POST /api/auth/register (Solo admins; primer usuario se permite sin token y queda como admin)
pub async fn register_handler(
    State(pool): State<DbPool>,
    // Token opcional: si ya existe un usuario, exigimos que sea admin
    maybe_auth: Option<TypedHeader<Authorization<Bearer>>>,
    Json(payload): Json<RegisterPayload>,
) -> impl IntoResponse {
    // Contamos usuarios existentes para decidir si es bootstrap
    let user_count = match sqlx::query_scalar!("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
    {
        Ok(count) => count.unwrap_or(0),
        Err(e) => {
            tracing::error!("Error contando usuarios: {:?}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Si ya hay usuarios, exigimos token admin
    if user_count > 0 {
        let TypedHeader(auth_header) = match maybe_auth {
            Some(h) => h,
            None => return (StatusCode::FORBIDDEN, "Solo un admin puede crear usuarios").into_response(),
        };

        let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET debe estar en .env");
        let token = auth_header.token();
        let validation = jsonwebtoken::Validation::default();
        let token_data = jsonwebtoken::decode::<Claims>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        );

        match token_data {
            Ok(data) => {
                if data.claims.role != "admin" {
                    return (StatusCode::FORBIDDEN, "Solo un admin puede crear usuarios").into_response();
                }
            }
            Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
        }
    }

    // 1. Hashear la contraseña (nunca guardarla plana)
    let hashed_password = match hash_password(&payload.password) {
        Ok(h) => h,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Error de seguridad").into_response(),
    };

    // Rol: el primer usuario se vuelve admin automáticamente; el resto, editor
    let role = if user_count == 0 { "admin" } else { "editor" };

    // 2. Insertar en Base de Datos
    let result = sqlx::query_as!(
        User,
        "INSERT INTO users (username, email, password_hash, role) 
         VALUES ($1, $2, $3, $4) 
         RETURNING id, username, email, password_hash, role",
        payload.username,
        payload.email,
        hashed_password,
        role
    )
    .fetch_one(&pool)
    .await;

    match result {
        Ok(_) => (StatusCode::CREATED, "Usuario creado exitosamente").into_response(),
        Err(e) => {
            tracing::error!("Error creando usuario: {:?}", e);
            // Probablemente el email ya existe
            (StatusCode::CONFLICT, "El usuario o email ya existe").into_response()
        }
    }
}

// POST /api/auth/login
pub async fn login_handler(
    State(pool): State<DbPool>,
    Json(payload): Json<LoginPayload>,
) -> impl IntoResponse {
    // 1. Buscar usuario por email
    let user = sqlx::query_as!(
        User,
        "SELECT id, username, email, password_hash, role FROM users WHERE email = $1",
        payload.email
    )
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    let user = match user {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, "Credenciales inválidas").into_response(),
    };

    // 2. Verificar contraseña (Argon2)
    let is_valid = verify_password(&payload.password, &user.password_hash);

    if !is_valid {
        return (StatusCode::UNAUTHORIZED, "Credenciales inválidas").into_response();
    }

    // 3. Generar JWT Token
    // Calculamos expiración (ej: 24 horas desde ahora)
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("Fecha inválida")
        .timestamp() as usize;

    let claims = Claims {
        sub: user.email.clone(),
        exp: expiration,
        iat: Utc::now().timestamp() as usize,
        user_id: user.id,
        role: user.role,
    };

    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET debe estar en .env");
    
    let token = encode(
        &Header::default(), 
        &claims, 
        &EncodingKey::from_secret(secret.as_bytes())
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);

    match token {
        Ok(t) => (StatusCode::OK, Json(AuthResponse { 
            token: t,
            token_type: "Bearer".to_string() 
        })).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Error generando token").into_response(),
    }
}
