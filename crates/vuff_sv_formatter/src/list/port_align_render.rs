//! Render an ANSI port list with per-column padding.
//!
//! The layout has four slots: direction / type / packed-dim / name-tail.
//! Each slot's width is the max across all rows; absent slots are blank-
//! padded so later slots still line up. Inside the packed-dim slot, the
//! high-bit expression is right-aligned to the colon and the low-bit
//! expression is left-aligned, so `[ 3:0]` lines up with `[31:0]`.

use crate::context::{FormatCtx, Formatter};
use crate::list::port_align::{PortList, PortRow, TokRange};
use crate::tokens::spacing::{force_space_between, no_space_between};

pub(crate) fn render_port_list(ctx: &FormatCtx<'_>, f: &mut Formatter, list: &PortList) {
    // Always a single space between the module name and the port `(`.
    // Any block comment or newline the user placed here is preserved via
    // the standard trivia machinery; a sea-of-spaces collapses to one.
    if list.paren_open > 0 {
        let prev_end = ctx.tokens[list.paren_open - 1].end();
        let open_offset = ctx.tokens[list.paren_open].offset;
        let between = &ctx.source[prev_end..open_offset];
        if between.contains('\n') || between.contains("//") || between.contains("/*") {
            crate::tokens::trivia::emit_trivia_at(f, between, false, f.depth, f.depth);
        } else {
            f.push_static(" ");
        }
    }

    f.push_text("(".to_owned());

    if list.rows.is_empty() {
        f.push_text(")".to_owned());
        return;
    }

    let widths = compute_widths(ctx, list);

    // Rows and the closing `)` indent at the *structural* depth of the
    // module header, not the statement-continuation depth. The module
    // keyword sets `in_statement = true` in the verbatim engine; if we
    // honored that, every row would be bumped another indent level.
    let row_depth = f.depth + 1;
    let close_depth = f.depth;

    for (i, row) in list.rows.iter().enumerate() {
        f.push_hardline();
        push_indent_at_depth(f, row_depth);

        render_row(ctx, f, row, &widths);

        let is_last = i == list.rows.len() - 1;
        if !is_last {
            f.push_text(",".to_owned());
        }
        emit_trailing_inline_comment(ctx, f, row, list, i);
    }

    f.push_hardline();
    push_indent_at_depth(f, close_depth);
    f.push_text(")".to_owned());
}

fn push_indent_at_depth(f: &mut Formatter, depth: u32) {
    if depth == 0 {
        return;
    }
    let mut buf = String::with_capacity(f.indent_text.len() * depth as usize);
    for _ in 0..depth {
        buf.push_str(&f.indent_text);
    }
    f.push_text(buf);
}

struct Widths {
    dir: usize,
    typ: usize,
    hi: usize,
    lo: usize,
    /// Whether any row has a packed dim at all. If false the packed slot
    /// collapses to zero (no trailing space before the name column).
    any_packed: bool,
    any_type: bool,
    any_dir: bool,
}

fn compute_widths(ctx: &FormatCtx<'_>, list: &PortList) -> Widths {
    let mut w = Widths {
        dir: 0,
        typ: 0,
        hi: 0,
        lo: 0,
        any_packed: false,
        any_type: false,
        any_dir: false,
    };
    for r in &list.rows {
        if let Some(range) = r.dir {
            w.any_dir = true;
            w.dir = w.dir.max(render_range(ctx, range).len());
        }
        if let Some(range) = r.typ {
            w.any_type = true;
            w.typ = w.typ.max(render_range(ctx, range).len());
        }
        if let Some(range) = r.packed {
            w.any_packed = true;
            let (hi, lo) = split_packed(ctx, range);
            w.hi = w.hi.max(hi.len());
            w.lo = w.lo.max(lo.len());
        }
    }
    w
}

fn render_row(ctx: &FormatCtx<'_>, f: &mut Formatter, row: &PortRow, w: &Widths) {
    // Direction slot.
    if w.any_dir {
        let text = row.dir.map(|r| render_range(ctx, r)).unwrap_or_default();
        f.push_text(left_align(&text, w.dir));
    }

    // Type slot.
    if w.any_type {
        if w.any_dir {
            f.push_static(" ");
        }
        let text = row.typ.map(|r| render_range(ctx, r)).unwrap_or_default();
        f.push_text(left_align(&text, w.typ));
    }

    // Packed-dim slot.
    if w.any_packed {
        if w.any_dir || w.any_type {
            f.push_static(" ");
        }
        if let Some(range) = row.packed {
            let (hi, lo) = split_packed(ctx, range);
            let mut s = String::with_capacity(w.hi + w.lo + 3);
            s.push('[');
            for _ in hi.len()..w.hi {
                s.push(' ');
            }
            s.push_str(&hi);
            s.push(':');
            s.push_str(&lo);
            for _ in lo.len()..w.lo {
                s.push(' ');
            }
            s.push(']');
            f.push_text(s);
        } else {
            let width = w.hi + w.lo + 3;
            f.push_text(" ".repeat(width));
        }
    }

    // Tail slot: name + unpacked dim + default.
    if w.any_dir || w.any_type || w.any_packed {
        f.push_static(" ");
    }
    f.push_text(render_range(ctx, row.tail));
}

