//! Locate `AttributeInstance` subtrees in the CST and decide whether each
//! one rendered across multiple lines in the original source. CST-based
//! detection is authoritative: we do not re-parse `(* ... *)` from the
//! token stream.

use svfmt_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

#[derive(Debug, Clone, Copy)]
pub(crate) struct AttributeSpan {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) is_multiline: bool,
}

pub(crate) fn find_attribute_spans(
    tree: &SyntaxTree,
    source: &str,
    tokens: &[Token<'_>],
) -> Vec<AttributeSpan> {
    let mut out = Vec::new();
    let mut depth: u32 = 0;
    let mut first_byte: Option<usize> = None;
    let mut last_byte_end: usize = 0;

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(RefNode::AttributeInstance(_)) => {
                if depth == 0 {
                    first_byte = None;
                    last_byte_end = 0;
                }
                depth += 1;
            }
            NodeEvent::Leave(RefNode::AttributeInstance(_)) => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    if let Some(fb) = first_byte {
                        if let Some(span) = to_span(tokens, source, fb, last_byte_end) {
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

fn to_span(
    tokens: &[Token<'_>],
    source: &str,
    first_byte: usize,
    last_byte_end: usize,
) -> Option<AttributeSpan> {
    let start = tokens.iter().position(|t| t.offset >= first_byte)?;
    let end = tokens.iter().rposition(|t| t.end() <= last_byte_end)?;
    if end < start {
        return None;
    }
    // Scan the source bytes covered by the attribute for any newline —
    // if the user wrote it multiline, we preserve that shape.
    let span_src = &source[tokens[start].offset..tokens[end].end()];
    let is_multiline = span_src.contains('\n');
    Some(AttributeSpan {
        start,
        end,
        is_multiline,
    })
}

/// Build the `Vec<bool>` indexed by token position that `verbatim` uses to
/// decide where to force a hard break. This is the compatibility shim that
/// keeps verbatim's `force_nl_before` mechanism working until per-span
/// dispatch lands.
pub(crate) fn force_nl_before_mask(spans: &[AttributeSpan], n_tokens: usize) -> Vec<bool> {
    let mut mask = vec![false; n_tokens];
    for span in spans {
        if !span.is_multiline {
            continue;
        }
        // First content token after `(*` — or `*)` itself if the attribute
        // is empty (which still gives the expected `(*\n*)` layout).
        if span.start < span.end {
            mask[span.start + 1] = true;
        }
        mask[span.end] = true;
    }
    mask
}
