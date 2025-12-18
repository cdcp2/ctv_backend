use axum::{
    middleware,
    routing::{get, post, delete, put},
    Router,
};
use tower_http::services::ServeDir;
use crate::{
    db::DbPool, 
    handlers::{article, category, auth, upload, site_config, tag},
    utils::jwt::{auth_middleware, admin_middleware} // <--- Importamos ambos middlewares
};

pub fn create_routes(pool: DbPool) -> Router {
    // 1. Rutas Públicas (Todo el mundo)
    let public_routes = Router::new()
        .route("/api/categories", get(category::list_categories_handler))
        .route("/api/auth/register", post(auth::register_handler))
        .route("/api/auth/login", post(auth::login_handler))
        .route("/api/articles", get(article::list_articles_handler))
        .route("/api/articles/:slug", get(article::get_article_handler))
        .route("/api/articles/:slug/view", post(article::increment_views_handler))
        .route("/api/site-config", get(site_config::get_site_config_handler))
        .route("/api/tags", get(tag::list_tags_handler))
        .nest_service("/uploads", ServeDir::new("uploads"));

    // 2. Rutas de Editores (Crear, Editar, Subir Foto) - Requieren Auth Básico
    let editor_routes = Router::new()
        .route("/api/articles", post(article::create_article_handler))
        .route("/api/admin/articles/:id", put(article::update_article_handler)) // Editar sí dejamos a editores
        .route("/api/upload", post(upload::upload_image_handler))
        .route_layer(middleware::from_fn(auth_middleware));

    // 3. Rutas de ADMIN (Borrar) - Requieren Auth de Admin
    let admin_routes = Router::new()
        .route("/api/admin/articles/:id", delete(article::delete_article_handler))
        .route("/api/admin/site-config", put(site_config::update_site_config_handler))
        .route("/api/admin/tags", post(tag::create_tag_handler))
        .route_layer(middleware::from_fn(admin_middleware));

    // Fusionamos todo
    Router::new()
        .merge(public_routes)
        .merge(editor_routes)
        .merge(admin_routes)
        .with_state(pool)
}
