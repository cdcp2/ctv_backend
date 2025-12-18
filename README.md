# CTV Backend (Rust/Axum)

Backend para el portal de noticias CTV: autenticación JWT, CRUD de artículos con video/imagen, tags, configuración de sitio y uploads estáticos.

## Arranque rápido
```bash
docker compose up -d
# primer admin (si DB vacía)
curl -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","email":"admin@ctv.com","password":"Admin123!"}'
```

## Endpoints principales
- Auth: `POST /api/auth/register` (solo primer usuario o admin), `POST /api/auth/login`.
- Artículos públicos:
  - `GET /api/articles` (filtros: `category_id`, `search`, `is_featured`, `is_breaking`, `has_video`, `tag_id`)
  - `GET /api/articles/:slug`
  - `POST /api/articles/:slug/view` (incrementa vistas)
  - `GET /api/articles/most-read` | `/featured` | `/breaking` | `/videos`
  - `GET /api/articles/:slug/related`
  - `GET /api/articles/:slug/tags`
- Artículos protegidos:
  - `POST /api/articles` (editor/admin; asigna `author_id` del token)
  - `PUT /api/admin/articles/:id` (autor o admin)
  - `DELETE /api/admin/articles/:id` (admin)
- Tags: `GET /api/tags`, `POST /api/admin/tags`, `POST /api/admin/articles/:id/tags` (reemplaza set).
- Configuración del sitio: `GET /api/site-config`, `PUT /api/admin/site-config`.
- Uploads: `POST /api/upload` (editor/admin, valida MIME imagen y max 5MB), estático en `/uploads/...`.
- Health: `GET /healthz`.

## Notas de seguridad/autorización
- Primer usuario creado vía `/auth/register` se vuelve `admin`; siguientes requieren token admin.
- Edición de artículos: autor o admin; borrado solo admin.
- Upload restringido a imágenes (jpg/png/webp/gif) y 5MB.

## Testing rápido
`ADMIN_EMAIL=admin@ctv.com ADMIN_PASSWORD=Admin123! ./test_api.sh`  
Verifica login admin, creación de editores, upload, creación de noticia con video, incremento de vistas y permisos de edición.

## Observabilidad
- Logs con `tracing` (creación/edición/borrado de artículos, asignación de tags).
- Healthcheck en `/healthz`.
