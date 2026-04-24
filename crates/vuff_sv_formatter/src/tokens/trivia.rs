//! Inter-token trivia normalization. Given the raw byte slice between two
//! tokens, emit the appropriate whitespace/comment sequence into the
//! `Formatter`'s IR buffer.

use vuff_formatter::FormatElement;

use crate::context::Formatter;

/// Normalize a trivia slice. Mutates `f.out` directly.
///
/// The caller has pre-set `f.depth` to the upcoming token's depth. This
/// entry point uses that depth for both comment lines and the trailing
/// indent. Use [`emit_trivia_at`] when the two should differ (e.g.
/// comments above a dedenting closer belong to the outgoing block).
pub(crate) fn emit_trivia(f: &mut Formatter, slice: &str, is_leading: bool) {
    let d = f.depth;
    emit_trivia_at(f, slice, is_leading, d, d);
}

/// Like [`emit_trivia`] but indent comments and blank-line markers at
/// `body_depth` while leaving the trailing indent (for the next token)
/// at `tail_depth`.
pub(crate) fn emit_trivia_at(
    f: &mut Formatter,
    slice: &str,
    is_leading: bool,
    body_depth: u32,
    tail_depth: u32,
) {
    if slice.is_empty() {
        return;
    }
    let newline_count = slice.bytes().filter(|&b| b == b'\n').count();
    let has_comment = slice.contains("//") || slice.contains("/*");
    let saved = f.depth;

    if has_comment {
        f.depth = body_depth;
        emit_trivia_with_comments(f, slice, is_leading);
        // Retarget any trailing indent that was just pushed.
        if let Some(FormatElement::Text(last)) = f.out.last() {
            if last.chars().all(|c| c == ' ' || c == '\t') {
                f.out.pop();
                f.depth = tail_depth;
                f.push_indent_for_new_line();
            }
        } else if let Some(FormatElement::StaticText(last)) = f.out.last() {
            if last.chars().all(|c| c == ' ' || c == '\t') {
                f.out.pop();
                f.depth = tail_depth;
                f.push_indent_for_new_line();
            }
        }
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
    f.depth = tail_depth;
    f.push_indent_for_new_line();
    f.depth = saved;
}

/// Trivia slice contains comments. For `//` line comments, normalize
/// leading whitespace to the current indent; for `/* … */` block
/// comments, preserve the interior byte-for-byte (only the first line's
/// leading whitespace is normalized). Strip trailing horizontal
/// whitespace from each physical line.
fn emit_trivia_with_comments(f: &mut Formatter, slice: &str, is_leading: bool) {
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
                f.push_indent_for_new_line();
                f.push_text(content.to_owned());
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

    if emitted_hardline {
        f.push_indent_for_new_line();
    }
}

/// Scan a single physical line and return the block-comment state after
/// it. Handles multiple opens/closes on one line; a `//` starts a line
/// comment that extinguishes any remaining `/*` on that line.
fn block_state_after(mut in_block: bool, line: &str) -> bool {
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
