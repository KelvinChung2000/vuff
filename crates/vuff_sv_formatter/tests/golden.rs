//! Golden-file format tests.
//!
//! Each `.sv` file under `tests/golden/` has three optional sections:
//!
//! 1. Config directives at the top (zero or more lines like
//!    `// config: indent_width=4`). Recognized keys are the six format
//!    options exposed by `vuff_config::FormatOptions`.
//! 2. The messy input SV.
//! 3. A marker line `// expected -----` (at least two dashes) followed by
//!    the expected formatted output — byte-for-byte.
//!
//! The harness formats the input under the parsed config and asserts the
//! output exactly equals the expected block. Failures mean either the test
//! file needs updating or the formatter has a gap.

use std::path::{Path, PathBuf};

use vuff_config::{BeginStyle, FormatOptions, IndentStyle};
use vuff_sv_formatter::format_source;

fn golden_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("golden")
}

struct Golden {
    options: FormatOptions,
    input: String,
    expected: String,
    /// If set, this case is known to fail — the harness inverts the result
    /// so the test stays green but flags regressions if the gap ever closes.
    xfail_reason: Option<String>,
}

fn parse_golden(src: &str) -> Result<Golden, String> {
    let mut options = FormatOptions::default();
    let mut input = String::new();
    let mut expected = String::new();
    let mut xfail_reason: Option<String> = None;
    let mut in_body = false;
    let mut in_expected = false;
    let mut saw_marker = false;

    for line in src.split_inclusive('\n') {
        let trimmed = line.trim();

        if !in_expected && is_expected_marker(trimmed) {
            in_expected = true;
            saw_marker = true;
            in_body = true;
            continue;
        }

        if in_expected {
            expected.push_str(line);
            continue;
        }

        if !in_body {
            if trimmed.is_empty() {
                continue;
            }
            if let Some(rest) = trimmed
                .strip_prefix("// config:")
                .or_else(|| trimmed.strip_prefix("//config:"))
            {
                apply_config_line(&mut options, rest.trim())?;
                continue;
            }
            if let Some(rest) = trimmed
                .strip_prefix("// xfail:")
                .or_else(|| trimmed.strip_prefix("//xfail:"))
            {
                xfail_reason = Some(rest.trim().to_owned());
                continue;
            }
            in_body = true;
        }

        input.push_str(line);
    }

    if !saw_marker {
        return Err("missing `// expected -----` marker".into());
    }

    Ok(Golden {
        options,
        input,
        expected,
        xfail_reason,
    })
}

fn is_expected_marker(line: &str) -> bool {
    let line = line.trim();
    let rest = match line.strip_prefix("//") {
        Some(r) => r.trim(),
        None => return false,
    };
    let rest = match rest.strip_prefix("expected") {
        Some(r) => r.trim(),
        None => return false,
    };
    !rest.is_empty() && rest.chars().all(|c| c == '-')
}

fn apply_config_line(opts: &mut FormatOptions, spec: &str) -> Result<(), String> {
    // Allow comma-separated pairs on one line: `// config: indent_width=4, line_width=80`
    for pair in spec.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| format!("bad config pair: {pair:?}"))?;
        let key = key.trim();
        let value = value.trim();
        apply_key(opts, key, value)?;
    }
    Ok(())
}

fn apply_key(opts: &mut FormatOptions, key: &str, value: &str) -> Result<(), String> {
    match key {
        "line_width" => {
            opts.line_width = value.parse().map_err(|e| format!("line_width: {e}"))?;
        }
        "indent_width" => {
            opts.indent_width = value.parse().map_err(|e| format!("indent_width: {e}"))?;
        }
        "indent_style" => {
            opts.indent_style = match value {
                "spaces" => IndentStyle::Spaces,
                "tabs" => IndentStyle::Tabs,
                other => return Err(format!("indent_style: {other}")),
            };
        }
        "begin_style" => {
            opts.begin_style = match value {
                "k_and_r" | "knr" => BeginStyle::KAndR,
                "allman" => BeginStyle::Allman,
                other => return Err(format!("begin_style: {other}")),
            };
        }
        "wrap_default_nettype" => {
            opts.wrap_default_nettype = match value {
                "true" => true,
                "false" => false,
                other => return Err(format!("wrap_default_nettype: {other}")),
            };
        }
        other => return Err(format!("unknown config key: {other}")),
    }
    Ok(())
}

