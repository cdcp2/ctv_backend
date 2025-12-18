use axum::{
    extract::{Json, Path, Query, State}, // --- NUEVO: Agregamos 'Path' para leer IDs de la URL
    Extension,
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;
use crate::{db::DbPool, models::article::{Article, CreateArticleSchema}, models::user::Claims};

#[derive(Debug, Deserialize)]
pub struct UpdateArticleSchema {
    pub title: Option<String>,
    pub content: Option<String>,
    pub excerpt: Option<String>,
    pub category_id: Option<i32>,
    pub main_image_url: Option<String>,
    pub video_embed_url: Option<String>,
    pub status: Option<String>,
    pub is_featured: Option<bool>,
    pub is_breaking: Option<bool>,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct FilterOptions {
    pub category_id: Option<i32>,
    pub search: Option<String>, // <--- NUEVO CAMPO DE BÚSQUEDA
    pub is_featured: Option<bool>,
    pub is_breaking: Option<bool>,
    pub has_video: Option<bool>,
}

// GET /api/articles (Soporta ?category_id=1&search=texto)
pub async fn list_articles_handler(
    opts: Option<Query<FilterOptions>>,
    State(pool): State<DbPool>,
) -> impl IntoResponse {
    // Extraemos los valores o los dejamos en None
    let Query(opts) = opts.unwrap_or(Query(FilterOptions { 
        category_id: None, 
        search: None,
        is_featured: None,
        is_breaking: None,
        has_video: None,
    }));

    let category_id = opts.category_id;
    let search_term = opts.search;
    let is_featured = opts.is_featured;
    let is_breaking = opts.is_breaking;
    let has_video = opts.has_video;

    // --- LA SÚPER QUERY ---
    // Usamos lógica booleana dentro del SQL para filtrar dinámicamente.
    // ($1::int IS NULL OR category_id = $1): Si no envían categoría, ignora el filtro.
    // ILIKE: Búsqueda insensible a mayúsculas.
    // '%' || $2 || '%': Agrega comodines para buscar "cualquier parte del texto".
    
    let result = sqlx::query_as!(
        Article,
        r#"
        SELECT 
            id, 
            title, 
            slug, 
            content, 
            excerpt, 
            main_image_url, 
            video_embed_url,
            author_id, 
            category_id, 
            status as "status!: String", 
            is_featured as "is_featured!: bool", 
            is_breaking as "is_breaking!: bool", 
            views_count as "views_count!: i64",
            published_at, created_at, updated_at 
        FROM articles 
        WHERE 
            ($1::int IS NULL OR category_id = $1)
            AND
            ($2::text IS NULL OR (title ILIKE '%' || $2 || '%' OR content ILIKE '%' || $2 || '%'))
            AND
            ($3::bool IS NULL OR is_featured = $3)
            AND
            ($4::bool IS NULL OR is_breaking = $4)
            AND
            (
                $5::bool IS NULL OR 
                ($5 = TRUE AND video_embed_url IS NOT NULL) OR
                ($5 = FALSE AND video_embed_url IS NULL)
            )
        ORDER BY created_at DESC 
        LIMIT 20
        "#,
        category_id,
        search_term,
        is_featured,
        is_breaking,
        has_video
    )
    .fetch_all(&pool)
    .await;

    match result {
        Ok(data) => (StatusCode::OK, Json(data)).into_response(),
        Err(e) => {
            tracing::error!("Error buscando noticias: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error de base de datos").into_response()
        }
    }
}

// POST /api/articles/:slug/view - incrementar vistas
pub async fn increment_views_handler(
    Path(slug): Path<String>,
    State(pool): State<DbPool>,
) -> impl IntoResponse {
    let result = sqlx::query!(
        r#"
        UPDATE articles 
        SET views_count = views_count + 1 
        WHERE slug = $1
        RETURNING views_count
        "#,
        slug
    )
    .fetch_optional(&pool)
    .await;

    match result {
        Ok(Some(row)) => (StatusCode::OK, Json(serde_json::json!({ "views_count": row.views_count }))).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Noticia no encontrada").into_response(),
        Err(e) => {
            tracing::error!("Error incrementando vistas: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

// POST /api/articles - Crear noticia (IGUAL QUE ANTES)
pub async fn create_article_handler(
    State(pool): State<DbPool>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<CreateArticleSchema>,
) -> impl IntoResponse {
    let mut slug = slugify(&body.title);
    if slug.is_empty() {
        slug = format!("article-{}", Uuid::new_v4().simple());
    }

    let status = body.status.unwrap_or_else(|| "draft".to_string());
    let is_featured = body.is_featured.unwrap_or(false);
    let is_breaking = body.is_breaking.unwrap_or(false);

    let query_result = sqlx::query_as!(
        Article,
        r#"
        INSERT INTO articles (
            title, slug, content, excerpt, main_image_url, video_embed_url,
            author_id, category_id, status, is_featured, is_breaking, published_at
        ) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) 
         RETURNING 
            id, 
            title, 
            slug, 
            content, 
            excerpt, 
            main_image_url, 
            video_embed_url,
            author_id, 
            category_id, 
            status as "status!: String", 
            is_featured as "is_featured!: bool", 
            is_breaking as "is_breaking!: bool", 
            views_count as "views_count!: i64",
            published_at, created_at, updated_at
        "#,
        body.title,
        slug,
        body.content,
        body.excerpt,
        body.main_image_url,
        body.video_embed_url,
        Some(claims.user_id),
        body.category_id,
        status,
        is_featured,
        is_breaking,
        body.published_at
    )
    .fetch_one(&pool)
    .await;

    match query_result {
        Ok(article) => (StatusCode::CREATED, Json(article)).into_response(),
        Err(e) => {
            tracing::error!("Error al crear noticia: {:?}", e);
            (StatusCode::BAD_REQUEST, "No se pudo crear la noticia").into_response()
        }
    }
}

// --- NUEVO: DELETE /api/articles/:id ---
pub async fn delete_article_handler(
    Path(id): Path<i64>, // Extraemos el ID de la URL
    State(pool): State<DbPool>,
) -> impl IntoResponse {
    // query! (con signo de admiración) verifica el SQL pero no devuelve filas mapeadas
    let result = sqlx::query!("DELETE FROM articles WHERE id = $1", id)
        .execute(&pool)
        .await;

    match result {
        Ok(res) => {
            // rows_affected nos dice si realmente borró algo
            if res.rows_affected() == 0 {
                (StatusCode::NOT_FOUND, "Noticia no encontrada").into_response()
            } else {
                (StatusCode::OK, "Noticia eliminada correctamente").into_response()
            }
        }
        Err(e) => {
            tracing::error!("Error eliminando noticia: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

// --- NUEVO: PUT /api/articles/:id ---
pub async fn update_article_handler(
    Path(id): Path<i64>,
    State(pool): State<DbPool>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<UpdateArticleSchema>,
) -> impl IntoResponse {
    // Verificamos si existe primero para no dar falsos positivos
    let existing = match sqlx::query!(
        "SELECT id, author_id FROM articles WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await {
        Ok(row) => row,
        Err(e) => {
            tracing::error!("Error buscando noticia {}: {:?}", id, e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response();
        }
    };

    let existing = match existing {
        Some(row) => row,
        None => return (StatusCode::NOT_FOUND, "Noticia no encontrada").into_response(),
    };

    // Autorización: admin puede todo, editor solo sus artículos
    let is_admin = claims.role == "admin";
    let is_owner = existing.author_id == Some(claims.user_id);

    if !is_admin && !is_owner {
        return (StatusCode::FORBIDDEN, "No puedes editar noticias de otros").into_response();
    }

    // Truco SQL: COALESCE($1, title) significa:
    // "Si el valor $1 que me envían es NULL, deja el 'title' que ya estaba en la base de datos".
    let result = sqlx::query_as!(
        Article,
        r#"
         UPDATE articles SET 
            title = COALESCE($1, title),
            content = COALESCE($2, content),
            excerpt = COALESCE($3, excerpt),
            category_id = COALESCE($4, category_id),
            main_image_url = COALESCE($5, main_image_url),
            video_embed_url = COALESCE($6, video_embed_url),
            status = COALESCE($7, status),
            is_featured = COALESCE($8, is_featured),
            is_breaking = COALESCE($9, is_breaking),
            published_at = COALESCE($10, published_at),
            updated_at = NOW() 
         WHERE id = $11
         RETURNING 
            id, 
            title, 
            slug, 
            content, 
            excerpt, 
            main_image_url, 
            video_embed_url,
            author_id, 
            category_id, 
            status as "status!: String", 
            is_featured as "is_featured!: bool", 
            is_breaking as "is_breaking!: bool", 
            views_count as "views_count!: i64",
            published_at, created_at, updated_at
        "#,
        body.title,
        body.content,
        body.excerpt,
        body.category_id,
        body.main_image_url,
        body.video_embed_url,
        body.status,
        body.is_featured,
        body.is_breaking,
        body.published_at,
        id
    )
    .fetch_one(&pool)
    .await;

    match result {
        Ok(updated_article) => (StatusCode::OK, Json(updated_article)).into_response(),
        Err(e) => {
            tracing::error!("Error actualizando noticia: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error al actualizar").into_response()
        }
    }
}

// GET /api/articles/:slug - Leer una noticia individual
pub async fn get_article_handler(
    Path(slug): Path<String>, // Leemos el slug (ej: "robo-en-centro")
    State(pool): State<DbPool>,
) -> impl IntoResponse {
    let result = sqlx::query_as!(
        Article,
        r#"
         SELECT 
            id, 
            title, 
            slug, 
            content, 
            excerpt, 
            main_image_url, 
            video_embed_url,
            author_id, 
            category_id, 
            status as "status!: String", 
            is_featured as "is_featured!: bool", 
            is_breaking as "is_breaking!: bool", 
            views_count as "views_count!: i64",
             published_at, created_at, updated_at 
         FROM articles 
         WHERE slug = $1
        "#,
        slug
    )
    .fetch_optional(&pool) // fetch_optional devuelve Option<Article> (puede ser None)
    .await;

    match result {
        Ok(Some(article)) => (StatusCode::OK, Json(article)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Noticia no encontrada").into_response(),
        Err(e) => {
            tracing::error!("Error buscando noticia {}: {:?}", slug, e);
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
