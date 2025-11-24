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
RUN git clone --depth 1 https://github.com/monandkey/cdde.git
WORKDIR /usr/src/cdde

# Build all binaries
RUN cargo build --release