enum Outcome {
    Pass,
    ExpectedFail(String), // xfail reason
    UnexpectedPass,       // xfail marked but it now passes — regression signal
    Fail(String),         // msg (expected vs got)
}

fn run_one(path: &Path) -> Result<Outcome, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let Golden {
        options,
        input,
        expected,
        xfail_reason,
    } = parse_golden(&src).map_err(|e| format!("parse {}: {e}", path.display()))?;
    let got =
        format_source(&input, &options).map_err(|e| format!("format {}: {e}", path.display()))?;
    let matched = got == expected;
    Ok(match (matched, xfail_reason) {
        (true, None) => Outcome::Pass,
        (false, Some(reason)) => Outcome::ExpectedFail(reason),
        (true, Some(_)) => Outcome::UnexpectedPass,
        (false, None) => Outcome::Fail(format!(
            "MISMATCH in {}\n--- expected\n{expected}--- got\n{got}",
            path.display()
        )),
    })
}

#[test]
fn golden_cases() {
    let dir = golden_dir();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("open golden dir")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("sv"))
        .collect();
    entries.sort();

    let mut passed = 0usize;
    let mut xfailed: Vec<(PathBuf, String)> = Vec::new();
    let mut unexpected_passes: Vec<PathBuf> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for path in &entries {
        match run_one(path) {
            Ok(Outcome::Pass) => passed += 1,
            Ok(Outcome::ExpectedFail(reason)) => xfailed.push((path.clone(), reason)),
            Ok(Outcome::UnexpectedPass) => unexpected_passes.push(path.clone()),
            Ok(Outcome::Fail(msg)) => failed.push(msg),
            Err(e) => failed.push(e),
        }
    }

    let total = entries.len();
    println!("\n==== golden summary ====");
    println!(
        "pass={} xfail={} unexpected_pass={} fail={} total={}",
        passed,
        xfailed.len(),
        unexpected_passes.len(),
        failed.len(),
        total
    );
    if !xfailed.is_empty() {
        println!("\nknown gaps (xfail):");
        for (p, r) in &xfailed {
            println!("  {}: {r}", p.file_name().unwrap().to_string_lossy());
        }
    }

    if !unexpected_passes.is_empty() {
        eprintln!(
            "\nThe following files are marked xfail but now PASS — remove their xfail directive:"
        );
        for p in &unexpected_passes {
            eprintln!("  {}", p.display());
        }
        panic!(
            "{} case(s) passed unexpectedly — formatter improved, update tests",
            unexpected_passes.len()
        );
    }

    if !failed.is_empty() {
        for msg in &failed {
            eprintln!("{msg}\n");
        }
        panic!("{} golden case(s) failed (see output above)", failed.len());
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn splits_on_marker() {
        let src = "// config: indent_width=4\nmodule m; endmodule\n// expected -----\nmodule m;\nendmodule\n";
        let g = parse_golden(src).unwrap();
        assert_eq!(g.options.indent_width, 4);
        assert_eq!(g.input, "module m; endmodule\n");
        assert_eq!(g.expected, "module m;\nendmodule\n");
    }

    #[test]
    fn multiple_config_lines() {
        let src = "// config: indent_width=4\n// config: indent_style=tabs\nmodule m; endmodule\n// expected -----\nmodule m;\nendmodule\n";
        let g = parse_golden(src).unwrap();
        assert_eq!(g.options.indent_width, 4);
        assert_eq!(g.options.indent_style, IndentStyle::Tabs);
    }

    #[test]
    fn comma_separated_config() {
        let src =
            "// config: indent_width=4, line_width=80\nmodule m;\n// expected -----\nmodule m;\n";
        let g = parse_golden(src).unwrap();
        assert_eq!(g.options.indent_width, 4);
        assert_eq!(g.options.line_width, 80);
    }

    #[test]
    fn marker_must_have_dashes() {
        assert!(is_expected_marker("// expected -----"));
        assert!(is_expected_marker("// expected --"));
        assert!(!is_expected_marker("// expected"));
        assert!(!is_expected_marker("// not expected"));
    }

    #[test]
    fn no_marker_is_error() {
        let src = "module m;\nendmodule\n";
        let r = parse_golden(src);
        assert!(r.is_err());
    }
}
