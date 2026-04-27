//! Pair runs of macro-expanded post-pp tokens with the original call-site
//! text, so the formatter re-emits `` `my_macro(args) `` instead of the
//! expanded body — i.e. format-don't-expand.
//!
//! Source of truth is sv-parser-pp's `DirectiveSpan` list (our fork's
//! additive API): each [`DirectiveKind::MacroUsage`] entry already
//! carries the call-site text and the post-pp byte range its expansion
//! occupies. We translate that range to a contiguous run of token
//! indices via `partition_point`. Verbatim emits the call-site text at
//! the run's first token and skips the rest.
//!
//! Cases the previous text-scan implementation missed but this one
//! catches:
//! * Macros whose body is a pure parameter pass-through
//!   (`` `define wrap(x) x ``) — sv-parser-pp records the call site
//!   regardless of whether the body has its own tokens.
//! * Nested macro calls — sv-parser-pp emits a separate usage record
//!   per call site; the outer run subsumes its inner runs naturally
//!   because their token indices fall inside the outer's.

use std::collections::{HashMap, HashSet};

use vuff_sv_ast::{DirectiveDetail, DirectiveKind, Parsed, Token};

#[derive(Debug)]
pub(crate) struct MacroRun {
    /// First token index in the run.
    pub(crate) start: usize,
    /// Last token index in the run (inclusive).
    pub(crate) end: usize,
    /// Original-source text of the macro call site (e.g.
    /// `` `assert(condition) ``).
    pub(crate) call_text: String,
}

#[derive(Debug, Default)]
pub(crate) struct MacroCallInfo {
    /// Token index of run start → run details. Verbatim consults this
    /// at each token: a hit on the run start emits `call_text` and
    /// jumps the cursor past the run; subsequent indices in
    /// `skip_tok` are silently skipped.
    pub(crate) run_at_start: HashMap<usize, MacroRun>,
    /// Token indices that are part of a macro run but not the first
    /// token. Verbatim must skip them.
    pub(crate) skip_tok: HashSet<usize>,
}

pub(crate) fn build_macro_calls(parsed: &Parsed, tokens: &[Token<'_>]) -> MacroCallInfo {
    if tokens.is_empty() {
        return MacroCallInfo::default();
    }
    let token_offsets: Vec<usize> = tokens.iter().map(|t| t.offset).collect();
    let mut run_at_start: HashMap<usize, MacroRun> = HashMap::new();
    let mut skip_tok: HashSet<usize> = HashSet::new();

    for d in parsed.tree.directives() {
        if d.kind != DirectiveKind::MacroUsage {
            continue;
        }
        let DirectiveDetail::MacroUsage(ref usage) = d.detail else {
            continue;
        };
        let Some(pp) = d.pp_range else {
            // Bodyless macro (`\`define EMPTY`) — nothing in the post-pp
            // stream to skip. Leaving the call site out is the legacy
            // behavior; preserving it is a follow-up.
            continue;
        };
        let start_idx = token_offsets.partition_point(|&o| o < pp.begin);
        if start_idx >= tokens.len() {
            continue;
        }
        if tokens[start_idx].offset >= pp.end {
            continue;
        }
        let mut end_idx = start_idx;
        while end_idx + 1 < tokens.len() && tokens[end_idx + 1].offset < pp.end {
            end_idx += 1;
        }
        // Nested usages: a tighter (inner) run may already occupy these
        // indices. Outer macros are recorded after their inner ones in
        // the visitation order sv-parser-pp uses, so the outer's wider
        // run reaches here second and overwrites — which is what we
        // want, since we re-emit the outer call-site text and skip
        // everything inside.
        for k in (start_idx + 1)..=end_idx {
            skip_tok.insert(k);
        }
        // Whoever wins start_idx owns the run. If a nested usage and
        // its outer share start_idx (the inner is the first token of
        // the outer's expansion) the outer's call_text replaces the
        // inner's — desired.
        run_at_start.insert(
            start_idx,
            MacroRun {
                start: start_idx,
                end: end_idx,
                call_text: usage.call_text.clone(),
            },
        );
    }

    MacroCallInfo {
        run_at_start,
        skip_tok,
    }
}
