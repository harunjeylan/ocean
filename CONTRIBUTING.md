# Contributing to Ocean

Thank you for considering contributing to Ocean.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/<your-username>/ocean.git`
3. Install Rust (edition 2024): `rustup update stable`
4. Build: `cargo build`
5. Run tests: `cargo test`

## Development Workflow

- Branch from `main`, keep changes focused
- Run `cargo fmt` and `cargo clippy -- -D warnings` before committing
- Write tests for new functionality
- Run `cargo test` before opening a PR — all tests must pass
- Use conventional commits: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `ci:`, `chore:`

## Pull Request Process

1. Open an issue first for significant changes (feature, refactor)
2. Link the issue in your PR description
3. Keep PRs small and focused — one change per PR
4. Update documentation (README, docs/*) if needed
5. Update CHANGELOG.md under "Unreleased" section
6. PRs require at least one maintainer review before merging
7. Squash-merge to main after approval

## Code Style

- No comments in production code
- Error types as enums with `Display` + `Error` impls
- Tests in `_test.rs` files alongside source, registered in `src/tests.rs`
- Use `crate::` paths in tests, never `use super::*`
- Follow existing patterns in the module you are modifying

## Reporting Issues

- Use the bug report template for bugs
- Use the feature request template for enhancements
- Include debug logs if possible (`--log-format json`)

## Questions?

Open a discussion on GitHub or check existing issues.
