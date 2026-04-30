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
use crate::tokens::delimiters::{is_closer, is_opener};
use crate::tokens::spacing::{force_space_between, no_space_between};
use crate::tokens::trivia::{emit_trivia_slice, SliceMode};

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
    let chain_depth = m.chain_depth.as_slice();
    let directive_start = m.directive_start.as_slice();
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
    // Per-chain anchor column for multi-line ternary `?` alignment.
    // `ternary_q_anchor` holds the column where the chain's leftmost
    // `?` landed; `ternary_start_col` holds the column where the
    // chain's leftmost token landed. Populated lazily as the CST
    // emission reaches each landmark.
    let mut ternary_q_anchor: std::collections::HashMap<usize, u32> =
        std::collections::HashMap::new();
    let mut ternary_start_col: std::collections::HashMap<usize, u32> =
        std::collections::HashMap::new();
    // Maps chain_id of a chain `:` we just emitted; the next token
    // pads to that chain's start column.
    let mut pending_chain_continuation: Option<usize> = None;

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
        // Macro call-site preservation: a token whose origin lies in a
        // `\`define` body is part of an expansion. We skip the
        // subsequent tokens of the run; the run's first token is
        // emitted as the original source's macro call (e.g.
        // `\`assert(cond)`) instead of the expanded text.
        if ctx.masks.macro_calls.skip_tok.contains(&global_i) {
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
        let curr_depth =
            cst_depth[global_i] + token_depth + wrap_depth + chain_depth[global_i];
        f.depth = curr_depth;
        // Indent inter-token trivia (comments, blank lines) at the max of
        // the previous and current token depths. A comment sitting on the
        // line above `end` / `endmodule` belongs to the outgoing block,
        // not to the dedented closer.
        let trivia_depth = prev_depth.max(curr_depth);

        // Pre-emit: clear `in_statement` when either the upcoming token
        // is a statement terminator (`;`, `end`, `endcase`, …) or it's
        // the CST-declared first token of a new Statement / CaseItem.
        // Surviving directive lines (`\`define`, `\`timescale`, …) also
        // start a fresh line — drop the carry-over so a chain of them
        // doesn't accumulate continuation-indent.
        if is_stmt_reset[global_i] || is_stmt_boundary[global_i] || directive_start[global_i] {
            f.in_statement = false;
        }

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
        let slice = &ctx.trivia.slices[global_i];
        // The slice is fully consumed by the caller when `cursor` has
        // already reached the upcoming token's offset. Special case
        // the file-leading slot: it can have classified segments
        // (stripped directives at BOF) even though its pp range is
        // empty — emit it as long as nothing has been written yet.
        let gap_has_bytes = cursor < t.offset || (global_i == 0 && f.out.is_empty());
        let slice_has_content =
            gap_has_bytes && (!slice.segments.is_empty() || slice.pp_newline_count > 0);

        if prev_was_wrap_open {
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
        } else if slice_has_content {
            // `body_depth = trivia_depth` so a comment that sits
            // above a dedenting closer (e.g. `end`) stays at the
            // outgoing block's indent. Directive / skipped-body /
            // empty-call segments ignore `body_depth` and emit at
            // their original column via per-segment `leading_ws`.
            // is_leading is only true for the first token of the file
            // when the buffer is still empty.
            let is_leading = global_i == 0 && f.out.is_empty();
            // Chain bump from neighbors that's already baked into
            // `trivia_depth` — directive segments subtract this so
            // they sit at their own scope, not their neighbor's.
            let prev_chain = if global_i > 0 {
                chain_depth[global_i - 1]
            } else {
                0
            };
            let chain_floor = prev_chain.max(chain_depth[global_i]);
            emit_trivia_slice(
                f,
                slice,
                trivia_depth,
                chain_floor,
                SliceMode::Standalone {
                    is_leading,
                    tail_depth: curr_depth,
                },
            );
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
                f.col = f.col.saturating_sub(1);
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

        // Multi-line ternary chain alignment.
        //
        // (1) Record the column where the chain begins (first token of
        //     the chain root CE). Continuations align to this column.
        // (2) When a chain `:` was just emitted and the next token
        //     starts a new line, pad up to the chain's start column.
        // (3) When this token is a `?` in a multi-line chain, pad up
        //     to the chain's `?` anchor column.
        if let Some(&chain_id) = ctx.masks.ternary_chains.first_tok.get(&global_i) {
            // Snapshot the chain start column AND seed the chain's `?`
            // anchor as `start + max_cond_width + 1` (the `+1` covers
            // the space the binary-op spacing rule emits before `?`).
            ternary_start_col.insert(chain_id, f.col);
            if let Some(&max_w) = ctx.masks.ternary_chains.max_cond_width.get(&chain_id) {
                ternary_q_anchor.insert(chain_id, f.col + max_w + 1);
            }
        }
        if let Some(chain_id) = pending_chain_continuation.take() {
            if let Some(&start) = ternary_start_col.get(&chain_id) {
                if start > f.col {
                    let pad = (start - f.col) as usize;
                    f.push_text(" ".repeat(pad));
                }
            }
        }
        if t.text == "?" {
            if let Some(&(chain_id, _pos)) = ctx.masks.ternary_chains.by_q_tok.get(&global_i) {
                if let Some(&anchor) = ternary_q_anchor.get(&chain_id) {
                    if anchor > f.col {
                        let pad = (anchor - f.col) as usize;
                        f.push_text(" ".repeat(pad));
                    }
                }
            }
        }

        // Macro run start: emit the original call-site text and jump
        // the cursor past the expansion. Subsequent tokens in the run
        // are filtered out at the top of this loop.
        if let Some(run) = ctx.masks.macro_calls.run_at_start.get(&global_i) {
            f.push_text(run.call_text.clone());
            cursor = ctx.tokens[run.end].end();
            prev_text = Some(t.text);
            prev_depth = curr_depth;
            prev_was_ternary_colon = false;
            prev_was_param_pound = false;
            prev_was_concat_open = false;
            prev_was_apostrophe_brace = false;
            continue;
        }

        f.push_text(t.text.to_owned());

        // Mark chain colons so the next token can align as a
        // continuation if it starts on a new line.
        if t.text == ":" {
            if let Some(&chain_id) = ctx.masks.ternary_chains.by_colon_tok.get(&global_i) {
                pending_chain_continuation = Some(chain_id);
            }
        }

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
