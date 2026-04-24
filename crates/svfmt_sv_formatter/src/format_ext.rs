//! The core per-node formatting trait. Every SystemVerilog CST node kind
//! will eventually implement [`Format`] in its own module under
//! `source_text/`, `module/`, `stmt/`, `expr/`, etc. Today only the
//! synthetic `SourceTextRoot` rule exists; everything else is delegated
//! to `verbatim`.

use crate::context::{FormatCtx, Formatter};

/// A rule that knows how to emit IR for a particular CST node or view.
///
/// Rules are stateless wrappers around the data they format; all mutable
/// emission goes through `f`, all read-only inputs through `ctx`.
pub(crate) trait Format {
    fn fmt(&self, ctx: &FormatCtx<'_>, f: &mut Formatter);
}
