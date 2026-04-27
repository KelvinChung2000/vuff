//! Detect tokens that came from preprocessor macro expansion and pair
//! each run of expanded tokens with its original call-site text.
//!
//! The formatter walks the post-preprocess CST, so a `\`assert(cond)`
//! call where the user wrote `\`define assert(...) empty_statement`
//! becomes the literal token `empty_statement` — the original macro
//! call has been replaced. Per the project policy of "format, don't
//! expand", we want the formatted output to read `\`assert(cond)`,
//! same as the input.
//!
//! Strategy:
//!
//! 1. Scan `parsed.original` for `` `define `` directive lines and
//!    record the byte ranges of each macro's body. A token is
//!    "macro-expanded" iff its origin (via `Parsed::origin_in_original`)
//!    falls inside any of those bodies.
//! 2. Walk the tokens and group consecutive macro-expanded ones into a
//!    `MacroRun`. A run's call-site source spans from the end of the
//!    previous non-expanded token's original range to the start of the
//!    next non-expanded token's original range, then trimmed.
//! 3. Verbatim emits the call-site text once at the run's first token
//!    and skips the rest.

use std::collections::HashMap;

use vuff_sv_ast::{Parsed, Token};

#[derive(Debug)]
pub(crate) struct MacroRun {
    /// First token index in the run.
    pub(crate) start: usize,
    /// Last token index in the run (inclusive).
    pub(crate) end: usize,
    /// Original-source text of the macro call site (e.g.
    /// `` `assert(condition) ``).
    pub(crate) call_text: String,
}

#[derive(Debug, Default)]
pub(crate) struct MacroCallInfo {
    /// Token index of run start → run details. Verbatim consults this
    /// at each token: a hit on the run start emits `call_text` and
    /// jumps the cursor past the run; subsequent indices in
    /// `skip_tok` are silently skipped.
    pub(crate) run_at_start: HashMap<usize, MacroRun>,
    /// Token indices that are part of a macro run but not the first
    /// token. Verbatim must skip them.
    pub(crate) skip_tok: std::collections::HashSet<usize>,
}

