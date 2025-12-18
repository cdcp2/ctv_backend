use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use crate::{db::DbPool, models::category::Category};

pub async fn list_categories_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let categories = sqlx::query_as!(
        Category,
        "SELECT id, name, slug, description FROM categories ORDER BY id ASC"
    )
    .fetch_all(&pool)
    .await;

    match categories {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err(e) => {
            tracing::error!("Error fetching categories: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}