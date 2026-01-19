use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post, delete, put},
    Router,
};
use tower_http::services::ServeDir;
use crate::{
    db::DbPool, 
    handlers::{article, category, auth, upload, site_config, tag, advertisement, live_stream},
    utils::jwt::{auth_middleware, admin_middleware, admin_or_subadmin_middleware} // <--- Importamos ambos middlewares
};

fn uploads_root() -> String {
    std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "/var/data/uploads".to_string())
}

pub fn create_routes(pool: DbPool) -> Router {
    let uploads_dir = uploads_root();
    // 1. Rutas Públicas (Todo el mundo)
    let public_routes = Router::new()
        .route("/api/categories", get(category::list_categories_handler))
        .route("/api/auth/register", post(auth::register_handler))
        .route("/api/auth/login", post(auth::login_handler))
        .route("/api/articles", get(article::list_articles_handler))
        .route("/api/articles/most-read", get(article::most_read_handler))
        .route("/api/articles/featured", get(article::featured_handler))
        .route("/api/articles/breaking", get(article::breaking_handler))
        .route("/api/articles/videos", get(article::videos_handler))
        .route("/api/articles/views", get(article::article_views_handler))
        .route("/api/articles/:slug/related", get(article::related_handler))
        .route("/api/articles/:slug", get(article::get_article_handler))
        .route("/api/articles/:slug/view", post(article::increment_views_handler))
        .route("/api/articles/:slug/tags", get(tag::list_article_tags_handler))
        .route("/api/site-config", get(site_config::get_site_config_handler))
        .route("/api/tags", get(tag::list_tags_handler))
        .route("/api/ads", get(advertisement::list_ads_handler))
        .route("/healthz", get(crate::handlers::health::health_handler))
        .nest_service("/uploads", ServeDir::new(uploads_dir));

    // 2. Rutas de Editores (Crear, Editar, Subir Foto) - Requieren Auth Básico
    let editor_routes = Router::new()
        .route("/api/articles", post(article::create_article_handler))
        .route("/api/admin/articles/:id", put(article::update_article_handler)) // Editar sí dejamos a editores
        .route("/api/admin/tags", post(tag::create_tag_handler))
        .route("/api/admin/articles/:id/tags", post(tag::set_article_tags_handler))
        .route("/api/upload", post(upload::upload_image_handler).layer(DefaultBodyLimit::disable()))
        .route_layer(middleware::from_fn(auth_middleware));

    // 3. Rutas de ADMIN (Borrar) - Requieren Auth de Admin
    let admin_routes = Router::new()
        .route("/api/admin/articles/:id", delete(article::delete_article_handler))
        .route("/api/admin/categories", post(category::create_category_handler))
        .route("/api/admin/categories/:id", delete(category::delete_category_handler))
        .route("/api/admin/tags/:id", delete(tag::delete_tag_handler))
        .route_layer(middleware::from_fn(admin_middleware));

    // 3.1 Configuración del sitio (Admin o Sub-Admin)
    let site_config_routes = Router::new()
        .route("/api/admin/site-config", put(site_config::update_site_config_handler))
        .route_layer(middleware::from_fn(admin_or_subadmin_middleware));

    // 3.2 Live stream (Admin o Sub-Admin)
    let live_stream_routes = Router::new()
        .route(
            "/api/admin/live-stream",
            get(live_stream::get_live_stream_config_handler)
                .put(live_stream::upsert_live_stream_config_handler),
        )
        .route(
            "/api/admin/live-stream/rotate-key",
            post(live_stream::rotate_stream_key_handler),
        )
        .route_layer(middleware::from_fn(admin_or_subadmin_middleware));

    // 4. Rutas de Ads (Admin o Sub-Admin)
    let ads_routes = Router::new()
        .route("/api/admin/ads", get(advertisement::list_admin_ads_handler))
        .route("/api/admin/ads", post(advertisement::create_ad_handler))
        .route("/api/admin/ads/:id", put(advertisement::update_ad_handler))
        .route("/api/admin/ads/:id", delete(advertisement::delete_ad_handler))
        .route_layer(middleware::from_fn(admin_or_subadmin_middleware));

    // Fusionamos todo
    Router::new()
        .merge(public_routes)
        .merge(editor_routes)
        .merge(admin_routes)
        .merge(site_config_routes)
        .merge(live_stream_routes)
        .merge(ads_routes)
        .with_state(pool)
}
