//! Annex A.1.3 / A.1.4 — `module_declaration`. Owns the header → body
//! → footer shape for one `module … endmodule` block. Body depth is
//! bumped locally around the body emission, so no global
//! `pending_body_bumps` state is needed.

use vuff_sv_ast::Token;

use crate::context::{FormatCtx, Formatter};
use crate::format_ext::Format;
use crate::list::{render_param_port_list, render_port_list};
use crate::module::spans::ModuleSpan;
use crate::verbatim::format_token_range;

pub(crate) struct ModuleDeclarationRule {
    pub(crate) span: ModuleSpan,
    /// Byte offset where the emitter's cursor sits when this rule starts.
    /// Any trivia between `leading_from` and the module's first token
    /// (blank lines, comments between top-level declarations) is emitted
    /// via the header's leading trivia.
    pub(crate) leading_from: usize,
}

impl Format for ModuleDeclarationRule {
    fn fmt(&self, ctx: &FormatCtx<'_>, f: &mut Formatter) {
        let start = self.span.start;
        let end = self.span.end;

        // A preceding top-level compiler directive (e.g. a stray
        // `` `default_nettype `` above this module) leaves `in_statement`
        // set by the verbatim engine — directives aren't statements, so
        // clear it before emitting the module.
        f.in_statement = false;

        let wrap = ctx.opts.wrap_default_nettype;
        let prepend = wrap && !has_preceding_directive(ctx.tokens, start, "none");
        // sv-parser attaches a trailing `` `default_nettype … `` directive
        // as whitespace on the `endmodule` keyword, which pulls its three
        // tokens INSIDE the module span. So we check the tail of the span
        // itself, not the tokens past `end`.
        let append = wrap && !has_trailing_directive_in_span(ctx.tokens, end, "wire");

        if prepend {
            f.push_text("`default_nettype none".to_owned());
            f.push_hardline();
            f.push_indent_for_new_line();
        }

        // If this module has a parameter port list `#( … )` and/or an
        // ANSI port list `( … )`, hand each off to its own renderer. We
        // splice the verbatim emission around them so the headers / tails
        // outside the lists still go through the standard token engine.
        // Wrap renderers fire only when the human inserted a newline inside
        // the `(...)` — otherwise the verbatim engine handles inline emission.
        let param_list = ctx
            .masks
            .param_port_lists
            .iter()
            .find(|pl| pl.pound_tok >= start && pl.paren_close <= end && pl.has_internal_newline);
        let port_list = ctx
            .masks
            .port_lists
            .iter()
            .find(|pl| pl.paren_open >= start && pl.paren_close <= end && pl.has_internal_newline);

        let mut cursor = start;
        let mut leading_from = self.leading_from;

        if let Some(ppl) = &param_list {
            // Emit header up through the `#` (verbatim handles `name #`
            // spacing via the param-pound mask), then the rendered (…)
            // block. The renderer owns `(` through `)`.
            format_token_range(ctx, f, cursor..ppl.pound_tok + 1, leading_from);
            render_param_port_list(ctx, f, ppl);
            cursor = ppl.paren_close + 1;
            leading_from = ctx.tokens[ppl.paren_close].end();
        }

        if let Some(pl) = &port_list {
            format_token_range(ctx, f, cursor..pl.paren_open, leading_from);
            render_port_list(ctx, f, pl);
            cursor = pl.paren_close + 1;
            leading_from = ctx.tokens[pl.paren_close].end();
        }

        format_token_range(ctx, f, cursor..end + 1, leading_from);

        if append {
            f.push_hardline();
            f.push_indent_for_new_line();
            f.push_text("`default_nettype wire".to_owned());
        }
    }
}

/// True when the three tokens immediately preceding index `start` are
/// `` `default_nettype <name> ``.
fn has_preceding_directive(tokens: &[Token<'_>], start: usize, name: &str) -> bool {
    if start < 3 {
        return false;
    }
    matches_directive(&tokens[start - 3..start], name)
}

/// True when the last three tokens of a module span (ending at inclusive
/// index `end`) are `` `default_nettype <name> ``.
fn has_trailing_directive_in_span(tokens: &[Token<'_>], end: usize, name: &str) -> bool {
    if end < 2 {
        return false;
    }
    matches_directive(&tokens[end - 2..=end], name)
}

fn matches_directive(slice: &[Token<'_>], name: &str) -> bool {
    slice.len() == 3
        && slice[0].text == "`"
        && slice[1].text == "default_nettype"
        && slice[2].text == name
}
