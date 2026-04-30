//! Inter-token trivia normalization. Given the raw byte slice between two
//! tokens, emit the appropriate whitespace/comment sequence into the
//! `Formatter`'s IR buffer.

use vuff_formatter::FormatElement;

use crate::context::Formatter;
use crate::trivia::{TriviaSegment, TriviaSlice};

/// Emit raw trivia bytes (for row-internal emitters in the list crates
/// that don't yet route through the classified [`TriviaSlice`] API).
/// Comment lines and the trailing indent both render at `depth`.
pub(crate) fn emit_trivia_at(f: &mut Formatter, slice: &str, is_leading: bool, depth: u32) {
    if slice.is_empty() {
        return;
    }
    let newline_count = slice.bytes().filter(|&b| b == b'\n').count();
    let has_comment = slice.contains("//") || slice.contains("/*");
    let saved = f.depth;

    if has_comment {
        f.depth = depth;
        // Inline / None trailing forms need no fixup; the indent already
        // sits at `depth` since body and tail share it.
        let _ = emit_trivia_with_comments(f, slice, is_leading);
        f.depth = saved;
        return;
    }

    if newline_count == 0 {
        if !is_leading {
            f.push_static(" ");
        }
        return;
    }

    // Pure whitespace with newlines → emit 1 or 2 hard lines, then indent.
    let emit_lines = newline_count.min(2) as u32;
    for _ in 0..emit_lines {
        f.push_hardline();
    }
    f.depth = depth;
    f.push_indent_for_new_line();
    f.depth = saved;
}

/// Trivia slice contains comments. For `//` line comments, normalize
/// leading whitespace to the current indent; for `/* … */` block
/// comments, preserve the interior byte-for-byte (only the first line's
/// leading whitespace is normalized). Strip trailing horizontal
/// whitespace from each physical line.
/// What was emitted at the very end of `emit_trivia_with_comments`, so the
/// caller can decide how to bridge to the next real token.
enum CommentTrailing {
    /// Last thing pushed is an indent for a token that will start on a
    /// fresh line — caller may retarget that indent to a different depth.
    NewLineIndent,
    /// Last thing pushed is a single space because the next token sits
    /// inline with the previous comment on the same physical line.
    InlineSpace,
    /// Nothing trailing — comment ended at the end of the slice with no
    /// following whitespace.
    None,
}

fn emit_trivia_with_comments(f: &mut Formatter, slice: &str, is_leading: bool) -> CommentTrailing {
    let lines: Vec<&str> = slice.split('\n').collect();
    let mut blank_run: u32 = 0;
    let mut emitted_hardline = false;
    // True between `/*` and `*/` — in those lines we preserve original
    // leading whitespace instead of reindenting.
    let mut in_block = false;

    for (idx, raw) in lines.iter().enumerate() {
        let is_last = idx + 1 == lines.len();
        let stripped = raw.trim_end_matches([' ', '\t']);
        let has_content = !stripped.is_empty();
        let was_in_block = in_block;

        if idx == 0 {
            if has_content {
                // First line of the trivia, sharing a line with the
                // preceding token. Normalize `x;// c` / `x;/*c*/` to have
                // exactly one leading space.
                let text = stripped.trim_start_matches([' ', '\t']);
                if !is_leading {
                    f.push_static(" ");
                }
                f.push_text(text.to_owned());
            }
        } else if has_content {
            if was_in_block {
                // Inside a multi-line `/* … */` — preserve original
                // leading whitespace of this continuation line (which
                // may include the `*/` that closes the block).
                f.push_text(stripped.to_owned());
            } else {
                let content = stripped.trim_start_matches([' ', '\t']);
                // `\`define` / `\`undef` / etc. lines that survive
                // preprocessing keep their original indentation — they
                // line up with surrounding `\`ifdef` / `\`endif`
                // anchors that also preserve user-written indent.
                // Comments (`//`) are re-indented to the current depth
                // so they match the code they sit alongside.
                if content.starts_with('`') {
                    f.push_text(stripped.to_owned());
                } else {
                    f.push_indent_for_new_line();
                    f.push_text(content.to_owned());
                }
            }
            blank_run = 0;
        }

        // Update in_block for the NEXT line by scanning THIS line's
        // `/*` / `*/` markers in order.
        in_block = block_state_after(in_block, stripped);

        if !is_last {
            if has_content {
                f.push_hardline();
                emitted_hardline = true;
            } else {
                blank_run += 1;
                if blank_run <= 1 {
                    f.push_hardline();
                    emitted_hardline = true;
                }
            }
        }
    }

    // Push a trailing indent only when the next real token will land on a
    // new line. That's true exactly when the last split-line is empty
    // (slice ended with `\n` or trailing whitespace following a newline).
    // If the last split-line has content, the next token sits inline with
    // the comment and a single space will be inserted instead.
    let last_line_has_content = lines
        .last()
        .is_some_and(|l| !l.trim_end_matches([' ', '\t']).is_empty());
    if emitted_hardline && !last_line_has_content {
        f.push_indent_for_new_line();
        CommentTrailing::NewLineIndent
    } else if emitted_hardline && last_line_has_content {
        f.push_static(" ");
        CommentTrailing::InlineSpace
    } else {
        CommentTrailing::None
    }
}

