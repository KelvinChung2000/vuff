//! Ternary chain detection for multi-line `?:` alignment.
//!
//! A "chain" here is a set of nested `ConditionalExpression`s whose
//! `?` operators should share a column when the user wrote them across
//! multiple lines. The trigger is per-spec: when at least one `:` in
//! the chain is followed by a newline in the original source, the
//! whole chain renders as multi-line and every `?` aligns. Single-line
//! chains keep their inline shape.
//!
//! sv-parser emits a `ConditionalExpression` whose `nodes.1` is the
//! `?` Symbol and `nodes.4` is the `:` Symbol; the inner `Locate`'s
//! `offset` is the byte offset in the preprocessed source. We treat
//! every nested `ConditionalExpression` as part of the surrounding
//! chain — that captures both the right-recursive `else` form and the
//! handful of expressions that nest a ternary inside a sub-expression.
//! For the common "newline after `:`" pattern this matches user
//! intuition.
//!
//! Output: a per-`?` map giving (chain id, position in chain) plus a
//! per-chain "is multiline" flag. Verbatim consumes both during
//! emission.
//!
//! `position_in_chain` is the order in which `?`s appear in source;
//! the first `?` becomes the chain anchor (its column is recorded at
//! emission time and subsequent `?`s pad to it).

use std::collections::HashMap;

use vuff_sv_ast::{NodeEvent, RefNode, SyntaxTree, Token};

use crate::context::build_token_index;

#[derive(Debug, Default)]
pub(crate) struct TernaryChainInfo {
    /// Token-index of `?` → `(chain_id, position_in_chain)`. Only
    /// `?` tokens that belong to a multi-line chain are present;
    /// single-line chains contribute nothing.
    pub(crate) by_q_tok: HashMap<usize, (usize, usize)>,
    /// Token-index of `:` → chain_id. Marks the colons that fall
    /// inside a multi-line chain — the formatter consults this so
    /// continuation lines (the next CE's cond) can be aligned to the
    /// chain's start column.
    pub(crate) by_colon_tok: HashMap<usize, usize>,
    /// Token index of the chain's leftmost token → chain id. The
    /// formatter records the column at that token's emission as the
    /// chain's start; continuations pad up to it.
    pub(crate) first_tok: HashMap<usize, usize>,
    /// Chain id → maximum cond width (in source bytes, trailing
    /// whitespace trimmed) across the chain's CEs. Used at emission
    /// to compute the chain-wide `?` anchor column.
    pub(crate) max_cond_width: HashMap<usize, u32>,
}

