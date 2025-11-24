# Build stage
FROM ghcr.io/monandkey/cdde-base:0.1 AS builder

# Runtime stage
FROM registry.access.redhat.com/ubi10-micro:10.1

WORKDIR /usr/local/bin

# Copy binaries from builder
COPY --from=builder /usr/src/cdde/target/release/cdde-dfl .

# Default command (can be overridden)
CMD ["./cdde-dfl"]
