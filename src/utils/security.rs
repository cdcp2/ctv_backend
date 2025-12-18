use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};

// Hashear contraseña
pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    match argon2.hash_password(password.as_bytes(), &salt) {
        Ok(hash) => Ok(hash.to_string()),
        Err(_) => Err("Error al encriptar contraseña".to_string())
    }
}

// Verificar contraseña
pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}