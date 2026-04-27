//! Language-agnostic Wadler/Oppen pretty-printer.
//!
//! Two-pass algorithm:
//!  1. Measure: annotate each `Group` as Flat or Expanded based on whether
//!     its flattened rendering fits in the remaining column budget. A
//!     `HardLine` or `ExpandParent` inside a group forces it Expanded and
//!     propagates that forcing to enclosing groups.
//!  2. Emit: walk the document, honoring indent stack and the per-group
//!     flat/expanded decision, buffering `LineSuffix` content until the
//!     next line break.

use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMode {
    Auto,
    Flat,
    Expanded,
}

#[derive(Debug, Clone)]
pub enum FormatElement {
    Text(Cow<'static, str>),
    StaticText(&'static str),
    Space,
    SoftLine,
    HardLine,
    EmptyLine,
    LineSuffix(Vec<FormatElement>),
    Group(Vec<FormatElement>, GroupMode),
    Indent(Vec<FormatElement>),
    Dedent(Vec<FormatElement>),
    Align(u8, Vec<FormatElement>),
    IfBreak {
        flat: Vec<FormatElement>,
        broken: Vec<FormatElement>,
    },
    ExpandParent,
    VerbatimComment(String),
}

#[derive(Debug, Default)]
pub struct IrBuilder {
    elements: Vec<FormatElement>,
}

impl IrBuilder {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    pub fn push(&mut self, element: FormatElement) -> &mut Self {
        self.elements.push(element);
        self
    }

    pub fn push_text(&mut self, text: impl Into<Cow<'static, str>>) -> &mut Self {
        self.push(FormatElement::Text(text.into()))
    }

    pub fn soft_line(&mut self) -> &mut Self {
        self.push(FormatElement::SoftLine)
    }

    pub fn hard_line(&mut self) -> &mut Self {
        self.push(FormatElement::HardLine)
    }

    #[must_use]
    pub fn into_elements(self) -> Vec<FormatElement> {
        self.elements
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PrintOptions {
    pub line_width: u16,
    pub indent_width: u8,
    pub use_tabs: bool,
}

impl Default for PrintOptions {
    fn default() -> Self {
        Self {
            line_width: 100,
            indent_width: 2,
            use_tabs: false,
        }
    }
}

// -------- Measurement pass --------

/// Returns `true` if any descendant forces the containing group to expand
/// (HardLine or ExpandParent).
fn contains_expand_forcer(elements: &[FormatElement]) -> bool {
    elements.iter().any(|el| match el {
        FormatElement::HardLine | FormatElement::EmptyLine | FormatElement::ExpandParent => true,
        FormatElement::Group(inner, mode) => {
            *mode == GroupMode::Expanded || contains_expand_forcer(inner)
        }
        FormatElement::Indent(inner)
        | FormatElement::Dedent(inner)
        | FormatElement::Align(_, inner)
        | FormatElement::LineSuffix(inner) => contains_expand_forcer(inner),
        FormatElement::IfBreak { flat, broken } => {
            contains_expand_forcer(flat) || contains_expand_forcer(broken)
        }
        _ => false,
    })
}

/// Measure the flattened width of `elements` — returns `None` if the content
/// cannot be flattened (contains a hard line) or exceeds `budget`.
fn flat_width(elements: &[FormatElement], budget: i32) -> Option<i32> {
    let mut width: i32 = 0;
    for el in elements {
        match el {
            FormatElement::Text(s) => width += s.chars().count() as i32,
            FormatElement::StaticText(s) => width += s.chars().count() as i32,
            FormatElement::Space | FormatElement::SoftLine => width += 1,
            FormatElement::HardLine | FormatElement::EmptyLine | FormatElement::ExpandParent => {
                return None
            }
            FormatElement::LineSuffix(_) => {} // suffix is off the live line
            FormatElement::Group(inner, mode) => match mode {
                GroupMode::Flat | GroupMode::Auto => {
                    width += flat_width(inner, budget - width)?;
                }
                GroupMode::Expanded => return None,
            },
            FormatElement::Indent(inner)
            | FormatElement::Dedent(inner)
            | FormatElement::Align(_, inner) => {
                width += flat_width(inner, budget - width)?;
            }
            FormatElement::IfBreak { flat, .. } => {
                width += flat_width(flat, budget - width)?;
            }
            FormatElement::VerbatimComment(s) => width += s.chars().count() as i32,
        }
        if width > budget {
            return None;
        }
    }
    Some(width)
}

// -------- Emission pass --------

struct Printer<'a> {
    opts: &'a PrintOptions,
    out: String,
    indent: u32, // in "levels"; each level is indent_width spaces OR 1 tab
    align: u32,  // additional columns of spaces (from Align)
    column: u32, // current column (for fit decisions)
    line_suffix_buf: Vec<FormatElement>,
    at_line_start: bool,
}

impl<'a> Printer<'a> {
    fn new(opts: &'a PrintOptions) -> Self {
        Self {
            opts,
            out: String::new(),
            indent: 0,
            align: 0,
            column: 0,
            line_suffix_buf: Vec::new(),
            at_line_start: true,
        }
    }

    fn emit_indent(&mut self) {
        if self.opts.use_tabs {
            for _ in 0..self.indent {
                self.out.push('\t');
            }
            self.column = self.indent; // tabs counted as 1 column for fit math
        } else {
            let spaces = self.indent * u32::from(self.opts.indent_width);
            for _ in 0..spaces {
                self.out.push(' ');
            }
            self.column = spaces;
        }
        for _ in 0..self.align {
            self.out.push(' ');
        }
        self.column += self.align;
        self.at_line_start = false;
    }

    fn ensure_indent(&mut self) {
        if self.at_line_start {
            self.emit_indent();
        }
    }

    fn push_text(&mut self, s: &str) {
        self.ensure_indent();
        self.out.push_str(s);
        self.column += s.chars().count() as u32;
    }

    fn newline(&mut self) {
        // Flush any queued line-suffix content (e.g. trailing comments) before
        // the actual break.
        if !self.line_suffix_buf.is_empty() {
            let buf: Vec<FormatElement> = self.line_suffix_buf.drain(..).collect();
            self.write_elements(&buf, /*flat=*/ false);
        }
        self.out.push('\n');
        self.column = 0;
        self.at_line_start = true;
    }

    fn remaining(&self) -> i32 {
        i32::from(self.opts.line_width) - self.column as i32
    }

    fn group_fits(&self, inner: &[FormatElement]) -> bool {
        if contains_expand_forcer(inner) {
            return false;
        }
        flat_width(inner, self.remaining()).is_some()
    }

    fn write_elements(&mut self, elements: &[FormatElement], flat: bool) {
        for el in elements {
            self.write_one(el, flat);
        }
    }

    fn write_one(&mut self, el: &FormatElement, flat: bool) {
        match el {
            FormatElement::Text(s) => self.push_text(s),
            FormatElement::StaticText(s) => self.push_text(s),
            FormatElement::VerbatimComment(s) => self.push_text(s),
            FormatElement::Space => self.push_text(" "),
            FormatElement::SoftLine => {
                if flat {
                    self.push_text(" ");
                } else {
                    self.newline();
                }
            }
            FormatElement::HardLine => self.newline(),
            FormatElement::EmptyLine => {
                self.newline();
                self.out.push('\n');
            }
            FormatElement::ExpandParent => {}
            FormatElement::LineSuffix(inner) => {
                self.line_suffix_buf.extend_from_slice(inner);
            }
            FormatElement::Group(inner, mode) => {
                let resolved_flat = match mode {
                    GroupMode::Flat => true,
                    GroupMode::Expanded => false,
                    GroupMode::Auto => self.group_fits(inner),
                };
                self.write_elements(inner, resolved_flat);
            }
            FormatElement::Indent(inner) => {
                self.indent += 1;
                self.write_elements(inner, flat);
                self.indent -= 1;
            }
            FormatElement::Dedent(inner) => {
                let saved = self.indent;
                self.indent = self.indent.saturating_sub(1);
                self.write_elements(inner, flat);
                self.indent = saved;
            }
            FormatElement::Align(n, inner) => {
                self.align += u32::from(*n);
                self.write_elements(inner, flat);
                self.align -= u32::from(*n);
            }
            FormatElement::IfBreak { flat: f, broken: b } => {
                let choice = if flat { f } else { b };
                self.write_elements(choice, flat);
            }
        }
    }

    fn finish(mut self) -> String {
        // Flush any stragglers (no final newline forced).
        if !self.line_suffix_buf.is_empty() {
            let buf: Vec<FormatElement> = self.line_suffix_buf.drain(..).collect();
            self.write_elements(&buf, false);
        }
        self.out
    }
}

#[must_use]
pub fn print(elements: &[FormatElement], opts: &PrintOptions) -> String {
    let mut p = Printer::new(opts);
    // Top-level flatness is false — top-level groups decide per their own fit.
    p.write_elements(elements, false);
    p.finish()
}

// Convenience constructors so callers do not need to import GroupMode constantly.
#[must_use]
pub fn group(elements: Vec<FormatElement>) -> FormatElement {
    FormatElement::Group(elements, GroupMode::Auto)
}
#[must_use]
pub fn indent(elements: Vec<FormatElement>) -> FormatElement {
    FormatElement::Indent(elements)
}
#[must_use]
pub fn text(s: &'static str) -> FormatElement {
    FormatElement::StaticText(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_opts() -> PrintOptions {
        PrintOptions {
            line_width: 20,
            indent_width: 2,
            use_tabs: false,
        }
    }

    #[test]
    fn flat_group_stays_on_one_line() {
        let doc = vec![group(vec![
            text("["),
            text("1"),
            FormatElement::SoftLine,
            text("2"),
            text("]"),
        ])];
        let out = print(&doc, &default_opts());
        assert_eq!(out, "[1 2]");
    }

    #[test]
    fn overflowing_group_breaks() {
        let doc = vec![group(vec![
            text("["),
            indent(vec![
                FormatElement::SoftLine,
                text("aaaaaaaaaa"),
                text(","),
                FormatElement::SoftLine,
                text("bbbbbbbbbb"),
            ]),
            FormatElement::SoftLine,
            text("]"),
        ])];
        let out = print(&doc, &default_opts());
        assert!(out.contains('\n'), "should have broken: {out:?}");
        assert!(out.starts_with('['));
        assert!(out.ends_with(']'));
    }

    #[test]
    fn hard_line_forces_expansion() {
        let doc = vec![group(vec![text("a"), FormatElement::HardLine, text("b")])];
        let out = print(&doc, &default_opts());
        assert_eq!(out, "a\nb");
    }

    #[test]
    fn indent_uses_spaces_by_default() {
        let doc = vec![
            text("x"),
            indent(vec![FormatElement::HardLine, text("y")]),
            FormatElement::HardLine,
            text("z"),
        ];
        let out = print(&doc, &default_opts());
        assert_eq!(out, "x\n  y\nz");
    }

    #[test]
    fn indent_uses_tabs_when_requested() {
        let doc = vec![text("x"), indent(vec![FormatElement::HardLine, text("y")])];
        let opts = PrintOptions {
            line_width: 20,
            indent_width: 2,
            use_tabs: true,
        };
        let out = print(&doc, &opts);
        assert_eq!(out, "x\n\ty");
    }

    #[test]
    fn if_break_emits_correct_variant() {
        let doc_flat = vec![group(vec![
            text("a"),
            FormatElement::IfBreak {
                flat: vec![text(",")],
                broken: vec![text(";")],
            },
        ])];
        assert_eq!(print(&doc_flat, &default_opts()), "a,");

        let doc_broken = vec![group(vec![
            text("aaaaaaaaaaaaaaaaaaaa"),
            FormatElement::SoftLine,
            FormatElement::IfBreak {
                flat: vec![text(",")],
                broken: vec![text(";")],
            },
        ])];
        let out = print(&doc_broken, &default_opts());
        assert!(out.ends_with(';'), "broken got {out:?}");
    }

    #[test]
    fn line_suffix_flushes_before_newline() {
        let doc = vec![
            text("x"),
            FormatElement::LineSuffix(vec![text(" // trailing")]),
            FormatElement::HardLine,
            text("y"),
        ];
        let opts = PrintOptions {
            line_width: 80,
            indent_width: 2,
            use_tabs: false,
        };
        let out = print(&doc, &opts);
        assert_eq!(out, "x // trailing\ny");
    }
}
