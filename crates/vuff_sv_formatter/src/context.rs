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
use std::collections::HashMap;

use vuff_config::{FormatOptions, IndentStyle};
use vuff_formatter::{FormatElement, PrintOptions};
use vuff_sv_ast::{Parsed, SyntaxTree, Token};

use crate::attribute::spans::{find_attribute_spans, AttributeSpan};
use crate::expr::{
    apostrophe_brace_mask, build_macro_calls, build_ternary_chains, call_open_paren_mask,
    concat_brace_masks, select_open_bracket_mask, streaming_concat_mask, ternary_colon_mask,
    MacroCallInfo, TernaryChainInfo,
};
use crate::indent_map::{chain_depth_map, cst_depth_map, directive_start_mask};
use crate::list::inst_port_list::{collect_inst_port_lists, InstPortList};
use crate::list::param_port_list::{collect_param_port_lists, ParamPortList};
use crate::list::port_align::{collect_port_lists, PortList};
use crate::list::wrap_mask::wrap_delimiter_masks;
use crate::list::{
    force_space_before_instance_paren_mask, force_space_before_port_paren_mask,
    param_assign_pound_mask,
};
use crate::stmt::{control_header_paren_mask, statement_boundary_mask, statement_reset_mask};
use crate::trivia::TriviaMap;

/// Reverse index from a token's preprocessed-source byte offset to its
/// position in the flat token list. The CST visitor surfaces every
/// token as a `Locate` event keyed by offset, and several mask builders
/// previously did a linear `tokens.iter().position(...)` per event —
/// quadratic on real files. This map lets each lookup be O(1).
pub(crate) type TokenIndex = HashMap<usize, usize>;

#[must_use]
pub(crate) fn build_token_index(tokens: &[Token<'_>]) -> TokenIndex {
    tokens
        .iter()
        .enumerate()
        .map(|(i, t)| (t.offset, i))
        .collect()
}

/// Cached CST-driven masks. Each field used to be rebuilt on every
/// `format_token_range` call; we now compute them once per file and
/// share via [`FormatCtx`]. The verbatim engine is the only consumer.
pub(crate) struct FormatCtxMasks {
    pub(crate) attr_spans: Vec<AttributeSpan>,
    pub(crate) port_paren: Vec<bool>,
    pub(crate) instance_paren: Vec<bool>,
    pub(crate) param_pound: Vec<bool>,
    pub(crate) is_ternary_colon: Vec<bool>,
    pub(crate) concat_open: Vec<bool>,
    pub(crate) concat_close: Vec<bool>,
    pub(crate) concat_before_open: Vec<bool>,
    pub(crate) apostrophe_brace: Vec<bool>,
    pub(crate) control_paren: Vec<bool>,
    pub(crate) select_bracket: Vec<bool>,
    pub(crate) call_paren: Vec<bool>,
    pub(crate) in_streaming: Vec<bool>,
    pub(crate) is_stmt_boundary: Vec<bool>,
    pub(crate) is_stmt_reset: Vec<bool>,
    pub(crate) cst_depth: Vec<u32>,
    /// Per-token count of `\`ifdef` branch bodies that enclose the
    /// token's original-source position. Bumps active code one level
    /// per enclosing chain so the keyword sits at the chain's outer
    /// scope.
    pub(crate) chain_depth: Vec<u32>,
    /// `true` at each token index that begins a surviving preprocessor
    /// directive (`\`define`, `\`timescale`, `\`include`, …). Used to
    /// reset the in-statement continuation bump.
    pub(crate) directive_start: Vec<bool>,
    pub(crate) inst_port_lists: Vec<InstPortList>,
    pub(crate) param_port_lists: Vec<ParamPortList>,
    pub(crate) port_lists: Vec<PortList>,
    pub(crate) wrap_open: Vec<bool>,
    pub(crate) wrap_close: Vec<bool>,
    pub(crate) ternary_chains: TernaryChainInfo,
    pub(crate) macro_calls: MacroCallInfo,
}

impl FormatCtxMasks {
    pub(crate) fn build(parsed: &Parsed, tokens: &[Token<'_>]) -> Self {
        let tree = &parsed.tree;
        let source = &parsed.text;
        let attr_spans = find_attribute_spans(tree, source, tokens);
        let port_paren = force_space_before_port_paren_mask(tree, tokens);
        let instance_paren = force_space_before_instance_paren_mask(tree, tokens);
        let param_pound = param_assign_pound_mask(tree, tokens);
        let is_ternary_colon = ternary_colon_mask(tree, tokens);
        let (concat_open, concat_close, concat_before_open) = concat_brace_masks(tree, tokens);
        let apostrophe_brace = apostrophe_brace_mask(tree, tokens);
        let control_paren = control_header_paren_mask(tree, tokens);
        let select_bracket = select_open_bracket_mask(tree, tokens);
        let call_paren = call_open_paren_mask(tree, tokens);
        let in_streaming = streaming_concat_mask(tree, tokens);
        let is_stmt_boundary = statement_boundary_mask(tree, tokens);
        let is_stmt_reset = statement_reset_mask(tree, tokens);
        let cst_depth = cst_depth_map(tree, tokens);
        let chain_depth = chain_depth_map(parsed, tokens);
        let directive_start = directive_start_mask(parsed, tokens);
        let inst_port_lists = collect_inst_port_lists(tree, tokens, source);
        let param_port_lists = collect_param_port_lists(tree, tokens, source);
        let port_lists = collect_port_lists(tree, tokens, source);
        let mut excluded: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for l in &inst_port_lists {
            excluded.insert(l.paren_open);
        }
        for l in &param_port_lists {
            excluded.insert(l.paren_open);
        }
        for l in &port_lists {
            excluded.insert(l.paren_open);
        }
        let (wrap_open, wrap_close) = wrap_delimiter_masks(tokens, source, &excluded);
        let ternary_chains = build_ternary_chains(tree, tokens, source);
        let macro_calls = build_macro_calls(parsed, tokens);
        Self {
            attr_spans,
            port_paren,
            instance_paren,
            param_pound,
            is_ternary_colon,
            concat_open,
            concat_close,
            concat_before_open,
            apostrophe_brace,
            control_paren,
            select_bracket,
            call_paren,
            in_streaming,
            is_stmt_boundary,
            is_stmt_reset,
            cst_depth,
            chain_depth,
            directive_start,
            inst_port_lists,
            param_port_lists,
            port_lists,
            wrap_open,
            wrap_close,
            ternary_chains,
            macro_calls,
        }
    }
}

/// Read-only state shared by every formatter rule. Cheap to pass by
/// reference; does not own any of its pointees.
pub(crate) struct FormatCtx<'a> {
    pub(crate) opts: &'a FormatOptions,
    pub(crate) source: &'a str,
    pub(crate) tokens: &'a [Token<'a>],
    #[allow(dead_code)] // wired for future per-node dispatch (step 3+)
    pub(crate) tree: &'a SyntaxTree,
    pub(crate) trivia: &'a TriviaMap,
    pub(crate) parsed: &'a Parsed,
    pub(crate) masks: &'a FormatCtxMasks,
}

