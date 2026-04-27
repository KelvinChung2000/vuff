//! Annex A.8.1 — `streaming_concatenation`. Walk the CST and flag every
//! token inside a [`StreamingConcatenation`] node so that adjacency
//! spacing in `verbatim` can suppress the binary-operator force-space
//! that would otherwise insert spaces around `<<` / `>>` (which inside a
//! streaming concat are direction markers, not shift operators) and
//! around the optional slice-size expression.

use vuff_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

/// Mask over `tokens` — true on every token whose CST position lies
/// inside any `StreamingConcatenation` node.
pub(crate) fn streaming_concat_mask(tree: &SyntaxTree, tokens: &[Token<'_>]) -> Vec<bool> {
    let mut mask = vec![false; tokens.len()];
    let mut depth: u32 = 0;

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(RefNode::StreamingConcatenation(_)) => depth += 1,
            NodeEvent::Leave(RefNode::StreamingConcatenation(_)) => {
                depth = depth.saturating_sub(1);
            }
            NodeEvent::Enter(RefNode::Locate(loc)) if depth > 0 => {
                if let Ok(idx) = tokens.binary_search_by_key(&loc.offset, |t| t.offset) {
                    mask[idx] = true;
                }
            }
            _ => {}
        }
    }
    mask
}
