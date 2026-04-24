//! Walk the CST and flag each `(` that opens a control-flow header
//! (`if`, `while`, `for`, `repeat`, `foreach`, `case`, `casex`, `casez`,
//! `wait`, `do…while`). Verbatim uses the mask to force a space between
//! the leading keyword and the opening `(`.
//!
//! Two shapes of control-flow node:
//!
//! * **Single-predicate** (while / repeat / for / foreach / do-while /
//!   wait / case): one `(` per header. Mark the first `(` token inside
//!   the node.
//! * **Multi-predicate** (`ConditionalStatement`): sv-parser flattens
//!   `else if` chains into a `Vec<(else, if, Paren, stmt)>` on a single
//!   node, so the node contains N parenthesized predicates. We track a
//!   stack of most-recent `(` tokens and mark the top of stack whenever a
//!   `CondPredicate` child enters — this works uniformly for the leading
//!   `if` and every `else if`.

use svfmt_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

pub(crate) fn control_header_paren_mask(
    tree: &SyntaxTree,
    tokens: &[Token<'_>],
) -> Vec<bool> {
    let mut mask = vec![false; tokens.len()];
    let mut paren_stack: Vec<usize> = Vec::new();
    // Count of single-predicate frames still awaiting their first `(`.
    let mut single_pending: u32 = 0;

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(ref node) if opens_single_predicate(node) => {
                single_pending += 1;
            }
            NodeEvent::Enter(RefNode::Locate(loc)) => {
                if let Some(idx) = tokens.iter().position(|t| t.offset == loc.offset) {
                    match tokens[idx].text {
                        "(" => {
                            paren_stack.push(idx);
                            if single_pending > 0 {
                                mask[idx] = true;
                                single_pending -= 1;
                            }
                        }
                        ")" => {
                            paren_stack.pop();
                        }
                        _ => {}
                    }
                }
            }
            NodeEvent::Enter(ref node) if marks_cond_predicate(node) => {
                if let Some(&idx) = paren_stack.last() {
                    mask[idx] = true;
                }
            }
            _ => {}
        }
    }
    mask
}

/// Control-flow nodes whose header has exactly one parenthesized predicate.
fn opens_single_predicate(node: &RefNode<'_>) -> bool {
    matches!(
        node,
        RefNode::CaseStatementNormal(_)
            | RefNode::CaseStatementMatches(_)
            | RefNode::CaseStatementInside(_)
            | RefNode::LoopStatementWhile(_)
            | RefNode::LoopStatementFor(_)
            | RefNode::LoopStatementDoWhile(_)
            | RefNode::LoopStatementRepeat(_)
            | RefNode::LoopStatementForeach(_)
            | RefNode::WaitStatementWait(_)
            | RefNode::WaitStatementOrder(_)
    )
}

/// Inner node kinds used to mark each predicate of a ConditionalStatement
/// (including every `else if`).
fn marks_cond_predicate(node: &RefNode<'_>) -> bool {
    matches!(node, RefNode::CondPredicate(_))
}
