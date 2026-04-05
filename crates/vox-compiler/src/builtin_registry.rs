//! Shared builtin registry to reduce drift between type registration and Rust emit mapping.

use crate::typeck::ty::Ty;

/// Parameter kind for registry-driven codegen (`str` lowers with `.as_str()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinArgKind {
    Str,
    Bool,
    Int,
}

/// Builtin callable shape shared by typecheck and codegen lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinRegistryEntry {
    pub namespace: &'static str,
    pub name: &'static str,
    pub arg_count: usize,
    pub signature: &'static str,
    /// Fully-qualified runtime function symbol, if implemented by `vox-runtime`.
    pub runtime_symbol: Option<&'static str>,
    /// Non-empty when arguments are not all `str` (must match [`Self::arg_count`]).
    pub arg_kinds: &'static [BuiltinArgKind],
    /// `true` → runtime returns `Result<(), String>` (Vox `Result[unit]`).
    pub returns_unit: bool,
}

/// Parameter types for a registry entry (shared by typecheck namespace methods).
#[must_use]
pub fn builtin_entry_param_tys(entry: BuiltinRegistryEntry) -> Option<Vec<Ty>> {
    if !entry.arg_kinds.is_empty() {
        if entry.arg_kinds.len() != entry.arg_count {
            return None;
        }
        return Some(
            entry
                .arg_kinds
                .iter()
                .map(|k| match k {
                    BuiltinArgKind::Str => Ty::Str,
                    BuiltinArgKind::Bool => Ty::Bool,
                    BuiltinArgKind::Int => Ty::Int,
                })
                .collect(),
        );
    }
    match entry.arg_count {
        0 => Some(vec![]),
        1 => Some(vec![Ty::Str]),
        2 => Some(vec![Ty::Str, Ty::Str]),
        3 => Some(vec![Ty::Str, Ty::Str, Ty::Str]),
        _ => None,
    }
}

