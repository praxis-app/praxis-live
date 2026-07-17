# syntax=docker/dockerfile:1

# Backend build stage
FROM rust:1.97.0-slim-bookworm AS backend-builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY api ./api
COPY cli ./cli
COPY entity ./entity
COPY migrations ./migrations
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release -p praxis-live \
    && cp /app/target/release/praxis-live /praxis-live

# Frontend build stage
FROM node:24.18.0-bookworm-slim AS frontend-builder

WORKDIR /app

COPY package.json package-lock.json ./
RUN npm ci

COPY tsconfig.json tsconfig.app.json tsconfig.node.json tsconfig.e2e.json ./
COPY vite.config.ts playwright.config.ts components.json ./
COPY e2e ./e2e
COPY view ./view

RUN npm run build

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=backend-builder /praxis-live ./
COPY --from=frontend-builder /app/view/dist ./static
COPY content ./content

ENV FRONTEND_DIST_DIR=/app/static

CMD ["./praxis-live"]
