mod db;
mod models;
mod handlers;
mod routes;
mod utils;

use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use http::HeaderValue;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

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

   
    let allowed_origins = vec![
        "https://ctvbarranquilla.com",
        "https://www.ctvbarranquilla.com",
        "https://admin.ctvbarranquilla.com",
        "https://api.ctvbarranquilla.com",
        // locales de desarrollo
        "http://localhost:3000",
        "http://localhost:3001",
        "http://localhost:5173",
        "http://127.0.0.1:5173",
    ];

    let allow_origin = AllowOrigin::list(
        allowed_origins
            .iter()
            .filter_map(|o| HeaderValue::from_str(o).ok())
    );

    let cors = CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods(AllowMethods::any())
        .allow_headers(AllowHeaders::any());

    let app = routes::create_routes(pool)
        .layer(cors);

    let puerto = std::env::var("PORT").unwrap_or("3000".to_string());
    let addr: SocketAddr = format!("0.0.0.0:{}", puerto).parse().expect("Dirección IP/Puerto inválido");
    
    tracing::info!("🚀 Servidor CTV corriendo en http://{}", addr);

    let listener = TcpListener::bind(addr).await.expect("Fallo al enlazar el puerto");
    axum::serve(listener, app).await.unwrap();
}
