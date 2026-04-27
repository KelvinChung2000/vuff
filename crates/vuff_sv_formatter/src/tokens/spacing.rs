//! Adjacency spacing rules. Decides whether a space is required, forbidden,
//! or neutral between two consecutive tokens. Pure functions — the builder
//! layers `force_space_between` over `no_space_between` so an explicit
//! force always wins over a forbid.
//!
//! Why string match (not CST) here: every rule below operates on
//! single-character or short fixed-symbol punctuation (`;`, `,`, `(`, `[`,
//! `::`, `==`, etc.). These are unambiguous at the lexical level — no SV
//! grammar production renames `;` or `,`, and `is_binary_operator` covers
//! exactly the operator symbols that always mean "binary op" in any
//! context where they appear. Where the string would be ambiguous
//! (`<<`/`>>` as shift vs. streaming-concat direction; `*` as multiply
//! vs. wildcard import), the disambiguation lives outside this file and
//! is delivered as a per-token CST mask — see
//! `crate::expr::streaming_concat_mask` and the `prev == "::"` short-
//! circuit below (which mirrors the existing `no_space_after` rule).

/// Tokens that should not be preceded by a space.
/// Note: `:` is deliberately NOT here — it is unambiguous only with CST
/// context (case label vs range vs ternary). We let the input decide.
fn no_space_before(t: &str) -> bool {
    matches!(t, ";" | "," | ")" | "]" | "." | "::" | "++" | "--")
}

/// Tokens that should not be followed by a space.
fn no_space_after(t: &str) -> bool {
    matches!(t, "(" | "[" | "." | "::" | "@")
}

pub(crate) fn no_space_between(prev: &str, curr: &str) -> bool {
    no_space_after(prev) || no_space_before(curr)
}

/// Binary operators that should always have a space on both sides.
/// Intentionally excludes `-` and `&`/`|`/`^`/`~` because they are also
/// unary — distinguishing needs CST context.
fn is_binary_operator(t: &str) -> bool {
    matches!(
        t,
        "+" | "*"
            | "/"
            | "%"
            | "=="
            | "!="
            | "==="
            | "!=="
            | "<="
            | ">="
            | "<"
            | ">"
            | "&&"
            | "||"
            | "<<"
            | ">>"
            | "<<<"
            | ">>>"
            | "**"
            | "?"
    )
}

/// Adjacencies that must have a space even if the input has none.
pub(crate) fn force_space_between(prev: &str, curr: &str) -> bool {
    // Attribute open `(*` must be followed by a space, and `*)` preceded by one.
    if prev == "(*" {
        return true;
    }
    if curr == "*)" {
        return true;
    }
    // Statement/list terminators are never preceded by a space, even if
    // the previous token would otherwise force one (e.g. wildcard `*` in
    // `pkg::*;` — the `*` looks like a binary multiply by string match).
    // The `*)` attribute closer is handled above and excluded here.
    if matches!(curr, ";" | "," | ")" | "]" | "}") {
        return false;
    }
    // Wildcard import: `pkg::*` — the `*` is not a binary multiply here.
    // The string-based binary-op rule below would otherwise force a space
    // around it. `::` always glues to whatever follows.
    if prev == "::" {
        return false;
    }
    // Comma followed by any non-closing token.
    if prev == "," && !matches!(curr, ")" | "]" | "}") {
        return true;
    }
    // `)` followed by `(` — group/port list closing then another opening.
    if prev == ")" && curr == "(" {
        return true;
    }
    // `=` gets whitespace on both sides (standalone `=`, not `<=`/`>=`/`==`/`!=`).
    if prev == "=" || curr == "=" {
        return true;
    }
    // Binary operators: space on both sides.
    if is_binary_operator(prev) || is_binary_operator(curr) {
        return true;
    }
    false
}
