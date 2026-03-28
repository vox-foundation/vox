//! Shared builtin registry to reduce drift between type registration and Rust emit mapping.

/// Builtin callable shape shared by typecheck and codegen lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinRegistryEntry {
    pub namespace: &'static str,
    pub name: &'static str,
    pub arg_count: usize,
    pub signature: &'static str,
    /// Fully-qualified runtime function symbol, if implemented by `vox-runtime`.
    pub runtime_symbol: Option<&'static str>,
}

/// Stable subset of builtins with shared registry ownership.
#[must_use]
pub fn builtin_registry_entries() -> &'static [BuiltinRegistryEntry] {
    &[
        BuiltinRegistryEntry {
            namespace: "std",
            name: "uuid",
            arg_count: 0,
            signature: "fn() -> str",
            runtime_symbol: Some("vox_runtime::builtins::vox_uuid"),
        },
        BuiltinRegistryEntry {
            namespace: "std",
            name: "now_ms",
            arg_count: 0,
            signature: "fn() -> int",
            runtime_symbol: Some("vox_runtime::builtins::vox_now_ms"),
        },
        BuiltinRegistryEntry {
            namespace: "std",
            name: "hash_fast",
            arg_count: 1,
            signature: "fn(str) -> str",
            runtime_symbol: Some("vox_runtime::builtins::vox_hash_fast"),
        },
        BuiltinRegistryEntry {
            namespace: "std",
            name: "hash_secure",
            arg_count: 1,
            signature: "fn(str) -> str",
            runtime_symbol: Some("vox_runtime::builtins::vox_hash_secure"),
        },
        BuiltinRegistryEntry {
            namespace: "OpenClaw",
            name: "list_skills",
            arg_count: 0,
            signature: "fn() -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_openclaw_list_skills"),
        },
        BuiltinRegistryEntry {
            namespace: "OpenClaw",
            name: "call",
            arg_count: 2,
            signature: "fn(str, str) -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_openclaw_call"),
        },
        BuiltinRegistryEntry {
            namespace: "OpenClaw",
            name: "subscribe",
            arg_count: 1,
            signature: "fn(str) -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_openclaw_subscribe"),
        },
        BuiltinRegistryEntry {
            namespace: "OpenClaw",
            name: "unsubscribe",
            arg_count: 1,
            signature: "fn(str) -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_openclaw_unsubscribe"),
        },
        BuiltinRegistryEntry {
            namespace: "OpenClaw",
            name: "notify",
            arg_count: 2,
            signature: "fn(str, str) -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_openclaw_notify"),
        },
    ]
}

#[must_use]
pub fn lookup_builtin(
    namespace: &str,
    name: &str,
    arg_count: usize,
) -> Option<BuiltinRegistryEntry> {
    builtin_registry_entries()
        .iter()
        .copied()
        .find(|e| e.namespace == namespace && e.name == name && e.arg_count == arg_count)
}
