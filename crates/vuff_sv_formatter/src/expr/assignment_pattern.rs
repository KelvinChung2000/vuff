//! Walk the CST and flag the `'{` token that opens an assignment pattern
//! or pattern literal (`'{...}`).
//!
//! sv-parser tokenizes `'{` as a single compound token (not `'` + `{`), so
//! this mask is "tight-after" — the token immediately following a marked
//! `'{` gets no leading space even if the input had one.
//!
//! Covered node kinds use `ApostropheBrace` in sv-parser:
//!   * `AssignmentPatternList` / `Structure` / `Array` / `Repeat`
//!   * `AssignmentPatternNetLvalue` / `VariableLvalue`
//!   * `PatternList` / `PatternIdentifierList`

use vuff_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

/// Mask over `tokens` — true on every `'{` token that opens an apostrophe-brace.
pub(crate) fn apostrophe_brace_mask(tree: &SyntaxTree, tokens: &[Token<'_>]) -> Vec<bool> {
    let mut mask = vec![false; tokens.len()];
    let mut pending: u32 = 0;

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(ref node) if opens_apostrophe(node) => {
                pending += 1;
            }
            NodeEvent::Enter(RefNode::Locate(loc)) if pending > 0 => {
                if let Some(idx) = tokens.iter().position(|t| t.offset == loc.offset) {
                    if tokens[idx].text == "'{" {
                        mask[idx] = true;
                        pending -= 1;
                    }
                }
            }
            _ => {}
        }
    }
    mask
}

fn opens_apostrophe(node: &RefNode<'_>) -> bool {
    matches!(
        node,
        RefNode::AssignmentPatternList(_)
            | RefNode::AssignmentPatternStructure(_)
            | RefNode::AssignmentPatternArray(_)
            | RefNode::AssignmentPatternRepeat(_)
            | RefNode::AssignmentPatternNetLvalue(_)
            | RefNode::AssignmentPatternVariableLvalue(_)
            | RefNode::PatternList(_)
            | RefNode::PatternIdentifierList(_)
    )
}
