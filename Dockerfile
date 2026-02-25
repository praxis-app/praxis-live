# ── Build stage ────────────────────────────────────────────────────────────────
FROM rust:1.93-slim AS builder

WORKDIR /app

COPY src/Cargo.toml src/Cargo.lock ./
COPY src/src ./src

RUN cargo build --release

# ── Runtime stage ──────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/praxis-live .

EXPOSE ${PORT}

CMD ["./praxis-live"]
