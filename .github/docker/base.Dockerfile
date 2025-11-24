# Build stage
FROM rust:1.88.0-slim-bookworm
WORKDIR /usr/src/

# Install build dependencies
RUN apt-get update && \
    apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    git && \
    rm -rf /var/lib/apt/lists/*

# Copy manifests
# COPY Cargo.toml Cargo.lock ./
# COPY crates ./crates
COPY . ./cdde

WORKDIR /usr/src/cdde

# Build all binaries
RUN cargo build --release
