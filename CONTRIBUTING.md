# Contributing

Thanks for considering a contribution.

## Local workflow

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --all-targets
```

## Guidelines

- keep APIs explicit and Rust-native
- prefer deterministic tests built around `MockChatModel`
- avoid adding provider-specific behavior to the core runtime
- document public APIs when they introduce new concepts

## Pull requests

- keep each PR focused
- explain runtime or API tradeoffs in the PR description
- include tests for new behavior
