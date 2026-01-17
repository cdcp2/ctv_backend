-- Añade servidores principal y backup para RTMP
ALTER TABLE live_stream_config
    ADD COLUMN IF NOT EXISTS server_main_url VARCHAR(255),
    ADD COLUMN IF NOT EXISTS server_backup_url VARCHAR(255);

-- Índice simple para consultas por id (sigue siendo singleton)
CREATE INDEX IF NOT EXISTS idx_live_stream_config_id ON live_stream_config(id);
