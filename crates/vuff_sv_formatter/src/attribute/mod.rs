//! Annex A.9.1 — `(* ... *)` attribute instances. Owns the decision of
//! single-line vs multi-line layout for each attribute.

pub(crate) mod spans;

pub(crate) use spans::force_nl_before_mask;
