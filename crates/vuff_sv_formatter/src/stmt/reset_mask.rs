//! Mark every token at which the "statement-continuation" state
//! (`Formatter::in_statement`) must reset to false. After such a token,
//! a following newline indents at the raw structural depth rather than
//! at depth+1.
//!
//! Two sources contribute to the mask:
//!
//! 1. **Block-scope nodes.** The first and last `Locate` of each
//!    block-like CST node (`SeqBlock`, `ModuleDeclaration*`,
//!    `FunctionDeclaration`, `GenerateRegion`, `CaseStatement*`, …) — i.e.
//!    the opening keyword (`begin`, `module`, `function`, `generate`,
//!    `case`, …) and its matching closer (`end`, `endmodule`,
//!    `endfunction`, `endgenerate`, `endcase`, …). These reset because
//!    body items inside a block are not statement continuations of the
//!    block-opening keyword.
//!
//! 2. **Statement terminators.** The token text `;` always resets. It's
//!    the only string match left here — punctuation rather than a
//!    keyword, so the CST node it belongs to varies (statement
//!    terminator, declaration terminator, port-list separator-then-end,
//!    etc.). Detecting it via the token itself is unambiguous.

use vuff_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

pub(crate) fn statement_reset_mask(tree: &SyntaxTree, tokens: &[Token<'_>]) -> Vec<bool> {
    let mut mask = vec![false; tokens.len()];

    // Each open block-scope node tracks: whether it's still expecting its
    // first Locate (the opener), and the most-recent Locate seen at the
    // outermost-Keyword level inside it (used to mark the closer on
    // Leave).
    let mut pending_first: Vec<bool> = Vec::new();
    let mut last_seen_in_block: Vec<Option<usize>> = Vec::new();

    // Stack of "next Locate is the primary Locate of this Keyword". One
    // entry per currently-open Keyword sub-node. Updates to
    // `last_seen_in_block` only happen when the stack depth is exactly 1
    // — i.e. the Locate is the primary of an outermost Keyword inside
    // the current block. sv-parser sometimes wraps trailing directive
    // tokens (`` `default_nettype wire `` after `endmodule`) as Keyword
    // children of the closer's own Keyword; the depth-1 filter prevents
    // those from overwriting the real closer position.
    let mut primary_pending: Vec<bool> = Vec::new();

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(n) if is_block_scope_node(&n) => {
                pending_first.push(true);
                last_seen_in_block.push(None);
            }
            NodeEvent::Leave(n) if is_block_scope_node(&n) => {
                pending_first.pop();
                if let Some(Some(last_idx)) = last_seen_in_block.pop() {
                    mask[last_idx] = true;
                }
            }
            NodeEvent::Enter(RefNode::Keyword(_)) => {
                primary_pending.push(true);
            }
            NodeEvent::Leave(RefNode::Keyword(_)) => {
                primary_pending.pop();
            }
            NodeEvent::Enter(RefNode::Locate(loc)) => {
                let Ok(idx) = tokens.binary_search_by_key(&loc.offset, |t| t.offset) else {
                    continue;
                };
                // Mark the first Locate inside any pending block-scope
                // node — the opener keyword.
                let mut marked_any = false;
                for entry in &mut pending_first {
                    if *entry {
                        *entry = false;
                        marked_any = true;
                    }
                }
                if marked_any {
                    mask[idx] = true;
                }
                // Closer-position tracker. See the comment on
                // `primary_pending` for why depth==1 is required.
                if primary_pending.len() == 1 && primary_pending[0] {
                    primary_pending[0] = false;
                    if let Some(slot) = last_seen_in_block.last_mut() {
                        *slot = Some(idx);
                    }
                } else if let Some(top) = primary_pending.last_mut() {
                    // Consume the flag so subsequent Locates inside this
                    // Keyword don't repeatedly fire.
                    *top = false;
                }
            }
            _ => {}
        }
    }

    // Statement terminators — punctuation, not a keyword. Marking by token
    // text avoids walking yet another CST relation just to find a `;`.
    for (idx, t) in tokens.iter().enumerate() {
        if t.text == ";" {
            mask[idx] = true;
        }
    }

    mask
}

fn is_block_scope_node(node: &RefNode<'_>) -> bool {
    matches!(
        node,
        RefNode::SeqBlock(_)
            | RefNode::ParBlock(_)
            // `begin … end` inside generate constructs is a separate
            // node (GenerateBlockMultiple), not SeqBlock, but its
            // opener/closer behave the same way.
            | RefNode::GenerateBlockMultiple(_)
            | RefNode::ModuleDeclarationAnsi(_)
            | RefNode::ModuleDeclarationNonansi(_)
            | RefNode::InterfaceDeclaration(_)
            | RefNode::ProgramDeclaration(_)
            | RefNode::PackageDeclaration(_)
            | RefNode::FunctionDeclaration(_)
            | RefNode::TaskDeclaration(_)
            | RefNode::ClassDeclaration(_)
            | RefNode::GenerateRegion(_)
            | RefNode::SpecifyBlock(_)
            | RefNode::CheckerDeclaration(_)
            | RefNode::CovergroupDeclaration(_)
            | RefNode::UdpDeclaration(_)
            | RefNode::ConfigDeclaration(_)
            | RefNode::CaseStatementNormal(_)
            | RefNode::CaseStatementMatches(_)
            | RefNode::CaseStatementInside(_)
            | RefNode::CaseGenerateConstruct(_)
    )
}
