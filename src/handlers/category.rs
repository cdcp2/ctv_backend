use crate::{db::DbPool, models::category::Category};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateCategoryPayload {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
}

pub async fn list_categories_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let categories = sqlx::query_as!(
        Category,
        "SELECT id, name, slug, description FROM categories ORDER BY id ASC"
    )
    .fetch_all(&pool)
    .await;

    match categories {
        Ok(data) if !data.is_empty() => (StatusCode::OK, Json(data)).into_response(),
        Ok(_) => {
            // Fallback de categorías básicas si la tabla está vacía
            let fallback = vec![
                Category {
                    id: 0,
                    name: "Barranquilla".into(),
                    slug: "barranquilla".into(),
                    description: None,
                },
                Category {
                    id: 0,
                    name: "Atlántico".into(),
                    slug: "atlantico".into(),
                    description: None,
                },
                Category {
                    id: 0,
                    name: "Judiciales".into(),
                    slug: "judiciales".into(),
                    description: None,
                },
                Category {
                    id: 0,
                    name: "Deportes".into(),
                    slug: "deportes".into(),
                    description: None,
                },
                Category {
                    id: 0,
                    name: "Cultura".into(),
                    slug: "cultura".into(),
                    description: None,
                },
                Category {
                    id: 0,
                    name: "Economía".into(),
                    slug: "economia".into(),
                    description: None,
                },
                Category {
                    id: 0,
                    name: "Opinión".into(),
                    slug: "opinion".into(),
                    description: None,
                },
            ];
            (StatusCode::OK, Json(fallback)).into_response()
        }
        Err(e) => {
            tracing::error!("Error fetching categories: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

// POST /api/admin/categories
pub async fn create_category_handler(
    State(pool): State<DbPool>,
    Json(body): Json<CreateCategoryPayload>,
) -> impl IntoResponse {
    let slug = body.slug.clone().unwrap_or_else(|| slugify(&body.name));
    let result = sqlx::query_as::<_, Category>(
        r#"INSERT INTO categories (name, slug, description) VALUES ($1, $2, $3) RETURNING id, name, slug, description"#,
    )
    .bind(&body.name)
    .bind(&slug)
    .bind(&body.description)
    .fetch_one(&pool)
    .await;

    match result {
        Ok(cat) => (StatusCode::CREATED, Json(cat)).into_response(),
        Err(e) => {
            tracing::error!("Error creando categoría: {:?}", e);
            (StatusCode::BAD_REQUEST, "No se pudo crear la categoría").into_response()
        }
    }
}

// DELETE /api/admin/categories/:id
pub async fn delete_category_handler(
    axum::extract::Path(id): axum::extract::Path<i32>,
    State(pool): State<DbPool>,
) -> impl IntoResponse {
    let res = sqlx::query("DELETE FROM categories WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await;

    match res {
        Ok(exec) if exec.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, "Categoría no encontrada").into_response(),
        Err(e) => {
            tracing::error!("Error borrando categoría: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

fn slugify(input: &str) -> String {
    let mut slug = String::with_capacity(input.len());
    let mut prev_hyphen = false;

    for ch in input.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            prev_hyphen = false;
        } else if !prev_hyphen {
            slug.push('-');
            prev_hyphen = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    while slug.starts_with('-') {
        slug.remove(0);
    }
    slug
}
