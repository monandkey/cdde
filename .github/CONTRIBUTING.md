# Contributing to CDDE

Thank you for your interest in contributing to the Cloud Diameter Distribution Engine (CDDE)!

## Development Setup

### Prerequisites

- Rust 1.70 or later
- PostgreSQL 15 or later
- Docker (for running tests)

### Local Development

1. Clone the repository:
```bash
git clone https://github.com/monandkey/cdde.git
cd cdde
```

2. Build the project:
```bash
cargo build --all
```

3. Run tests:
```bash
# Run unit tests
cargo test --all

# Run integration tests (requires PostgreSQL)
TEST_DATABASE_URL=postgres://postgres:postgres@localhost/cdde_test cargo test -- --ignored

# Run all tests including ignored
cargo test-all
```

## Code Style

### Formatting

We use `rustfmt` for code formatting. Before submitting a PR, ensure your code is formatted:

```bash
cargo fmt --all
```

### Linting

We use `clippy` for linting. Fix all clippy warnings before submitting:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Or use the alias:
```bash
cargo lint
```

## Testing

### Writing Tests

- Add unit tests in the same file as the code being tested
- Add integration tests in the `tests/` directory
- Mark database-dependent tests with `#[ignore]` and document the required environment variables

### Test Coverage

We aim for >70% test coverage. Generate a coverage report locally:

```bash
cargo coverage
open coverage/index.html
```

## Pull Request Process

1. **Create a feature branch** from `main`:
```bash
git checkout -b feature/your-feature-name
```

2. **Make your changes** following the code style guidelines

3. **Add tests** for new functionality

4. **Run the full test suite**:
```bash
cargo test --all
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

5. **Commit your changes** with clear, descriptive commit messages:
```bash
git commit -m "feat: Add new feature description"
```

We follow [Conventional Commits](https://www.conventionalcommits.org/):
- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `test:` - Test additions or modifications
- `refactor:` - Code refactoring
- `chore:` - Maintenance tasks

6. **Push to your fork** and create a pull request

7. **Ensure CI passes** - All GitHub Actions workflows must pass

## CI/CD

Our CI/CD pipeline runs automatically on every push and pull request:

- **Tests**: All unit and integration tests
- **Lint**: Clippy and rustfmt checks
- **Coverage**: Code coverage reporting

Make sure all checks pass before requesting review.

## Questions?

Feel free to open an issue for any questions or concerns!
