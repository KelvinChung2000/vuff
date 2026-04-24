//! Annex A.1.3 / A.1.4 — module declarations and their items.

pub(crate) mod module_declaration;
pub(crate) mod spans;

pub(crate) use module_declaration::ModuleDeclarationRule;
pub(crate) use spans::{find_module_spans, ModuleSpan};