/// Result type for a registry entry (`Result[str]` or `Result[unit]`).
#[must_use]
pub fn builtin_entry_result_ty(entry: BuiltinRegistryEntry) -> Ty {
    if entry.returns_unit {
        Ty::Result(Box::new(Ty::Unit))
    } else {
        Ty::Result(Box::new(Ty::Str))
    }
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
            arg_kinds: &[],
            returns_unit: false,
        },
        BuiltinRegistryEntry {
            namespace: "std",
            name: "now_ms",
            arg_count: 0,
            signature: "fn() -> int",
            runtime_symbol: Some("vox_runtime::builtins::vox_now_ms"),
            arg_kinds: &[],
            returns_unit: false,
        },
        BuiltinRegistryEntry {
            namespace: "std",
            name: "hash_fast",
            arg_count: 1,
            signature: "fn(str) -> str",
            runtime_symbol: Some("vox_runtime::builtins::vox_hash_fast"),
            arg_kinds: &[],
            returns_unit: false,
        },
        BuiltinRegistryEntry {
            namespace: "std",
            name: "hash_secure",
            arg_count: 1,
            signature: "fn(str) -> str",
            runtime_symbol: Some("vox_runtime::builtins::vox_hash_secure"),
            arg_kinds: &[],
            returns_unit: false,
        },
        BuiltinRegistryEntry {
            namespace: "std.http",
            name: "get_text",
            arg_count: 1,
            signature: "fn(str) -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_http_get_text"),
            arg_kinds: &[],
            returns_unit: false,
        },
        BuiltinRegistryEntry {
            namespace: "std.http",
            name: "post_json",
            arg_count: 2,
            signature: "fn(str, str) -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_http_post_json"),
            arg_kinds: &[],
            returns_unit: false,
        },
        BuiltinRegistryEntry {
            namespace: "OpenClaw",
            name: "list_skills",
            arg_count: 0,
            signature: "fn() -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_openclaw_list_skills"),
            arg_kinds: &[],
            returns_unit: false,
        },
        BuiltinRegistryEntry {
            namespace: "OpenClaw",
            name: "call",
            arg_count: 2,
            signature: "fn(str, str) -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_openclaw_call"),
            arg_kinds: &[],
            returns_unit: false,
        },
        BuiltinRegistryEntry {
            namespace: "OpenClaw",
            name: "subscribe",
            arg_count: 1,
            signature: "fn(str) -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_openclaw_subscribe"),
            arg_kinds: &[],
            returns_unit: false,
        },
        BuiltinRegistryEntry {
            namespace: "OpenClaw",
            name: "unsubscribe",
            arg_count: 1,
            signature: "fn(str) -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_openclaw_unsubscribe"),
            arg_kinds: &[],
            returns_unit: false,
        },
        BuiltinRegistryEntry {
            namespace: "OpenClaw",
            name: "notify",
            arg_count: 2,
            signature: "fn(str, str) -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_openclaw_notify"),
            arg_kinds: &[],
            returns_unit: false,
        },
        // Chromium / CDP — native scripts only (see codegen `wasm32` guard).
        BuiltinRegistryEntry {
            namespace: "Browser",
            name: "open",
            arg_count: 2,
            signature: "fn(str, bool) -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_browser_open"),
            arg_kinds: &[BuiltinArgKind::Str, BuiltinArgKind::Bool],
            returns_unit: false,
        },
        BuiltinRegistryEntry {
            namespace: "Browser",
            name: "close",
            arg_count: 1,
            signature: "fn(str) -> Result[unit]",
            runtime_symbol: Some("vox_runtime::builtins::vox_browser_close"),
            arg_kinds: &[],
            returns_unit: true,
        },
        BuiltinRegistryEntry {
            namespace: "Browser",
            name: "goto",
            arg_count: 2,
            signature: "fn(str, str) -> Result[unit]",
            runtime_symbol: Some("vox_runtime::builtins::vox_browser_goto"),
            arg_kinds: &[],
            returns_unit: true,
        },
        BuiltinRegistryEntry {
            namespace: "Browser",
            name: "click",
            arg_count: 2,
            signature: "fn(str, str) -> Result[unit]",
            runtime_symbol: Some("vox_runtime::builtins::vox_browser_click"),
            arg_kinds: &[],
            returns_unit: true,
        },
        BuiltinRegistryEntry {
            namespace: "Browser",
            name: "fill",
            arg_count: 3,
            signature: "fn(str, str, str) -> Result[unit]",
            runtime_symbol: Some("vox_runtime::builtins::vox_browser_fill"),
            arg_kinds: &[],
            returns_unit: true,
        },
        BuiltinRegistryEntry {
            namespace: "Browser",
            name: "wait_for",
            arg_count: 3,
            signature: "fn(str, str, int) -> Result[unit]",
            runtime_symbol: Some("vox_runtime::builtins::vox_browser_wait_for"),
            arg_kinds: &[
                BuiltinArgKind::Str,
                BuiltinArgKind::Str,
                BuiltinArgKind::Int,
            ],
            returns_unit: true,
        },
        BuiltinRegistryEntry {
            namespace: "Browser",
            name: "text",
            arg_count: 2,
            signature: "fn(str, str) -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_browser_text"),
            arg_kinds: &[],
            returns_unit: false,
        },
        BuiltinRegistryEntry {
            namespace: "Browser",
            name: "html",
            arg_count: 2,
            signature: "fn(str, str) -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_browser_html"),
            arg_kinds: &[],
            returns_unit: false,
        },
        BuiltinRegistryEntry {
            namespace: "Browser",
            name: "screenshot",
            arg_count: 2,
            signature: "fn(str, str) -> Result[str]",
            runtime_symbol: Some("vox_runtime::builtins::vox_browser_screenshot"),
            arg_kinds: &[],
            returns_unit: false,
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

/// `std.<field>` type members on the root namespace.
#[must_use]
pub fn std_root_field_ty(field: &str) -> Option<Ty> {
    Some(match field {
        "fs" => Ty::Named("StdFsNs".into()),
        "path" => Ty::Named("StdPathNs".into()),
        "env" => Ty::Named("StdEnvNs".into()),
        "process" => Ty::Named("StdProcessNs".into()),
        "json" => Ty::Named("StdJsonNs".into()),
        "http" => Ty::Named("StdHttpNs".into()),
        "crypto" => Ty::Named("StdCryptoNs".into()),
        "time" => Ty::Named("StdTimeNs".into()),
        "log" => Ty::Named("StdLogNs".into()),
        "uuid" => Ty::Fn(vec![], Box::new(Ty::Str)),
        "now_ms" => Ty::Fn(vec![], Box::new(Ty::Int)),
        "hash_fast" | "hash_secure" => Ty::Fn(vec![Ty::Str], Box::new(Ty::Str)),
        "args" => Ty::List(Box::new(Ty::Str)),
        _ => return None,
    })
}

/// `std.<namespace>.<method>` signatures used by type checking.
#[must_use]
pub fn std_namespace_method_ty(namespace: &str, method: &str) -> Option<Ty> {
    Some(match (namespace, method) {
        ("fs", "read") | ("fs", "remove") | ("fs", "mkdir") => {
            Ty::Fn(vec![Ty::Str], Box::new(Ty::Result(Box::new(Ty::Str))))
        }
        ("fs", "read_bytes") => Ty::Fn(vec![Ty::Str], Box::new(Ty::Result(Box::new(Ty::Str)))),
        ("fs", "write") => Ty::Fn(
            vec![Ty::Str, Ty::Str],
            Box::new(Ty::Result(Box::new(Ty::Unit))),
        ),
        ("fs", "exists") => Ty::Fn(vec![Ty::Str], Box::new(Ty::Bool)),
        ("fs", "is_file") | ("fs", "is_dir") => Ty::Fn(vec![Ty::Str], Box::new(Ty::Bool)),
        ("fs", "canonicalize") => Ty::Fn(vec![Ty::Str], Box::new(Ty::Result(Box::new(Ty::Str)))),
        ("fs", "list_dir") | ("fs", "glob") => Ty::Fn(
            vec![Ty::Str],
            Box::new(Ty::Result(Box::new(Ty::List(Box::new(Ty::Str))))),
        ),
        ("fs", "remove_dir_all") => Ty::Fn(vec![Ty::Str], Box::new(Ty::Result(Box::new(Ty::Unit)))),
        ("fs", "copy") => Ty::Fn(
            vec![Ty::Str, Ty::Str],
            Box::new(Ty::Result(Box::new(Ty::Unit))),
        ),
        ("path", "join") => Ty::Fn(vec![Ty::Str, Ty::Str], Box::new(Ty::Str)),
        ("path", "join_many") => Ty::Fn(vec![Ty::List(Box::new(Ty::Str))], Box::new(Ty::Str)),
        ("path", "basename") | ("path", "dirname") | ("path", "extension") => {
            Ty::Fn(vec![Ty::Str], Box::new(Ty::Str))
        }
        ("env", "get") => Ty::Fn(vec![Ty::Str], Box::new(Ty::Option(Box::new(Ty::Str)))),
        ("process", "which") => Ty::Fn(vec![Ty::Str], Box::new(Ty::Option(Box::new(Ty::Str)))),
        ("process", "run") => Ty::Fn(
            vec![Ty::Str, Ty::List(Box::new(Ty::Str))],
            Box::new(Ty::Result(Box::new(Ty::Int))),
        ),
        ("process", "run_ex") => Ty::Fn(
            vec![
                Ty::Str,
                Ty::List(Box::new(Ty::Str)),
                Ty::Str,
                Ty::List(Box::new(Ty::Str)),
            ],
            Box::new(Ty::Result(Box::new(Ty::Int))),
        ),
        ("process", "run_capture") => Ty::Fn(
            vec![Ty::Str, Ty::List(Box::new(Ty::Str))],
            Box::new(Ty::Result(Box::new(Ty::Record(vec![
                ("exit".into(), Ty::Int),
                ("stdout".into(), Ty::Str),
                ("stderr".into(), Ty::Str),
            ])))),
        ),
        ("process", "run_capture_ex") => Ty::Fn(
            vec![
                Ty::Str,
                Ty::List(Box::new(Ty::Str)),
                Ty::Str,
                Ty::List(Box::new(Ty::Str)),
            ],
            Box::new(Ty::Result(Box::new(Ty::Record(vec![
                ("exit".into(), Ty::Int),
                ("stdout".into(), Ty::Str),
                ("stderr".into(), Ty::Str),
            ])))),
        ),
        ("process", "exit") => Ty::Fn(vec![Ty::Int], Box::new(Ty::Never)),
        ("json", "read_str") => Ty::Fn(
            vec![Ty::Str, Ty::Str],
            Box::new(Ty::Result(Box::new(Ty::Str))),
        ),
        ("json", "read_f64") => Ty::Fn(
            vec![Ty::Str, Ty::Str],
            Box::new(Ty::Result(Box::new(Ty::Float))),
        ),
        ("json", "quote") => Ty::Fn(vec![Ty::Str], Box::new(Ty::Str)),
        ("http", "get_text") => Ty::Fn(vec![Ty::Str], Box::new(Ty::Result(Box::new(Ty::Str)))),
        ("http", "post_json") => Ty::Fn(
            vec![Ty::Str, Ty::Str],
            Box::new(Ty::Result(Box::new(Ty::Str))),
        ),
        ("crypto", "hash_fast") | ("crypto", "hash_secure") => {
            Ty::Fn(vec![Ty::Str], Box::new(Ty::Str))
        }
        ("crypto", "uuid") => Ty::Fn(vec![], Box::new(Ty::Str)),
        ("time", "now_ms") => Ty::Fn(vec![], Box::new(Ty::Int)),
        ("log", "debug") | ("log", "info") | ("log", "warn") | ("log", "error") => {
            Ty::Fn(vec![Ty::Str], Box::new(Ty::Unit))
        }
        _ => return None,
    })
}

/// Shared runtime call lowering for `std.<namespace>.<method>` in Rust codegen.
#[must_use]
pub fn std_namespace_runtime_call(
    namespace: &str,
    method: &str,
    args: &[String],
) -> Option<String> {
    match (namespace, method) {
        ("crypto", "hash_fast") if !args.is_empty() => Some(format!(
            "vox_runtime::builtins::vox_hash_fast(&{})",
            args[0]
        )),
        ("crypto", "hash_secure") if !args.is_empty() => Some(format!(
            "vox_runtime::builtins::vox_hash_secure(&{})",
            args[0]
        )),
        ("crypto", "uuid") => Some("vox_runtime::builtins::vox_uuid()".to_string()),
        ("time", "now_ms") => Some("vox_runtime::builtins::vox_now_ms()".to_string()),
        ("log", "debug") if !args.is_empty() => Some(format!(
            "vox_runtime::builtins::vox_log_debug(({}).as_str())",
            args[0]
        )),
        ("log", "info") if !args.is_empty() => Some(format!(
            "vox_runtime::builtins::vox_log_info(({}).as_str())",
            args[0]
        )),
        ("log", "warn") if !args.is_empty() => Some(format!(
            "vox_runtime::builtins::vox_log_warn(({}).as_str())",
            args[0]
        )),
        ("log", "error") if !args.is_empty() => Some(format!(
            "vox_runtime::builtins::vox_log_error(({}).as_str())",
            args[0]
        )),
        ("fs", "read") if !args.is_empty() => Some(format!(
            "std::fs::read_to_string({}).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?",
            args[0]
        )),
        ("fs", "write") if args.len() >= 2 => Some(format!(
            "std::fs::write({}, {}).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?",
            args[0], args[1]
        )),
        ("fs", "exists") if !args.is_empty() => {
            Some(format!("std::path::Path::new(&{}).exists()", args[0]))
        }
        ("fs", "is_file") if !args.is_empty() => {
            Some(format!("std::path::Path::new(&{}).is_file()", args[0]))
        }
        ("fs", "is_dir") if !args.is_empty() => {
            Some(format!("std::path::Path::new(&{}).is_dir()", args[0]))
        }
        ("fs", "canonicalize") if !args.is_empty() => Some(format!(
            "std::fs::canonicalize({}).map(|p| p.to_string_lossy().to_string()).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?",
            args[0]
        )),
        ("fs", "remove") if !args.is_empty() => Some(format!(
            "std::fs::remove_file({}).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?",
            args[0]
        )),
        ("fs", "read_bytes") if !args.is_empty() => Some(format!(
            "std::fs::read({}).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?",
            args[0]
        )),
        ("fs", "mkdir") if !args.is_empty() => Some(format!(
            "std::fs::create_dir_all({}).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?",
            args[0]
        )),
        ("path", "join") if args.len() >= 2 => Some(format!(
            "std::path::Path::new(&{}).join(&{}).to_string_lossy().to_string()",
            args[0], args[1]
        )),
        ("path", "basename") if !args.is_empty() => Some(format!(
            "std::path::Path::new(&{}).file_name().unwrap_or_default().to_string_lossy().to_string()",
            args[0]
        )),
        ("path", "dirname") if !args.is_empty() => Some(format!(
            "std::path::Path::new(&{}).parent().unwrap_or(std::path::Path::new(\".\")).to_string_lossy().to_string()",
            args[0]
        )),
        ("path", "extension") if !args.is_empty() => Some(format!(
            "std::path::Path::new(&{}).extension().unwrap_or_default().to_string_lossy().to_string()",
            args[0]
        )),
        ("env", "get") if !args.is_empty() => Some(format!(
            "(vox_runtime::builtins::vox_env_get(({}).as_str()))",
            args[0]
        )),
        ("process", "which") if !args.is_empty() => Some(format!(
            "(vox_runtime::builtins::vox_process_which(({}).as_str()))",
            args[0]
        )),
        ("process", "run") if args.len() >= 2 => Some(format!(
            "(match vox_runtime::builtins::vox_process_run(({}).as_str(), {}.as_slice()) {{ Ok(c) => Ok(c as i64), Err(m) => Error(m) }})",
            args[0], args[1]
        )),
        ("process", "run_ex") if args.len() >= 4 => Some(format!(
            "(match vox_runtime::builtins::vox_process_run_ex(({}).as_str(), {}.as_slice(), ({}).as_str(), {}.as_slice()) {{ Ok(c) => Ok(c as i64), Err(m) => Error(m) }})",
            args[0], args[1], args[2], args[3]
        )),
        ("process", "run_capture") if args.len() >= 2 => Some(format!(
            "(match vox_runtime::builtins::vox_process_run_capture(({}).as_str(), {}.as_slice()) {{ Ok(p) => Ok(serde_json::json!({{ \"exit\": p.exit as i64, \"stdout\": p.stdout, \"stderr\": p.stderr }})), Err(m) => Error(m) }})",
            args[0], args[1]
        )),
        ("process", "run_capture_ex") if args.len() >= 4 => Some(format!(
            "(match vox_runtime::builtins::vox_process_run_capture_ex(({}).as_str(), {}.as_slice(), ({}).as_str(), {}.as_slice()) {{ Ok(p) => Ok(serde_json::json!({{ \"exit\": p.exit as i64, \"stdout\": p.stdout, \"stderr\": p.stderr }})), Err(m) => Error(m) }})",
            args[0], args[1], args[2], args[3]
        )),
        ("process", "exit") if !args.is_empty() => {
            Some(format!("{{ std::process::exit({} as i32) }}", args[0]))
        }
        ("fs", "list_dir") if !args.is_empty() => Some(format!(
            "(match vox_runtime::builtins::vox_list_dir(({}).as_str()) {{ Ok(v) => Ok(v), Err(m) => Error(m) }})",
            args[0]
        )),
        ("fs", "glob") if !args.is_empty() => Some(format!(
            "(match vox_runtime::builtins::vox_fs_glob(({}).as_str()) {{ Ok(v) => Ok(v), Err(m) => Error(m) }})",
            args[0]
        )),
        ("fs", "remove_dir_all") if !args.is_empty() => Some(format!(
            "(match vox_runtime::builtins::vox_fs_remove_dir_all(({}).as_str()) {{ Ok(()) => Ok(()), Err(m) => Error(m) }})",
            args[0]
        )),
        ("fs", "copy") if args.len() >= 2 => Some(format!(
            "(match vox_runtime::builtins::vox_fs_copy(({}).as_str(), ({}).as_str()) {{ Ok(()) => Ok(()), Err(m) => Error(m) }})",
            args[0], args[1]
        )),
        ("path", "join_many") if !args.is_empty() => Some(format!(
            "vox_runtime::builtins::vox_path_join_many({}.as_slice())",
            args[0]
        )),
        ("json", "read_str") if args.len() >= 2 => Some(format!(
            "(match vox_runtime::builtins::vox_json_read_str(({}).as_str(), ({}).as_str()) {{ Ok(s) => Ok(s), Err(m) => Error(m) }})",
            args[0], args[1]
        )),
        ("json", "read_f64") if args.len() >= 2 => Some(format!(
            "(match vox_runtime::builtins::vox_json_read_f64(({}).as_str(), ({}).as_str()) {{ Ok(v) => Ok(v), Err(m) => Error(m) }})",
            args[0], args[1]
        )),
        ("json", "quote") if !args.is_empty() => Some(format!(
            "vox_runtime::builtins::vox_json_quote(({}).as_str())",
            args[0]
        )),
        ("http", "get_text") if !args.is_empty() => Some(format!(
            "({{ #[cfg(target_arch = \"wasm32\")] {{ Error(\"std.http.get_text is not supported in WASI scripts\".to_string()) }} #[cfg(not(target_arch = \"wasm32\"))] {{ match vox_runtime::builtins::vox_http_get_text(({}).as_str()) {{ Ok(s) => Ok(s), Err(m) => Error(m) }} }} }})",
            args[0]
        )),
        ("http", "post_json") if args.len() >= 2 => Some(format!(
            "({{ #[cfg(target_arch = \"wasm32\")] {{ Error(\"std.http.post_json is not supported in WASI scripts\".to_string()) }} #[cfg(not(target_arch = \"wasm32\"))] {{ match vox_runtime::builtins::vox_http_post_json(({}).as_str(), ({}).as_str()) {{ Ok(s) => Ok(s), Err(m) => Error(m) }} }} }})",
            args[0], args[1]
        )),
        _ => None,
    }
}

#[cfg(test)]
mod browser_registry_tests {
    use super::builtin_registry_entries;

    #[test]
    fn browser_builtins_map_to_vox_runtime() {
        let browser: Vec<_> = builtin_registry_entries()
            .iter()
            .copied()
            .filter(|e| e.namespace == "Browser")
            .collect();
        assert_eq!(
            browser.len(),
            9,
            "Browser registry size drift (update typeck + runtime if intentional)"
        );
        for e in browser {
            let sym = e
                .runtime_symbol
                .unwrap_or_else(|| panic!("Browser.{} missing runtime_symbol", e.name));
            assert!(
                sym.starts_with("vox_runtime::builtins::vox_browser_"),
                "unexpected symbol for Browser.{}: {sym}",
                e.name
            );
        }
    }
}
