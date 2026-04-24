//! Walk the CST and flag the `{` / `}` tokens that bracket a
//! concatenation-family node. Verbatim uses the mask to enforce tight
//! spacing: no space after a marked `{`, no space before a marked `}`.
//!
//! Covered node kinds:
//!   * `Concatenation` / `ConstantConcatenation` — `{a, b, c}`
//!   * `MultipleConcatenation` / `ConstantMultipleConcatenation` — `{N{x}}`
//!   * `ModulePathConcatenation` / `ModulePathMultipleConcatenation`
//!   * `StreamingConcatenation` / `StreamConcatenation` — `{<<{…}}`
//!   * `EmptyUnpackedArrayConcatenation` — `{}`
//!   * `AssignmentPatternExpression` — `'{…}` (the inner `{` / `}`; the
//!     leading `'` is tracked separately)
//!
//! The inner `{` of a `N{…}` replication (the second `{`) also needs
//! "no space before" since it follows an expression — that's the third
//! mask returned here.

use svfmt_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

/// Returns `(tight_after_open, tight_before_close, tight_before_open)`.
pub(crate) fn concat_brace_masks(
    tree: &SyntaxTree,
    tokens: &[Token<'_>],
) -> (Vec<bool>, Vec<bool>, Vec<bool>) {
    let mut after_open = vec![false; tokens.len()];
    let mut before_close = vec![false; tokens.len()];
    let mut before_open = vec![false; tokens.len()];
    let mut concat_depth: u32 = 0;
    // Per active MultipleConcatenation frame, count of `{` tokens seen so
    // far inside it. The 2nd brace is the replication's inner `{`.
    let mut rep_stack: Vec<u32> = Vec::new();

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(ref node) => {
                if let RefNode::Locate(loc) = node {
                    if concat_depth > 0 {
                        if let Some(idx) = tokens.iter().position(|t| t.offset == loc.offset) {
                            match tokens[idx].text {
                                "{" => {
                                    after_open[idx] = true;
                                    if let Some(count) = rep_stack.last_mut() {
                                        *count += 1;
                                        if *count == 2 {
                                            before_open[idx] = true;
                                        }
                                    }
                                }
                                "}" => before_close[idx] = true,
                                _ => {}
                            }
                        }
                    }
                }
                if is_concat_like(node) {
                    concat_depth += 1;
                }
                if is_replication(node) {
                    rep_stack.push(0);
                }
            }
            NodeEvent::Leave(ref node) => {
                if is_replication(node) {
                    rep_stack.pop();
                }
                if is_concat_like(node) {
                    concat_depth = concat_depth.saturating_sub(1);
                }
            }
        }
    }
    (after_open, before_close, before_open)
}

fn is_concat_like(node: &RefNode<'_>) -> bool {
    matches!(
        node,
        RefNode::Concatenation(_)
            | RefNode::ConstantConcatenation(_)
            | RefNode::MultipleConcatenation(_)
            | RefNode::ConstantMultipleConcatenation(_)
            | RefNode::ModulePathConcatenation(_)
            | RefNode::ModulePathMultipleConcatenation(_)
            | RefNode::StreamingConcatenation(_)
            | RefNode::StreamConcatenation(_)
            | RefNode::EmptyUnpackedArrayConcatenation(_)
            | RefNode::AssignmentPatternExpression(_)
            | RefNode::AssignmentPattern(_)
    )
}

fn is_replication(node: &RefNode<'_>) -> bool {
    matches!(
        node,
        RefNode::MultipleConcatenation(_)
            | RefNode::ConstantMultipleConcatenation(_)
            | RefNode::ModulePathMultipleConcatenation(_)
    )
}
