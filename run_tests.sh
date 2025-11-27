#!/bin/bash
set -e

# Build everything first
echo "Building..."
cargo build --bin cdde-cms
cargo test --no-run

# Start CMS in background using binary
echo "Starting CMS..."
RUST_LOG=debug DATABASE_URL=postgres://postgres:postgres@db-dev:5432/cdde ./target/debug/cdde-cms > cms.log 2>&1 &
CMS_PID=$!

# Wait for CMS to start
echo "Waiting for CMS to start..."
sleep 5

# Run tests
echo "Running tests..."
cargo test --test vr_test --test peer_test --test routing_rule_test --test manipulation_rule_test

# Kill CMS
echo "Stopping CMS..."
kill $CMS_PID
