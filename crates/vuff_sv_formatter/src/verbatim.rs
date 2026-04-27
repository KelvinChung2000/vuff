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

use crate::attribute::{find_attribute_spans, force_nl_before_mask};
use crate::context::{FormatCtx, Formatter};
use crate::expr::{
    apostrophe_brace_mask, call_open_paren_mask, concat_brace_masks, select_open_bracket_mask,
    streaming_concat_mask, ternary_colon_mask,
};
use crate::indent_map::cst_depth_map;
use crate::list::{
    collect_inst_port_lists, collect_param_port_lists, collect_port_lists,
    force_space_before_instance_paren_mask, force_space_before_port_paren_mask,
    param_assign_pound_mask, render_wrapped, wrap_delimiter_masks,
};
use crate::stmt::control_header_paren_mask;
use crate::stmt::seq_block::wants_allman_break;
use crate::stmt::{statement_boundary_mask, statement_reset_mask};
use crate::tokens::delimiters::{is_closer, is_opener};
use crate::tokens::spacing::{force_space_between, no_space_between};
use crate::tokens::trivia::emit_trivia_at;

fn emit_directives_around(f: &mut Formatter, _between: &str, dirs: &[&str], tail_depth: u32) {
    // Directives always live on their own line at column 0. If prior
    // emission already ended in a hardline (e.g. inter-description trivia
    // from the root rule), don't add another — it would create a stray
    // blank line.
    if !matches!(
        f.out.last(),
        Some(FormatElement::HardLine | FormatElement::EmptyLine)
    ) {
        f.push_hardline();
    }
    for (i, dir) in dirs.iter().enumerate() {
        f.push_text((*dir).to_owned());
        if i + 1 < dirs.len() {
            f.push_hardline();
        }
    }
    f.push_hardline();
    let saved = f.depth;
    f.depth = tail_depth;
    f.push_indent_for_new_line();
    f.depth = saved;
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

    // CST-driven attribute span detection → force_nl_before mask.
    let attr_spans = find_attribute_spans(ctx.tree, src, ctx.tokens);
    let force_nl_before = force_nl_before_mask(&attr_spans, ctx.tokens.len());
    let port_paren = force_space_before_port_paren_mask(ctx.tree, ctx.tokens);
    let instance_paren = force_space_before_instance_paren_mask(ctx.tree, ctx.tokens);
    let param_pound = param_assign_pound_mask(ctx.tree, ctx.tokens);
    let is_ternary_colon = ternary_colon_mask(ctx.tree, ctx.tokens);
    let (concat_open, concat_close, concat_before_open) = concat_brace_masks(ctx.tree, ctx.tokens);
    let apostrophe_brace = apostrophe_brace_mask(ctx.tree, ctx.tokens);
    let control_paren = control_header_paren_mask(ctx.tree, ctx.tokens);
    let select_bracket = select_open_bracket_mask(ctx.tree, ctx.tokens);
    let call_paren = call_open_paren_mask(ctx.tree, ctx.tokens);
    let in_streaming = streaming_concat_mask(ctx.tree, ctx.tokens);
    let is_stmt_boundary = statement_boundary_mask(ctx.tree, ctx.tokens);
    let is_stmt_reset = statement_reset_mask(ctx.tree, ctx.tokens);
    let cst_depth = cst_depth_map(ctx.tree, ctx.tokens);
    let inst_port_lists = collect_inst_port_lists(ctx.tree, ctx.tokens, ctx.source);
    // Map from `(` token index → the inst-port list it opens. Used to splice
    // a wrapped renderer in mid-stream when the user has inserted a newline
    // inside the `(...)` (the canonical "wrap me" signal).
    let inst_open_to_list: std::collections::HashMap<
        usize,
        &crate::list::inst_port_list::InstPortList,
    > = inst_port_lists.iter().map(|l| (l.paren_open, l)).collect();

    // Generic newline-trigger wrap for any other delimited group. Excludes
    // openers already owned by the inst/param/port-list renderers so their
    // dedicated logic owns the canonical shape.
    let mut excluded_openers: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for l in &inst_port_lists {
        excluded_openers.insert(l.paren_open);
    }
    for l in collect_param_port_lists(ctx.tree, ctx.tokens, ctx.source) {
        excluded_openers.insert(l.paren_open);
    }
    for l in collect_port_lists(ctx.tree, ctx.tokens, ctx.source) {
        excluded_openers.insert(l.paren_open);
    }
    let (wrap_open, wrap_close) = wrap_delimiter_masks(ctx.tokens, ctx.source, &excluded_openers);

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

        // If stripped directive lines anchor to this token, replace the
        // normal trivia emission with: source newline(s) → directive
        // lines at column 0 → indent for the upcoming token.
        let dirs: Vec<&str> = ctx
            .directive_anchors
            .iter()
            .filter(|a| a.anchor_tok == global_i)
            .map(|a| a.text.as_str())
            .collect();

        if !dirs.is_empty() {
            emit_directives_around(f, between, &dirs, curr_depth);
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
