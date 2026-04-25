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

This repository is pre-release. The formatter exists. Linting is wired through
the `svlint` crate behind the `vuff lint` frontend, and the language server is
vendored from `svls` so `vuff` can remain the public interface while still
reading `vuff.toml`.

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

Lint files:

```sh
vuff lint rtl/
```

Show resolved configuration:

```sh
vuff config show
```

Run the language server over stdio:

```sh
vuff server
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
[option]
line_width = 100
indent_width = 2
indent_style = "spaces"

[format]
begin_style = "k_and_r"
port_list_style = "one_per_line"
trailing_comma = "multiline"
wrap_default_nettype = false
```

Settings that must agree between linting and formatting live under
`[option]`. Formatter-only behavior lives under `[format]`.

The formatter intentionally has few knobs. New options should only be added
when they cover real SystemVerilog style constraints that cannot be handled by
one stable default.

## Pre-commit

`vuff` ships hook definitions for [pre-commit](https://pre-commit.com) so
downstream SystemVerilog projects can wire formatting and linting into their
commit flow without installing a Rust toolchain.

Add the repo to the consuming project's `.pre-commit-config.yaml`, pinning
`rev` to a tagged release:

```yaml
repos:
  - repo: https://github.com/KelvinChung2000/vuff
    rev: v0.1.0  # any tag from the GitHub Releases page
    hooks:
      - id: vuff-format
```

`pre-commit install` and the first hook run will transparently download the
matching prebuilt `vuff` binary from the GitHub release, verify its sha256,
cache it under `$XDG_CACHE_HOME/vuff/<tag>/<target>/`, and reuse it on every
later run. No `cargo`, no manual install.

Three hook ids are exposed:

| id | Behavior | Typical use |
|---|---|---|
| `vuff-format` | Runs `vuff format` and rewrites files in place. pre-commit then fails the commit if anything changed, prompting a re-stage. | Local commits — autofix style. |
| `vuff-format-check` | Runs `vuff format --check`. Reports would-reformat files and fails without writing. | CI / `pre-commit run --all-files` in pipelines that should never mutate the tree. |
| `vuff-lint` | Runs `vuff lint`. | Optional; pair with a `vuff.toml` that enables the lint rules you want. |

All three hooks match `*.sv`, `*.svh`, `*.v`, `*.vh`. A project-local
`vuff.toml` is picked up via the normal upward-walk (see [Configuration](#configuration)),
so no extra hook arguments are needed to share config between editor, CLI, and
pre-commit runs.

Supported prebuilt platforms (matching the [release matrix](#releases)):

- `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`, `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

Run hooks manually without committing:

```sh
pre-commit run vuff-format --all-files
pre-commit run vuff-format-check --all-files
```

### Bootstrap escape hatches

The download step is implemented by `scripts/vuff-pre-commit`. It honors a
few environment variables, mainly for development and locked-down networks:

| Variable | Purpose |
|---|---|
| `VUFF_BIN` | Absolute path to a pre-existing `vuff` binary. The bootstrap skips all download logic and execs this directly. Use during development against a `cargo build` artifact. |
| `VUFF_VERSION` | Override the version tag used to construct the download URL. Defaults to `workspace.package.version` from this repo's `Cargo.toml` at the cloned `rev`. |
| `VUFF_REPO` | `owner/name` of the GitHub repo to download from (default: `KelvinChung2000/vuff`). |
| `VUFF_CACHE_DIR` | Cache root (default: `$XDG_CACHE_HOME/vuff` or `~/.cache/vuff`). |
| `VUFF_SKIP_VERIFY` | Set to `1` to skip sha256 verification of the downloaded archive. |

If a user pins `rev` to a non-tagged commit, the bootstrap will try and fail
to download a release that does not exist; the error message points at
pinning to a tagged release or setting `VUFF_BIN`.

## Releases

Releases are fully automated via [release-please](https://github.com/googleapis/release-please)
and [Conventional Commits](https://www.conventionalcommits.org). There is no
manual version bumping or tagging.

How it works:

1. Pull request titles must follow Conventional Commits (`feat:`, `fix:`,
   `chore:`, `docs:`, `refactor:`, `test:`, `ci:`, `build:`, `perf:`,
   `style:`, `revert:`). The `lint-pr` workflow gates this on every PR.
2. On every push to `main`, `.github/workflows/release-please.yml` runs.
   release-please scans commits since the last release and either opens or
   updates a single **Release PR** that bumps `[workspace.package].version`
   in `Cargo.toml`, syncs `Cargo.lock`, and writes `CHANGELOG.md`.
3. Merging that Release PR creates the `vX.Y.Z` tag and the GitHub Release.
4. The same workflow then calls `.github/workflows/release.yml` (a reusable
   workflow) to build the cross-platform matrix, package each target as
   `vuff-vX.Y.Z-<target>.{tar.gz,zip}` with a sibling `.sha256`, and upload
   the artifacts onto the Release that release-please just created.

Pre-commit consumers can then pin `rev: vX.Y.Z`.

Version bump rules (pre-1.0, configured via `bump-minor-pre-major`):

- `feat:` → minor bump (`0.1.0` → `0.2.0`)
- `fix:` / `perf:` → patch bump (`0.1.0` → `0.1.1`)
- `feat!:` or any commit with `BREAKING CHANGE:` footer → minor bump while
  pre-1.0; switches to major once `1.0.0` is reached.
- `chore:`, `docs:`, `refactor:`, `test:`, `ci:`, `build:`, `style:` do not
  trigger a release on their own.

Manual dry runs of the build matrix are supported via `workflow_dispatch` on
`release.yml`; they upload artifacts as a workflow run but do not publish a
Release.

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

Vendored upstream server code retains its original MIT license:

- `crates/vuff_server/SVLS_LICENSE`