pub(crate) fn build_ternary_chains(
    tree: &SyntaxTree,
    tokens: &[Token<'_>],
    source: &str,
) -> TernaryChainInfo {
    let tok_idx = build_token_index(tokens);
    // Each chain frame collects the `?` token indices it has seen and
    // whether any `:` so far was followed by a newline.
    struct Frame {
        chain_id: usize,
        questions: Vec<usize>,
        multiline: bool,
    }
    let mut stack: Vec<Frame> = Vec::new();
    let mut next_chain_id: usize = 0;
    let mut by_q_tok: HashMap<usize, (usize, usize)> = HashMap::new();
    let mut by_colon_tok: HashMap<usize, usize> = HashMap::new();
    let mut first_tok: HashMap<usize, usize> = HashMap::new();
    // Set when entering a chain root; cleared on the next Locate event
    // (which gives us the chain's leftmost token).
    let mut pending_first_capture: Option<usize> = None;
    // Per-CE: the byte offset of the cond's first token. Set when
    // entering a CE and cleared by the next Locate. We need it for
    // every CE in a chain, not just the root.
    let mut pending_cond_offsets: Vec<(usize, usize)> = Vec::new(); // (chain_id, q_offset)
    let mut cond_widths: HashMap<usize, Vec<u32>> = HashMap::new();
    // For each chain id we want to commit, the colons collected so
    // far. (Chains are committed on the root CE's Leave.)
    let mut chain_colons: HashMap<usize, Vec<usize>> = HashMap::new();
    // Buffered chains; we commit them on the outermost CE Leave.
    let mut finished: Vec<(usize, Vec<usize>, bool)> = Vec::new();

    for ev in tree.into_iter().event() {
        match ev {
            NodeEvent::Enter(RefNode::ConditionalExpression(ce)) => {
                // `?` is at nodes.1, `:` is at nodes.4. Pull both
                // token indices via the offset map.
                let q_offset = ce.nodes.1.nodes.0.offset;
                let colon_offset = ce.nodes.4.nodes.0.offset;
                let Some(&q_idx) = tok_idx.get(&q_offset) else {
                    continue;
                };
                let Some(&colon_idx) = tok_idx.get(&colon_offset) else {
                    continue;
                };
                // Determine chain id: nest into parent's chain if any.
                let chain_id = if let Some(parent) = stack.last() {
                    parent.chain_id
                } else {
                    let id = next_chain_id;
                    next_chain_id += 1;
                    id
                };

                // Look at the bytes between the `:` and the next
                // significant char to decide whether this colon
                // forces multi-line shape on its chain.
                let colon_end = tokens[colon_idx].end();
                let trailing = source.get(colon_end..).unwrap_or("");
                let has_newline = trailing
                    .chars()
                    .take_while(|c| c.is_whitespace())
                    .any(|c| c == '\n');

                // Track this CE's colon under its chain.
                chain_colons.entry(chain_id).or_default().push(colon_idx);
                // Track per-CE cond width. We need the cond's first
                // token offset, captured on the next Locate event.
                pending_cond_offsets.push((chain_id, q_offset));

                // Push a frame so children inherit chain_id.
                if let Some(parent) = stack.last_mut() {
                    parent.questions.push(q_idx);
                    if has_newline {
                        parent.multiline = true;
                    }
                    stack.push(Frame {
                        chain_id,
                        questions: Vec::new(),
                        multiline: false,
                    });
                } else {
                    pending_first_capture = Some(chain_id);
                    stack.push(Frame {
                        chain_id,
                        questions: vec![q_idx],
                        multiline: has_newline,
                    });
                }
            }
            NodeEvent::Enter(RefNode::Locate(loc)) => {
                if let Some(chain_id) = pending_first_capture.take() {
                    if let Some(&idx) = tok_idx.get(&loc.offset) {
                        first_tok.insert(idx, chain_id);
                    }
                }
                // Drain every pending cond capture (we may have entered
                // several CEs in a row, e.g. when the next Locate is
                // shared across nested CondPredicate openers). The
                // first Locate inside each CE is the cond's first
                // token, so its offset gives that cond's start.
                while let Some((chain_id, q_offset)) = pending_cond_offsets.pop() {
                    let raw = q_offset.saturating_sub(loc.offset);
                    // Trim the trailing whitespace between the cond's
                    // last byte and the `?` token by looking at the
                    // bytes immediately before `?`.
                    let trimmed = source
                        .get(loc.offset..q_offset)
                        .map(|s| s.trim_end().chars().count())
                        .unwrap_or(raw);
                    let width = u32::try_from(trimmed).unwrap_or(u32::MAX);
                    cond_widths.entry(chain_id).or_default().push(width);
                }
            }
            NodeEvent::Leave(RefNode::ConditionalExpression(_)) => {
                // Merge child frame back into parent (or commit when
                // outermost).
                if let Some(child) = stack.pop() {
                    if let Some(parent) = stack.last_mut() {
                        if parent.chain_id == child.chain_id {
                            parent.questions.extend(child.questions);
                            if child.multiline {
                                parent.multiline = true;
                            }
                        } else {
                            // Distinct chain; commit it.
                            finished.push((child.chain_id, child.questions, child.multiline));
                        }
                    } else {
                        finished.push((child.chain_id, child.questions, child.multiline));
                    }
                }
            }
            _ => {}
        }
    }
    // Drain any remaining frames (should be zero in well-formed CST).
    while let Some(rem) = stack.pop() {
        finished.push((rem.chain_id, rem.questions, rem.multiline));
    }

    let mut multiline_chains: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut max_cond_width: HashMap<usize, u32> = HashMap::new();
    for (chain_id, mut questions, multiline) in finished {
        if !multiline {
            continue;
        }
        multiline_chains.insert(chain_id);
        // Walk order pushed the parent CE's `?` first even though its
        // token offset is larger than nested CEs' `?`s (the parent
        // covers the inner CE in its `cond` slot). Re-sort by token
        // index so position 0 is the leftmost `?` in source — that's
        // the one whose column we record as the chain's anchor at
        // emission time.
        questions.sort_unstable();
        for (pos, q_idx) in questions.into_iter().enumerate() {
            by_q_tok.insert(q_idx, (chain_id, pos));
        }
        if let Some(colons) = chain_colons.remove(&chain_id) {
            for colon_idx in colons {
                by_colon_tok.insert(colon_idx, chain_id);
            }
        }
        if let Some(widths) = cond_widths.remove(&chain_id) {
            if let Some(&w) = widths.iter().max() {
                max_cond_width.insert(chain_id, w);
            }
        }
    }
    // Drop entries for chains that turned out to be single-line.
    first_tok.retain(|_, chain_id| multiline_chains.contains(chain_id));

    TernaryChainInfo {
        by_q_tok,
        by_colon_tok,
        first_tok,
        max_cond_width,
    }
}