fn left_align(s: &str, w: usize) -> String {
    if s.len() >= w {
        return s.to_owned();
    }
    let mut out = String::with_capacity(w);
    out.push_str(s);
    for _ in s.len()..w {
        out.push(' ');
    }
    out
}

/// Render a contiguous token range as a string, applying the same
/// force/forbid spacing rules as the verbatim engine and otherwise
/// preserving source spacing. So `W+1` gets a forced space (binary `+`),
/// `W-1` keeps whatever the source had (no force because `-` is also
/// unary), and `8'd42` stays glued (source has no gap).
fn render_range(ctx: &FormatCtx<'_>, r: TokRange) -> String {
    let toks = &ctx.tokens[r.start..=r.end];
    let mut out = String::new();
    for (i, t) in toks.iter().enumerate() {
        if i > 0 {
            let prev_tok = toks[i - 1];
            let curr = t.text;
            let prev = prev_tok.text;
            let needs = force_space_between(prev, curr);
            let forbids = no_space_between(prev, curr);
            let between_has_ws = !ctx.source[prev_tok.end()..t.offset].is_empty();
            if needs || (!forbids && between_has_ws) {
                out.push(' ');
            }
        }
        out.push_str(t.text);
    }
    out
}

/// Split a packed-dim `[...]` into re-spaced `(hi, lo)` strings. Tokens
/// inside are emitted through [`render_range`] so expressions like
/// `W+1:0` come out as `W + 1` / `0`. If the content isn't a single
/// `hi:lo` range, the whole content is returned as hi and lo is empty.
fn split_packed(ctx: &FormatCtx<'_>, r: TokRange) -> (String, String) {
    // Find the `:` token at the shallowest bracket depth inside the range.
    let toks = &ctx.tokens[r.start..=r.end];
    let mut depth: i32 = 0;
    let mut colon_local: Option<usize> = None;
    for (local_i, t) in toks.iter().enumerate() {
        match t.text {
            "[" | "(" | "{" => depth += 1,
            "]" | ")" | "}" => depth -= 1,
            ":" if depth == 1 && colon_local.is_none() => {
                colon_local = Some(local_i);
            }
            _ => {}
        }
    }
    let Some(colon) = colon_local else {
        let inner = TokRange {
            start: r.start + 1,
            end: r.end - 1,
        };
        if inner.start > inner.end {
            return (String::new(), String::new());
        }
        return (render_range(ctx, inner), String::new());
    };
    // `[` is at local 0, `]` is at local toks.len()-1. Hi spans (0+1 ..
    // colon-1); lo spans (colon+1 .. toks.len()-2). Returning empty for
    // degenerate shapes like `[:0]` or `[7:]` keeps the renderer stable.
    let hi_local_start = 1;
    let hi_local_end = colon.saturating_sub(1);
    let lo_local_start = colon + 1;
    let lo_local_end = toks.len().saturating_sub(2);
    let hi = if hi_local_start <= hi_local_end {
        render_range(
            ctx,
            TokRange {
                start: r.start + hi_local_start,
                end: r.start + hi_local_end,
            },
        )
    } else {
        String::new()
    };
    let lo = if lo_local_start <= lo_local_end {
        render_range(
            ctx,
            TokRange {
                start: r.start + lo_local_start,
                end: r.start + lo_local_end,
            },
        )
    } else {
        String::new()
    };
    (hi, lo)
}

fn emit_trailing_inline_comment(
    ctx: &FormatCtx<'_>,
    f: &mut Formatter,
    row: &PortRow,
    list: &PortList,
    row_idx: usize,
) {
    // Region between this row's `,` (or end of row) and the next row's
    // first token (or `)`). If it contains a `//` or `/*` BEFORE the first
    // newline, the comment is an inline trailer.
    let after_tok = row.comma_tok.unwrap_or(row.tail.end);
    let start_byte = ctx.tokens[after_tok].end();
    let end_byte = if row_idx + 1 < list.rows.len() {
        let next_row = &list.rows[row_idx + 1];
        let next_first = next_row
            .dir
            .map(|r| r.start)
            .or_else(|| next_row.typ.map(|r| r.start))
            .or_else(|| next_row.packed.map(|r| r.start))
            .unwrap_or(next_row.tail.start);
        ctx.tokens[next_first].offset
    } else {
        ctx.tokens[list.paren_close].offset
    };
    let between = &ctx.source[start_byte..end_byte];
    let Some(first_nl) = between.find('\n') else {
        return;
    };
    let first_line = &between[..first_nl];
    let trimmed = first_line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with("/*") {
        f.push_static(" ");
        f.push_text(trimmed.trim_end().to_owned());
    }
}
