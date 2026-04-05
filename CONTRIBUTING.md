# Contributing

Thanks for your interest in contributing to `tauri-plugin-hotswap`!

## Development setup

### Prerequisites

- Rust 1.77.2+
- Node.js 18+
- A Tauri v2 development environment ([setup guide](https://v2.tauri.app/start/prerequisites/))

### Building

```bash
# Check the Rust crate compiles
cargo check

# Check with zip feature
cargo check --features zip

# Build the guest-js package
pnpm -C guest-js install --frozen-lockfile && pnpm -C guest-js build
```

### Testing

```bash
# Run all Rust tests
cargo test

# Run with zip feature
cargo test --features zip

# Lint
cargo fmt --check
cargo clippy --all-features -- -D warnings
```

### Running the example

```bash
cd examples/basic
pnpm install
cd src-tauri
cargo run
```

Note: The example uses a placeholder pubkey — it won't actually connect to an update server. It demonstrates the plugin API and UI integration.

## Pull requests

1. Fork the repo and create a branch from `main`
2. Make your changes
3. Add tests for new functionality
4. Run `cargo fmt`, `cargo clippy --all-features -- -D warnings`, and `cargo test`
5. Update relevant documentation if you changed the public API
6. Update `CHANGELOG.md` — add your change under `[Unreleased]`
7. Open a PR with a clear description of what changed and why

## Code style

- Follow `rustfmt` defaults (see `rustfmt.toml`)
- Use the existing error types in `error.rs` — don't use raw `String` errors
- Keep `pub` visibility minimal — prefer `pub(crate)` for internal functions
- All public items should have doc comments

## License

By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.