pub(crate) fn build_macro_calls(parsed: &Parsed, tokens: &[Token<'_>]) -> MacroCallInfo {
    let define_bodies = scan_define_bodies(&parsed.original);
    if define_bodies.is_empty() || tokens.is_empty() {
        return MacroCallInfo::default();
    }
    // Tokens whose post-pp position falls inside the verbatim
    // `\`define` line (which sv-parser preserves in `parsed.text`) are
    // the directive itself, not a macro expansion. Their origin still
    // points at the body, so we exclude them by post-pp position.
    let pp_define_ranges = scan_define_lines_pp(&parsed.text);

    // Per-token origin in original source. `None` if sv-parser couldn't
    // map (synthesized whitespace, included files, etc.).
    let origins: Vec<Option<usize>> = tokens
        .iter()
        .map(|t| parsed.origin_in_original(t.offset))
        .collect();
    let in_macro: Vec<bool> = origins
        .iter()
        .zip(tokens.iter())
        .map(|(o, t)| {
            o.is_some_and(|p| inside_any_body(&define_bodies, p))
                && !inside_any_body(&pp_define_ranges, t.offset)
        })
        .collect();

    let mut run_at_start: HashMap<usize, MacroRun> = HashMap::new();
    let mut skip_tok: std::collections::HashSet<usize> = std::collections::HashSet::new();

    let mut i = 0;
    while i < tokens.len() {
        if !in_macro[i] {
            i += 1;
            continue;
        }
        // Extend the run while we keep seeing macro-origin tokens.
        let start = i;
        let mut end = i;
        while end + 1 < tokens.len() && in_macro[end + 1] {
            end += 1;
        }
        // Find the call-site span in `parsed.original` from the
        // previous non-macro token's original end to the next
        // non-macro token's original start.
        let prev_end = if start == 0 {
            0
        } else {
            origins[start - 1]
                .map(|o| o + tokens[start - 1].len)
                .unwrap_or(0)
        };
        let next_start = if end + 1 >= tokens.len() {
            parsed.original.len()
        } else {
            origins[end + 1].unwrap_or(parsed.original.len())
        };
        if prev_end < next_start {
            let raw = &parsed.original[prev_end..next_start];
            // Trim leading and trailing whitespace; the trivia between
            // the surrounding tokens is reproduced separately by the
            // trivia path (it lives in `parsed.text`).
            let call_text = raw.trim().to_owned();
            if !call_text.is_empty() {
                for k in (start + 1)..=end {
                    skip_tok.insert(k);
                }
                run_at_start.insert(
                    start,
                    MacroRun {
                        start,
                        end,
                        call_text,
                    },
                );
            }
        }
        i = end + 1;
    }

    MacroCallInfo {
        run_at_start,
        skip_tok,
    }
}

/// Byte ranges of `` `define ... `` lines in `parsed.text` (post-pp).
/// sv-parser preserves the verbatim directive, so these ranges cover
/// the tokens that ARE the directive (not an expansion of it).
fn scan_define_lines_pp(src: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let bytes = src.as_bytes();
    let mut line_start: usize = 0;
    let mut i: usize = 0;
    while i <= bytes.len() {
        let end_of_line = i == bytes.len() || bytes[i] == b'\n';
        if end_of_line {
            let line = &src[line_start..i];
            let trimmed = line.trim_start();
            if trimmed.starts_with("`define")
                && trimmed[7..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
            {
                out.push((line_start, i));
            }
            line_start = i + 1;
        }
        i += 1;
    }
    out
}

/// Byte ranges in the original source that fall inside the body of a
/// `` `define ID(args) body `` directive. We stop at the line end (no
/// continuation handling for v0.1 — line-continuation backslashes are
/// not common in the bodies we need to detect).
fn scan_define_bodies(src: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let bytes = src.as_bytes();
    let mut line_start: usize = 0;
    let mut i: usize = 0;
    while i <= bytes.len() {
        let end_of_line = i == bytes.len() || bytes[i] == b'\n';
        if end_of_line {
            consider_define(src, line_start, i, &mut out);
            line_start = i + 1;
        }
        i += 1;
    }
    out
}

fn consider_define(src: &str, start: usize, end: usize, out: &mut Vec<(usize, usize)>) {
    let line = &src[start..end];
    let trimmed = line.trim_start();
    let lead_ws = line.len() - trimmed.len();
    let Some(rest) = trimmed.strip_prefix("`define") else {
        return;
    };
    if !rest.starts_with(|c: char| c.is_ascii_whitespace()) {
        return;
    }
    let after_kw = rest.trim_start();
    // Skip the macro name.
    let name_end = after_kw
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(after_kw.len());
    if name_end == 0 {
        return;
    }
    let after_name = &after_kw[name_end..];
    // Optional argument list `(...)`. Otherwise body starts after
    // whitespace.
    let after_args = if after_name.starts_with('(') {
        // Find matching `)`. Macros generally don't nest parens in
        // their argument list.
        let depth_close = after_name.find(')');
        match depth_close {
            Some(idx) => &after_name[idx + 1..],
            None => return,
        }
    } else {
        after_name
    };
    let body = after_args.trim_start();
    if body.is_empty() {
        return;
    }
    // Compute byte offsets of `body` inside `src`.
    let body_offset_in_line = lead_ws + (rest.len() - after_kw.len()) // `\`define` + first ws
        + name_end
        + (after_name.len() - after_args.len())
        + (after_args.len() - body.len());
    let body_start = start + body_offset_in_line;
    out.push((body_start, end));
}

fn inside_any_body(bodies: &[(usize, usize)], pos: usize) -> bool {
    bodies.iter().any(|&(s, e)| pos >= s && pos < e)
}
