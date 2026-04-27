# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`vuff` is an unreleased SystemVerilog formatter (and future linter) inspired by ruff/black. The CLI binary, the formatter, the linter (backed by `svlint`), and the language server (vendored from `svls`) all live in one Rust workspace and share the `vuff.toml` config.

## Common commands

Build / run the CLI during development:

```sh
cargo run -p vuff --bin vuff -- --help
cargo run -p vuff --bin vuff -- format path/to/file.sv             # prints to stdout, file untouched
cargo run -p vuff --bin vuff -- format -i path/to/file.sv          # rewrites in place
cargo run -p vuff --bin vuff -- format -o out.sv path/to/file.sv   # write to a different path (single input only)
cargo run -p vuff --bin vuff -- format --check rtl/
cargo run -p vuff --bin vuff -- format --diff rtl/
cargo run -p vuff --bin vuff -- lint rtl/
cargo run -p vuff --bin vuff -- config show
cargo run -p vuff --bin vuff -- server          # LSP over stdio
cargo build --release --bin vuff                # produces target/release/vuff
```

`vuff format` defaults to writing to stdout. Pass `-i/--inplace` to rewrite each input file, or `-o/--output <path>` to redirect formatted output to a specific path. `-i` and `-o` are mutually exclusive; `-o` requires exactly one input (one file or stdin); `-i` is not valid with stdin.

Pre-PR checks (matches `CONTRIBUTING.md`):

