//! Typed core IR v2 — semantic single source of truth before web/runtime projections.
//!
//! [`TypedCoreIR_v2`] is currently a type alias for [`super::HirModule`]. New code should name
//! the lowered module as **core IR** when the distinction from [`crate::web_ir::WebProjectionIR`]
//! matters (typecheck, runtime projection, orchestration manifests).

/// Version tag for core IR layout and tooling contracts. Bump when serialized core snapshots change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoreIrVersion {
    /// Initial unified naming / projection split (no extra fields on [`super::HirModule`] yet).
    #[default]
    V2_0,
}

/// Stable numeric id for a web entrypoint (reactive component name index within the module).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WebEntrypointId(pub u32);

/// Authoritative lowered module: same representation as [`super::HirModule`], distinct name for IR layering.
#[allow(non_camel_case_types)]
pub type TypedCoreIR_v2 = super::HirModule;

#[must_use]
pub const fn typed_core_version() -> CoreIrVersion {
    CoreIrVersion::V2_0
}
