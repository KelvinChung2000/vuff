//! Snapshot tests driven by `tests/fixtures/<name>/input.sv` files at the
//! repo root. If a fixture directory contains a `vuff.toml`, its
//! `[option]` and `[format]` sections are used; otherwise defaults apply.

use std::path::{Path, PathBuf};

use vuff_config::{FormatOptions, VuffConfigFile, CONFIG_FILE_NAME};
use vuff_sv_formatter::format_source;

fn fixtures_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/vuff_sv_formatter — go up two to reach repo root.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
}

fn load_fixture_options(dir: &Path) -> FormatOptions {
    let cfg_path = dir.join(CONFIG_FILE_NAME);
    if !cfg_path.is_file() {
        return FormatOptions::default();
    }
    let src = std::fs::read_to_string(&cfg_path).expect("read vuff.toml");
    let cfg: VuffConfigFile = toml::from_str(&src).expect("parse vuff.toml");
    FormatOptions::resolve(&cfg.option, &cfg.format)
}

#[test]
fn fixtures() {
    let root = fixtures_dir();
    insta::glob!(&root, "*/input.sv", |input_path| {
        let dir = input_path.parent().unwrap();
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let src = std::fs::read_to_string(input_path).expect("read fixture input");
        let opts = load_fixture_options(dir);
        let formatted = format_source(&src, &opts).expect("format");

        // Re-format the output and assert idempotence.
        let twice = format_source(&formatted, &opts).expect("format twice");
        assert_eq!(formatted, twice, "fixture {name} is not idempotent");

        let mut settings = insta::Settings::clone_current();
        settings.set_snapshot_suffix(&name);
        settings.bind(|| insta::assert_snapshot!("formatted", formatted));
    });
}
