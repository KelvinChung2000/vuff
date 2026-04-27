//! Locate top-level `module … endmodule` declarations in the CST and map
//! each one's byte span onto token indices in `ctx.tokens`.

use vuff_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

/// Inclusive token-index range `[start, end]` for one `ModuleDeclarationAnsi`
/// or `ModuleDeclarationNonansi` subtree.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ModuleSpan {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) fn find_module_spans(tree: &SyntaxTree, tokens: &[Token<'_>]) -> Vec<ModuleSpan> {
    let mut out = Vec::new();
    let mut depth: u32 = 0;
    let mut first_byte: Option<usize> = None;
    let mut last_byte_end: usize = 0;

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(node) if is_module_body_node(&node) => {
                if depth == 0 {
                    first_byte = None;
                    last_byte_end = 0;
                }
                depth += 1;
            }
            NodeEvent::Leave(node) if is_module_body_node(&node) => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    if let Some(fb) = first_byte {
                        if let Some(span) = byte_range_to_token_range(tokens, fb, last_byte_end) {
                            out.push(span);
                        }
                    }
                    first_byte = None;
                    last_byte_end = 0;
                }
            }
            NodeEvent::Enter(RefNode::Locate(loc)) if depth > 0 => {
                if first_byte.is_none() {
                    first_byte = Some(loc.offset);
                }
                let end = loc.offset + loc.len;
                if end > last_byte_end {
                    last_byte_end = end;
                }
            }
            _ => {}
        }
    }
    out
}

/// `ModuleDeclarationAnsi` and `ModuleDeclarationNonansi` are the two
/// variants that have a header + body + `endmodule`. `Wildcard` and the
/// `Extern*` variants are header-only or unusual; they fall through to
/// verbatim until they get their own rule.
fn is_module_body_node(node: &RefNode<'_>) -> bool {
    matches!(
        node,
        RefNode::ModuleDeclarationAnsi(_) | RefNode::ModuleDeclarationNonansi(_)
    )
}

fn byte_range_to_token_range(
    tokens: &[Token<'_>],
    first_byte: usize,
    last_byte_end: usize,
) -> Option<ModuleSpan> {
    // Tokens are in source order; binary-search both endpoints.
    let start = tokens.partition_point(|t| t.offset < first_byte);
    if start >= tokens.len() {
        return None;
    }
    let end_after = tokens.partition_point(|t| t.end() <= last_byte_end);
    if end_after == 0 {
        return None;
    }
    let end = end_after - 1;
    if end < start {
        return None;
    }
    Some(ModuleSpan { start, end })
}
