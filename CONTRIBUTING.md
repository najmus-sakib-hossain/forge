# Contributing to DX Forge

Thank you for your interest in contributing to DX Forge! This document provides guidelines and instructions for contributing.

## Code of Conduct

Be respectful, inclusive, and constructive in all interactions.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/forge.git
   cd forge
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/najmus-sakib-hossain/forge.git
   ```

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Cargo
- Git

### Building

```bash
cargo build
```

### Running Tests

```bash
# All tests
cargo test

# Integration tests only
cargo test --test integration_test

# With logging
RUST_LOG=debug cargo test

# Specific test
cargo test test_orchestrator_priority_ordering
```

### Running Examples

```bash
cargo run --example simple
cargo run --example full_workflow
```

## Making Changes

### Branch Naming

- `feature/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation updates
- `refactor/description` - Code refactoring

### Commit Messages

Follow conventional commits format:

```
type(scope): subject

body (optional)

footer (optional)
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance

Example:
```
feat(orchestrator): add parallel execution support

Implements wave-based parallel execution for tools that have no
dependencies on each other. Adds OrchestratorConfig.parallel flag.

Closes #123
```

## Pull Request Process

1. **Update your fork**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Make your changes** in a feature branch

3. **Add tests** for new functionality

4. **Run tests and linting**:
   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt --check
   ```

5. **Update documentation**:
   - Add rustdoc comments for public APIs
   - Update README.md if needed
   - Update CHANGELOG.md

6. **Submit pull request**:
   - Provide clear description
   - Reference related issues
   - Ensure CI passes

## Code Style

### Rust Guidelines

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Write idiomatic Rust code

### Documentation

- Document all public APIs with rustdoc
- Include examples in documentation
- Keep docs up to date with code changes

### Testing

- Write unit tests for all new functionality
- Add integration tests for complex features
- Maintain test coverage above 70%

## Project Structure

```
forge/
├── src/
│   ├── lib.rs              # Public API exports
│   ├── core/               # Core Forge functionality
│   ├── orchestrator.rs     # Tool orchestration
│   ├── watcher.rs          # File change detection
│   ├── version/            # Version control
│   ├── storage/            # Storage layer
│   └── ...
├── examples/               # Example implementations
├── tests/                  # Integration tests
└── docs/                   # Documentation
```

## Adding New Features

### New DX Tool Integration

1. Implement the `DxTool` trait
2. Add example in `examples/`
3. Document usage in README
4. Add integration test

### Core Features

1. Discuss design in GitHub issue first
2. Maintain backward compatibility
3. Update API documentation
4. Add comprehensive tests

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create git tag: `git tag v0.x.x`
4. Push tag: `git push --tags`
5. CI will publish to crates.io

## Questions?

- Open an issue for bugs or feature requests
- Use discussions for questions
- Check existing issues and docs first

## License

By contributing, you agree that your contributions will be licensed under either:

- Apache License, Version 2.0
- MIT License

at your option.

Thank you for contributing to DX Forge! 🎉
