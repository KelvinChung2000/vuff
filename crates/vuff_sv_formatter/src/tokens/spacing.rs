//! Adjacency spacing rules. Decides whether a space is required, forbidden,
//! or neutral between two consecutive tokens. Pure functions — the builder
//! layers `force_space_between` over `no_space_between` so an explicit
//! force always wins over a forbid.

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
