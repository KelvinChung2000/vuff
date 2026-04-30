//! Annex A.1.2 — `source_text`. The synthetic root rule: walk top-level
//! descriptions, dispatching each to its node rule (`ModuleDeclarationRule`
//! for modules) or to `verbatim` for kinds that don't have a rule yet.
//! Then emit any BOF / EOF trivia around them.

use crate::context::{FormatCtx, Formatter};
use crate::format_ext::Format;
use crate::module::{find_module_spans, ModuleDeclarationRule, ModuleSpan};
use crate::tokens::trivia::{emit_trivia_slice, ensure_trailing_newline, SliceMode};
use crate::verbatim::format_token_range;

pub(crate) struct SourceTextRoot;

impl Format for SourceTextRoot {
    fn fmt(&self, ctx: &FormatCtx<'_>, f: &mut Formatter) {
        let n = ctx.tokens.len();
        if n == 0 {
            // Empty / trivia-only file — emit slot 0 (which spans the
            // whole file) at depth 0.
            emit_trivia_slice(
                f,
                &ctx.trivia.slices[0],
                0,
                0,
                SliceMode::Standalone {
                    is_leading: true,
                    tail_depth: 0,
                },
            );
            ensure_trailing_newline(&mut f.out);
            return;
        }

        let spans = find_module_spans(ctx.tree, ctx.tokens);
        dispatch(ctx, f, &spans, n);

        // File-trailing trivia (slot tokens.len()).
        emit_trivia_slice(
            f,
            &ctx.trivia.slices[n],
            0,
            0,
            SliceMode::Standalone {
                is_leading: false,
                tail_depth: 0,
            },
        );

        ensure_trailing_newline(&mut f.out);
    }
}

fn dispatch(ctx: &FormatCtx<'_>, f: &mut Formatter, spans: &[ModuleSpan], n: usize) {
    let mut cursor_tok: usize = 0;
    let mut cursor_byte: usize = 0;

    for span in spans {
        if span.start > cursor_tok {
            format_token_range(ctx, f, cursor_tok..span.start, cursor_byte);
        }
        f.in_statement = false;
        // Emit the trivia slot that sits *before* this module's first
        // token — comments, blank lines, surviving directives in the
        // inter-module gap. Doing it here (rather than letting the
        // verbatim engine pick it up on the first token) gives the
        // module rule a clean fresh-line baseline before any prepend
        // (e.g. `\`default_nettype none` injection) runs.
        let module_first_offset = ctx.tokens[span.start].offset;
        if cursor_byte < module_first_offset {
            emit_trivia_slice(
                f,
                &ctx.trivia.slices[span.start],
                0,
                0,
                SliceMode::Standalone {
                    is_leading: f.out.is_empty(),
                    tail_depth: 0,
                },
            );
        }
        // Tell the module rule the trivia is already emitted by
        // pointing `leading_from` at the module's first token offset.
        // The verbatim engine's trivia branch is gated on
        // `cursor < t.offset` so it won't double-emit.
        ModuleDeclarationRule {
            span: *span,
            leading_from: module_first_offset,
        }
        .fmt(ctx, f);
        cursor_tok = span.end + 1;
        cursor_byte = ctx.tokens[span.end].end();
    }

    if cursor_tok < n {
        format_token_range(ctx, f, cursor_tok..n, cursor_byte);
    }
}
