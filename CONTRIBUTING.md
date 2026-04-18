# Contributing to ServerlessWorkflow-rs

Thank you for your interest in contributing! This project follows the [CNCF Community Code of Conduct](https://github.com/cncf/foundation/blob/main/code-of-conduct.md).

## Getting Started

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Ensure all tests pass (`cargo test --workspace`)
5. Run clippy (`cargo clippy --workspace --all-targets`)
6. Check formatting (`cargo fmt --check`)
7. Submit a pull request

## Development Setup

```bash
# Build
cargo build --workspace

# Run all tests
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets

# Format
cargo fmt --all
```

## Pull Request Guidelines

- Keep PRs focused on a single change
- Add tests for new functionality
- Update documentation if applicable
- Ensure CI passes (test, clippy, fmt)

## Reporting Issues

When filing an issue, please include:

- Rust version (`rustc --version`)
- OS and architecture
- Minimal reproduction steps
- Expected vs actual behavior