impl<'a> FormatCtx<'a> {
    pub(crate) fn new(
        opts: &'a FormatOptions,
        parsed: &'a Parsed,
        tokens: &'a [Token<'a>],
        trivia: &'a TriviaMap,
        masks: &'a FormatCtxMasks,
    ) -> Self {
        Self {
            opts,
            source: &parsed.text,
            tokens,
            tree: &parsed.tree,
            trivia,
            parsed,
            masks,
        }
    }
}

/// Mutable emission state. Rules call `push_*` to write into `out`.
pub(crate) struct Formatter {
    pub(crate) indent_text: String,
    /// `[option].indent_width`. Stored alongside `indent_text` so
    /// helpers that re-grid space-indented source (e.g. skipped
    /// `\`ifdef` bodies) can convert source columns to indent levels.
    pub(crate) indent_width: u8,
    pub(crate) depth: u32,
    /// True while inside an un-terminated statement — bumps continuation
    /// indentation by one level until the terminator (`;`, `begin`, …).
    pub(crate) in_statement: bool,
    pub(crate) out: Vec<FormatElement>,
    /// Best-effort tracker of the current visual column. Used by rules
    /// that need to align tokens (e.g. multi-line ternary `?`). Updated
    /// alongside every `push_*`. The pretty-printer in `vuff_formatter`
    /// is the source of truth for actual output, but our IR only uses
    /// `Text` / `StaticText` / `HardLine` so the tracker stays accurate.
    pub(crate) col: u32,
}

impl Formatter {
    pub(crate) fn new(opts: &FormatOptions, expected_capacity: usize) -> Self {
        Self {
            indent_text: indent_unit(opts),
            indent_width: opts.indent_width,
            depth: 0,
            in_statement: false,
            out: Vec::with_capacity(expected_capacity),
            col: 0,
        }
    }

    pub(crate) fn push_text(&mut self, s: impl Into<String>) {
        let s = s.into();
        self.col += u32::try_from(s.chars().count()).unwrap_or(u32::MAX);
        self.out.push(FormatElement::Text(Cow::Owned(s)));
    }

    pub(crate) fn push_static(&mut self, s: &'static str) {
        self.col += u32::try_from(s.chars().count()).unwrap_or(u32::MAX);
        self.out.push(FormatElement::StaticText(s));
    }

    pub(crate) fn push_hardline(&mut self) {
        self.col = 0;
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

    pub(crate) fn push_indent_levels(&mut self, levels: u32) {
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
