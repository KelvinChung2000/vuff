//! Annex A.1.3 — `parameter_port_list`. Walk the CST and collect each
//! `ParameterPortList` (the `#( … )` after a module/interface name) along
//! with the comma-separated rows inside it. The renderer here produces a
//! one-per-line layout when the list has at least one parameter, mirroring
//! the port-list renderer's conservative wrap policy.
//!
//! v0.1 scope: no column alignment across rows. Each row is emitted as a
//! single token sequence (verbatim spacing) at body depth.

use vuff_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

use crate::context::{build_token_index, FormatCtx, Formatter};
use crate::tokens::trivia::{emit_trivia_at, emit_trivia_slice, ensure_fresh_line, SliceMode};

#[derive(Debug)]
pub(crate) struct ParamPortRow {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) comma_tok: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct ParamPortList {
    pub(crate) pound_tok: usize,
    pub(crate) paren_open: usize,
    pub(crate) paren_close: usize,
    pub(crate) rows: Vec<ParamPortRow>,
    /// True when the source between `paren_open` and `paren_close` contains
    /// a `\n`. Drives the wrap trigger.
    pub(crate) has_internal_newline: bool,
}

pub(crate) fn collect_param_port_lists(
    tree: &SyntaxTree,
    tokens: &[Token<'_>],
    source: &str,
) -> Vec<ParamPortList> {
    let mut out: Vec<ParamPortList> = Vec::new();
    let mut cur: Option<ParamPortList> = None;
    let mut in_list: u32 = 0;
    let mut in_decl: u32 = 0;
    let mut decl_start: Option<usize> = None;
    let mut decl_end: Option<usize> = None;
    let tok_idx = build_token_index(tokens);

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(RefNode::ParameterPortList(_)) => {
                in_list += 1;
                if in_list == 1 {
                    cur = Some(ParamPortList {
                        pound_tok: usize::MAX,
                        paren_open: usize::MAX,
                        paren_close: usize::MAX,
                        rows: Vec::new(),
                        has_internal_newline: false,
                    });
                }
            }
            NodeEvent::Leave(RefNode::ParameterPortList(_)) => {
                in_list = in_list.saturating_sub(1);
                if in_list == 0 {
                    if let Some(mut list) = cur.take() {
                        if list.paren_open != usize::MAX
                            && list.paren_close != usize::MAX
                            && !list.rows.is_empty()
                        {
                            let from = tokens[list.paren_open].end();
                            let to = tokens[list.paren_close].offset;
                            list.has_internal_newline = source[from..to].contains('\n');
                            out.push(list);
                        }
                    }
                }
            }
            NodeEvent::Enter(
                RefNode::ParameterPortDeclaration(_)
                | RefNode::ParameterDeclaration(_)
                | RefNode::LocalParameterDeclaration(_),
            ) if in_list > 0 => {
                if in_decl == 0 {
                    decl_start = None;
                    decl_end = None;
                }
                in_decl += 1;
            }
            NodeEvent::Leave(
                RefNode::ParameterPortDeclaration(_)
                | RefNode::ParameterDeclaration(_)
                | RefNode::LocalParameterDeclaration(_),
            ) if in_list > 0 => {
                in_decl = in_decl.saturating_sub(1);
                if in_decl == 0 {
                    if let (Some(s), Some(e)) = (decl_start, decl_end) {
                        if let Some(list) = cur.as_mut() {
                            list.rows.push(ParamPortRow {
                                start: s,
                                end: e,
                                comma_tok: None,
                            });
                        }
                    }
                }
            }
            NodeEvent::Enter(RefNode::Locate(loc)) if in_list > 0 => {
                let Some(&idx) = tok_idx.get(&loc.offset) else {
                    continue;
                };
                let text = tokens[idx].text;
                if in_decl > 0 {
                    if decl_start.is_none() {
                        decl_start = Some(idx);
                    }
                    decl_end = Some(idx);
                    continue;
                }
                if in_list == 1 {
                    if let Some(list) = cur.as_mut() {
                        if text == "#" && list.pound_tok == usize::MAX {
                            list.pound_tok = idx;
                        } else if text == "(" && list.paren_open == usize::MAX {
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

/// Render `#( row, row, … )` with each row on its own line at body depth.
/// The leading `#` is emitted by the caller's verbatim pass; `render` here
/// owns the `(` opener through the `)` closer.
pub(crate) fn render_param_port_list(ctx: &FormatCtx<'_>, f: &mut Formatter, list: &ParamPortList) {
    f.push_text("(".to_owned());

    let row_depth = f.depth + 1;
    let close_depth = f.depth;

    for (i, row) in list.rows.iter().enumerate() {
        emit_trivia_slice(f, &ctx.trivia.slices[row.start], row_depth, 0, SliceMode::Embedded);

        ensure_fresh_line(f);
        f.push_indent_levels(row_depth);

        let saved_depth = f.depth;
        f.depth = row_depth;
        emit_row_tokens(ctx, f, row);
        f.depth = saved_depth;

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


fn emit_row_tokens(ctx: &FormatCtx<'_>, f: &mut Formatter, row: &ParamPortRow) {
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
