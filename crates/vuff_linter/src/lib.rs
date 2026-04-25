//! Thin `svlint` compatibility layer for `vuff`.
//!
//! The upstream API is re-exported so callers can keep using `svlint` types
//! directly. The additions here are limited to loading lint configuration from
//! `vuff.toml` and translating the shared `[option]` fields into the names
//! expected by `svlint`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub use svlint::*;

pub const CONFIG_FILE_NAME: &str = vuff_config::CONFIG_FILE_NAME;

#[derive(Debug, thiserror::Error)]
pub enum LinterConfigError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("toml serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("config error: {0}")]
    VuffConfig(#[from] vuff_config::ConfigError),
    #[error("config file not found: {0}")]
    NotFound(PathBuf),
}

#[derive(Debug, Clone)]
pub struct ResolvedLintConfig {
    pub config: svlint::config::Config,
    pub source: vuff_config::ConfigSource,
}

#[must_use]
pub fn find_config_file(start: &Path) -> Option<PathBuf> {
    vuff_config::find_config_file(start)
}

pub fn load_file(path: &Path) -> Result<svlint::config::Config, LinterConfigError> {
    let src = std::fs::read_to_string(path)?;
    config_from_vuff_toml(&src)
}

pub fn load_config(
    explicit: Option<&Path>,
    env_override: Option<&OsStr>,
    search_start: &Path,
) -> Result<ResolvedLintConfig, LinterConfigError> {
    if let Some(p) = explicit {
        if !p.is_file() {
            return Err(LinterConfigError::NotFound(p.to_path_buf()));
        }
        return Ok(ResolvedLintConfig {
            config: load_file(p)?,
            source: vuff_config::ConfigSource::File(p.to_path_buf()),
        });
    }

    if let Some(env) = env_override {
        let p = PathBuf::from(env);
        if !p.is_file() {
            return Err(LinterConfigError::NotFound(p));
        }
        return Ok(ResolvedLintConfig {
            config: load_file(&p)?,
            source: vuff_config::ConfigSource::File(p),
        });
    }

    if let Some(found) = find_config_file(search_start) {
        return Ok(ResolvedLintConfig {
            config: load_file(&found)?,
            source: vuff_config::ConfigSource::File(found),
        });
    }

    Ok(ResolvedLintConfig {
        config: config_from_vuff_toml("")?,
        source: vuff_config::ConfigSource::Defaults,
    })
}

pub fn config_from_vuff_toml(src: &str) -> Result<svlint::config::Config, LinterConfigError> {
    let cfg: vuff_config::VuffConfigFile = toml::from_str(src)?;
    let format_options = vuff_config::FormatOptions::resolve(&cfg.option, &cfg.format);
    let mut value: toml::Value = toml::from_str(src)?;

    let Some(root) = value.as_table_mut() else {
        return Ok(toml::from_str("")?);
    };

    root.remove("format");
    let option = root
        .entry("option".to_owned())
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));

    if let Some(option) = option.as_table_mut() {
        option.remove("line_width");
        option.remove("indent_width");
        option.remove("indent_style");
        option.insert(
            "textwidth".to_owned(),
            toml::Value::Integer(i64::from(format_options.line_width)),
        );
        option.insert(
            "indent".to_owned(),
            toml::Value::Integer(i64::from(format_options.indent_width)),
        );
        if let Some(exclude) = option.get("exclude").cloned() {
            option.insert("exclude_paths".to_owned(), exclude);
        }
    }

    Ok(toml::from_str(&toml::to_string(&value)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_shared_vuff_options_for_svlint() {
        let cfg = config_from_vuff_toml(
            r#"
                [option]
                line_width = 120
                indent_width = 4
                exclude = ["target/.*"]

                [format]
                begin_style = "allman"

                [syntaxrules]
                module_nonansi_forbidden = true
            "#,
        )
        .unwrap();

        assert_eq!(cfg.option.textwidth, 120);
        assert_eq!(cfg.option.indent, 4);
        assert_eq!(cfg.option.exclude_paths.len(), 1);
    }

    #[test]
    fn defaults_match_vuff_shared_format_options() {
        let cfg = config_from_vuff_toml("").unwrap();
        assert_eq!(cfg.option.textwidth, 100);
        assert_eq!(cfg.option.indent, 2);
    }

    #[test]
    fn discovers_vuff_toml() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("rtl");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(
            root.path().join(CONFIG_FILE_NAME),
            "[option]\nline_width = 90\n",
        )
        .unwrap();

        let cfg = load_config(None, None, &nested).unwrap();
        assert_eq!(cfg.config.option.textwidth, 90);
    }
}
