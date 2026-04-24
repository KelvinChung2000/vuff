# vuff

`vuff` is an unreleased SystemVerilog formatter and future linter inspired by
Ruff, Black, and the low-configuration style of tools that can be dropped into
a project without a long formatting debate.

The project started from formatter behavior that is painful in existing tools.
For example, Verible can join a compiler directive and an attribute onto one
line:

```systemverilog
`default_nettype none
(* ... *)
```

into:

```systemverilog
`default_nettype none (* ... *)
```

`vuff` aims to format SystemVerilog conservatively, preserve syntax, and keep
directive and attribute placement sane.

## Status

This repository is pre-release. The formatter exists, but linting is not
implemented yet. The linter is planned to be based on, or act as a mostly
transparent compatibility layer around, `svlint`.

The public tool and Rust workspace now use the `vuff` name throughout.

## Build

Install a Rust toolchain, then build from the repository root:

```sh
cargo build --release --bin vuff
```

During development, run the CLI through Cargo:

```sh
cargo run -p vuff --bin vuff -- --help
```

## Usage

Format files in place:

```sh
vuff format path/to/file.sv
vuff format rtl/
```

Check whether files are already formatted:

```sh
vuff format --check rtl/
```

Print a unified diff without writing files:

```sh
vuff format --diff rtl/
```

Format stdin:

```sh
vuff format --stdin-filename rtl/example.sv < rtl/example.sv
```

Show resolved configuration:

```sh
vuff config show
```

Directory inputs are walked recursively. Files with these extensions are
formatted: `.sv`, `.svh`, `.v`, and `.vh`.

## Configuration

`vuff` reads a `vuff.toml` file so formatting and future linting can share one
project config. Resolution order is:

1. `--config path/to/vuff.toml`
2. `VUFF_CONFIG`
3. Walk up from the input path looking for `vuff.toml`
4. Built-in defaults

Example:

```toml
[format]
line_width = 100
indent_width = 2
indent_style = "spaces"
begin_style = "k_and_r"
port_list_style = "one_per_line"
trailing_comma = "multiline"
wrap_default_nettype = false
```

The formatter intentionally has few knobs. New options should only be added
when they cover real SystemVerilog style constraints that cannot be handled by
one stable default.

## Development

Useful checks:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

Formatter coverage is tracked with golden tests under `tests/golden/` and the
feature tracker in `docs/spec-tracker.md`.

## License

Licensed under the Apache License, Version 2.0. See `LICENSE`.
