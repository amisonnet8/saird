//! The small audited layer of Saird's trusted base (docs/design/trust.md):
//! parsing contracts and assemblies (docs/reference/syntax.md), types and
//! constraints, the evaluator of the expressions in ensures, when and where,
//! kv range templates and the inclusion rule (docs/reference/permissions.md),
//! the static checker, the hashes of sealed artifacts, and the formats and
//! verifier of receipts, contract approvals and conformance certificates.
//!
//! It depends on no other crate of this workspace.

pub mod syntax;
pub mod types;
