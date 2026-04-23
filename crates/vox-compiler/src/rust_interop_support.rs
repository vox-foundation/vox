/// Support classification for `import rust:...` crate names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustInteropSupportClass {
    FirstClassWrapper,
    InternalRuntimeOnly,
    EscapeHatchOnly,
    Deferred,
}

impl RustInteropSupportClass {
    #[must_use]
    pub fn as_label(self) -> &'static str {
        match self {
            Self::FirstClassWrapper => "first_class",
            Self::InternalRuntimeOnly => "internal_runtime_only",
            Self::EscapeHatchOnly => "escape_hatch_only",
            Self::Deferred => "deferred",
        }
    }
}

/// Semantics maturity for a rust crate import lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustInteropSemanticsState {
    Implemented,
    PartiallyImplemented,
    Planned,
    DocsOnly,
}

impl RustInteropSemanticsState {
    #[must_use]
    pub fn as_label(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::PartiallyImplemented => "partially_implemented",
            Self::Planned => "planned",
            Self::DocsOnly => "docs_only",
        }
    }
}

/// Supported deployment/runtime targets for a rust crate family in Vox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustInteropTarget {
    Native,
    Wasi,
    Container,
}

impl RustInteropTarget {
    #[must_use]
    pub fn as_label(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Wasi => "wasi",
            Self::Container => "container",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RustInteropPolicyEntry {
    pub crate_name: &'static str,
    pub support_class: RustInteropSupportClass,
    pub semantics_state: RustInteropSemanticsState,
    pub supported_targets: &'static [RustInteropTarget],
}

include!(concat!(env!("OUT_DIR"), "/rust_interop_policy.rs"));

#[must_use]
pub fn rust_interop_policy_entries() -> &'static [RustInteropPolicyEntry] {
    GENERATED_RUST_INTEROP_POLICY
}

#[must_use]
pub fn template_managed_app_dependencies() -> &'static [&'static str] {
    GENERATED_TEMPLATE_MANAGED_APP_DEPS
}

#[must_use]
pub fn template_managed_script_native_dependencies() -> &'static [&'static str] {
    GENERATED_TEMPLATE_MANAGED_SCRIPT_NATIVE_DEPS
}

#[must_use]
pub fn template_managed_script_wasi_dependencies() -> &'static [&'static str] {
    GENERATED_TEMPLATE_MANAGED_SCRIPT_WASI_DEPS
}

#[must_use]
pub fn wasi_unsupported_rust_imports() -> &'static [&'static str] {
    GENERATED_WASI_UNSUPPORTED_RUST_IMPORTS
}

#[must_use]
pub fn lookup_rust_interop_policy(crate_name: &str) -> Option<RustInteropPolicyEntry> {
    rust_interop_policy_entries()
        .iter()
        .copied()
        .find(|e| e.crate_name == crate_name)
}

/// Canonical support-class mapping for rust crate imports.
#[must_use]
pub fn classify_rust_crate(crate_name: &str) -> RustInteropSupportClass {
    lookup_rust_interop_policy(crate_name)
        .map(|e| e.support_class)
        .unwrap_or(RustInteropSupportClass::EscapeHatchOnly)
}

/// Canonical semantics-state mapping aligned to contracts/rust/ecosystem-support.yaml.
#[must_use]
pub fn semantics_state_for_rust_crate(crate_name: &str) -> RustInteropSemanticsState {
    lookup_rust_interop_policy(crate_name)
        .map(|e| e.semantics_state)
        .unwrap_or(RustInteropSemanticsState::PartiallyImplemented)
}

/// Crates pre-supplied in generated app Cargo.toml and therefore skipped from rust import append.
#[must_use]
pub fn is_template_managed_app_dependency(crate_name: &str) -> bool {
    template_managed_app_dependencies().contains(&crate_name)
}

/// Crates pre-supplied in generated native script Cargo.toml.
#[must_use]
pub fn is_template_managed_script_native_dependency(crate_name: &str) -> bool {
    template_managed_script_native_dependencies().contains(&crate_name)
}

