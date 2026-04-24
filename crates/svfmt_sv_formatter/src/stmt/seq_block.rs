//! Annex A.6.3 — `seq_block` (`begin ... end`) and `par_block`
//! (`fork ... join*`). Today this module owns a single decision: whether
//! `begin_style = allman` should force a line break before a `begin`
//! token that currently sits on the same line as its predecessor.

use svfmt_config::{BeginStyle, FormatOptions};

/// Returns true when `t_text = "begin"` should be pushed onto its own
/// line (Allman style) because:
///
/// 1. The user selected `begin_style = allman`.
/// 2. The `begin` is not the very first token in the range being formatted.
/// 3. The original source had no newline between the previous token and
///    this one — we only rewrite shape when the user cuddled the block
///    opener; otherwise we preserve their explicit layout.
pub(crate) fn wants_allman_break(
    opts: &FormatOptions,
    t_text: &str,
    between: &str,
    prev_text: Option<&str>,
) -> bool {
    matches!(opts.begin_style, BeginStyle::Allman)
        && t_text == "begin"
        && !between.contains('\n')
        && prev_text.is_some()
}
