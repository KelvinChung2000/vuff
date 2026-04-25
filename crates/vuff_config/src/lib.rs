//! `vuff.toml` loader with shared `[option]` settings and a `[format]` section.
//!
//! Milestone 1 stub: type surface only. Discovery + file loading land in
//! milestone 3.

use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IndentStyle {
    Spaces,
    Tabs,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BeginStyle {
    #[serde(alias = "k_and_r", alias = "knr")]
    KAndR,
    Allman,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PortListStyle {
    OnePerLine,
    Compact,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrailingComma {
    Never,
    Multiline,
}

pub const CONFIG_FILE_NAME: &str = "vuff.toml";

/// Raw on-disk shape of `vuff.toml`. Unknown sections (`[textrules]`,
/// `[syntaxrules]`) are captured into `_other` so they do not break
/// deserialization.
#[derive(Debug, Default, Deserialize)]
pub struct VuffConfigFile {
    #[serde(default)]
    pub option: OptionSection,
    #[serde(default)]
    pub format: FormatSection,
    #[serde(flatten)]
    #[allow(dead_code)]
    _other: toml::Table,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct OptionSection {
    pub line_width: Option<u16>,
    pub indent_width: Option<u8>,
    pub indent_style: Option<IndentStyle>,
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct FormatSection {
    pub begin_style: Option<BeginStyle>,
    pub port_list_style: Option<PortListStyle>,
    pub trailing_comma: Option<TrailingComma>,
    pub wrap_default_nettype: Option<bool>,
    #[serde(flatten)]
    other: toml::Table,
}

/// Resolved options — every field defaulted, ready for the formatter to read.
#[derive(Debug, Clone, Copy)]
pub struct FormatOptions {
    pub line_width: u16,
    pub indent_width: u8,
    pub indent_style: IndentStyle,
    pub begin_style: BeginStyle,
    pub port_list_style: PortListStyle,
    pub trailing_comma: TrailingComma,
    /// When true, every `module … endmodule` gets wrapped with
    /// `` `default_nettype none `` above and `` `default_nettype wire ``
    /// below. Idempotent: if the directives are already present the
    /// wrap is skipped.
    pub wrap_default_nettype: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            line_width: 100,
            indent_width: 2,
            indent_style: IndentStyle::Spaces,
            begin_style: BeginStyle::KAndR,
            port_list_style: PortListStyle::OnePerLine,
            trailing_comma: TrailingComma::Multiline,
            wrap_default_nettype: false,
        }
    }
}

impl FormatOptions {
    #[must_use]
    pub fn resolve(option: &OptionSection, format: &FormatSection) -> Self {
        let d = Self::default();
        Self {
            line_width: option.line_width.unwrap_or(d.line_width),
            indent_width: option.indent_width.unwrap_or(d.indent_width),
            indent_style: option.indent_style.unwrap_or(d.indent_style),
            begin_style: format.begin_style.unwrap_or(d.begin_style),
            port_list_style: format.port_list_style.unwrap_or(d.port_list_style),
            trailing_comma: format.trailing_comma.unwrap_or(d.trailing_comma),
            wrap_default_nettype: format
                .wrap_default_nettype
                .unwrap_or(d.wrap_default_nettype),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("config file not found: {0}")]
    NotFound(std::path::PathBuf),
    #[error("shared option `{0}` must be set under [option], not [format]")]
    SharedOptionInFormat(String),
}

/// Resolved view of a config load — the options plus where they came from.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub options: FormatOptions,
    pub source: ConfigSource,
}

#[derive(Debug, Clone)]
pub enum ConfigSource {
    Defaults,
    File(std::path::PathBuf),
}

/// Walk up from `start` looking for `vuff.toml`. If `start` is a file,
/// we begin the search in its parent directory.
#[must_use]
pub fn find_config_file(start: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut cur = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        let candidate = cur.join(CONFIG_FILE_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !cur.pop() {
            return None;
        }
    }
}

/// Read and parse a `vuff.toml`, applying shared `[option]` settings and
/// formatter-specific `[format]` settings.
pub fn load_file(path: &std::path::Path) -> Result<FormatOptions, ConfigError> {
    let src = std::fs::read_to_string(path)?;
    let cfg: VuffConfigFile = toml::from_str(&src)?;
    validate_config(&cfg)?;
    Ok(FormatOptions::resolve(&cfg.option, &cfg.format))
}

fn validate_config(cfg: &VuffConfigFile) -> Result<(), ConfigError> {
    for key in ["line_width", "indent_width", "indent_style"] {
        if cfg.format.other.contains_key(key) {
            return Err(ConfigError::SharedOptionInFormat(key.to_string()));
        }
    }
    Ok(())
}

/// Full resolution pipeline.
///
/// Order: explicit `--config` > `VUFF_CONFIG` env > walk-up discovery > defaults.
/// `search_start` is where walk-up begins (typically cwd, or the first input file).
pub fn load_config(
    explicit: Option<&std::path::Path>,
    env_override: Option<&std::ffi::OsStr>,
    search_start: &std::path::Path,
) -> Result<ResolvedConfig, ConfigError> {
    if let Some(p) = explicit {
        if !p.is_file() {
            return Err(ConfigError::NotFound(p.to_path_buf()));
        }
        return Ok(ResolvedConfig {
            options: load_file(p)?,
            source: ConfigSource::File(p.to_path_buf()),
        });
    }
    if let Some(env) = env_override {
        let p = std::path::PathBuf::from(env);
        if !p.is_file() {
            return Err(ConfigError::NotFound(p));
        }
        return Ok(ResolvedConfig {
            options: load_file(&p)?,
            source: ConfigSource::File(p),
        });
    }
    if let Some(found) = find_config_file(search_start) {
        return Ok(ResolvedConfig {
            options: load_file(&found)?,
            source: ConfigSource::File(found),
        });
    }
    Ok(ResolvedConfig {
        options: FormatOptions::default(),
        source: ConfigSource::Defaults,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerates_unknown_sections() {
        let src = r#"
            [option]
            exclude = ["a"]
            line_width = 120
            indent_style = "tabs"

            [textrules]
            style_textwidth = true

            [format]
            begin_style = "allman"
        "#;
        let cfg: VuffConfigFile = toml::from_str(src).unwrap();
        validate_config(&cfg).unwrap();
        let opts = FormatOptions::resolve(&cfg.option, &cfg.format);
        assert_eq!(opts.line_width, 120);
        assert_eq!(opts.indent_style, IndentStyle::Tabs);
        assert_eq!(opts.begin_style, BeginStyle::Allman);
    }

    #[test]
    fn defaults_when_empty() {
        let cfg: VuffConfigFile = toml::from_str("").unwrap();
        let opts = FormatOptions::resolve(&cfg.option, &cfg.format);
        assert_eq!(opts.line_width, 100);
    }

    #[test]
    fn shared_options_apply_to_formatting() {
        let src = r#"
            [option]
            line_width = 88
            indent_width = 4
            indent_style = "tabs"
        "#;
        let cfg: VuffConfigFile = toml::from_str(src).unwrap();
        let opts = FormatOptions::resolve(&cfg.option, &cfg.format);
        assert_eq!(opts.line_width, 88);
        assert_eq!(opts.indent_width, 4);
        assert_eq!(opts.indent_style, IndentStyle::Tabs);
    }

    #[test]
    fn shared_options_are_not_allowed_in_format_section() {
        let src = r#"
            [option]
            line_width = 88
            indent_width = 4
            indent_style = "tabs"

            [format]
            line_width = 120
        "#;
        let cfg: VuffConfigFile = toml::from_str(src).unwrap();
        assert!(matches!(
            validate_config(&cfg),
            Err(ConfigError::SharedOptionInFormat(key)) if key == "line_width"
        ));
    }

    #[test]
    fn walk_up_discovery_finds_nearest() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("a").join("b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            root.path().join(CONFIG_FILE_NAME),
            "[option]\nline_width = 77\n",
        )
        .unwrap();
        let found = find_config_file(&nested).unwrap();
        assert_eq!(found, root.path().join(CONFIG_FILE_NAME));
        let resolved = load_config(None, None, &nested).unwrap();
        assert_eq!(resolved.options.line_width, 77);
    }

    #[test]
    fn explicit_path_wins_over_walk_up() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join(CONFIG_FILE_NAME),
            "[option]\nline_width = 50\n",
        )
        .unwrap();
        let override_file = root.path().join("other.toml");
        std::fs::write(&override_file, "[option]\nline_width = 200\n").unwrap();
        let r = load_config(Some(&override_file), None, root.path()).unwrap();
        assert_eq!(r.options.line_width, 200);
    }

    #[test]
    fn missing_explicit_errors() {
        let r = load_config(
            Some(std::path::Path::new("/does/not/exist.toml")),
            None,
            std::path::Path::new("."),
        );
        assert!(matches!(r, Err(ConfigError::NotFound(_))));
    }

    #[test]
    fn defaults_when_no_file() {
        let root = tempfile::tempdir().unwrap();
        let r = load_config(None, None, root.path()).unwrap();
        assert!(matches!(r.source, ConfigSource::Defaults));
        assert_eq!(r.options.line_width, 100);
    }
}