/// Crates pre-supplied in generated WASI script Cargo.toml.
#[must_use]
pub fn is_template_managed_script_wasi_dependency(crate_name: &str) -> bool {
    template_managed_script_wasi_dependencies().contains(&crate_name)
}

/// Whether a known rust import crate is explicitly unsupported in WASI script mode.
#[must_use]
pub fn is_wasi_unsupported_rust_import(crate_name: &str) -> bool {
    wasi_unsupported_rust_imports().contains(&crate_name)
}

/// Supported target mapping aligned to contracts/rust/ecosystem-support.yaml.
#[must_use]
pub fn supported_targets_for_rust_crate(crate_name: &str) -> Option<&'static [RustInteropTarget]> {
    lookup_rust_interop_policy(crate_name).map(|e| e.supported_targets)
}

#[cfg(test)]
mod tests {
    use super::{
        RustInteropSemanticsState, RustInteropSupportClass, RustInteropTarget, classify_rust_crate,
        is_template_managed_app_dependency, is_template_managed_script_native_dependency,
        is_template_managed_script_wasi_dependency, is_wasi_unsupported_rust_import,
        semantics_state_for_rust_crate, supported_targets_for_rust_crate,
        wasi_unsupported_rust_imports,
    };

    #[test]
    fn first_class_and_runtime_internal_mappings_are_stable() {
        assert_eq!(
            classify_rust_crate("serde_json"),
            RustInteropSupportClass::FirstClassWrapper
        );
        assert_eq!(
            classify_rust_crate("time"),
            RustInteropSupportClass::FirstClassWrapper
        );
        assert_eq!(
            classify_rust_crate("tokio"),
            RustInteropSupportClass::InternalRuntimeOnly
        );
    }

    #[test]
    fn deferred_and_fallback_mappings_are_stable() {
        assert_eq!(
            classify_rust_crate("sqlx"),
            RustInteropSupportClass::Deferred
        );
        assert_eq!(
            classify_rust_crate("some_future_crate"),
            RustInteropSupportClass::EscapeHatchOnly
        );
    }

    #[test]
    fn semantics_state_mapping_is_stable() {
        assert_eq!(
            semantics_state_for_rust_crate("serde_json"),
            RustInteropSemanticsState::Implemented
        );
        assert_eq!(
            semantics_state_for_rust_crate("time"),
            RustInteropSemanticsState::Planned
        );
        assert_eq!(
            semantics_state_for_rust_crate("sqlx"),
            RustInteropSemanticsState::DocsOnly
        );
        assert_eq!(
            RustInteropSemanticsState::PartiallyImplemented.as_label(),
            "partially_implemented"
        );
        assert_eq!(
            semantics_state_for_rust_crate("some_future_crate"),
            RustInteropSemanticsState::PartiallyImplemented
        );
    }

    #[test]
    fn template_dependency_sets_are_stable() {
        assert!(is_template_managed_app_dependency("reqwest"));
        assert!(is_template_managed_app_dependency("vox-db"));
        assert!(is_template_managed_script_native_dependency("tokio"));
        assert!(is_template_managed_script_wasi_dependency("serde"));
        assert!(!is_template_managed_script_wasi_dependency("reqwest"));
    }

    #[test]
    fn wasi_unsupported_mapping_is_stable() {
        assert!(is_wasi_unsupported_rust_import("reqwest"));
        assert!(is_wasi_unsupported_rust_import("turso"));
        assert!(!is_wasi_unsupported_rust_import("serde_json"));
        assert!(!is_wasi_unsupported_rust_import("uuid"));
        assert!(wasi_unsupported_rust_imports().contains(&"reqwest"));
    }

    #[test]
    fn supported_targets_mapping_is_stable() {
        let serde_targets =
            supported_targets_for_rust_crate("serde_json").expect("serde_json targets");
        assert!(serde_targets.contains(&RustInteropTarget::Wasi));
        let reqwest_targets = supported_targets_for_rust_crate("reqwest").expect("reqwest targets");
        assert!(!reqwest_targets.contains(&RustInteropTarget::Wasi));
        assert!(supported_targets_for_rust_crate("future_crate").is_none());
    }
}
