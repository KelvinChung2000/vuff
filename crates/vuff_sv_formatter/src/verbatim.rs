//! Token-range passthrough. Emits IR for a contiguous slice of the token
//! stream using the v0.1 token-aware re-indenter. Any CST node kind that
//! does not yet have a dedicated rule delegates its subtree to
//! [`format_token_range`] so the output is still produced — just without
//! shape-aware decisions.
//!
//! The caller supplies `leading_from` (a byte offset in `ctx.source`)
//! which fixes where the emitter's cursor starts. This lets rules chain
//! sub-ranges while preserving inter-token trivia at the boundary:
//!
//! * First call in a file: `leading_from = ctx.tokens[0].offset` (the
//!   file's leading trivia is emitted separately by `SourceTextRoot`).
//! * Chained call after another range ended at token index `k`:
//!   `leading_from = ctx.tokens[k].end()`.

use std::ops::Range;

use vuff_formatter::FormatElement;

use crate::attribute::force_nl_before_mask;
use crate::context::{FormatCtx, Formatter};
use crate::list::render_wrapped;
use crate::stmt::seq_block::wants_allman_break;
use crate::directives::DirectiveAnchor;
use crate::tokens::delimiters::{is_closer, is_opener};
use crate::tokens::spacing::{force_space_between, no_space_between};
use crate::tokens::trivia::{block_state_after, emit_trivia_at};

/// Emit a span of inter-token trivia that contains stripped preprocessor
/// directives. We can't just dump the directive lines and ignore the
/// `between` slice — that would drop comments and active `\`define` lines
/// that the preprocessor preserved in `parsed.text`. Instead, walk both
/// inputs (the directive list and `between`'s non-blank lines) tagged
/// with their byte offset in `parsed.original`, sort by that offset, and
/// emit in original-source order.
///
/// `body_depth` is the indent for re-emitted comment / `\`define` lines;
/// directive keywords always print at column 0. `tail_depth` is the
/// indent used for the upcoming CST token.
pub(crate) fn emit_directives_around(
    ctx: &FormatCtx<'_>,
    f: &mut Formatter,
    between: &str,
    between_offset: usize,
    dirs: &[&DirectiveAnchor],
    body_depth: u32,
    tail_depth: u32,
) {
    enum Kind {
        Directive,
        Content,
        Blank,
    }
    let mut events: Vec<(usize, usize, Kind, String)> = Vec::new();

    for d in dirs {
        events.push((d.orig_start, 0, Kind::Directive, d.text.clone()));
    }

    // Walk `between` (post-pp) line by line. Non-blank lines are emitted
    // at `body_depth`; runs of blank lines collapse to at most one.
    let mut pp_pos = between_offset;
    let mut in_block = false;
    let mut last_content_orig: Option<usize> = None;
    let mut pending_blank = false;
    let mut idx_seq: usize = 0;
    for raw in between.split_inclusive('\n') {
        let line_pp_start = pp_pos;
        pp_pos += raw.len();
        let line = raw
            .trim_end_matches('\n')
            .trim_end_matches([' ', '\t']);
        let trimmed = line.trim_start_matches([' ', '\t']);
        let was_in_block = in_block;
        in_block = block_state_after(in_block, line);

        if trimmed.is_empty() {
            if last_content_orig.is_some() {
                pending_blank = true;
            }
            continue;
        }

        let leading_ws = line.bytes().take_while(|&b| b == b' ' || b == b'\t').count();
        // If origin lookup fails (the line came from a macro expansion or
        // an `\`include`), fall back to "after every directive in this
        // batch" — better to land at the end than at column 0 ahead of
        // unrelated directives.
        let orig = ctx
            .parsed
            .origin_in_original(line_pp_start + leading_ws)
            .unwrap_or(usize::MAX);
        let text = if was_in_block {
            line.to_owned()
        } else {
            trimmed.to_owned()
        };
        if pending_blank {
            idx_seq += 1;
            events.push((orig.saturating_sub(1), idx_seq, Kind::Blank, String::new()));
            pending_blank = false;
        }
        idx_seq += 1;
        events.push((orig, idx_seq, Kind::Content, text));
        last_content_orig = Some(orig);
    }

    // Sort by orig pos with `idx_seq` as tie-breaker so equal-pos events
    // keep the order they were inserted (directives before content when a
    // directive's recorded orig_start coincides with a content line).
    events.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    // Make sure we start on a fresh line — but only if anything has
    // been emitted yet. At file start the buffer is empty and adding a
    // hardline would inject a stray leading blank line.
    if !f.out.is_empty()
        && !matches!(
            f.out.last(),
            Some(FormatElement::HardLine | FormatElement::EmptyLine)
        )
    {
        f.push_hardline();
    }

    let saved_depth = f.depth;
    let saved_in_stmt = f.in_statement;
    // Comments / `\`define` lines stand outside any in-progress statement;
    // suppress continuation-indent for the entire emitted block,
    // including the trailing indent for the upcoming token. The
    // upcoming token starts on its own line at `tail_depth` — adding
    // continuation indent here would over-shift it.
    f.in_statement = false;
    let mut emitted = false;
    let mut blank_pending = false;
    for (_, _, kind, text) in events {
        match kind {
            Kind::Blank => {
                if emitted {
                    blank_pending = true;
                }
            }
            Kind::Directive => {
                if emitted {
                    f.push_hardline();
                    if blank_pending {
                        f.push_hardline();
                    }
                }
                blank_pending = false;
                f.push_text(text);
                emitted = true;
            }
            Kind::Content => {
                if emitted {
                    f.push_hardline();
                    if blank_pending {
                        f.push_hardline();
                    }
                }
                blank_pending = false;
                f.depth = body_depth;
                f.push_indent_for_new_line();
                f.push_text(text);
                emitted = true;
            }
        }
    }

    if emitted {
        f.push_hardline();
    }
    f.depth = tail_depth;
    f.push_indent_for_new_line();
    f.depth = saved_depth;
    f.in_statement = saved_in_stmt;
}

