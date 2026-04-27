//! Punctuation tables that classify tokens by their bracket role. Pure
//! data — no state, no context dependency.
//!
//! All keyword block openers / closers (`begin`, `end`, `module`,
//! `endmodule`, `function`, `endfunction`, `generate`, `endgenerate`,
//! `case`, `endcase`, `fork`, `join*`, …) used to live here too, in a
//! `resets_statement` table that the verbatim engine consulted to
//! decide statement-continuation reset. That table has been migrated to
//! `crate::stmt::reset_mask::statement_reset_mask`, which derives the
//! same set from the CST so new SV constructs can't silently drift past
//! it. What remains here is just the four bracket-pair punctuators —
//! they have no CST node distinguishing them from "regular" `(` etc.

/// Bracket pairs that open a nesting level. Tokens here contribute +1 to
/// the verbatim token-depth state on emission.
///
/// All *keyword* block openers (`begin`, `fork`, `case`, `generate`, …)
/// are omitted here. Their scope is derived from the CST in
/// `indent_map::cst_depth_map` — the CST knows exactly which nodes are
/// body-items of which blocks, so no hand-maintained keyword list is
/// needed. This table is now only the four paired bracket punctuators.
pub(crate) fn is_opener(t: &str) -> bool {
    matches!(t, "(" | "[" | "{" | "(*")
}

/// Bracket closers. Keyword closers (`end`, `endcase`, `join*`, …) are
/// handled by the CST indent map.
pub(crate) fn is_closer(t: &str) -> bool {
    matches!(t, ")" | "]" | "}" | "*)")
}
