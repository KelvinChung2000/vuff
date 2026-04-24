//! Annex A.1.2 — `source_text`. The synthetic root rule: walk top-level
//! descriptions, dispatching each to its node rule (`ModuleDeclarationRule`
//! for modules) or to `verbatim` for kinds that don't have a rule yet.
//! Then emit any BOF / EOF trivia around them.

use vuff_formatter::FormatElement;

use crate::context::{FormatCtx, Formatter};
use crate::format_ext::Format;
use crate::module::{find_module_spans, ModuleDeclarationRule, ModuleSpan};
use crate::tokens::trivia::{emit_trivia, ensure_trailing_newline};
use crate::verbatim::format_token_range;

pub(crate) struct SourceTextRoot;

impl Format for SourceTextRoot {
    fn fmt(&self, ctx: &FormatCtx<'_>, f: &mut Formatter) {
        let n = ctx.tokens.len();
        if n == 0 {
            // Empty / trivia-only file — possibly with preserved
            // directives (`ifdef-stripped source or just comments).
            emit_trivia(f, ctx.source, true);
            for anchor in ctx.directive_anchors {
                if !matches!(
                    f.out.last(),
                    Some(FormatElement::HardLine | FormatElement::EmptyLine)
                ) {
                    f.push_hardline();
                }
                f.push_text(anchor.text.clone());
                f.push_hardline();
            }
            ensure_trailing_newline(&mut f.out);
            return;
        }

        // File-leading trivia (comments, blank lines before the first token).
        emit_trivia(f, &ctx.source[..ctx.tokens[0].offset], true);

        let spans = find_module_spans(ctx.tree, ctx.tokens);
        dispatch(ctx, f, &spans, n);

        // File-trailing trivia past the last token.
        let last_end = ctx.tokens[n - 1].end();
        emit_trivia(f, &ctx.source[last_end..], false);

        // Preserved directives whose anchor is past the last token.
        let tail_dirs: Vec<&str> = ctx
            .directive_anchors
            .iter()
            .filter(|a| a.anchor_tok == n)
            .map(|a| a.text.as_str())
            .collect();
        if !tail_dirs.is_empty() {
            if !matches!(
                f.out.last(),
                Some(FormatElement::HardLine | FormatElement::EmptyLine)
            ) {
                f.push_hardline();
            }
            for dir in tail_dirs {
                f.push_text(dir.to_owned());
                f.push_hardline();
            }
        }

        ensure_trailing_newline(&mut f.out);
    }
}

fn dispatch(ctx: &FormatCtx<'_>, f: &mut Formatter, spans: &[ModuleSpan], n: usize) {
    let mut cursor_tok: usize = 0;
    let mut cursor_byte: usize = ctx.tokens[0].offset;

    for span in spans {
        let trivia_start = if span.start > cursor_tok {
            // Non-module tokens in the gap before this module: verbatim.
            format_token_range(ctx, f, cursor_tok..span.start, cursor_byte);
            ctx.tokens[span.start - 1].end()
        } else {
            cursor_byte
        };
        // Gap tokens (typically compiler directives) leave `in_statement`
        // set by the verbatim engine. At a description boundary that flag
        // is meaningless — clear it so the pre-module trivia's indent
        // doesn't get bumped an extra level.
        f.in_statement = false;
        // Emit the pre-module trivia (blank lines / free comments between
        // modules) *before* the module rule runs. This lets the rule's
        // optional directive prepend land on its own line without relying
        // on internal leading-trivia handoff.
        let first_offset = ctx.tokens[span.start].offset;
        emit_trivia(f, &ctx.source[trivia_start..first_offset], false);
        ModuleDeclarationRule {
            span: *span,
            leading_from: first_offset,
        }
        .fmt(ctx, f);
        cursor_tok = span.end + 1;
        cursor_byte = ctx.tokens[span.end].end();
    }

    if cursor_tok < n {
        // Trailing non-module descriptions.
        format_token_range(ctx, f, cursor_tok..n, cursor_byte);
    }
}
