//! Token-level classification and emission primitives shared across the
//! formatter. No CST awareness lives here — see per-node rule modules
//! (`module/`, `stmt/`, `expr/`, …) for shape-aware logic.

pub(crate) mod delimiters;
pub(crate) mod spacing;
pub(crate) mod trivia;
