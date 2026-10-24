FROM node:22-bookworm-slim AS web
WORKDIR /build/web
COPY web/package*.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

FROM rust:1-bookworm AS rust
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY server/ server/
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
COPY --from=rust /build/target/release/recetaco /usr/local/bin/recetaco
COPY --from=web /build/web/dist /app/web
USER 65532:65532
WORKDIR /app
ENV RECETACO_BIND=0.0.0.0:8080 RECETACO_DB=/data/recetaco.db RECETACO_WEB_DIR=/app/web
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s CMD curl -fsS http://127.0.0.1:8080/health || exit 1
ENTRYPOINT ["/usr/local/bin/recetaco"]
