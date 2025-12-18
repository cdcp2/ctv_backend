use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::env;

// Definimos un alias para "Pool<Postgres>"
pub type DbPool = Pool<Postgres>;

pub async fn init_db() -> DbPool {
    // Leemos la URL del archivo .env
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL no está definido en .env");

    // Creamos el pool de conexiones
    PgPoolOptions::new()
        .max_connections(5) // Máximo 5 conexiones simultáneas (ajustable)
        .connect(&db_url)
        .await
        .expect("Error al conectar a la Base de Datos. ¿Está corriendo Postgres?")
}