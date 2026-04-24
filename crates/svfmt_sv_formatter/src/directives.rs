//! Re-splice compiler directives that `sv-parser`'s preprocessor drops.
//!
//! `sv-parser` strips `` `ifdef / `ifndef / `elsif / `else / `endif ``
//! entirely before handing tokens to the parser, so the CST never sees
//! them. We reconstruct their byte ranges in the original source by
//! inverting the pp→original mapping that [`svfmt_sv_ast::Parsed`]
//! exposes: bytes in the original that have no pp-counterpart belong to
//! something the preprocessor consumed (directives, inactive branch
//! bodies, expanded macro usages). Of those, we keep the lines that start
//! with a directive keyword — the inactive-branch bodies were
//! *intentionally* dropped and we must not re-emit them.
//!
//! Each kept directive is anchored to the CST token it should precede in
//! the formatted output. [`format_token_range`] consults the anchor map
//! and emits directive lines before the anchor token.

use svfmt_sv_ast::{Parsed, Token};

/// One preserved directive line, pre-stripped of trailing whitespace.
/// Stored with the token index it should be emitted *before*. Directives
/// whose anchor would be past the last token get `anchor = tokens.len()`
/// and are emitted by the root rule after the final token.
#[derive(Clone, Debug)]
pub(crate) struct DirectiveAnchor {
    pub(crate) anchor_tok: usize,
    pub(crate) text: String,
}

/// Full scan result: anchors in emission order.
pub(crate) type DirectiveAnchors = Vec<DirectiveAnchor>;

pub(crate) fn scan(parsed: &Parsed, tokens: &[Token<'_>]) -> DirectiveAnchors {
    let covered = build_coverage(parsed);
    let lines = find_directive_lines(&parsed.original, &covered);
    let token_orig_starts = compute_token_original_starts(parsed, tokens);
    let mut out: DirectiveAnchors = Vec::with_capacity(lines.len());
    for (orig_start, text) in lines {
        let anchor = anchor_for(&token_orig_starts, orig_start);
        out.push(DirectiveAnchor {
            anchor_tok: anchor,
            text,
        });
    }
    out
}

/// Build a bitmap over `parsed.original` where `covered[i]` is true iff
/// some pp byte has origin `(original_path, i)`.
fn build_coverage(parsed: &Parsed) -> Vec<bool> {
    let mut covered = vec![false; parsed.original.len()];
    let pp_len = parsed.text.len();
    for pp_pos in 0..pp_len {
        if let Some(orig) = parsed.origin_in_original(pp_pos) {
            if orig < covered.len() {
                covered[orig] = true;
            }
        }
    }
    covered
}

/// Scan original source line by line. Keep every line that is fully
/// uncovered *and* starts with a preserved directive keyword. Return
/// `(line_start_offset, trimmed_text)` pairs.
fn find_directive_lines(src: &str, covered: &[bool]) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let bytes = src.as_bytes();
    let mut line_start: usize = 0;
    let mut i: usize = 0;
    while i <= bytes.len() {
        let end_of_line = i == bytes.len() || bytes[i] == b'\n';
        if end_of_line {
            consider_line(src, covered, line_start, i, &mut out);
            line_start = i + 1;
        }
        i += 1;
    }
    out
}

fn consider_line(
    src: &str,
    covered: &[bool],
    start: usize,
    end: usize,
    out: &mut Vec<(usize, String)>,
) {
    let line = &src[start..end];
    let trimmed = line.trim();
    if !is_preserved_directive(trimmed) {
        return;
    }
    // Every non-whitespace byte on this line must be uncovered; otherwise
    // the line is shared with CST content (e.g. `logic a; `ifdef X` on
    // the same line — not a pattern worth handling in the MVP).
    let bytes = line.as_bytes();
    for (off, b) in bytes.iter().enumerate() {
        if b.is_ascii_whitespace() {
            continue;
        }
        let idx = start + off;
        if idx < covered.len() && covered[idx] {
            return;
        }
    }
    out.push((start, trimmed.to_owned()));
}

fn is_preserved_directive(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('`') else {
        return false;
    };
    // Keyword is the leading identifier: letters only.
    let kw_end = rest
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(rest.len());
    let kw = &rest[..kw_end];
    matches!(kw, "ifdef" | "ifndef" | "elsif" | "else" | "endif")
}

/// For each token, its starting offset in the *original* source. If the
/// token has no mapping (synthesized or included from another file), we
/// fall back to the previous token's original end so the sequence stays
/// monotonic.
fn compute_token_original_starts(parsed: &Parsed, tokens: &[Token<'_>]) -> Vec<usize> {
    let mut out = Vec::with_capacity(tokens.len());
    let mut last: usize = 0;
    for t in tokens {
        let orig = parsed
            .origin_in_original(t.offset)
            .unwrap_or(last);
        last = orig;
        out.push(orig);
    }
    out
}

/// The anchor is the first token whose original start is `>= orig_start`.
/// If none, the directive goes after the last token.
fn anchor_for(token_orig_starts: &[usize], orig_start: usize) -> usize {
    token_orig_starts
        .iter()
        .position(|&o| o >= orig_start)
        .unwrap_or(token_orig_starts.len())
}
