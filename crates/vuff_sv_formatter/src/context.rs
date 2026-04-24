//! Two pieces of formatter state, kept deliberately separate:
//!
//! * [`FormatCtx`] — immutable bundle every rule reads: options, the
//!   preprocessed source bytes, the flat token list, and the CST. Shared
//!   by every node rule; never mutated during emission.
//! * [`Formatter`] — mutable buffer + indent state that a rule writes into.
//!   Owns the IR output and the depth / statement-continuation machine.
//!
//! This split mirrors ruff's `FormatContext` + `Formatter` separation.

use std::borrow::Cow;

use vuff_config::{FormatOptions, IndentStyle};
use vuff_formatter::{FormatElement, PrintOptions};
use vuff_sv_ast::{SyntaxTree, Token};

use crate::directives::DirectiveAnchors;

/// Read-only state shared by every formatter rule. Cheap to pass by
/// reference; does not own any of its pointees.
pub(crate) struct FormatCtx<'a> {
    pub(crate) opts: &'a FormatOptions,
    pub(crate) source: &'a str,
    pub(crate) tokens: &'a [Token<'a>],
    #[allow(dead_code)] // wired for future per-node dispatch (step 3+)
    pub(crate) tree: &'a SyntaxTree,
    pub(crate) directive_anchors: &'a DirectiveAnchors,
}

impl<'a> FormatCtx<'a> {
    pub(crate) fn new(
        opts: &'a FormatOptions,
        source: &'a str,
        tokens: &'a [Token<'a>],
        tree: &'a SyntaxTree,
        directive_anchors: &'a DirectiveAnchors,
    ) -> Self {
        Self {
            opts,
            source,
            tokens,
            tree,
            directive_anchors,
        }
    }
}

/// Mutable emission state. Rules call `push_*` to write into `out`.
pub(crate) struct Formatter {
    pub(crate) indent_text: String,
    pub(crate) depth: u32,
    /// True while inside an un-terminated statement — bumps continuation
    /// indentation by one level until the terminator (`;`, `begin`, …).
    pub(crate) in_statement: bool,
    pub(crate) out: Vec<FormatElement>,
}

impl Formatter {
    pub(crate) fn new(opts: &FormatOptions, expected_capacity: usize) -> Self {
        Self {
            indent_text: indent_unit(opts),
            depth: 0,
            in_statement: false,
            out: Vec::with_capacity(expected_capacity),
        }
    }

    pub(crate) fn push_text(&mut self, s: impl Into<String>) {
        self.out.push(FormatElement::Text(Cow::Owned(s.into())));
    }

    pub(crate) fn push_static(&mut self, s: &'static str) {
        self.out.push(FormatElement::StaticText(s));
    }

    pub(crate) fn push_hardline(&mut self) {
        self.out.push(FormatElement::HardLine);
    }

    pub(crate) fn push_indent_for_new_line(&mut self) {
        let effective = self.depth + u32::from(self.in_statement);
        self.push_indent_levels(effective);
    }

    /// Indent at the structural depth only — used for tokens that start a
    /// block (Allman `begin`, block-close keywords) so continuation-indent
    /// does not push them a level too deep.
    pub(crate) fn push_indent_structural(&mut self) {
        let d = self.depth;
        self.push_indent_levels(d);
    }

    fn push_indent_levels(&mut self, levels: u32) {
        if levels > 0 {
            let mut buf = String::with_capacity(self.indent_text.len() * levels as usize);
            for _ in 0..levels {
                buf.push_str(&self.indent_text);
            }
            self.push_text(buf);
        }
    }
}

pub(crate) fn indent_unit(opts: &FormatOptions) -> String {
    match opts.indent_style {
        IndentStyle::Tabs => "\t".to_owned(),
        IndentStyle::Spaces => " ".repeat(usize::from(opts.indent_width)),
    }
}

pub(crate) fn print_options_from(opts: &FormatOptions) -> PrintOptions {
    PrintOptions {
        line_width: opts.line_width,
        indent_width: opts.indent_width,
        use_tabs: matches!(opts.indent_style, IndentStyle::Tabs),
    }
}
