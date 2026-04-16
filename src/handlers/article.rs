use crate::{
    db::DbPool,
    models::article::{Article, CreateArticleSchema},
    models::user::Claims,
};
use axum::{
    Extension,
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use tracing;
use uuid::Uuid;

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
    pub tag_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ViewsQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ArticleViewsRow {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub views_count: i64,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
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
        tag_id: None,
    }));

    let category_id = opts.category_id;
    let search_term = opts.search;
    let is_featured = opts.is_featured;
    let is_breaking = opts.is_breaking;
    let has_video = opts.has_video;
    let tag_id = opts.tag_id;

    // Usamos lógica booleana dentro del SQL para filtrar dinámicamente.
    // ($1::int IS NULL OR category_id = $1): Si no envían categoría, ignora el filtro.
    // ILIKE: Búsqueda insensible a mayúsculas.
    // '%' || $2 || '%': Agrega comodines para buscar "cualquier parte del texto".

    let result = sqlx::query_as::<_, Article>(
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
            status,
            is_featured,
            is_breaking,
            views_count,
            published_at, created_at, updated_at 
        FROM articles 
        WHERE 
            status = 'published'
            AND (published_at IS NULL OR published_at <= NOW())
            AND
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
                ($5 = TRUE AND NULLIF(BTRIM(video_embed_url), '') IS NOT NULL) OR
                ($5 = FALSE AND NULLIF(BTRIM(video_embed_url), '') IS NULL)
            )
            AND
            ($6::int IS NULL OR EXISTS (
                SELECT 1 FROM article_tags at WHERE at.article_id = articles.id AND at.tag_id = $6
            ))
        ORDER BY published_at DESC NULLS LAST, created_at DESC 
        LIMIT 20
        "#,
    )
    .bind(category_id)
    .bind(search_term)
    .bind(is_featured)
    .bind(is_breaking)
    .bind(has_video)
    .bind(tag_id)
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

