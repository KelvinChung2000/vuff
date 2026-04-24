//! Annex A.6.4 — mark every token that begins a new statement. Consulted
//! by verbatim to clear `in_statement` before emitting such a token, so
//! the trivia that precedes it indents at structural depth and not one
//! level deeper.
//!
//! Replaces the old keyword-text heuristic. Uses the CST as authority:
//! `Statement`, `StatementOrNull`, and the `CaseItem*` variants each
//! carry a first `Locate` whose offset is a statement boundary.

use svfmt_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

pub(crate) fn statement_boundary_mask(tree: &SyntaxTree, tokens: &[Token<'_>]) -> Vec<bool> {
    let mut mask = vec![false; tokens.len()];
    // Each entry is `true` iff we're still waiting for the first `Locate`
    // inside a boundary node opened at that stack position.
    let mut pending: Vec<bool> = Vec::new();

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(n) if is_boundary_node(&n) => pending.push(true),
            NodeEvent::Leave(n) if is_boundary_node(&n) => {
                pending.pop();
            }
            NodeEvent::Enter(RefNode::Locate(loc)) => {
                let mut marked = false;
                for entry in &mut pending {
                    if *entry {
                        *entry = false;
                        marked = true;
                    }
                }
                if marked {
                    if let Ok(idx) = tokens.binary_search_by_key(&loc.offset, |t| t.offset) {
                        mask[idx] = true;
                    }
                }
            }
            _ => {}
        }
    }
    mask
}

fn is_boundary_node(node: &RefNode<'_>) -> bool {
    matches!(
        node,
        RefNode::Statement(_)
            | RefNode::StatementOrNull(_)
            | RefNode::CaseItemNondefault(_)
            | RefNode::CaseItemDefault(_)
            | RefNode::CasePatternItemNondefault(_)
            | RefNode::CaseInsideItemNondefault(_)
    )
}
