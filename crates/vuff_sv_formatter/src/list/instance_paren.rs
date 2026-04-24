//! Walk the CST and flag each `(` token that opens a `HierarchicalInstance`
//! port-connection list (e.g. `sub u_sub(.clk(clk))`). Verbatim uses the
//! resulting mask to force a space between the instance name and its
//! opening `(`.
//!
//! The inner `(` of named port connections (`.clk(clk)`) is NOT flagged —
//! those sit inside `ListOfPortConnections` and stay tight.

use vuff_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

/// Mask over `tokens` — true on every `(` that opens a hierarchical
/// instance's port-connection list.
pub(crate) fn force_space_before_instance_paren_mask(
    tree: &SyntaxTree,
    tokens: &[Token<'_>],
) -> Vec<bool> {
    let mut mask = vec![false; tokens.len()];
    let mut inside: u32 = 0;
    let mut pending_first_paren = false;

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(RefNode::HierarchicalInstance(_)) => {
                if inside == 0 {
                    pending_first_paren = true;
                }
                inside += 1;
            }
            NodeEvent::Leave(RefNode::HierarchicalInstance(_)) => {
                inside = inside.saturating_sub(1);
                if inside == 0 {
                    pending_first_paren = false;
                }
            }
            NodeEvent::Enter(RefNode::Locate(loc)) if pending_first_paren => {
                if let Some(idx) = tokens.iter().position(|t| t.offset == loc.offset) {
                    if tokens[idx].text == "(" {
                        mask[idx] = true;
                        pending_first_paren = false;
                    }
                }
            }
            _ => {}
        }
    }
    mask
}
