//! Walk the CST and flag the opening `(` of a subroutine call
//! (function / task / system task / method / randomize). Verbatim uses
//! the mask to force the call `(` tight to the callee identifier —
//! `$clog2(16)` not `$clog2 (16)`, `foo.bar(x)` not `foo.bar (x)`.
//!
//! Two event shapes:
//!
//! * For **system tf calls**, the identifier + Paren both live inside the
//!   call node (`SystemTfCallArgOptional`, etc.). We arm a "first `(`"
//!   mark scoped to the node's subtree — if the call has no `(` (e.g.
//!   `$finish` with no args), we disarm on node Leave.
//! * For **user tf / method calls**, the callee's identifier is emitted
//!   *before* the `Paren`, which wraps `ListOfArguments`. The call's `(`
//!   fires before `ListOfArguments` enters, so we use a paren stack:
//!   when `ListOfArguments` enters, mark the most-recent `(`.

use vuff_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

/// Mask over `tokens` — true on every `(` that opens a subroutine call.
pub(crate) fn call_open_paren_mask(tree: &SyntaxTree, tokens: &[Token<'_>]) -> Vec<bool> {
    let mut mask = vec![false; tokens.len()];
    let mut paren_stack: Vec<usize> = Vec::new();
    // Stack of unresolved first-paren-arms per active sys-tf call frame.
    // `true` = still waiting for the first `(`; flipped to false once
    // consumed. Popped on Leave.
    let mut armed_stack: Vec<bool> = Vec::new();

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(ref node) if pre_paren_call(node) => {
                armed_stack.push(true);
            }
            NodeEvent::Leave(ref node) if pre_paren_call(node) => {
                armed_stack.pop();
            }
            NodeEvent::Enter(RefNode::Locate(loc)) => {
                if let Some(idx) = tokens.iter().position(|t| t.offset == loc.offset) {
                    match tokens[idx].text {
                        "(" => {
                            if let Some(armed) = armed_stack.last_mut() {
                                if *armed {
                                    mask[idx] = true;
                                    *armed = false;
                                }
                            }
                            paren_stack.push(idx);
                        }
                        ")" => {
                            paren_stack.pop();
                        }
                        _ => {}
                    }
                }
            }
            NodeEvent::Enter(ref node) if inside_call_paren(node) => {
                if let Some(&idx) = paren_stack.last() {
                    mask[idx] = true;
                }
            }
            _ => {}
        }
    }
    mask
}

/// Nodes whose children include the call `(`. The `(` hasn't entered yet
/// when we see these.
fn pre_paren_call(node: &RefNode<'_>) -> bool {
    matches!(
        node,
        RefNode::SystemTfCallArgOptional(_)
            | RefNode::SystemTfCallArgDataType(_)
            | RefNode::SystemTfCallArgExpression(_)
    )
}

/// Nodes that appear inside the call `(` (the `(` is already on the
/// paren stack by the time we see these).
fn inside_call_paren(node: &RefNode<'_>) -> bool {
    matches!(node, RefNode::ListOfArguments(_))
}