```sh
cargo fmt --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

Targeted tests (these are the ones that move when formatter rules change):

```sh
cargo test -p vuff_sv_formatter                    # all formatter tests
cargo test -p vuff_sv_formatter --test golden -- golden_cases   # golden suite
cargo test -p vuff_sv_formatter --test fixture_snapshots        # insta snapshots
INSTA_UPDATE=always cargo test -p vuff_sv_formatter --test fixture_snapshots   # accept new snapshots
```

Optional smoke run against pinned external SV repos (clones into `target/corpus/`, never fails CI):

```sh
scripts/corpus.sh
```

Toolchain is pinned in `rust-toolchain.toml` (stable, rustc ≥ 1.80, with `rustfmt` + `clippy`).

## Workspace layout

Cargo workspace with one binary and eight library crates under `crates/`:

- `vuff` — the CLI binary (`src/main.rs`). Owns argument parsing, file walking (via `ignore`), diff printing, and dispatch into the format / lint / server / config-show subcommands. Recognized SV extensions: `.sv`, `.svh`, `.v`, `.vh`. Exit codes are `0` ok, `1` would-change (for `--check`/`--diff`), `2` error.
- `vuff_sv_ast` — wraps `sv-parser`. Provides `parse`, the canonical token stream, and `assert_roundtrip` (input bytes must reproduce from the CST before formatting begins).
- `vuff_formatter` — generic IR + pretty-printer (`print`) used by language-specific formatters. Not SystemVerilog-aware.
- `vuff_sv_formatter` — the SystemVerilog formatter. CST-driven token re-emitter: tokens are preserved verbatim and inter-token trivia is normalized; spacing/indent decisions come from precomputed CST masks (see `src/lib.rs` doc comment for the module map). New per-node rules accrete as they migrate off the shared `verbatim` engine.
- `vuff_config` — `vuff.toml` schema, defaults, and `load_config` resolution.
- `vuff_diagnostics` — shared error types.
- `vuff_workspace` — file discovery / config-walk helpers shared by CLI, linter, server.
- `vuff_linter` — svlint-backed linter (currently surfaced through `vuff lint`; full lint roadmap is in `CONTRIBUTING.md`).
- `vuff_server` — `tower-lsp` language server, vendored from `svls`. Retains its upstream MIT license at `crates/vuff_server/SVLS_LICENSE`; the rest of the workspace is Apache-2.0.

Workspace-wide lints in `Cargo.toml`: `unsafe_code = "forbid"` and `clippy::pedantic` at warn. Several pedantic lints are intentionally allowed (cast lints, `module_name_repetitions`, `missing_errors_doc`, etc.) — keep that allowlist tight when adding new code.

## Configuration model

`vuff.toml` resolution order (see `crates/vuff_config` and `crates/vuff/src/main.rs::resolve_config`):

1. `--config <path>`
2. `VUFF_CONFIG` env var
3. Walk up from the input path looking for `vuff.toml`
4. Built-in defaults

`[option]` holds settings that must agree between formatter and linter (`line_width`, `indent_width`, `indent_style`). `[format]` holds formatter-only knobs (`begin_style`, `wrap_default_nettype`). The formatter is intentionally low-config — only add a new option when one stable default cannot serve common SystemVerilog code.

## Wrap policy

The formatter never auto-wraps based on `line_width`. Wrap is **newline-triggered**: any delimited group `(...)`, `{...}`, `[...]` whose source bytes contain a `\n` is reformatted to the canonical hanging shape (opener on its own line, content at `depth + 1`, closer on its own line at outer `depth`); a group without an internal newline stays on one line, even if it overflows `line_width`. Long lines are the user's responsibility — they signal "wrap me" by inserting a newline. This is uniform across instance port lists, module declaration port/param lists, function call argument lists, concatenations, control-statement headers, and any other delimited expression. The mechanism: `list/wrap_mask.rs` builds two masks over the token stream; `verbatim.rs` consumes them via a `wrap_depth` counter that adds an extra indent level for contents inside a wrapped group. Internal split points written by the user are preserved (only indent is normalized). `line_width` remains in the config because the linter still uses it; the formatter ignores it.

## Formatter testing model

Three layers, each with a different purpose:

1. **Golden tests** — `tests/golden/*.sv`, runner at `crates/vuff_sv_formatter/tests/golden.rs`. Each file has optional `// config: key=value, ...` and `// xfail: reason` header lines, the messy input, then a `// expected -----` marker, then the byte-exact expected output. The harness inverts xfail cases so the suite stays green but flags regressions if the gap closes (an "unexpected pass" panics the test). Adding a formatter rule almost always means adding goldens for the smallest valid shape, a messy real-world shape, and one that exercises the wrap policy (an input with a newline inside the relevant `(...)` / `{...}`).
2. **Insta fixture snapshots** — `tests/fixtures/<case>/input.sv` (with optional per-fixture `vuff.toml`). Snapshots live in `crates/vuff_sv_formatter/tests/snapshots/`. Use `INSTA_UPDATE=always` to accept changes after reviewing the diff.
3. **Inline unit tests** — `#[test]` in `crates/vuff_sv_formatter/src/lib.rs` and friends, for invariants (idempotency, trailing-newline, blank-line collapsing, etc.).

Round-trip is enforced before formatting (`assert_roundtrip` in `vuff_sv_ast`) — if the parser cannot reproduce the original source, the formatter refuses to run rather than silently mangling tokens.

## Spec coverage tracker

`docs/spec-tracker.md` is the source of truth for which IEEE 1800-2017 Annex A productions are implemented. Status values are `todo`, `wip`, `done`, `skip-v0.1`. When you touch a row: bump status, name the owning module file under `crates/vuff_sv_formatter/src/`, cite the goldens proving it, and never mark `done` without both default-config and at least one non-default-config golden passing. Spec PDF is at `docs/spec/ieee1800-2017.pdf`; sv-parser's `RefNode` variants are 1:1 with Annex A productions and are the dispatch key.

## Distribution: pre-commit + release binaries

`.pre-commit-hooks.yaml` exposes `vuff-format`, `vuff-format-check`, and `vuff-lint` via `language: script`. The script is `scripts/vuff-pre-commit` (a bash bootstrap) — it reads the version from `[workspace.package]` in `Cargo.toml`, detects host triple, downloads the matching archive from `github.com/KelvinChung2000/vuff/releases/download/<tag>/`, sha256-verifies it, caches under `$XDG_CACHE_HOME/vuff/<tag>/<target>/vuff`, and execs. Env overrides exist for development: `VUFF_BIN`, `VUFF_VERSION`, `VUFF_REPO`, `VUFF_CACHE_DIR`, `VUFF_SKIP_VERIFY`. When changing the bootstrap, keep it POSIX bash and shebang-driven so pre-commit on Windows (Git Bash) keeps working.

Releases are fully automated. Workflows in `.github/workflows/`:

- `ci.yml` runs on every PR and push to `main`: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --locked` across Linux/macOS/Windows. PR runs cancel in-flight on new pushes; main runs do not.
- `release-please.yml` runs on every push to `main`. It uses `googleapis/release-please-action@v4` (config: `release-please-config.json`, manifest: `.release-please-manifest.json`) to maintain a single Release PR that bumps `[workspace.package].version`, syncs `Cargo.lock`, and writes `CHANGELOG.md` based on Conventional Commits since the last release. Merging that PR creates the `vX.Y.Z` tag + GitHub Release. The same workflow then calls `release.yml` to build and upload binaries onto the just-created Release.
- `release.yml` is a reusable workflow (`workflow_call`) that builds the cross-platform matrix (Linux x86_64/aarch64 via cross, macOS x86_64/aarch64, Windows x86_64), packages archives + sha256 sidecars, and uploads them to the supplied tag. It also keeps `workflow_dispatch` for manual dry-run builds (skips publish). The `push: tags` trigger has been removed — release-please is the only path that ships binaries.

Conventional Commit PR titles are not gated by a workflow; the maintainer enforces this manually at merge time. release-please only counts commits whose subjects match `feat`, `fix`, `perf`, etc.

Version bump rules (pre-1.0, via `bump-minor-pre-major: true` in `release-please-config.json`): `feat` → minor, `fix`/`perf` → patch, breaking → minor (becomes major once 1.0.0 lands). `chore`/`docs`/`ci`/etc. do not trigger a release.

Why the indirection between release-please and release.yml: tags created by `GITHUB_TOKEN` do not trigger downstream workflows, so a `push: tags` trigger on the build workflow would not fire. Calling `release.yml` as a reusable workflow from inside `release-please.yml` sidesteps that — the build runs in the same workflow run as the release-please job that produced the tag.

Do not hand-edit `[workspace.package].version`, do not run `git tag`, and do not push tags manually. The pre-commit bootstrap reads the version field, and release-please owns it.

## Formatter design rules (from `CONTRIBUTING.md`)

- The formatter must be conservative and idempotent: preserve original tokens, normalize only layout and trivia.
- Prefer small, focused rules over global rewrites.
- Do not document lint behavior as available until the CLI exposes it.
