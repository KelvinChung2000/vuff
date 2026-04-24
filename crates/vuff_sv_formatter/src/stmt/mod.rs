//! Annex A.6 — behavioral statements. Each sub-module owns one statement
//! shape (seq_block, if_statement, case_statement, assignment, …).

pub(crate) mod boundaries;
pub(crate) mod control_paren;
pub(crate) mod seq_block;

pub(crate) use boundaries::statement_boundary_mask;
pub(crate) use control_paren::control_header_paren_mask;
