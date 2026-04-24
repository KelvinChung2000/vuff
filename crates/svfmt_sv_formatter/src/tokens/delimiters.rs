//! Keyword and punctuation tables that classify tokens by their layout
//! role. Pure data — no state, no context dependency.

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

/// Tokens that terminate the "statement continuation" scope: after any of
/// these, an intervening newline indents at the raw structural depth, not
/// at depth+1.
pub(crate) fn resets_statement(t: &str) -> bool {
    matches!(
        t,
        ";" | "begin"
            | "end"
            | "endcase"
            | "endmodule"
            | "endinterface"
            | "endfunction"
            | "endtask"
            | "endclass"
            | "endpackage"
            | "endgenerate"
            | "endspecify"
            | "endchecker"
            | "endgroup"
            | "endprimitive"
            | "endconfig"
            | "endprogram"
            | "fork"
            | "join"
            | "join_any"
            | "join_none"
    )
}

/// Block keywords that may be followed by an optional `: label`. Used by
/// the statement-continuation state machine to keep `in_statement = false`
/// across the `: label` tail before the block body starts on the next
/// line.
pub(crate) fn allows_trailing_label(t: &str) -> bool {
    resets_statement(t) && t != ";"
}

