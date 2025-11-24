# Build stage
FROM rust:1.88.0-slim-bookworm as builder

WORKDIR /usr/src

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config \
    libssl-dev \
    protobuf-compiler \
    git \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
RUN git clone --depth 1 https://github.com/monandkey/cdde.git
WORKDIR /usr/src/cdde

# Build all binaries
RUN cargo build --release

# Runtime stage
FROM registry.access.redhat.com/ubi10-micro:10.1

WORKDIR /usr/local/bin

# Copy binaries from builder
COPY --from=builder /usr/src/cdde/target/release/cdde-cms .
COPY --from=builder /usr/src/cdde/target/release/cdde-dfl .
COPY --from=builder /usr/src/cdde/target/release/cdde-dcr .
COPY --from=builder /usr/src/cdde/target/release/cdde-dpa .

# Default command (can be overridden)
CMD ["./cdde-cms"]
