//! Generic newline-triggered wrap masks for delimited groups.
//!
//! Walks the token stream pairing `(`/`)`, `{`/`}`, `[`/`]`. For each pair
//! whose source bytes strictly between opener and closer contain a `\n`,
//! both endpoints are flagged in the resulting masks. The verbatim engine
//! turns those flags into the canonical hanging shape:
//!
//! ```text
//! opener
//!   contents at depth+1 (preserving user split points)
//! closer
//! ```
//!
//! Openers that already belong to a special-purpose renderer (module port
//! lists, parameter port lists, instance port lists) are excluded so that
//! their dedicated logic owns the wrap shape.
//!
//! No CST traversal: parens/braces/brackets in the SV token stream are
//! always balanced, and the masks only care about pairing — not semantic
//! role. Tokens that look like delimiters but live inside string literals
//! or comments never appear in the token stream, so we don't need to skip
//! them here.

use std::collections::HashSet;

use vuff_sv_ast::Token;

#[must_use]
pub(crate) fn wrap_delimiter_masks(
    tokens: &[Token<'_>],
    source: &str,
    excluded_openers: &HashSet<usize>,
) -> (Vec<bool>, Vec<bool>) {
    let mut open_mask = vec![false; tokens.len()];
    let mut close_mask = vec![false; tokens.len()];
    let mut stack: Vec<(usize, &str)> = Vec::new();

    for (i, t) in tokens.iter().enumerate() {
        match t.text {
            "(" | "{" | "[" => stack.push((i, t.text)),
            ")" | "}" | "]" => {
                if let Some((open_i, open_text)) = stack.pop() {
                    if !matches_close(open_text, t.text) {
                        // Mismatched — the parser would have rejected this,
                        // so in well-formed input we never reach here. Skip
                        // defensively rather than panicking.
                        continue;
                    }
                    if excluded_openers.contains(&open_i) {
                        continue;
                    }
                    let from = tokens[open_i].end();
                    let to = t.offset;
                    if from <= to && source[from..to].contains('\n') {
                        open_mask[open_i] = true;
                        close_mask[i] = true;
                    }
                }
            }
            _ => {}
        }
    }

    (open_mask, close_mask)
}

fn matches_close(open: &str, close: &str) -> bool {
    matches!((open, close), ("(", ")") | ("{", "}") | ("[", "]"))
}
