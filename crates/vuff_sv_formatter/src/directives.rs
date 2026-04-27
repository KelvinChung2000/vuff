//! Re-splice compiler directives that `sv-parser`'s preprocessor drops.
//!
//! `sv-parser` strips `` `ifdef / `ifndef / `elsif / `else / `endif ``
//! entirely before handing tokens to the parser, so the CST never sees
//! them. We rescan the original source line by line and pick out every
//! line whose trimmed content begins with one of those keywords — the
//! preprocessor stripped them regardless of which branch was taken, so
//! re-emitting every match in source order reconstructs the original
//! conditional structure (active branch bodies remain in the CST and
//! emit normally).
//!
//! Each kept directive is anchored to the CST token it should precede in
//! the formatted output. [`format_token_range`] consults the anchor map
//! and emits directive lines before the anchor token.

use vuff_sv_ast::{Parsed, Token};

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
    let lines = find_directive_lines(&parsed.original);
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

/// Scan original source line by line. Keep every line whose trimmed
/// content begins with a preserved directive keyword. We rely purely on
/// the keyword pattern rather than the pp→original coverage map: the
/// preprocessor strips these directives from the CST regardless of which
/// branch is active, so re-splicing every matching line in original
/// source order is the right thing. Lines containing other content
/// alongside a directive (`logic a; `ifdef X`) are not handled — the
/// pattern requires the trimmed line to start with `` ` ``.
fn find_directive_lines(src: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let bytes = src.as_bytes();
    let mut line_start: usize = 0;
    let mut i: usize = 0;
    while i <= bytes.len() {
        let end_of_line = i == bytes.len() || bytes[i] == b'\n';
        if end_of_line {
            consider_line(src, line_start, i, &mut out);
            line_start = i + 1;
        }
        i += 1;
    }
    out
}

fn consider_line(src: &str, start: usize, end: usize, out: &mut Vec<(usize, String)>) {
    let line = &src[start..end];
    let trimmed = line.trim();
    if !is_preserved_directive(trimmed) {
        return;
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
        let orig = parsed.origin_in_original(t.offset).unwrap_or(last);
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
