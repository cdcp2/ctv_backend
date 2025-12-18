# ETAPA 1: BUILDER
FROM rust:1-slim-bookworm AS builder

WORKDIR /app

# Instalar dependencias de compilación
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copiar TODO el código fuente de una vez
COPY . .

# Usar modo offline (usa el cache generado en .sqlx)
ENV SQLX_OFFLINE=true

# Compilar en modo release
RUN cargo build --release

# ETAPA 2: RUNNER
FROM debian:bookworm-slim

WORKDIR /app

# Dependencias para correr (SSL y certificados)
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

# Copiar el binario compilado
COPY --from=builder /app/target/release/ctv_backend ./server

EXPOSE 3000

# Le damos permisos de ejecución por si acaso
RUN chmod +x ./server

CMD ["./server"]
