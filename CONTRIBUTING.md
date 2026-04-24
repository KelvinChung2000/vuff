# Contributing

`vuff` is currently unreleased. Contributions should keep the formatter
conservative, idempotent, and predictable.

## Toolchain

Use the Rust toolchain from `rust-toolchain.toml`. The workspace currently
requires Rust 1.80 or newer and uses stable Rust with `rustfmt` and `clippy`.

## Checks

Before sending changes, run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

For CLI behavior, also check:

```sh
cargo run -p vuff --bin vuff -- --help
cargo run -p vuff --bin vuff -- format --help
cargo run -p vuff --bin vuff -- config show
```

## Formatter Changes

Formatter behavior should be covered with golden tests under `tests/golden/`.
When adding support for a new SystemVerilog construct, update
`docs/spec-tracker.md` with the implementation status, owning module, and
golden coverage.

Prefer small, focused rules that preserve original tokens and normalize only
layout/trivia. Avoid adding configuration unless one stable default cannot
serve common SystemVerilog code.

## Linting

Linting is not implemented yet. The intended direction is a `svlint`-inspired
or compatibility-oriented pass that can coexist with the formatter in a single
tool. Do not document lint rules as available until the CLI exposes them.
