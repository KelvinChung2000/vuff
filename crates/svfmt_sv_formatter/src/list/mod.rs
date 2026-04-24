//! Annex A.1.3 — list constructs that appear in module/interface/program
//! headers: `ListOfPorts`, `ListOfPortDeclarations`, `ParameterPortList`.
//! v0.1 scope: just enough CST awareness to drive spacing around the
//! opening `(` / `#(`.

pub(crate) mod instance_paren;
pub(crate) mod param_assign;
pub(crate) mod port_align;
pub(crate) mod port_align_render;
pub(crate) mod port_paren;

pub(crate) use instance_paren::force_space_before_instance_paren_mask;
pub(crate) use param_assign::param_assign_pound_mask;
pub(crate) use port_align::collect_port_lists;
pub(crate) use port_align_render::render_port_list;
pub(crate) use port_paren::force_space_before_port_paren_mask;
