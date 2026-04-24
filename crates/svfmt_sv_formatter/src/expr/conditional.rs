//! Annex A.8.3 — `ConditionalExpression` (`cond ? then : else`). Walks the
//! CST to identify the `:` tokens that are ternary colons (paired with a
//! `?`), producing a mask verbatim consults to force a space around the
//! `:`. Replaces the old `detect_ternary_colons` token-scan pre-pass.

use svfmt_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

/// Mask over `tokens` — true on every `:` token that is a ternary colon
/// (the `:` child of a `ConditionalExpression`).
pub(crate) fn ternary_colon_mask(tree: &SyntaxTree, tokens: &[Token<'_>]) -> Vec<bool> {
    let mut mask = vec![false; tokens.len()];
    for ev in tree.into_iter().event() {
        if let NodeEvent::Enter(RefNode::ConditionalExpression(ce)) = ev {
            // ConditionalExpression.nodes = (CondPredicate, Symbol[?],
            // Vec<AttributeInstance>, Expression, Symbol[:], Expression).
            // The `:` Symbol is at index 4; its inner Locate is at offset
            // `ce.nodes.4.nodes.0.offset`.
            let colon_offset = ce.nodes.4.nodes.0.offset;
            if let Ok(idx) = tokens.binary_search_by_key(&colon_offset, |t| t.offset) {
                mask[idx] = true;
            }
        }
    }
    mask
}
