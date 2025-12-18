mod db;
mod models;
mod handlers;
mod routes;
mod utils;

use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tower_http::cors::{CorsLayer, Any};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool = db::init_db().await;
    tracing::info!("✅ Conexión a Postgres exitosa");

   
    let cors = CorsLayer::new()
        .allow_origin(Any) 
        .allow_methods(Any)
        .allow_headers(Any);

    let app = routes::create_routes(pool)
        .layer(cors);

    let puerto = std::env::var("PORT").unwrap_or("3000".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", puerto).parse().expect("Dirección IP/Puerto inválido");
    
    tracing::info!("🚀 Servidor CTV corriendo en http://{}", addr);

    let listener = TcpListener::bind(addr).await.expect("Fallo al enlazar el puerto");
    axum::serve(listener, app).await.unwrap();
}