/// Scan a single physical line and return the block-comment state after
/// it. Handles multiple opens/closes on one line; a `//` starts a line
/// comment that extinguishes any remaining `/*` on that line.
pub(crate) fn block_state_after(mut in_block: bool, line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if !in_block {
            if &bytes[i..i + 2] == b"//" {
                return in_block;
            }
            if &bytes[i..i + 2] == b"/*" {
                in_block = true;
                i += 2;
                continue;
            }
        } else if &bytes[i..i + 2] == b"*/" {
            in_block = false;
            i += 2;
            continue;
        }
        i += 1;
    }
    in_block
}

/// How a [`TriviaSlice`] is rendered.
#[derive(Clone, Copy)]
pub(crate) enum SliceMode {
    /// Standalone emission. Handles file-leading suppression, blank
    /// lines, and pushes a tail indent for the upcoming token.
    Standalone { is_leading: bool, tail_depth: u32 },
    /// Caller (e.g. a list renderer) owns row structure. Blank-line
    /// segments are dropped; no tail indent is pushed; the caller
    /// places its own hardline / indent for the next row.
    Embedded,
}

/// Emit a classified [`TriviaSlice`] into the formatter's IR buffer.
///
/// `body_depth` indents comment lines that render on their own line.
/// `chain_floor` is the maximum `\`ifdef`-chain depth contributed by
/// the slice's adjacent tokens; directive segments subtract it so a
/// keyword renders at its own structural scope (= `body_depth -
/// chain_floor + segment.chain_extra`) rather than inheriting the
/// chain bump that bumped its neighbors deeper.
pub(crate) fn emit_trivia_slice(
    f: &mut Formatter,
    slice: &TriviaSlice,
    body_depth: u32,
    chain_floor: u32,
    mode: SliceMode,
) {
    if slice.segments.is_empty() {
        if let SliceMode::Standalone {
            is_leading,
            tail_depth,
        } = mode
        {
            emit_pure_whitespace_gap(f, slice.pp_newline_count, tail_depth, is_leading);
        }
        return;
    }

    let saved_depth = f.depth;
    // Comments / preserved directive lines stand outside any in-progress
    // statement. A surviving `\`define X Y` at top level leaves
    // `in_statement = true` because nothing in the token stream resets it,
    // which would over-shift the following comment by one continuation
    // indent. Suppress for the duration of this slice and restore at the
    // end.
    let saved_in_statement = f.in_statement;
    f.in_statement = false;
    let mut last_was_inline_only = false;

    for (idx, seg) in slice.segments.iter().enumerate() {
        let is_first = idx == 0;
        match seg {
            TriviaSegment::Blank => {
                ensure_fresh_line(f);
                f.push_hardline();
                last_was_inline_only = false;
            }
            TriviaSegment::LineComment {
                text,
                nl_before,
                nl_after,
                chain_extra,
            }
            | TriviaSegment::BlockComment {
                text,
                nl_before,
                nl_after,
                chain_extra,
            } => {
                let depth = body_depth.saturating_sub(chain_floor) + *chain_extra;
                if *nl_before || !is_first {
                    place_on_fresh_line(f, is_first, mode, depth);
                } else if needs_inline_space(f, mode) {
                    f.push_static(" ");
                }
                push_multiline_text(f, text);
                last_was_inline_only = !*nl_after;
            }
            TriviaSegment::DirectiveLine { text, chain_extra }
            | TriviaSegment::IfdefKeyword { text, chain_extra }
            | TriviaSegment::EmptyMacroCall {
                call_text: text,
                chain_extra,
            } => {
                let depth = body_depth.saturating_sub(chain_floor) + *chain_extra;
                place_on_fresh_line(f, is_first, mode, depth);
                f.push_text(text.clone());
                last_was_inline_only = false;
            }
            TriviaSegment::SkippedBody { text, chain_extra } => {
                // First line lands at column 0; `push_skipped_body_text`
                // writes the indent itself so each continuation line
                // also gets it.
                let depth = body_depth.saturating_sub(chain_floor) + *chain_extra;
                place_on_fresh_line(f, is_first, mode, 0);
                push_skipped_body_text(f, text, depth);
                last_was_inline_only = false;
            }
        }
    }

    f.depth = saved_depth;
    f.in_statement = saved_in_statement;

    if let SliceMode::Standalone { tail_depth, .. } = mode {
        if last_was_inline_only {
            f.push_static(" ");
        } else {
            ensure_fresh_line(f);
            let saved = f.depth;
            f.depth = tail_depth;
            f.push_indent_for_new_line();
            f.depth = saved;
        }
    }
}

