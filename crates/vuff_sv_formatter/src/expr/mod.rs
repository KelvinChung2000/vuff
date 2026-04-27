//! Annex A.8 — expressions. Each sub-module owns one expression shape.

pub(crate) mod assignment_pattern;
pub(crate) mod call_paren;
pub(crate) mod concat;
pub(crate) mod conditional;
pub(crate) mod macro_calls;
pub(crate) mod select;
pub(crate) mod streaming;
pub(crate) mod ternary_chain;

pub(crate) use assignment_pattern::apostrophe_brace_mask;
pub(crate) use call_paren::call_open_paren_mask;
pub(crate) use concat::concat_brace_masks;
pub(crate) use conditional::ternary_colon_mask;
pub(crate) use macro_calls::{build_macro_calls, MacroCallInfo};
pub(crate) use select::select_open_bracket_mask;
pub(crate) use streaming::streaming_concat_mask;
pub(crate) use ternary_chain::{build_ternary_chains, TernaryChainInfo};
