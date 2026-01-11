-- Add migration script here
-- 1. Configuraciones iniciales y extensiones
CREATE EXTENSION IF NOT EXISTS "uuid-ossp"; -- Para IDs únicos si los necesitamos
CREATE EXTENSION IF NOT EXISTS "pg_trgm";   -- Para búsquedas de texto difusas (fuzzy search)

-- 2. Tabla de Usuarios (Periodistas y Admins)
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(50) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(20) NOT NULL DEFAULT 'editor', -- 'admin', 'editor'
    full_name VARCHAR(100),
    active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 3. Tabla de Categorías (Judiciales, Deportes, etc.)
CREATE TABLE categories (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL,
    slug VARCHAR(50) NOT NULL UNIQUE, -- ej: /judiciales
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 4. Tabla de Etiquetas (Tags: Carnaval, Junior, Lluvias)
CREATE TABLE tags (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL UNIQUE,
    slug VARCHAR(50) NOT NULL UNIQUE
);

-- 5. Tabla PRINCIPAL: Noticias (Articles)
CREATE TABLE articles (
    id BIGSERIAL PRIMARY KEY,
    
    -- Contenido Básico
    title VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL UNIQUE, -- URL amigable
    excerpt TEXT, -- La bajada o resumen corto
    content TEXT NOT NULL, -- El cuerpo de la noticia (HTML/Markdown)
    
    -- Multimedia
    main_image_url VARCHAR(255), -- Foto de portada
    video_embed_url VARCHAR(255), -- Youtube ID o URL para la noticia específica
    
    -- Clasificación
    author_id BIGINT REFERENCES users(id),
    category_id INTEGER REFERENCES categories(id),
    
    -- Estados y Visibilidad
    status VARCHAR(20) DEFAULT 'draft', -- 'draft', 'published', 'archived'
    is_featured BOOLEAN DEFAULT FALSE,  -- ¿Sale en el Sidebar destacado?
    is_breaking BOOLEAN DEFAULT FALSE, -- ¿Es alerta roja?
    
    -- Métricas
    views_count BIGINT DEFAULT 0, -- Para "Lo más leído"
    
    -- Búsqueda (Índice de texto completo nativo de Postgres)
    search_vector tsvector, 
    
    -- Fechas
    published_at TIMESTAMPTZ, -- Importante: TIMESTAMPTZ maneja zona horaria de Colombia
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- 6. Tabla pivote para relación Noticias <-> Etiquetas (Muchos a Muchos)
CREATE TABLE article_tags (
    article_id BIGINT REFERENCES articles(id) ON DELETE CASCADE,
    tag_id INTEGER REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (article_id, tag_id)
);

-- 7. Configuración Global del Sitio (Singleton)
-- Aquí guardas cosas que cambian poco pero no son noticias
CREATE TABLE site_config (
    id INTEGER PRIMARY KEY DEFAULT 1,
    live_stream_url VARCHAR(255), -- El link permanente del noticiero en vivo
    is_live_active BOOLEAN DEFAULT TRUE, -- Interruptor para mostrar/ocultar el player gigante
    breaking_news_banner TEXT, -- Texto para cintilla roja
    CHECK (id = 1) -- Asegura que solo haya una fila de configuración
);

-- ÍNDICES (Para que vuele en rendimiento) ----------------
-- Índice para buscar rápido por URL
CREATE INDEX idx_articles_slug ON articles(slug);
-- Índice para filtrar rápido por categoría y fecha (la query más común en el Home)
CREATE INDEX idx_articles_cat_date ON articles(category_id, published_at DESC);
-- Índice para "Lo más leído"
CREATE INDEX idx_articles_views ON articles(views_count DESC);
-- Actualizar el vector de búsqueda automáticamente
CREATE TRIGGER articles_search_update
    BEFORE INSERT OR UPDATE ON articles
    FOR EACH ROW EXECUTE FUNCTION
    tsvector_update_trigger(search_vector, 'pg_catalog.spanish', title, content);