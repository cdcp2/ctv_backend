use serde::{Deserialize, Serialize};
use sqlx::FromRow;

pub const ROLE_ADMIN: &str = "admin";
pub const ROLE_SUB_ADMIN: &str = "sub_admin";
pub const ROLE_SUBADMIN_ALIAS: &str = "subadmin";
pub const ROLE_EDITOR: &str = "editor";

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
    pub role: Option<String>,
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

pub fn canonicalize_role(role: &str) -> &str {
    match role.trim() {
        ROLE_SUBADMIN_ALIAS => ROLE_SUB_ADMIN,
        other => other,
    }
}

pub fn is_admin_role(role: &str) -> bool {
    canonicalize_role(role) == ROLE_ADMIN
}

pub fn is_sub_admin_role(role: &str) -> bool {
    canonicalize_role(role) == ROLE_SUB_ADMIN
}

pub fn is_admin_or_sub_admin_role(role: &str) -> bool {
    is_admin_role(role) || is_sub_admin_role(role)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_subadmin_alias() {
        assert_eq!(canonicalize_role("subadmin"), ROLE_SUB_ADMIN);
        assert_eq!(canonicalize_role("sub_admin"), ROLE_SUB_ADMIN);
    }

    #[test]
    fn recognizes_admin_and_subadmin_roles() {
        assert!(is_admin_role("admin"));
        assert!(is_admin_or_sub_admin_role("subadmin"));
        assert!(is_admin_or_sub_admin_role("sub_admin"));
        assert!(!is_admin_or_sub_admin_role(ROLE_EDITOR));
    }
}
