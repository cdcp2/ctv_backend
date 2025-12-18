use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// Lo que guardamos en la base de datos
#[derive(Debug, Serialize, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    #[serde(skip)] // ¡Jamás envíes el hash de la contraseña en el JSON!
    pub password_hash: String,
    pub role: String,
}

// Lo que recibimos para hacer Login
#[derive(Debug, Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
}

// Lo que recibimos para Registrar un usuario (solo para uso interno inicial)
#[derive(Debug, Deserialize)]
pub struct RegisterPayload {
    pub username: String,
    pub email: String,
    pub password: String,
}

// Lo que devolvemos cuando el login es exitoso
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub token_type: String,
}

// Lo que viaja DENTRO del token encriptado (Claims)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // Subject (email o usuario)
    pub exp: usize,  // Expiración
    pub iat: usize,  // Issued At
    pub user_id: i64,
    pub role: String,
}
