# ── Frontend build stage ───────────────────────────────────────────────────────
FROM node:24.14.1-bookworm-slim AS frontend-builder

WORKDIR /app

COPY package.json package-lock.json ./
COPY tsconfig.json tsconfig.app.json tsconfig.node.json ./
COPY vite.config.ts components.json ./
COPY view ./view

RUN npm ci
RUN npm run build

# ── Backend build stage ────────────────────────────────────────────────────────
FROM rust:1.93-slim-bookworm AS backend-builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY api ./api
COPY cli ./cli
COPY entity ./entity
COPY migrations ./migrations
COPY src ./src

RUN cargo build --release -p praxis-live

# ── Runtime stage ──────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=backend-builder /app/target/release/praxis-live ./
COPY --from=frontend-builder /app/view/dist ./static
COPY content ./content

ENV FRONTEND_DIST_DIR=/app/static

CMD ["./praxis-live"]