// GET /api/articles/views
pub async fn article_views_handler(
    opts: Option<Query<ViewsQuery>>,
    State(pool): State<DbPool>,
) -> impl IntoResponse {
    let Query(opts) = opts.unwrap_or(Query(ViewsQuery { limit: None }));
    let limit = opts.limit.filter(|value| *value > 0);

    let result = if let Some(limit) = limit {
        sqlx::query_as::<_, ArticleViewsRow>(
            r#"
            SELECT
                id,
                title,
                slug,
                views_count,
                published_at,
                created_at
            FROM articles
            ORDER BY views_count DESC, published_at DESC NULLS LAST, created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&pool)
        .await
    } else {
        sqlx::query_as::<_, ArticleViewsRow>(
            r#"
            SELECT
                id,
                title,
                slug,
                views_count,
                published_at,
                created_at
            FROM articles
            ORDER BY views_count DESC, published_at DESC NULLS LAST, created_at DESC
            "#,
        )
        .fetch_all(&pool)
        .await
    };

    match result {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e) => {
            tracing::error!("Error consultando vistas: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

// GET /api/articles/most-read
pub async fn most_read_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Article>(
        r#"
        SELECT 
            id, title, slug, content, excerpt, main_image_url, video_embed_url,
            author_id, category_id, status, is_featured,
            is_breaking, views_count,
            published_at, created_at, updated_at
        FROM articles
        WHERE status = 'published'
          AND (published_at IS NULL OR published_at <= NOW())
        ORDER BY views_count DESC, published_at DESC NULLS LAST, created_at DESC
        LIMIT 10
        "#
    )
    .fetch_all(&pool)
    .await;

    match result {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e) => {
            tracing::error!("Error consultando más leídas: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

pub async fn featured_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Article>(
        r#"
        SELECT 
            id, title, slug, content, excerpt, main_image_url, video_embed_url,
            author_id, category_id, status, is_featured,
            is_breaking, views_count,
            published_at, created_at, updated_at
        FROM articles
        WHERE status = 'published'
          AND (published_at IS NULL OR published_at <= NOW())
          AND is_featured = TRUE
        ORDER BY published_at DESC NULLS LAST, created_at DESC
        LIMIT 10
        "#
    )
    .fetch_all(&pool)
    .await;

    match result {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e) => {
            tracing::error!("Error consultando destacadas: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

pub async fn breaking_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Article>(
        r#"
        SELECT 
            id, title, slug, content, excerpt, main_image_url, video_embed_url,
            author_id, category_id, status, is_featured,
            is_breaking, views_count,
            published_at, created_at, updated_at
        FROM articles
        WHERE status = 'published'
          AND (published_at IS NULL OR published_at <= NOW())
          AND is_breaking = TRUE
        ORDER BY published_at DESC NULLS LAST, updated_at DESC
        LIMIT 10
        "#
    )
    .fetch_all(&pool)
    .await;

    match result {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e) => {
            tracing::error!("Error consultando breaking: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

pub async fn videos_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Article>(
        r#"
        SELECT 
            id, title, slug, content, excerpt, main_image_url, video_embed_url,
            author_id, category_id, status, is_featured,
            is_breaking, views_count,
            published_at, created_at, updated_at
        FROM articles
        WHERE status = 'published'
          AND (published_at IS NULL OR published_at <= NOW())
          AND NULLIF(BTRIM(video_embed_url), '') IS NOT NULL
        ORDER BY published_at DESC NULLS LAST, created_at DESC
        LIMIT 10
        "#
    )
    .fetch_all(&pool)
    .await;

    match result {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e) => {
            tracing::error!("Error consultando videos: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
        }
    }
}

pub async fn related_handler(
    Path(slug): Path<String>,
    State(pool): State<DbPool>,
) -> impl IntoResponse {
    // Obtener artículo base
    let base = sqlx::query(
        r#"
        SELECT id, category_id
        FROM articles
        WHERE slug = $1
          AND status = 'published'
          AND (published_at IS NULL OR published_at <= NOW())
        "#,
    )
    .bind(&slug)
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    let base = match base {
        Some(row) => (
            row.get::<i64, _>("id"),
            row.get::<Option<i32>, _>("category_id"),
        ),
        None => return (StatusCode::NOT_FOUND, "Noticia no encontrada").into_response(),
    };

    // Relacionados por categoría o tags compartidos
    let result = sqlx::query_as::<_, Article>(
        r#"
        SELECT 
            a.id, a.title, a.slug, a.content, a.excerpt, a.main_image_url, a.video_embed_url,
            a.author_id, a.category_id, a.status, a.is_featured,
            a.is_breaking, a.views_count,
            a.published_at, a.created_at, a.updated_at
        FROM articles a
        WHERE a.id <> $1
          AND a.status = 'published'
          AND (a.published_at IS NULL OR a.published_at <= NOW())
          AND (
              (a.category_id IS NOT NULL AND a.category_id = $2)
              OR EXISTS (
                  SELECT 1 FROM article_tags at1
                  WHERE at1.article_id = a.id
                    AND at1.tag_id IN (SELECT tag_id FROM article_tags WHERE article_id = $1)
              )
          )
        ORDER BY a.published_at DESC NULLS LAST, a.created_at DESC
        LIMIT 5
        "#,
    )
    .bind(base.0)
    .bind(base.1)
    .fetch_all(&pool)
    .await;

    match result {
        Ok(rows) => (StatusCode::OK, Json(rows)).into_response(),
        Err(e) => {
            tracing::error!("Error consultando relacionados: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error interno").into_response()
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
        Ok(Some(row)) => (
            StatusCode::OK,
            Json(serde_json::json!({ "views_count": row.views_count })),
        )
            .into_response(),
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
        Ok(article) => {
            tracing::info!(
                "article_created id={} author_id={}",
                article.id,
                claims.user_id
            );
            (StatusCode::CREATED, Json(article)).into_response()
        }
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
                tracing::info!("article_deleted id={}", id);
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
    let existing = match sqlx::query!("SELECT id, author_id FROM articles WHERE id = $1", id)
        .fetch_optional(&pool)
        .await
    {
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
        Ok(updated_article) => {
            tracing::info!(
                "article_updated id={} by_user={}",
                updated_article.id,
                claims.user_id
            );
            (StatusCode::OK, Json(updated_article)).into_response()
        }
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
) -> Response {
    // Si llega "/api/articles/videos" y por algún motivo el router cae aquí, devolvemos el listado de videos
    if slug == "videos" {
        return videos_handler(State(pool)).await.into_response();
    }

    let result = sqlx::query_as::<_, Article>(
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
            status,
            is_featured,
            is_breaking,
            views_count,
             published_at, created_at, updated_at 
         FROM articles 
         WHERE slug = $1
           AND status = 'published'
           AND (published_at IS NULL OR published_at <= NOW())
        "#,
    )
    .bind(slug)
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
