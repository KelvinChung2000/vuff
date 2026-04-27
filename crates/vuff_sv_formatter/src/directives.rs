//! Re-splice the conditional-compilation directives that sv-parser's
//! preprocessor strips before tokenization.
//!
//! `` `ifdef ``, `` `ifndef ``, `` `elsif ``, `` `else ``, and `` `endif ``
//! lines never reach the CST — only the active-branch bodies do. Without
//! intervention the formatted output would be missing them entirely.
//!
//! sv-parser-pp records every directive it visits (via the additive
//! `DirectiveSpan` API in our fork). We walk the
//! [`DirectiveKind::IfdefChain`] entries, emit one anchor per branch
//! keyword line plus a trailing `` `endif `` anchor, and let
//! [`crate::verbatim::emit_directives_around`] interleave them with
//! comments / `` `define `` lines in original-source order.
//!
//! Behavior parity with the previous text-scan implementation: only
//! line-leading directives are anchored. Same-line shapes
//! (`logic a;` `\`ifdef X`) are left unhandled — same as before — so
//! the migration doesn't change observable output.

use vuff_sv_ast::{DirectiveDetail, DirectiveKind, IfdefChain, Parsed, PpRange, Token};

/// One preserved directive line, pre-stripped of trailing whitespace.
/// Stored with the token index it should be emitted *before* and the
/// directive's byte offset in the *original* source — used by the
/// emission path to interleave directives with comments / active
/// `\`define`s in original source order. Directives whose anchor would
/// be past the last token get `anchor = tokens.len()` and are emitted by
/// the root rule after the final token.
#[derive(Clone, Debug)]
pub(crate) struct DirectiveAnchor {
    pub(crate) anchor_tok: usize,
    pub(crate) orig_start: usize,
    pub(crate) text: String,
}

/// Full scan result: anchors in emission order.
pub(crate) type DirectiveAnchors = Vec<DirectiveAnchor>;

pub(crate) fn scan(parsed: &Parsed, tokens: &[Token<'_>]) -> DirectiveAnchors {
    let token_orig_starts = compute_token_original_starts(parsed, tokens);
    let mut out: DirectiveAnchors = Vec::new();
    for d in parsed.tree.directives() {
        if d.kind != DirectiveKind::IfdefChain {
            continue;
        }
        // Anchors are byte offsets into `parsed.original` — directives
        // from `\`include`d files have offsets in a different file's
        // coordinate system and can't be anchored there. The included
        // file's tokens still emit normally; only the file-level
        // directives are skipped for v1.
        if d.original_path != parsed.original_path {
            continue;
        }
        let DirectiveDetail::IfdefChain(ref chain) = d.detail else {
            continue;
        };
        push_chain_anchors(parsed, &token_orig_starts, &d.original_range, chain, &mut out);
    }
    out.sort_by_key(|a| a.orig_start);
    out
}

fn push_chain_anchors(
    parsed: &Parsed,
    token_orig_starts: &[usize],
    chain_range: &PpRange,
    chain: &IfdefChain,
    out: &mut DirectiveAnchors,
) {
    for branch in &chain.branches {
        if let Some((orig_start, text)) =
            extract_directive_line(&parsed.original, branch.keyword_original_range.begin)
        {
            out.push(DirectiveAnchor {
                anchor_tok: anchor_for(token_orig_starts, orig_start),
                orig_start,
                text,
            });
        }
    }
    if let Some(endif_pos) = find_endif_in_chain(&parsed.original, chain_range) {
        if let Some((orig_start, text)) = extract_directive_line(&parsed.original, endif_pos) {
            out.push(DirectiveAnchor {
                anchor_tok: anchor_for(token_orig_starts, orig_start),
                orig_start,
                text,
            });
        }
    }
}

/// Read the line containing `keyword_pos` from `src`. Returns the
/// trimmed line text and the byte offset of the line's first
/// non-whitespace character. Returns `None` when the keyword is not at
/// the start of its line — that matches the legacy text-scan behavior
/// (it required the trimmed line to start with `` ` ``) and avoids
/// duplicating content from same-line shapes that the CST already
/// emits.
///
/// sv-parser-pp's keyword spans cover only the bare keyword text
/// (e.g. `ifdef`), not the leading `` ` ``. Treat a single `` ` `` byte
/// immediately before the keyword as part of the directive line lead.
fn extract_directive_line(src: &str, keyword_pos: usize) -> Option<(usize, String)> {
    let line_start = src[..keyword_pos].rfind('\n').map_or(0, |i| i + 1);
    let lead = &src[line_start..keyword_pos];
    let lead_trimmed = lead.trim_start_matches([' ', '\t']);
    if lead_trimmed != "`" {
        return None;
    }
    let directive_start = keyword_pos - 1;
    let line_end = src[directive_start..]
        .find('\n')
        .map_or(src.len(), |i| directive_start + i);
    let text = src[directive_start..line_end].trim_end().to_owned();
    Some((directive_start, text))
}

/// Locate the byte offset of the `endif` keyword (i.e. the byte just
/// after the leading `` ` ``) inside the chain's original range.
/// Returning the keyword position keeps the contract uniform with
/// sv-parser-pp's `keyword_original_range`, which already excludes the
/// backtick. Searches from the end so trailing whitespace on the chain
/// doesn't matter.
fn find_endif_in_chain(src: &str, chain: &PpRange) -> Option<usize> {
    src.get(chain.begin..chain.end)?
        .rfind("`endif")
        .map(|i| chain.begin + i + 1)
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
