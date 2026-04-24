//! Annex A.8 — expressions. Each sub-module owns one expression shape.

pub(crate) mod assignment_pattern;
pub(crate) mod call_paren;
pub(crate) mod concat;
pub(crate) mod conditional;
pub(crate) mod select;

pub(crate) use assignment_pattern::apostrophe_brace_mask;
pub(crate) use call_paren::call_open_paren_mask;
pub(crate) use concat::concat_brace_masks;
pub(crate) use conditional::ternary_colon_mask;
pub(crate) use select::select_open_bracket_mask;
