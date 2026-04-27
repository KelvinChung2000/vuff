//! Walk the CST and flag each `(` token that opens a port list
//! (`ListOfPorts` / `ListOfPortDeclarations`). Verbatim uses the resulting
//! mask to force a space between a preceding identifier and the port
//! list's `(`, the one place where `ident(` must become `ident (`.
//!
//! Parameter port lists start with `#(`; their `(` is NOT included here
//! because `#(` should stay glued.

use vuff_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

use crate::context::build_token_index;

/// Mask over `tokens` — true on every `(` that begins a port list.
pub(crate) fn force_space_before_port_paren_mask(
    tree: &SyntaxTree,
    tokens: &[Token<'_>],
) -> Vec<bool> {
    let mut mask = vec![false; tokens.len()];
    let mut inside_port_list: u32 = 0;
    let mut pending_first_paren = false;
    let tok_idx = build_token_index(tokens);

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(RefNode::ListOfPorts(_) | RefNode::ListOfPortDeclarations(_)) => {
                if inside_port_list == 0 {
                    pending_first_paren = true;
                }
                inside_port_list += 1;
            }
            NodeEvent::Leave(RefNode::ListOfPorts(_) | RefNode::ListOfPortDeclarations(_)) => {
                inside_port_list = inside_port_list.saturating_sub(1);
                if inside_port_list == 0 {
                    pending_first_paren = false;
                }
            }
            NodeEvent::Enter(RefNode::Locate(loc)) if pending_first_paren => {
                // First Locate inside the port list node should be `(`. Find
                // the token at this offset and mark it.
                if let Some(&idx) = tok_idx.get(&loc.offset) {
                    if tokens[idx].text == "(" {
                        mask[idx] = true;
                    }
                }
                pending_first_paren = false;
            }
            _ => {}
        }
    }
    mask
}
