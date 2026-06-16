# Build stage - Base with cargo-chef
FROM rust:slim-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

# Install build dependencies
RUN apt-get update --allow-releaseinfo-change && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

# Planner stage - Compute recipe
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage - Cache dependencies and build
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the cached layer
RUN cargo chef cook --release --recipe-path recipe.json

# Build the application
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --jobs 1

# Runtime stage
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/target/release/mailora-hub-imap /usr/local/bin/mailora-hub-imap

# Copy static assets and migrations
COPY static /app/static
COPY migrations /app/migrations

# Environment variables
ENV PORT=3030
ENV DATABASE_URL=sqlite:///data/mailora_imap.db
ENV RUST_LOG=info,mailora_hub_imap=debug

# Create data directory
RUN mkdir -p /data
EXPOSE 3030

CMD ["mailora-hub-imap"]
