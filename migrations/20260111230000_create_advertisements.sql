-- Tabla de anuncios manuales
CREATE TABLE advertisements (
    id BIGSERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    image_url VARCHAR(255),
    target_url VARCHAR(255),
    position VARCHAR(50) NOT NULL,
    html_snippet TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    weight INTEGER NOT NULL DEFAULT 1,
    starts_at TIMESTAMPTZ,
    ends_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ads_active_position ON advertisements(position, is_active);
CREATE INDEX idx_ads_schedule ON advertisements(is_active, starts_at, ends_at);
