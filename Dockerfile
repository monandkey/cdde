# Build stage
FROM rust:1.75-slim-bookworm as builder

WORKDIR /usr/src/cdde

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build all binaries
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /usr/local/bin

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

# Copy binaries from builder
COPY --from=builder /usr/src/cdde/target/release/cdde-cms .
COPY --from=builder /usr/src/cdde/target/release/cdde-dfl .
COPY --from=builder /usr/src/cdde/target/release/cdde-dcr .
COPY --from=builder /usr/src/cdde/target/release/cdde-dpa .

# Default command (can be overridden)
CMD ["./cdde-cms"]
