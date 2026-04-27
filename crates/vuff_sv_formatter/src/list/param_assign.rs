//! Walk the CST and flag the `#` token that opens a
//! `ParameterValueAssignment` on a module instantiation
//! (e.g. `m #(.WIDTH(8)) u1(...)`).
//!
//! Verbatim uses the mask to:
//!   * force a space before the `#` (between the module name and `#(...)`),
//!   * force no-space between `#` and its following `(` — `#(` stays glued
//!     even if the input wrote `# (`.

use vuff_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

use crate::context::build_token_index;

/// Mask over `tokens` — true on every `#` token that opens a
/// `ParameterValueAssignment`.
pub(crate) fn param_assign_pound_mask(tree: &SyntaxTree, tokens: &[Token<'_>]) -> Vec<bool> {
    let mut mask = vec![false; tokens.len()];
    let mut inside: u32 = 0;
    let mut pending_first = false;
    let tok_idx = build_token_index(tokens);

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(RefNode::ParameterValueAssignment(_)) => {
                if inside == 0 {
                    pending_first = true;
                }
                inside += 1;
            }
            NodeEvent::Leave(RefNode::ParameterValueAssignment(_)) => {
                inside = inside.saturating_sub(1);
                if inside == 0 {
                    pending_first = false;
                }
            }
            NodeEvent::Enter(RefNode::Locate(loc)) if pending_first => {
                if let Some(&idx) = tok_idx.get(&loc.offset) {
                    if tokens[idx].text == "#" {
                        mask[idx] = true;
                        pending_first = false;
                    }
                }
            }
            _ => {}
        }
    }
    mask
}
