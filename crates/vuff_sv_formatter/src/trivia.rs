//! Trivia layer.
//!
//! For each gap between consecutive tokens in the preprocessed stream
//! (plus a leading slot before the first token and a trailing slot
//! after the last), classify what the user wrote in the *original*
//! source into typed segments: blank lines, line / block comments,
//! surviving directive lines (`` `define ``, `` `undef ``, …), the
//! preprocessor-stripped `` `ifdef `` / `` `elsif `` / `` `else `` /
//! `` `endif `` keyword lines, the verbatim bytes of a `taken=false`
//! branch, and macro call sites whose expansion is empty.
//!
//! The classifier decides what each segment is, never where it
//! renders — indent and spacing live in the emitter
//! ([`crate::tokens::trivia::emit_trivia_slice`]).

use std::ops::Range;

use vuff_sv_ast::{DirectiveDetail, DirectiveKind, Parsed, PpRange, Token};

use crate::indent_map::{chain_depth_at, collect_branch_body_intervals};
use crate::tokens::trivia::block_state_after;

/// Per-token-boundary classification of the original source. `slices[i]`
/// covers the gap *before* `tokens[i]`; `slices[tokens.len()]` covers
/// from after the last token to end-of-file. Always has length
/// `tokens.len() + 1`.
#[derive(Debug, Default)]
pub(crate) struct TriviaMap {
    pub(crate) slices: Vec<TriviaSlice>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TriviaSlice {
    /// Number of `\n` bytes in the preprocessed gap. Drives spacing for
    /// pure-whitespace gaps (no segments) — 0 → single space, 1 →
    /// newline, ≥2 → blank line.
    pub(crate) pp_newline_count: u32,
    /// Segments in original-source order.
    pub(crate) segments: Vec<TriviaSegment>,
}

/// One classified piece of the original source between two real tokens.
#[derive(Debug, Clone)]
pub(crate) enum TriviaSegment {
    /// One blank line. Two consecutive `Blank` segments collapse to a
    /// single visual gap at emission time.
    Blank,
    /// `// …` comment. `text` includes the leading `//`.
    /// `nl_before`/`nl_after` describe whether the comment's source
    /// line is preceded / followed by a newline within the gap, which
    /// fully determines the four layouts:
    ///   * both → own line (hardline + indent + text + hardline)
    ///   * only `nl_after` → after prev token (space + text + hardline)
    ///   * only `nl_before` → before next token (hardline + indent + text + space)
    ///   * neither → between prev and next on same line (space + text + space)
    LineComment {
        text: String,
        nl_before: bool,
        nl_after: bool,
        chain_extra: u32,
    },
    /// `/* … */` comment, verbatim. May span multiple lines. Same
    /// layout flags as `LineComment` (`nl_before` refers to the
    /// comment's first line, `nl_after` to its last).
    BlockComment {
        text: String,
        nl_before: bool,
        nl_after: bool,
        chain_extra: u32,
    },
    /// `` `define `` / `` `undef `` / `` `include `` / `` `timescale ``
    /// / etc. — a directive line that survives preprocessing. Emitted
    /// on its own line at the surrounding scope's indent. `chain_extra`
    /// is the count of `\`ifdef` branch bodies enclosing the line; an
    /// active `\`define` inside a taken `\`ifdef` body is one level
    /// deeper than the keyword itself.
    DirectiveLine { text: String, chain_extra: u32 },
    /// `` `ifdef `` / `` `ifndef `` / `` `elsif `` / `` `else `` /
    /// `` `endif `` — keyword line stripped by the preprocessor before
    /// tokenization. The keyword sits *between* branches (outside any
    /// branch body), so `chain_extra` only counts outer enclosing
    /// chains, not the keyword's own.
    IfdefKeyword { text: String, chain_extra: u32 },
    /// Verbatim body of a `taken=false` branch. `text` is the raw bytes
    /// the preprocessor dropped, with original indentation preserved
    /// and trailing newline / horizontal whitespace stripped.
    /// `chain_extra` already includes the +1 for being inside this
    /// chain's body.
    SkippedBody { text: String, chain_extra: u32 },
    /// Macro call site whose expansion was empty (e.g. `\`debug(x)`
    /// when `\`define debug(x)` has no body). `chain_extra` counts the
    /// branch bodies enclosing the call site.
    EmptyMacroCall { call_text: String, chain_extra: u32 },
}

/// Build the trivia map from a parse result + token list.
pub(crate) fn build(parsed: &Parsed, tokens: &[Token<'_>]) -> TriviaMap {
    let total_orig = parsed.original.len();
    let total_pp = parsed.text.len();

    // Map every token to its (start, end) in original coords. Falls
    // back to `last` when origin lookup fails or points backwards
    // (macro-expansion-internal tokens), so the sequence is monotonic
    // and gap windows stay sensible.
    let (tok_orig_start, tok_orig_end) = compute_token_origin_ranges(parsed, tokens);

    // Indexed views of the parser's directive list — sorted by
    // `original_range.begin` so the classifier can walk a window in
    // one pass.
    let intervals = collect_branch_body_intervals(parsed);
    let directives = collect_directives(parsed, &intervals);

    let mut slices = Vec::with_capacity(tokens.len() + 1);
    for i in 0..=tokens.len() {
        let (pp_start, pp_end) = pp_gap(tokens, i, total_pp);
        let (orig_start, orig_end) = orig_gap(tokens, &tok_orig_start, &tok_orig_end, i, total_orig);

        let segments = classify(parsed, &directives, &intervals, orig_start..orig_end, i == 0);
        #[allow(clippy::naive_bytecount)]
        let pp_newline_count = u32::try_from(
            parsed.text.as_bytes()[pp_start..pp_end]
                .iter()
                .filter(|&&b| b == b'\n')
                .count(),
        )
        .unwrap_or(u32::MAX);
        slices.push(TriviaSlice {
            pp_newline_count,
            segments,
        });
    }

    TriviaMap { slices }
}

fn pp_gap(tokens: &[Token<'_>], i: usize, total_pp: usize) -> (usize, usize) {
    if tokens.is_empty() {
        (0, total_pp)
    } else if i == 0 {
        (0, tokens[0].offset)
    } else if i == tokens.len() {
        (tokens[i - 1].end(), total_pp)
    } else {
        (tokens[i - 1].end(), tokens[i].offset)
    }
}

fn orig_gap(
    tokens: &[Token<'_>],
    starts: &[usize],
    ends: &[usize],
    i: usize,
    total_orig: usize,
) -> (usize, usize) {
    if tokens.is_empty() {
        (0, total_orig)
    } else if i == 0 {
        (0, starts[0])
    } else if i == tokens.len() {
        (ends[i - 1], total_orig)
    } else {
        (ends[i - 1], starts[i])
    }
}

/// One directive emission unit, anchored at a single original byte.
/// Each `\`ifdef` / `\`else` / `\`endif` / branch body / empty macro
/// call becomes its own entry; whole-chain expansion happens at
/// collection time.
#[derive(Clone)]
struct DirectiveEntry {
    /// Anchor — the byte where the segment "begins" in the original
    /// source. Used to bucket the entry into a trivia slot.
    anchor: usize,
    /// Half-open range of bytes the entry consumes. The classifier
    /// jumps the cursor to `consume.end` after emitting the entry, so
    /// the same bytes don't get scanned a second time as plain text.
    consume: PpRange,
    segment: TriviaSegment,
}

fn collect_directives(parsed: &Parsed, intervals: &[(usize, usize)]) -> Vec<DirectiveEntry> {
    // Single tree walk: gather IfdefChain records and every skipped-body
    // range together. Then run a linear sweep to find the outer-most
    // skipped bodies, and drop chains nested inside one (the outer's
    // `SkippedBody` segment already covers them verbatim).
    let mut chains: Vec<(PpRange, &vuff_sv_ast::IfdefChain)> = Vec::new();
    let mut entries: Vec<DirectiveEntry> = Vec::new();
    let mut all_skipped: Vec<(usize, usize)> = Vec::new();

    for d in parsed.tree.directives() {
        if d.original_path != parsed.original_path {
            continue;
        }
        match d.kind {
            DirectiveKind::IfdefChain => {
                let DirectiveDetail::IfdefChain(ref chain) = d.detail else {
                    continue;
                };
                for b in &chain.branches {
                    if !b.taken {
                        if let Some(body) = &b.body_original_range {
                            all_skipped.push((body.begin, body.end));
                        }
                    }
                }
                chains.push((d.original_range, chain));
            }
            DirectiveKind::MacroUsage if d.pp_range.is_none() => {
                let DirectiveDetail::MacroUsage(ref usage) = d.detail else {
                    continue;
                };
                let chain_extra = chain_depth_at(intervals, d.original_range.begin);
                entries.push(DirectiveEntry {
                    anchor: d.original_range.begin,
                    consume: d.original_range,
                    segment: TriviaSegment::EmptyMacroCall {
                        call_text: usage.call_text.trim_end().to_owned(),
                        chain_extra,
                    },
                });
            }
            _ => {
                // Other directive flavors that the preprocessor
                // preserved in the post-pp text reach the classifier
                // as ordinary `\``-leading lines via line scanning.
            }
        }
    }

    let outer_skipped = outermost_ranges(all_skipped);
    for (range, chain) in chains {
        if range_inside_any(&range, &outer_skipped) {
            continue;
        }
        expand_chain(parsed, &range, chain, intervals, &mut entries);
    }
    entries.sort_by_key(|e| e.anchor);
    entries
}

/// Pick the outermost ranges from a set of `(begin, end)` pairs that are
/// either disjoint or strictly nested. Linear after sort.
fn outermost_ranges(mut ranges: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    if ranges.is_empty() {
        return ranges;
    }
    // Sort by begin asc; ties → end desc so a wrapping range comes first.
    ranges.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    let mut outer: Vec<(usize, usize)> = Vec::new();
    let mut current_end: usize = 0;
    for r in ranges {
        if outer.is_empty() || r.0 >= current_end {
            current_end = r.1;
            outer.push(r);
        }
    }
    outer
}

fn expand_chain(
    parsed: &Parsed,
    chain_range: &PpRange,
    chain: &vuff_sv_ast::IfdefChain,
    intervals: &[(usize, usize)],
    entries: &mut Vec<DirectiveEntry>,
) {
    for b in &chain.branches {
        if let Some((text, _leading_ws, kw_start, line_end)) =
            extract_directive_keyword_line(&parsed.original, &b.keyword_original_range)
        {
            // Keyword sits *between* branches, never inside any of its
            // own chain's bodies — `chain_extra` here counts only outer
            // enclosing chains.
            let chain_extra = chain_depth_at(intervals, kw_start);
            entries.push(DirectiveEntry {
                anchor: kw_start,
                consume: PpRange {
                    begin: kw_start,
                    end: line_end,
                },
                segment: TriviaSegment::IfdefKeyword { text, chain_extra },
            });
        }
        if !b.taken {
            if let Some(body) = &b.body_original_range {
                if let Some((text, body_begin, consume_end)) =
                    extract_skipped_body(&parsed.original, body)
                {
                    if !text.is_empty() {
                        // Query at `body.begin` (a position guaranteed
                        // to be inside this branch's body interval) so
                        // the count includes this chain itself; the
                        // entry's anchor we publish is `body_begin`,
                        // which may sit a byte earlier on the line.
                        let chain_extra = chain_depth_at(intervals, body.begin);
                        entries.push(DirectiveEntry {
                            anchor: body_begin,
                            consume: PpRange {
                                begin: body_begin,
                                end: consume_end,
                            },
                            segment: TriviaSegment::SkippedBody { text, chain_extra },
                        });
                    }
                }
            }
        }
    }
    if let Some((text, _leading_ws, kw_start, line_end)) =
        extract_endif(&parsed.original, chain_range)
    {
        let chain_extra = chain_depth_at(intervals, kw_start);
        entries.push(DirectiveEntry {
            anchor: kw_start,
            consume: PpRange {
                begin: kw_start,
                end: line_end,
            },
            segment: TriviaSegment::IfdefKeyword { text, chain_extra },
        });
    }
}

fn range_inside_any(r: &PpRange, ranges: &[(usize, usize)]) -> bool {
    ranges.iter().any(|&(b, e)| b <= r.begin && r.end <= e)
}

fn compute_token_origin_ranges(parsed: &Parsed, tokens: &[Token<'_>]) -> (Vec<usize>, Vec<usize>) {
    let mut starts = Vec::with_capacity(tokens.len());
    let mut ends = Vec::with_capacity(tokens.len());
    let mut last_end: usize = 0;
    for t in tokens {
        let start = parsed
            .origin_in_original(t.offset)
            .unwrap_or(last_end)
            .max(last_end);
        // Map the token's last byte to its origin and add 1 for the
        // exclusive end. Falls back to `start + len` if the token's
        // last byte cannot be mapped (synthesized / cross-file).
        let last_byte = t.end().saturating_sub(1);
        let end = parsed
            .origin_in_original(last_byte)
            .map_or(start + t.len, |p| p + 1)
            .max(start + 1);
        starts.push(start);
        ends.push(end);
        last_end = end;
    }
    (starts, ends)
}

fn classify(
    parsed: &Parsed,
    directives: &[DirectiveEntry],
    intervals: &[(usize, usize)],
    range: Range<usize>,
    is_leading: bool,
) -> Vec<TriviaSegment> {
    let mut out: Vec<TriviaSegment> = Vec::new();
    if range.start >= range.end {
        return out;
    }
    let src = &parsed.original;
    let window = src.get(range.start..range.end).unwrap_or("");
    if window.is_empty() {
        return out;
    }

    // Locate the first directive entry whose anchor falls inside this
    // slot. Entries are sorted by anchor; we walk forward only.
    let mut dir_idx = directives.partition_point(|e| e.anchor < range.start);

    let mut cursor = range.start;
    let mut blank_run: u32 = 0;
    // Leading slot has no previous token to share a physical line with —
    // any first-line content there is on its own line.
    let mut crossed_newline = is_leading;
    // Two regimes:
    //   * Initial slot bytes follow a token's last char, so the slot's
    //     first `\n` is the prev token's terminator — threshold 2.
    //   * After we emit a comment / directive segment, that line's `\n`
    //     was already eaten by `had_newline` (or by the explicit skip
    //     in the directive-entry path), so any further `\n` already
    //     means a blank line — threshold 1.
    let mut consumed_terminator = is_leading;
    let blank_threshold = |consumed: bool| -> u32 { if consumed { 1 } else { 2 } };

    while cursor < range.end {
        // Drop directive entries whose anchor is before the cursor
        // (already past).
        while dir_idx < directives.len() && directives[dir_idx].anchor < cursor {
            dir_idx += 1;
        }
        // If a directive anchors at the cursor (or anywhere up to the
        // current line's end), emit it and jump the cursor past its
        // consume range.
        if dir_idx < directives.len() && directives[dir_idx].anchor < range.end {
            let entry = &directives[dir_idx];
            if entry.anchor > cursor {
                consume_plain_lines(
                    src,
                    cursor,
                    entry.anchor,
                    intervals,
                    &mut out,
                    &mut blank_run,
                    &mut crossed_newline,
                    &mut consumed_terminator,
                );
            }
            if blank_run >= blank_threshold(consumed_terminator) {
                out.push(TriviaSegment::Blank);
            }
            blank_run = 0;
            out.push(entry.segment.clone());
            // Eat the directive's line-ending `\n` so the next
            // plain-text scan starts on the following physical line.
            // Without this, the directive's own terminator inflates
            // `blank_run` and a real blank-line below would emit two
            // gaps where one is intended.
            let mut next = entry.consume.end.max(cursor + 1);
            if next < range.end && src.as_bytes().get(next) == Some(&b'\n') {
                next += 1;
            }
            cursor = next;
            crossed_newline = true;
            consumed_terminator = true;
            dir_idx += 1;
            continue;
        }

        // No more directives in this slot — consume the rest as plain
        // text lines.
        consume_plain_lines(
            src,
            cursor,
            range.end,
            intervals,
            &mut out,
            &mut blank_run,
            &mut crossed_newline,
            &mut consumed_terminator,
        );
        if blank_run >= blank_threshold(consumed_terminator) {
            out.push(TriviaSegment::Blank);
        }
        blank_run = 0;
        cursor = range.end;
    }

    out
}

/// Walk physical lines in `[from, to)` and append classified segments.
/// Blank lines are accumulated in `blank_run`; the caller flushes one
/// `Blank` segment when transitioning to a directive or end-of-slot.
///
/// Multi-line `/* … */` comments are accumulated into a single
/// `BlockComment` segment whose `text` preserves the interior verbatim
/// (including original leading whitespace on continuation lines).
fn consume_plain_lines(
    src: &str,
    from: usize,
    to: usize,
    intervals: &[(usize, usize)],
    out: &mut Vec<TriviaSegment>,
    blank_run: &mut u32,
    crossed_newline: &mut bool,
    consumed_terminator: &mut bool,
) {
    let mut cursor = from;
    let mut in_block = false;
    while cursor < to {
        let nl = src[cursor..to].find('\n');
        let line_end = nl.map_or(to, |i| cursor + i + 1);
        let had_newline = nl.is_some();
        let raw = &src[cursor..line_end];
        let line = raw.trim_end_matches('\n').trim_end_matches([' ', '\t']);
        let trimmed = line.trim_start_matches([' ', '\t']);

        if in_block {
            if let Some(TriviaSegment::BlockComment { text, .. }) = out.last_mut() {
                text.push('\n');
                text.push_str(line);
            }
            in_block = block_state_after(in_block, line);
            if had_newline {
                *crossed_newline = true;
            }
            cursor = line_end;
            continue;
        }

        if trimmed.is_empty() {
            if had_newline {
                *blank_run += 1;
            }
        } else if let Some(seg) = classify_line(
            trimmed,
            chain_depth_at(intervals, cursor),
            *crossed_newline,
            had_newline,
        ) {
            let threshold: u32 = if *consumed_terminator { 1 } else { 2 };
            if *blank_run >= threshold {
                out.push(TriviaSegment::Blank);
            }
            *blank_run = 0;
            out.push(seg);
            in_block = block_state_after(false, line);
            // This line's `\n` was eaten by `had_newline` and counts as
            // the next emission's terminator — drop the threshold to 1.
            *consumed_terminator = true;
        } else {
            *blank_run = 0;
        }
        if had_newline {
            *crossed_newline = true;
        }
        cursor = line_end;
    }
}

/// Classify a single non-empty trimmed line of original source. Returns
/// `None` when the line is plain code (which means the trivia window
/// boundaries straddled a token — caller drops the line).
fn classify_line(
    trimmed: &str,
    chain_extra: u32,
    nl_before: bool,
    nl_after: bool,
) -> Option<TriviaSegment> {
    if trimmed.starts_with("//") {
        return Some(TriviaSegment::LineComment {
            text: trimmed.to_owned(),
            nl_before,
            nl_after,
            chain_extra,
        });
    }
    if trimmed.starts_with("/*") {
        return Some(TriviaSegment::BlockComment {
            text: trimmed.to_owned(),
            nl_before,
            nl_after,
            chain_extra,
        });
    }
    if trimmed.starts_with('`') {
        // Active surviving directive line picked up by line scanning.
        // Stripped chain keywords (`ifdef` / `endif` / etc.) reach the
        // classifier through `directives` and are emitted there; if we
        // also see one here it means the parser didn't index it (rare
        // edge), in which case still preserve it.
        let kind = first_word_after_backtick(trimmed);
        let seg = if matches!(kind, "ifdef" | "ifndef" | "elsif" | "else" | "endif") {
            TriviaSegment::IfdefKeyword {
                text: trimmed.to_owned(),
                chain_extra,
            }
        } else {
            TriviaSegment::DirectiveLine {
                text: trimmed.to_owned(),
                chain_extra,
            }
        };
        return Some(seg);
    }
    None
}

fn first_word_after_backtick(s: &str) -> &str {
    let after = s.trim_start_matches('`');
    let end = after
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(after.len());
    &after[..end]
}

/// `` `<keyword> `` line. sv-parser-pp's `keyword_original_range` covers
/// only the bare keyword text; we back up over the leading `` ` `` and
/// up to end-of-line (trailing whitespace stripped).
///
/// Returns `(text, leading_ws, directive_start, line_end_exclusive)`.
fn extract_directive_keyword_line(
    src: &str,
    keyword_range: &PpRange,
) -> Option<(String, String, usize, usize)> {
    let kw_pos = keyword_range.begin;
    if kw_pos == 0 {
        return None;
    }
    let line_start = src[..kw_pos].rfind('\n').map_or(0, |i| i + 1);
    let lead = &src[line_start..kw_pos];
    let lead_trimmed = lead.trim_start_matches([' ', '\t']);
    if lead_trimmed != "`" {
        return None;
    }
    let directive_start = kw_pos - 1;
    let leading_ws = src[line_start..directive_start].to_owned();
    let line_end = src[directive_start..]
        .find('\n')
        .map_or(src.len(), |i| directive_start + i);
    let text = src[directive_start..line_end].trim_end().to_owned();
    Some((text, leading_ws, directive_start, line_end))
}

fn extract_endif(src: &str, chain: &PpRange) -> Option<(String, String, usize, usize)> {
    let slice = src.get(chain.begin..chain.end)?;
    let rel = slice.rfind("`endif")?;
    let kw_pos = chain.begin + rel + 1;
    let kw_end = kw_pos + "endif".len();
    extract_directive_keyword_line(
        src,
        &PpRange {
            begin: kw_pos,
            end: kw_end,
        },
    )
}

/// Extract the verbatim body text of a `taken=false` branch.
///
/// Returns `(text, body_begin, consume_end)`:
///   * `text` — the body bytes as the formatter should emit them, with
///     original indent on each line and trailing `\n` / horizontal
///     whitespace stripped.
///   * `body_begin` — the line-start anchor (used as the slot key).
///   * `consume_end` — the position the classifier should skip its
///     cursor to. Equals the byte just past the body's final `\n`,
///     which is typically the start of the next directive's line.
///     Clamping here matters: sv-parser-pp's `body_original_range.end`
///     may extend into the next directive's leading whitespace, and
///     unclamped that would cause the next directive's keyword line
///     to be silently skipped.
fn extract_skipped_body(src: &str, body: &PpRange) -> Option<(String, usize, usize)> {
    let line_start = src[..body.begin].rfind('\n').map_or(0, |i| i + 1);
    let begin = line_start.min(body.begin);
    let raw = src.get(begin..body.end)?;
    let trimmed = raw
        .trim_end_matches([' ', '\t'])
        .strip_suffix('\n')
        .unwrap_or_else(|| raw.trim_end_matches([' ', '\t']));
    // The final `\n` we just stripped is at position
    // `begin + trimmed.len()`; consuming up to and including it puts
    // the cursor at the start of the next line, which is where the
    // next directive's keyword line begins.
    let consume_end = (begin + trimmed.len() + 1).min(body.end);
    Some((trimmed.to_owned(), begin, consume_end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use vuff_sv_ast::{parse, tokens};

    fn build_for(src: &str) -> (Parsed, TriviaMap) {
        let parsed = parse(src, &PathBuf::from("test.sv")).expect("parse ok");
        let toks = tokens(&parsed.tree);
        let map = build(&parsed, &toks);
        (parsed, map)
    }

    #[test]
    fn empty_source_one_slice() {
        let (_p, map) = build_for("");
        assert_eq!(map.slices.len(), 1, "one slice for tokenless input");
    }

    #[test]
    fn no_trivia_between_tokens() {
        let (_p, map) = build_for("module m;endmodule\n");
        for (i, s) in map.slices.iter().enumerate() {
            assert!(
                s.segments.is_empty(),
                "slot {i} should be segment-free: {:?}",
                s.segments
            );
        }
    }

    #[test]
    fn line_comment_classified() {
        let src = "module m;\n// hi\nendmodule\n";
        let (_p, map) = build_for(src);
        let any_line_comment = map.slices.iter().any(|s| {
            s.segments
                .iter()
                .any(|seg| matches!(seg, TriviaSegment::LineComment { .. }))
        });
        assert!(any_line_comment, "expected a LineComment somewhere");
    }

    #[test]
    fn blank_line_classified() {
        let src = "module m;\n\nendmodule\n";
        let (_p, map) = build_for(src);
        let blank_count: usize = map
            .slices
            .iter()
            .map(|s| {
                s.segments
                    .iter()
                    .filter(|s| matches!(s, TriviaSegment::Blank))
                    .count()
            })
            .sum();
        assert_eq!(blank_count, 1, "exactly one blank-line segment");
    }

    #[test]
    fn ifdef_chain_classified() {
        let src = "module m;\n`ifdef A\n  logic a;\n`else\n  logic b;\n`endif\nendmodule\n";
        let (_p, map) = build_for(src);
        let mut keyword_count = 0;
        let mut skipped_body_count = 0;
        for s in &map.slices {
            for seg in &s.segments {
                match seg {
                    TriviaSegment::IfdefKeyword { .. } => keyword_count += 1,
                    TriviaSegment::SkippedBody { .. } => skipped_body_count += 1,
                    _ => {}
                }
            }
        }
        // `ifdef A`, `else`, `endif` → 3 keywords. `B` body skipped.
        assert_eq!(keyword_count, 3, "ifdef/else/endif keywords");
        assert_eq!(skipped_body_count, 1, "one skipped body (else branch)");
    }
}