// This function is intentionally long: it's the transitional engine
// covering every CST shape that doesn't yet have a dedicated rule. Each
// future rule migration deletes a branch. Allowed while that migration
// is still ongoing — see `docs/spec-tracker.md`.
#[allow(clippy::too_many_lines)]
pub(crate) fn format_token_range(
    ctx: &FormatCtx<'_>,
    f: &mut Formatter,
    range: Range<usize>,
    leading_from: usize,
) {
    let toks = &ctx.tokens[range.clone()];
    if toks.is_empty() {
        return;
    }
    let src = ctx.source;
    let opts = ctx.opts;

    // All CST-driven masks were built once in `format_source` and live in
    // `ctx.masks`. Bind locals so the rest of the body reads naturally.
    let m = ctx.masks;
    let force_nl_before = force_nl_before_mask(&m.attr_spans, ctx.tokens.len());
    let port_paren = m.port_paren.as_slice();
    let instance_paren = m.instance_paren.as_slice();
    let param_pound = m.param_pound.as_slice();
    let is_ternary_colon = m.is_ternary_colon.as_slice();
    let concat_open = m.concat_open.as_slice();
    let concat_close = m.concat_close.as_slice();
    let concat_before_open = m.concat_before_open.as_slice();
    let apostrophe_brace = m.apostrophe_brace.as_slice();
    let control_paren = m.control_paren.as_slice();
    let select_bracket = m.select_bracket.as_slice();
    let call_paren = m.call_paren.as_slice();
    let in_streaming = m.in_streaming.as_slice();
    let is_stmt_boundary = m.is_stmt_boundary.as_slice();
    let is_stmt_reset = m.is_stmt_reset.as_slice();
    let cst_depth = m.cst_depth.as_slice();
    let inst_port_lists = &m.inst_port_lists;
    // Map from `(` token index → the inst-port list it opens. Used to splice
    // a wrapped renderer in mid-stream when the user has inserted a newline
    // inside the `(...)` (the canonical "wrap me" signal).
    let inst_open_to_list: std::collections::HashMap<
        usize,
        &crate::list::inst_port_list::InstPortList,
    > = inst_port_lists.iter().map(|l| (l.paren_open, l)).collect();
    let wrap_open = m.wrap_open.as_slice();
    let wrap_close = m.wrap_close.as_slice();

    let first_global_idx = range.start;
    let mut cursor: usize = leading_from;

    let mut prev_text: Option<&str> = None;
    let mut prev_was_ternary_colon = false;
    let mut prev_was_param_pound = false;
    let mut prev_was_concat_open = false;
    let mut prev_was_apostrophe_brace = false;
    // Generic newline-triggered wrap state. `wrap_depth` adds an extra
    // indent level for tokens inside a wrapped delimited group;
    // `prev_was_wrap_open` forces a hardline + indent immediately after the
    // opener regardless of source trivia.
    let mut wrap_depth: u32 = 0;
    let mut prev_was_wrap_open = false;
    let mut stmt_stack: Vec<bool> = Vec::new();
    let mut bracket_depth: u32 = 0;
    // Token-level block depth: `begin`/`end`, `case`/`endcase`, `fork`/`join*`,
    // brackets. CST depth (from module items and implicit bodies) adds on
    // top of this.
    let mut token_depth: u32 = 0;
    // Block-label continuation: 2 = expect `:` after block keyword,
    // 1 = expect label identifier after `:`, 0 = not in a label.
    let mut label_pending: u8 = 0;
    // Depth at which the previous token emitted. Comments in the trivia
    // between a statement and a *dedenting* closing keyword belong to
    // the just-closed block, so we indent them at `prev_depth`, not the
    // closer's (smaller) depth.
    let mut prev_depth: u32 = 0;

    for (local_i, t) in toks.iter().enumerate() {
        let global_i = first_global_idx + local_i;
        if t.offset < cursor {
            continue;
        }
        let between = &src[cursor..t.offset];

        // A closer dedents immediately so the token itself prints at the
        // outer depth (e.g., `end` sits at the begin's parent level).
        // Skip when this closer is owned by a newline-triggered wrap pair —
        // the dedent is then handled by `wrap_depth` instead, avoiding a
        // double pop.
        if is_closer(t.text) && !wrap_close[global_i] {
            token_depth = token_depth.saturating_sub(1);
        }
        // Generic wrap closer (`)`, `}`, `]` of a newline-bearing pair):
        // pop the wrap depth so the closer itself prints at outer level.
        if wrap_close[global_i] {
            wrap_depth = wrap_depth.saturating_sub(1);
        }

        // Pre-emit structural depth: CST-computed base + current token-level
        // block depth + any active newline-triggered wrap depth.
        let curr_depth = cst_depth[global_i] + token_depth + wrap_depth;
        f.depth = curr_depth;
        // Indent inter-token trivia (comments, blank lines) at the max of
        // the previous and current token depths. A comment sitting on the
        // line above `end` / `endmodule` belongs to the outgoing block,
        // not to the dedented closer.
        let trivia_depth = prev_depth.max(curr_depth);

        // Pre-emit: clear `in_statement` when either the upcoming token
        // is a statement terminator (`;`, `end`, `endcase`, …) or it's
        // the CST-declared first token of a new Statement / CaseItem.
        if is_stmt_reset[global_i] || is_stmt_boundary[global_i] {
            f.in_statement = false;
        }

        let has_newline = between.contains('\n');
        let has_comment = between.contains("//") || between.contains("/*");
        let mut forbids_space = prev_text.is_some_and(|p| no_space_between(p, t.text));
        let mut needs_space = prev_text.is_some_and(|p| force_space_between(p, t.text));
        if is_ternary_colon[global_i] || prev_was_ternary_colon {
            needs_space = true;
        }
        if port_paren[global_i] || instance_paren[global_i] || control_paren[global_i] {
            needs_space = true;
        }
        // ParameterValueAssignment: force space before `#`, then glue `#(`.
        if param_pound[global_i] {
            needs_space = true;
        }
        if prev_was_param_pound {
            forbids_space = true;
            needs_space = false;
        }
        // Concatenation / replication / assignment-pattern braces stay tight.
        if prev_was_concat_open || concat_close[global_i] || concat_before_open[global_i] {
            forbids_space = true;
            needs_space = false;
        }
        // `'{` opening of an assignment pattern — no space after it.
        if prev_was_apostrophe_brace {
            forbids_space = true;
            needs_space = false;
        }
        // Bit-select / part-select `[` stays glued to the identifier.
        if select_bracket[global_i] {
            forbids_space = true;
            needs_space = false;
        }
        // Subroutine-call `(` stays glued to the callee identifier.
        if call_paren[global_i] {
            forbids_space = true;
            needs_space = false;
        }
        // Streaming concatenation `{<<{ … }}` / `{>>{ … }}`. The `<<`/`>>`
        // direction marker and the optional slice-size sit at the
        // structural top of the streaming-concat node — they are NOT
        // shift / multiply operators, so the binary-op force-space rule
        // must not apply around them. Detect via the `in_streaming`
        // CST mask rather than string-matching the surrounding tokens.
        if in_streaming[global_i]
            && (matches!(t.text, "<<" | ">>")
                || prev_text.is_some_and(|p| matches!(p, "<<" | ">>")))
        {
            needs_space = false;
            forbids_space = true;
        }

        // If stripped directive lines anchor to this token, fold them
        // into the trivia emission: directives and any preserved
        // comment / `\`define` lines from `between` are interleaved by
        // their original-source position, so the file's source order
        // round-trips even when conditional directives were stripped.
        let dirs: Vec<&DirectiveAnchor> = ctx
            .directive_anchors
            .iter()
            .filter(|a| a.anchor_tok == global_i)
            .collect();

        if !dirs.is_empty() {
            // Use `curr_depth` (the upcoming token's depth) as the body
            // indent for re-emitted comments / active `\`define`s.
            // `trivia_depth` would carry over the previous token's deeper
            // depth, which over-indents top-level content sitting between
            // a closing punctuation and the next description.
            emit_directives_around(ctx, f, between, cursor, &dirs, curr_depth, curr_depth);
        } else if prev_was_wrap_open {
            // First token after a newline-triggered wrap opener: force a
            // hardline + indent at the new (deeper) wrap level, ignoring
            // the user's source spacing.
            f.push_hardline();
            f.push_indent_for_new_line();
        } else if wrap_close[global_i] {
            // The closer of a newline-triggered wrap: force a hardline +
            // indent at the outer (already-popped) depth before the closer
            // itself prints.
            f.push_hardline();
            f.push_indent_for_new_line();
        } else if force_nl_before[global_i] {
            f.push_hardline();
            f.push_indent_for_new_line();
        } else if has_newline || has_comment {
            emit_trivia_at(f, between, false, trivia_depth, curr_depth);
        } else if needs_space {
            f.push_static(" ");
        } else if forbids_space {
            // drop any whitespace between
        } else if between.is_empty() {
            // emit nothing
        } else {
            f.push_static(" ");
        }

        if wants_allman_break(opts, t.text, between, prev_text) {
            if let Some(FormatElement::StaticText(" ")) = f.out.last() {
                f.out.pop();
            }
            f.push_hardline();
            f.push_indent_structural();
        }

        // Module-instantiation port list: if the human inserted any newline
        // inside the `( … )`, splice the wrapped renderer; otherwise keep it
        // inline (no auto-wrap on width — humans signal wrap with newlines).
        if let Some(list) = inst_open_to_list.get(&global_i) {
            if list.has_internal_newline {
                let saved_depth = f.depth;
                f.depth = curr_depth;
                render_wrapped(ctx, f, list);
                f.depth = saved_depth;
                cursor = ctx.tokens[list.paren_close].end();
                prev_text = Some(")");
                prev_depth = curr_depth;
                prev_was_ternary_colon = false;
                prev_was_param_pound = false;
                prev_was_concat_open = false;
                prev_was_apostrophe_brace = false;
                continue;
            }
        }

        f.push_text(t.text.to_owned());

        // Increment token_depth on openers — except when this opener is the
        // start of a newline-triggered wrap pair, since `wrap_depth` already
        // owns the indent contribution for that group.
        if is_opener(t.text) && !wrap_open[global_i] {
            token_depth += 1;
        }

        cursor = t.end();
        prev_text = Some(t.text);
        prev_depth = curr_depth;
        prev_was_ternary_colon = is_ternary_colon[global_i];
        prev_was_param_pound = param_pound[global_i];
        prev_was_concat_open = concat_open[global_i];
        prev_was_apostrophe_brace = apostrophe_brace[global_i];
        prev_was_wrap_open = wrap_open[global_i];
        if wrap_open[global_i] {
            wrap_depth += 1;
        }

        if matches!(t.text, "(" | "[" | "{" | "(*") {
            // `(*` is the attribute opener; it groups like a bracket for
            // the purpose of suppressing statement-continuation indent.
            stmt_stack.push(f.in_statement);
            bracket_depth += 1;
            f.in_statement = false;
            label_pending = 0;
        } else if matches!(t.text, ")" | "]" | "}" | "*)") {
            bracket_depth = bracket_depth.saturating_sub(1);
            if let Some(saved) = stmt_stack.pop() {
                f.in_statement = saved;
            }
            label_pending = 0;
        } else if bracket_depth > 0 {
            f.in_statement = false;
            label_pending = 0;
        } else {
            // Consume block-label continuation (`: ident` after a block
            // keyword) without flipping `in_statement` back to true.
            let suppress_in_stmt = if label_pending == 2 && t.text == ":" {
                label_pending = 1;
                true
            } else if label_pending == 1 {
                label_pending = 0;
                true
            } else {
                label_pending = 0;
                false
            };
            f.in_statement = !(suppress_in_stmt || is_stmt_reset[global_i]);
            // Prime the label-pending countdown if the just-emitted token
            // is a block keyword that may take a trailing `: label`. The
            // mask flags both block openers AND `;`; `;` doesn't take a
            // label, so exclude it explicitly.
            if is_stmt_reset[global_i] && t.text != ";" {
                label_pending = 2;
            }
        }
    }
}