/// Place the cursor on a fresh line at `body_depth`. Skips the leading
/// hardline when this is the first segment of a file-leading slot
/// (nothing has been written yet), so the file doesn't open with a
/// stray blank line. A `body_depth` of 0 is used by directive /
/// skipped-body / empty-call segments that emit their original column
/// via per-segment `leading_ws` instead of a structural indent.
fn place_on_fresh_line(f: &mut Formatter, is_first: bool, mode: SliceMode, body_depth: u32) {
    let suppress_leading = matches!(
        mode,
        SliceMode::Standalone { is_leading: true, .. }
    ) && is_first
        && f.col == 0
        && f.out.is_empty();
    if !suppress_leading {
        ensure_fresh_line(f);
    }
    if body_depth > 0 {
        f.depth = body_depth;
        f.push_indent_for_new_line();
    }
}

/// First segment is inline (no `nl_before`). Emit a separating space
/// unless the buffer is empty at file start.
fn needs_inline_space(f: &Formatter, mode: SliceMode) -> bool {
    if let SliceMode::Standalone { is_leading: true, .. } = mode {
        f.col != 0
    } else {
        true
    }
}

fn emit_pure_whitespace_gap(f: &mut Formatter, newlines: u32, tail_depth: u32, is_leading: bool) {
    if newlines == 0 {
        if !is_leading && f.col != 0 {
            f.push_static(" ");
        }
        return;
    }
    let lines = newlines.min(2);
    for _ in 0..lines {
        f.push_hardline();
    }
    let saved = f.depth;
    f.depth = tail_depth;
    f.push_indent_for_new_line();
    f.depth = saved;
}

pub(crate) fn ensure_fresh_line(f: &mut Formatter) {
    if f.col != 0 {
        f.push_hardline();
    }
}

/// Push text that may contain `\n`, splitting on newlines and emitting
/// one hard-line between each line so the IR carries newlines as
/// `HardLine` rather than embedded in `Text`. Each continuation line is
/// pushed verbatim — its original indent is part of the text.
fn push_multiline_text(f: &mut Formatter, text: &str) {
    if !text.contains('\n') {
        f.push_text(text.to_owned());
        return;
    }
    let mut first = true;
    for line in text.split('\n') {
        if !first {
            f.push_hardline();
        }
        first = false;
        if !line.is_empty() {
            f.push_text(line.to_owned());
        }
    }
}

/// Re-indent the verbatim text of a `taken=false` branch onto the
/// formatter's grid. Each line's leading whitespace (tabs and spaces,
/// where one tab counts as `indent_width` spaces) is converted to a
/// column count; the smallest column across non-blank lines is treated
/// as level 0 and stripped, then `base_depth + relative_levels` of
/// `indent_text` is prepended. Sub-indent-width residue carries over
/// as raw spaces so weird alignment inside the body isn't destroyed.
///
/// `place_on_fresh_line` is expected to have positioned the cursor at
/// column 0 — this helper writes its own indent on every line.
fn push_skipped_body_text(f: &mut Formatter, text: &str, base_depth: u32) {
    let tab_width = usize::from(f.indent_width.max(1));
    let lead_columns = |line: &str| -> usize {
        let mut col = 0usize;
        for b in line.bytes() {
            match b {
                b' ' => col += 1,
                b'\t' => col += tab_width,
                _ => break,
            }
        }
        col
    };

    let common_columns = text
        .split('\n')
        .filter(|l| {
            !l.is_empty() && !l.bytes().all(|b| b == b' ' || b == b'\t')
        })
        .map(lead_columns)
        .min()
        .unwrap_or(0);

    let mut first = true;
    for line in text.split('\n') {
        if !first {
            f.push_hardline();
        }
        first = false;
        if line.is_empty() {
            continue;
        }
        let lead_byte_count = line
            .bytes()
            .take_while(|&b| b == b' ' || b == b'\t')
            .count();
        let lead_cols = lead_columns(line);
        let rel_cols = lead_cols.saturating_sub(common_columns);
        let rel_levels = rel_cols / tab_width;
        let rel_residue = rel_cols % tab_width;
        let rest = &line[lead_byte_count..];
        let total_levels = base_depth as usize + rel_levels;
        let mut buf =
            String::with_capacity(f.indent_text.len() * total_levels + rel_residue + rest.len());
        for _ in 0..total_levels {
            buf.push_str(&f.indent_text);
        }
        for _ in 0..rel_residue {
            buf.push(' ');
        }
        buf.push_str(rest);
        f.push_text(buf);
    }
}

/// Pop trailing hard-lines / blank indent texts and re-add exactly one
/// terminating hard-line. Ensures every file ends with a single `\n`.
pub(crate) fn ensure_trailing_newline(out: &mut Vec<FormatElement>) {
    while let Some(last) = out.last() {
        match last {
            FormatElement::HardLine | FormatElement::EmptyLine => {
                out.pop();
            }
            FormatElement::Text(s) if s.chars().all(|c| c == ' ' || c == '\t') => {
                out.pop();
            }
            FormatElement::StaticText(s) if s.chars().all(|c| c == ' ' || c == '\t') => {
                out.pop();
            }
            _ => break,
        }
    }
    out.push(FormatElement::HardLine);
}
