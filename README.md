# Cloud Diameter Distribution Engine (CDDE)

[![Tests](https://github.com/monandkey/cdde/workflows/Tests/badge.svg)](https://github.com/monandkey/cdde/actions/workflows/test.yml)
[![Coverage](https://github.com/monandkey/cdde/workflows/Coverage/badge.svg)](https://github.com/monandkey/cdde/actions/workflows/coverage.yml)
[![Lint](https://github.com/monandkey/cdde/workflows/Lint/badge.svg)](https://github.com/monandkey/cdde/actions/workflows/lint.yml)

A high-performance, cloud-native Diameter routing and manipulation engine built with Rust.

## Features

- **High Performance**: Built with Rust for maximum performance and safety
- **Cloud Native**: Designed for Kubernetes deployment with Multus networking
- **Flexible Routing**: Advanced routing rules based on realm, application ID, and destination host
- **Message Manipulation**: Powerful DSL for AVP manipulation and topology hiding
- **Dictionary Management**: Dynamic Diameter dictionary loading and management
- **RESTful API**: Complete management API for configuration and monitoring

## Architecture

CDDE consists of four main components:

- **DFL (Diameter Front Layer)**: TCP/SCTP connection management and session handling
- **DCR (Diameter Core Router)**: Message routing and manipulation engine
- **DPA (Diameter Peer Agent)**: Peer connection management and state machine
- **CMS (Configuration Management Service)**: REST API and database for configuration

## Quick Start

### Prerequisites

- Rust 1.70 or later
- PostgreSQL 15 or later
- Docker (optional, for containerized deployment)

### Building

```bash
# Clone the repository
git clone https://github.com/monandkey/cdde.git
cd cdde

# Build all components
cargo build --release

# Run tests
cargo test --all
```

### Running

```bash
# Set database URL
export DATABASE_URL=postgres://postgres:postgres@localhost/cdde

# Run CMS
cargo run --bin cdde-cms

# Run DFL
cargo run --bin cdde-dfl

# Run DCR
cargo run --bin cdde-dcr

# Run DPA
cargo run --bin cdde-dpa
```

## Development

### Running Tests

```bash
# Run unit tests
cargo test --all

# Run integration tests (requires PostgreSQL)
TEST_DATABASE_URL=postgres://postgres:postgres@localhost/cdde_test cargo test -- --ignored

# Run with coverage
cargo coverage
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy --all-targets --all-features -- -D warnings
```

## API Documentation

The CMS provides a RESTful API for managing the CDDE system:

- Virtual Routers: `/api/v1/vrs`
- Peers: `/api/v1/peers`
- Dictionaries: `/api/v1/dictionaries`
- Routing Rules: `/api/v1/vrs/:vr_id/routing-rules`
- Manipulation Rules: `/api/v1/vrs/:vr_id/manipulation-rules`

See the [API documentation](docs/api.md) for detailed endpoint information.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](.github/CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the MIT OR Apache-2.0 license.

## Documentation

- [Requirements](docs/design/01_requirement.md)
- [Basic Design](docs/design/02_basic_design.md)
- [Module Plan](docs/design/03_rust_module_plan.md)
- [Testing Strategy](docs/design/13_testing_strategy.md)

## Status

Current Phase: **Phase 4 - Testing and CI/CD**

- ✅ Phase 1: Prototype (SCTP/gRPC communication)
- ✅ Phase 2: Core Logic (Session management, Manipulation engine)
- ✅ Phase 3: Management (CMS API, Dictionary management, Configuration)
- 🔄 Phase 4: Testing and CI/CD (In Progress)
- ⏳ Phase 5: Release (Load testing, Production deployment)
