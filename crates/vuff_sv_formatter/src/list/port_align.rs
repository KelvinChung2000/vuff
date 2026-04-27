//! ANSI port-list column alignment. Walks the CST for every
//! [`ListOfPortDeclarations`] and, for each [`AnsiPortDeclaration`] inside,
//! classifies its tokens into the four columns the alignment renderer
//! uses: direction keyword, net/var type (minus the packed dim), packed
//! dimension `[hi:lo]`, and name-plus-tail (identifier + unpacked dim +
//! optional default value).
//!
//! This file is pure CST analysis — it produces data. Rendering with
//! per-column padding lives in [`crate::list::port_align_render`].

use vuff_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

/// Inclusive token-index range `[start, end]` describing one port-list
/// column.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TokRange {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[derive(Debug)]
pub(crate) struct PortRow {
    pub(crate) dir: Option<TokRange>,
    pub(crate) typ: Option<TokRange>,
    pub(crate) packed: Option<TokRange>,
    /// Name identifier onward (unpacked dimension, `= default` if any).
    pub(crate) tail: TokRange,
    /// The `,` token immediately after this port, if any.
    pub(crate) comma_tok: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct PortList {
    pub(crate) paren_open: usize,
    pub(crate) paren_close: usize,
    pub(crate) rows: Vec<PortRow>,
    /// True when the source between `paren_open` and `paren_close` contains
    /// a `\n`. Drives the wrap trigger.
    pub(crate) has_internal_newline: bool,
}

// Long by design: one event-machine walks every relevant node kind in
// the port-list sub-tree. Splitting would scatter the shared mutable
// state across helpers without clarifying anything.
#[allow(clippy::too_many_lines)]
pub(crate) fn collect_port_lists(
    tree: &SyntaxTree,
    tokens: &[Token<'_>],
    source: &str,
) -> Vec<PortList> {
    let mut out: Vec<PortList> = Vec::new();
    let mut cur: Option<PortList> = None;

    // Depth counters — a single port may appear inside parenthesised
    // expressions, so we track nesting instead of boolean flags.
    let mut in_port_list: u32 = 0;
    let mut in_port: u32 = 0;
    let mut in_direction: u32 = 0;
    let mut in_type: u32 = 0;
    let mut in_packed: u32 = 0;
    let mut saw_name: bool = false;

    // Accumulators for the port currently being parsed.
    let mut dir_range: Option<TokRange> = None;
    let mut type_range: Option<TokRange> = None;
    let mut packed_range: Option<TokRange> = None;
    let mut tail_start: Option<usize> = None;
    let mut tail_end: Option<usize> = None;

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(RefNode::ListOfPortDeclarations(_)) => {
                in_port_list += 1;
                if in_port_list == 1 {
                    cur = Some(PortList {
                        paren_open: usize::MAX,
                        paren_close: usize::MAX,
                        rows: Vec::new(),
                        has_internal_newline: false,
                    });
                }
            }
            NodeEvent::Leave(RefNode::ListOfPortDeclarations(_)) => {
                in_port_list = in_port_list.saturating_sub(1);
                if in_port_list == 0 {
                    if let Some(mut list) = cur.take() {
                        if list.paren_open != usize::MAX && list.paren_close != usize::MAX {
                            let from = tokens[list.paren_open].end();
                            let to = tokens[list.paren_close].offset;
                            list.has_internal_newline = source[from..to].contains('\n');
                            out.push(list);
                        }
                    }
                }
            }
            NodeEvent::Enter(RefNode::AnsiPortDeclaration(_)) => {
                in_port += 1;
                if in_port == 1 {
                    dir_range = None;
                    type_range = None;
                    packed_range = None;
                    tail_start = None;
                    tail_end = None;
                    saw_name = false;
                }
            }
            NodeEvent::Leave(RefNode::AnsiPortDeclaration(_)) => {
                in_port = in_port.saturating_sub(1);
                if in_port == 0 {
                    if let (Some(ts), Some(te)) = (tail_start, tail_end) {
                        if let Some(list) = cur.as_mut() {
                            list.rows.push(PortRow {
                                dir: dir_range,
                                typ: type_range,
                                packed: packed_range,
                                tail: TokRange { start: ts, end: te },
                                comma_tok: None,
                            });
                        }
                    }
                }
            }
            NodeEvent::Enter(RefNode::PortDirection(_)) => in_direction += 1,
            NodeEvent::Leave(RefNode::PortDirection(_)) => {
                in_direction = in_direction.saturating_sub(1);
            }
            NodeEvent::Enter(RefNode::NetPortType(_) | RefNode::VariablePortType(_)) => {
                in_type += 1;
            }
            NodeEvent::Leave(RefNode::NetPortType(_) | RefNode::VariablePortType(_)) => {
                in_type = in_type.saturating_sub(1);
            }
            NodeEvent::Enter(RefNode::PackedDimension(_)) => {
                if in_port > 0 {
                    in_packed += 1;
                }
            }
            NodeEvent::Leave(RefNode::PackedDimension(_)) => {
                if in_port > 0 {
                    in_packed = in_packed.saturating_sub(1);
                }
            }
            NodeEvent::Enter(RefNode::PortIdentifier(_)) => {
                if in_port > 0 {
                    saw_name = true;
                }
            }
            NodeEvent::Enter(RefNode::Locate(loc)) if in_port_list > 0 => {
                let Some(idx) = tokens.iter().position(|t| t.offset == loc.offset) else {
                    continue;
                };
                let text = tokens[idx].text;

                // `(` / `)` of the port list itself: only captured at
                // port-list nesting depth 1, outside any port body.
                if in_port == 0 && in_port_list == 1 {
                    if let Some(list) = cur.as_mut() {
                        if text == "(" && list.paren_open == usize::MAX {
                            list.paren_open = idx;
                        } else if text == ")" {
                            list.paren_close = idx;
                        } else if text == "," {
                            if let Some(last) = list.rows.last_mut() {
                                if last.comma_tok.is_none() {
                                    last.comma_tok = Some(idx);
                                }
                            }
                        }
                    }
                    continue;
                }

                if in_port == 0 {
                    continue;
                }

                if in_direction > 0 {
                    update_range(&mut dir_range, idx);
                } else if in_packed > 0 {
                    update_range(&mut packed_range, idx);
                } else if in_type > 0 && !saw_name {
                    update_range(&mut type_range, idx);
                } else if saw_name {
                    if tail_start.is_none() {
                        tail_start = Some(idx);
                    }
                    tail_end = Some(idx);
                }
            }
            _ => {}
        }
    }
    out
}

fn update_range(slot: &mut Option<TokRange>, idx: usize) {
    match slot {
        Some(r) => r.end = idx,
        None => {
            *slot = Some(TokRange {
                start: idx,
                end: idx,
            });
        }
    }
}
