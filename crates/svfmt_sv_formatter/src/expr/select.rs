//! Walk the CST and flag the `[` / `]` tokens that bracket a bit-select
//! or part-select on a primary expression (e.g. `mem[10]`, `bus[7:0]`).
//! Verbatim uses the mask to force the select brackets tight to the
//! identifier on the left — `mem[...]` not `mem [...]`.
//!
//! Packed / unpacked dimensions on a *declaration* (`logic [7:0] a
//! [0:255]`) use different CST nodes (`PackedDimension` /
//! `UnpackedDimension`) and are deliberately NOT flagged — their `[`
//! should keep a space before it.

use svfmt_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

/// Mask over `tokens` — true on every `[` of a bit-select or part-select.
pub(crate) fn select_open_bracket_mask(
    tree: &SyntaxTree,
    tokens: &[Token<'_>],
) -> Vec<bool> {
    let mut mask = vec![false; tokens.len()];
    let mut select_depth: u32 = 0;

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(ref node) => {
                if let RefNode::Locate(loc) = node {
                    if select_depth > 0 {
                        if let Some(idx) = tokens.iter().position(|t| t.offset == loc.offset) {
                            if tokens[idx].text == "[" {
                                mask[idx] = true;
                            }
                        }
                    }
                }
                if is_select(node) {
                    select_depth += 1;
                }
            }
            NodeEvent::Leave(ref node) => {
                if is_select(node) {
                    select_depth = select_depth.saturating_sub(1);
                }
            }
        }
    }
    mask
}

fn is_select(node: &RefNode<'_>) -> bool {
    matches!(
        node,
        RefNode::BitSelect(_)
            | RefNode::ConstantBitSelect(_)
            | RefNode::Select(_)
            | RefNode::ConstantSelect(_)
            | RefNode::NonrangeSelect(_)
    )
}
