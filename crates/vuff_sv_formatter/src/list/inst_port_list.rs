//! Annex A.4.1.1 — `module_instantiation` port-connection list. Walk each
//! `HierarchicalInstance` and collect its `(` / `)` and the comma-separated
//! `OrderedPortConnection` / `NamedPortConnection` rows. The renderer below
//! emits a one-row-per-line wrapped form; the caller decides flat-vs-wrap
//! by measuring the inline width against `line_width`.
//!
//! Multi-instance `ModuleInstantiation` (e.g. `m u1(...), u2(...);`) is
//! supported: each `HierarchicalInstance` is collected as its own entry.

use vuff_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

use crate::context::{build_token_index, FormatCtx, Formatter};
use crate::tokens::trivia::{emit_trivia_at, emit_trivia_slice, ensure_fresh_line, SliceMode};

#[derive(Debug, Clone, Copy)]
pub(crate) struct InstPortRow {
    pub(crate) start: usize,
    pub(crate) end: usize,
    /// The `,` token immediately after this row, if any.
    pub(crate) comma_tok: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct InstPortList {
    pub(crate) paren_open: usize,
    pub(crate) paren_close: usize,
    pub(crate) rows: Vec<InstPortRow>,
    /// True when the source text strictly between `paren_open` and
    /// `paren_close` contains a `\n`. Drives the wrap trigger: humans
    /// signal "wrap me" by inserting a newline.
    pub(crate) has_internal_newline: bool,
}

pub(crate) fn collect_inst_port_lists(
    tree: &SyntaxTree,
    tokens: &[Token<'_>],
    source: &str,
) -> Vec<InstPortList> {
    let mut out: Vec<InstPortList> = Vec::new();
    let mut cur: Option<InstPortList> = None;

    let mut in_inst: u32 = 0;
    let mut in_row: u32 = 0;
    let mut row_start: Option<usize> = None;
    let mut row_end: Option<usize> = None;
    let tok_idx = build_token_index(tokens);

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(RefNode::HierarchicalInstance(_)) => {
                if in_inst == 0 {
                    cur = Some(InstPortList {
                        paren_open: usize::MAX,
                        paren_close: usize::MAX,
                        rows: Vec::new(),
                        has_internal_newline: false,
                    });
                }
                in_inst += 1;
            }
            NodeEvent::Leave(RefNode::HierarchicalInstance(_)) => {
                in_inst = in_inst.saturating_sub(1);
                if in_inst == 0 {
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
            NodeEvent::Enter(
                RefNode::OrderedPortConnection(_) | RefNode::NamedPortConnection(_),
            ) if in_inst > 0 => {
                if in_row == 0 {
                    row_start = None;
                    row_end = None;
                }
                in_row += 1;
            }
            NodeEvent::Leave(
                RefNode::OrderedPortConnection(_) | RefNode::NamedPortConnection(_),
            ) if in_inst > 0 => {
                in_row = in_row.saturating_sub(1);
                if in_row == 0 {
                    if let (Some(s), Some(e)) = (row_start, row_end) {
                        if let Some(list) = cur.as_mut() {
                            list.rows.push(InstPortRow {
                                start: s,
                                end: e,
                                comma_tok: None,
                            });
                        }
                    }
                }
            }
            NodeEvent::Enter(RefNode::Locate(loc)) if in_inst > 0 => {
                let Some(&idx) = tok_idx.get(&loc.offset) else {
                    continue;
                };
                let text = tokens[idx].text;
                if in_row > 0 {
                    if row_start.is_none() {
                        row_start = Some(idx);
                    }
                    row_end = Some(idx);
                    continue;
                }
                if in_inst == 1 {
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
                }
            }
            _ => {}
        }
    }
    out
}

/// Render `( row, row, … )` with one row per line at body depth. The leading
/// `(` is owned by this renderer; the caller's verbatim pass emits everything
/// up to (but not including) `paren_open`.
pub(crate) fn render_wrapped(ctx: &FormatCtx<'_>, f: &mut Formatter, list: &InstPortList) {
    f.push_text("(".to_owned());

    let row_depth = f.depth + 1;
    let close_depth = f.depth;

    if list.rows.is_empty() {
        emit_trivia_slice(
            f,
            &ctx.trivia.slices[list.paren_close],
            row_depth,
            0,
            SliceMode::Embedded,
        );
        f.push_text(")".to_owned());
        return;
    }

    for (i, row) in list.rows.iter().enumerate() {
        emit_trivia_slice(
            f,
            &ctx.trivia.slices[row.start],
            row_depth,
            0,
            SliceMode::Embedded,
        );

        ensure_fresh_line(f);
        f.push_indent_levels(row_depth);

        let saved = f.depth;
        f.depth = row_depth;
        emit_row_tokens(ctx, f, row);
        f.depth = saved;

        let is_last = i == list.rows.len() - 1;
        if !is_last {
            f.push_text(",".to_owned());
        }
    }

    emit_trivia_slice(
        f,
        &ctx.trivia.slices[list.paren_close],
        row_depth,
        0,
        SliceMode::Embedded,
    );

    ensure_fresh_line(f);
    f.push_indent_levels(close_depth);
    f.push_text(")".to_owned());
}

fn emit_row_tokens(ctx: &FormatCtx<'_>, f: &mut Formatter, row: &InstPortRow) {
    let mut prev_text: Option<&str> = None;
    let mut prev_end: usize = ctx.tokens[row.start].offset;
    for idx in row.start..=row.end {
        let t = &ctx.tokens[idx];
        if idx > row.start {
            let between = &ctx.source[prev_end..t.offset];
            let needs_space =
                prev_text.is_some_and(|p| crate::tokens::spacing::force_space_between(p, t.text));
            let no_space =
                prev_text.is_some_and(|p| crate::tokens::spacing::no_space_between(p, t.text));
            if between.contains('\n') || between.contains("//") || between.contains("/*") {
                emit_trivia_at(f, between, false, f.depth);
            } else if no_space {
                // drop
            } else if needs_space {
                f.push_static(" ");
            } else if between.is_empty() {
                // emit nothing
            } else {
                f.push_static(" ");
            }
        }
        f.push_text(t.text.to_owned());
        prev_text = Some(t.text);
        prev_end = t.end();
    }
}
