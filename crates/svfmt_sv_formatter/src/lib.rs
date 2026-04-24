//! SystemVerilog formatter.
//!
//! v0.1 strategy: a CST-driven token re-emitter. Tokens are preserved
//! verbatim; inter-token trivia is normalized; spacing/indent decisions
//! are driven by precomputed CST masks (attributes, ternary colons,
//! control-header parens, concat braces, select brackets, call parens,
//! …) and a per-token CST depth map.
//!
//! Module map:
//! * [`context`] — `FormatCtx` (read-only) + `Formatter` (mutable emit)
//! * [`format_ext`] — the `Format` trait for per-node rules
//! * [`verbatim`] — the sole token-range emitter; consumes all masks
//! * [`tokens`] — adjacency spacing, opener/closer tables, trivia emission
//! * [`source_text`] — Annex A.1.2 root rule
//! * [`module`], [`stmt`], [`expr`], [`list`], [`attribute`] — CST-mask
//!   providers grouped by grammar area. Each file computes one mask
//!   (e.g. `concat_brace_masks`, `control_header_paren_mask`).
//!
//! Feature rules will accrete as dedicated per-node emitters migrate off
//! the shared `verbatim` engine per `docs/spec-tracker.md`.

mod attribute;
mod context;
mod directives;
mod expr;
mod format_ext;
mod indent_map;
mod list;
mod module;
mod source_text;
mod stmt;
mod tokens;
mod verbatim;

use std::path::PathBuf;

use svfmt_config::FormatOptions;
use svfmt_formatter::print;
use svfmt_sv_ast::{assert_roundtrip, parse, tokens};

use crate::context::{print_options_from, FormatCtx, Formatter};
use crate::format_ext::Format;
use crate::source_text::SourceTextRoot;

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("parse error: {0}")]
    Parse(#[from] svfmt_sv_ast::ParseError),
    #[error("round-trip check failed: {0}")]
    RoundTrip(#[from] svfmt_sv_ast::RoundTripError),
}

/// Format a SystemVerilog source string.
pub fn format_source(src: &str, opts: &FormatOptions) -> Result<String, FormatError> {
    let parsed = parse(src, &PathBuf::from("<input>"))?;
    assert_roundtrip(&parsed.text, &parsed.tree)?;

    let toks = tokens(&parsed.tree);
    let directive_anchors = directives::scan(&parsed, &toks);
    let ctx = FormatCtx::new(opts, &parsed.text, &toks, &parsed.tree, &directive_anchors);
    let mut f = Formatter::new(opts, toks.len() * 2 + 4);
    SourceTextRoot.fmt(&ctx, &mut f);

    Ok(print(&f.out, &print_options_from(opts)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use svfmt_config::{BeginStyle, IndentStyle};

    fn fmt(src: &str) -> String {
        format_source(src, &FormatOptions::default()).unwrap()
    }

    #[test]
    fn empty_module_is_preserved() {
        let src = "module m;\nendmodule\n";
        assert_eq!(fmt(src), src);
    }

    #[test]
    fn normalizes_trailing_whitespace() {
        let src = "module m;   \nendmodule\n";
        let out = fmt(src);
        for line in out.lines() {
            assert_eq!(line, line.trim_end(), "trailing ws: {line:?}");
        }
    }

    #[test]
    fn collapses_blank_line_runs() {
        let src = "module m;\n\n\n\nendmodule\n";
        let out = fmt(src);
        assert!(
            !out.contains("\n\n\n"),
            "more than one blank line survived: {out:?}"
        );
    }

    #[test]
    fn reindents_inside_begin() {
        let src = "module m;\ninitial begin\nx = 1;\nend\nendmodule\n";
        let out = fmt(src);
        let lines: Vec<&str> = out.lines().collect();
        let assign = lines
            .iter()
            .find(|l| l.trim() == "x = 1;")
            .expect("x = 1; line present");
        assert!(
            assign.starts_with(' ') || assign.starts_with('\t'),
            "not indented inside begin: {assign:?}"
        );
    }

    #[test]
    fn tab_indent_style() {
        let opts = FormatOptions {
            indent_style: IndentStyle::Tabs,
            ..FormatOptions::default()
        };
        let src = "module m;\ninitial begin\nx = 1;\nend\nendmodule\n";
        let out = format_source(src, &opts).unwrap();
        assert!(out.contains("\tx = 1;"), "expected tab in {out:?}");
    }

    #[test]
    fn ensures_terminating_newline() {
        let src = "module m; endmodule";
        let out = fmt(src);
        assert!(out.ends_with('\n'));
        let nl_count = out.chars().filter(|&c| c == '\n').count();
        assert_eq!(nl_count, 1, "exactly one trailing nl: {out:?}");
    }

    #[test]
    fn comments_preserved_inline() {
        let src = "module m; // inline\nendmodule\n";
        let out = fmt(src);
        assert!(out.contains("// inline"), "output: {out:?}");
    }

    #[test]
    fn comments_preserved_leading() {
        let src = "// leading\nmodule m;\nendmodule\n";
        let out = fmt(src);
        assert!(out.contains("// leading"), "output: {out:?}");
    }

    #[test]
    fn begin_style_allman_breaks_before_begin() {
        let opts = FormatOptions {
            begin_style: BeginStyle::Allman,
            ..FormatOptions::default()
        };
        let src = "module m;\ninitial begin\nend\nendmodule\n";
        let out = format_source(src, &opts).unwrap();
        let has_begin_on_line = out.lines().any(|l| l.trim() == "begin");
        assert!(
            has_begin_on_line,
            "allman style did not isolate begin: {out:?}"
        );
    }
}
