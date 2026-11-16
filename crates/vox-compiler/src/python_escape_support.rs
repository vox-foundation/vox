//! Tier 2 Python escape-hatch policy, `.vox.pyi` sidecar resolution, and hashed telemetry.
//!
//! Enforces:
//! - Compile-time & runtime `Denied` tier blocking (`pickle`, `marshal`, `shelve`,
//!   `subprocess`, `torch.load`, `eval`, `exec`, `importlib`, `os.system`, `os.popen`,
//!   `os.spawn*`, `os.exec*`).
//! - Typed `.vox.pyi` sidecar parsing + `uv.lock` BLAKE3 staleness verification
//!   (via `vox-crypto`).
//! - Privacy-preserving hashed `python.escape_call` telemetry payload generation.

use crate::typeck::ty::Ty;
use std::path::{Path, PathBuf};

/// Parsed signature from a `.vox.pyi` sidecar declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct PySidecarSig {
    pub name: String,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
}

/// Loaded `.vox.pyi` sidecar contract for a Python module.
#[derive(Debug, Clone, PartialEq)]
pub struct PySidecar {
    pub module: String,
    pub lock_blake3: Option<String>,
    pub is_curated: bool,
    pub fns: Vec<PySidecarSig>,
    pub path: PathBuf,
}

/// Check whether a Python module or `module.symbol` call falls in the permanently
/// `Denied` tier. Returns a remediation hint when blocked.
#[must_use]
pub fn is_denied_python_target(module: &str, symbol: Option<&str>) -> Option<&'static str> {
    let m = module.trim();
    let s = symbol.map(str::trim).filter(|sym| !sym.is_empty());

    let top = m.split('.').next().unwrap_or(m);
    if matches!(top, "pickle" | "marshal" | "shelve" | "_pickle") {
        return Some(
            "Use `std.tensor.load_exported` for `.pt2` models or structured JSON instead of Python pickle/marshal serialization.",
        );
    }
    if matches!(top, "subprocess" | "_posixsubprocess") {
        return Some("Use `std.process` instead of Python `subprocess`.");
    }
    if matches!(top, "importlib" | "_imp" | "imp") {
        return Some(
            "Dynamic `importlib` imports are disallowed; use static `import py <alias> from \"<module>\"`.",
        );
    }
    if matches!(m, "eval" | "exec" | "builtins.eval" | "builtins.exec")
        || ((m == "torch" || m.starts_with("torch.")) && m.ends_with(".load"))
        || (matches!(top, "os" | "posix" | "nt")
            && m.split('.').nth(1).is_some_and(|sub| {
                sub == "system"
                    || sub == "popen"
                    || sub.starts_with("spawn")
                    || sub.starts_with("exec")
            }))
    {
        return Some(denied_symbol_hint(m));
    }

    if let Some(sym) = s {
        if sym == "__import__" || matches!(sym, "eval" | "exec") && top == "builtins" {
            return Some(
                "Dynamic `eval`/`exec`/`__import__` calls are disallowed in Tier 2 Python.",
            );
        }
        if (m == "torch" || m.starts_with("torch.")) && sym == "load" {
            return Some(
                "Use `std.tensor.load_exported` with `.pt2` files instead of `torch.load`.",
            );
        }
        if matches!(top, "os" | "posix" | "nt")
            && matches!(m, "os" | "posix" | "nt")
            && (sym == "system"
                || sym == "popen"
                || sym.starts_with("spawn")
                || sym.starts_with("exec"))
        {
            return Some(
                "Use `std.process` or `std.fs` instead of `os.system`/`os.popen`/`os.spawn*`/`os.exec*`.",
            );
        }
    }

    None
}

fn denied_symbol_hint(target: &str) -> &'static str {
    if target.starts_with("torch") {
        "Use `std.tensor.load_exported` with `.pt2` files instead of `torch.load`."
    } else if target.starts_with("os.") || target.starts_with("posix.") || target.starts_with("nt.")
    {
        "Use `std.process` or `std.fs` instead of `os` process spawning."
    } else {
        "Dynamic `eval`/`exec` calls are disallowed in Tier 2 Python."
    }
}

/// Reject path traversal and unexpected characters in sidecar filenames (`<module>.vox.pyi`).
pub fn sanitize_sidecar_filename(module: &str) -> Result<String, String> {
    let trimmed = module.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('.')
        || trimmed.ends_with('.')
        || trimmed.contains("..")
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        return Err(format!(
            "invalid Python module name for sidecar: `{module}`"
        ));
    }
    Ok(format!("{trimmed}.vox.pyi"))
}

/// Convert a type string from `.vox.pyi` (or `.decode(...)`) into compiler [`Ty`].
#[must_use]
pub fn parse_vox_type_str(raw: &str) -> Ty {
    let s = raw.trim();
    match s {
        "int" | "Int" | "i64" => return Ty::Int,
        "float" | "Float" | "f64" | "f32" => return Ty::Float,
        "str" | "Str" | "String" | "string" => return Ty::Str,
        "bool" | "Bool" => return Ty::Bool,
        "unit" | "Unit" | "()" | "None" => return Ty::Unit,
        "PyObject" => return Ty::Named("PyObject".to_string()),
        _ => {}
    }

    if let Some(inner) = strip_bracket_type(s, &["list[", "List["]) {
        return Ty::List(Box::new(parse_vox_type_str(inner)));
    }
    if let Some(inner) = strip_bracket_type(s, &["Option[", "option["]) {
        return Ty::Option(Box::new(parse_vox_type_str(inner)));
    }
    if let Some(inner) = strip_bracket_type(s, &["Array[", "array["]) {
        return Ty::Array(Box::new(parse_vox_type_str(inner)));
    }
    if let Some(inner) = strip_bracket_type(s, &["Result[", "result["]) {
        let parts = split_top_level_commas(inner);
        return match parts.as_slice() {
            [ok] => Ty::Result(Box::new(parse_vox_type_str(ok)), Box::new(Ty::Str)),
            [ok, err] => Ty::Result(
                Box::new(parse_vox_type_str(ok)),
                Box::new(parse_vox_type_str(err)),
            ),
            _ => Ty::Named("PyObject".to_string()),
        };
    }
    if let Some(inner) = strip_bracket_type(s, &["Map[", "map[", "Dict[", "dict["]) {
        let parts = split_top_level_commas(inner);
        if let [k, v] = parts.as_slice() {
            return Ty::Map(
                Box::new(parse_vox_type_str(k)),
                Box::new(parse_vox_type_str(v)),
            );
        }
    }

    Ty::Named(s.to_string())
}

fn strip_bracket_type<'a>(s: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    if !s.ends_with(']') {
        return None;
    }
    for prefix in prefixes {
        if let Some(rest) = s.strip_prefix(prefix) {
            return Some(&rest[..rest.len() - 1]);
        }
    }
    None
}

fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (idx, ch) in s.char_indices() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let piece = s[start..idx].trim();
                if !piece.is_empty() {
                    out.push(piece);
                }
                start = idx + 1;
            }
            _ => {}
        }
    }
    let tail = s[start..].trim();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

/// Parse a `.vox.pyi` file's text into a [`PySidecar`].
pub fn parse_sidecar_text(
    module: &str,
    text: &str,
    path: PathBuf,
    is_curated: bool,
) -> Result<PySidecar, String> {
    let mut lock_blake3 = None;
    let mut fns = Vec::new();

    for (line_idx, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(comment) = line.strip_prefix('#') {
            if let Some(hash) = comment.trim().strip_prefix("vox-lock-blake3:") {
                lock_blake3 = Some(hash.trim().to_string());
            }
            continue;
        }
        let Some(rest) = line.strip_prefix("fn ") else {
            return Err(format!(
                "{}:{}: expected `fn <name>(...) -> <Type>` in sidecar",
                path.display(),
                line_idx + 1
            ));
        };
        let Some(open_idx) = rest.find('(') else {
            return Err(format!(
                "{}:{}: missing `(` in sidecar signature",
                path.display(),
                line_idx + 1
            ));
        };
        let fn_name = rest[..open_idx].trim().to_string();
        let after_open = &rest[open_idx + 1..];
        let Some(close_idx) = find_matching_paren(after_open) else {
            return Err(format!(
                "{}:{}: missing `)` in sidecar signature",
                path.display(),
                line_idx + 1
            ));
        };
        let params_raw = after_open[..close_idx].trim();
        let tail = after_open[close_idx + 1..].trim();
        let ret =
            if let Some(ret_str) = tail.strip_prefix("->").or_else(|| tail.strip_prefix("to ")) {
                parse_vox_type_str(ret_str)
            } else if tail.is_empty() {
                Ty::Unit
            } else {
                return Err(format!(
                    "{}:{}: expected `-> <Type>` or `to <Type>` in sidecar signature",
                    path.display(),
                    line_idx + 1
                ));
            };

        let mut params = Vec::new();
        for p in split_top_level_commas(params_raw) {
            let Some((pname, pty)) = p.split_once(':') else {
                return Err(format!(
                    "{}:{}: parameter `{p}` must be `<name>: <Type>`",
                    path.display(),
                    line_idx + 1
                ));
            };
            params.push((pname.trim().to_string(), parse_vox_type_str(pty)));
        }
        fns.push(PySidecarSig {
            name: fn_name,
            params,
            ret,
        });
    }

    Ok(PySidecar {
        module: module.to_string(),
        lock_blake3,
        is_curated,
        fns,
        path,
    })
}

fn find_matching_paren(after_open: &str) -> Option<usize> {
    let mut depth = 1usize;
    for (idx, ch) in after_open.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn resolve_start_dir(source_path: Option<&Path>) -> PathBuf {
    match source_path {
        Some(p) if p.is_dir() => p.to_path_buf(),
        Some(p) if p.is_file() => p.parent().unwrap_or(Path::new(".")).to_path_buf(),
        Some(p) if !p.as_os_str().is_empty() => p.parent().unwrap_or(p).to_path_buf(),
        _ => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    }
}

fn load_curated_from_start(
    module: &str,
    filename: &str,
    start_dir: &Path,
) -> Result<Option<PySidecar>, String> {
    for anc in start_dir.ancestors() {
        let curated = anc
            .join("contracts")
            .join("python")
            .join("sidecars")
            .join(filename);
        if curated.is_file() {
            let text = std::fs::read_to_string(&curated)
                .map_err(|e| format!("read {}: {e}", curated.display()))?;
            return parse_sidecar_text(module, &text, curated, true).map(Some);
        }
    }

    let workspace_curated = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/python/sidecars")
        .join(filename);
    if workspace_curated.is_file() {
        let text = std::fs::read_to_string(&workspace_curated)
            .map_err(|e| format!("read {}: {e}", workspace_curated.display()))?;
        return parse_sidecar_text(module, &text, workspace_curated, true).map(Some);
    }

    Ok(None)
}

/// Resolve `.vox/python/bindings/<module>.vox.pyi` (with `uv.lock` BLAKE3 check)
/// or curated `contracts/python/sidecars/<module>.vox.pyi` for `module`.
pub fn load_sidecar(module: &str, source_path: Option<&Path>) -> Result<Option<PySidecar>, String> {
    let filename = sanitize_sidecar_filename(module)?;
    let start_dir = resolve_start_dir(source_path);

    for anc in start_dir.ancestors() {
        let local_binding = anc
            .join(".vox")
            .join("python")
            .join("bindings")
            .join(&filename);
        if local_binding.is_file() {
            let text = std::fs::read_to_string(&local_binding)
                .map_err(|e| format!("read {}: {e}", local_binding.display()))?;
            let sidecar = parse_sidecar_text(module, &text, local_binding.clone(), false)?;
            verify_sidecar_freshness(anc, module, &sidecar)?;
            return Ok(Some(sidecar));
        }

        let curated = anc
            .join("contracts")
            .join("python")
            .join("sidecars")
            .join(&filename);
        if curated.is_file() {
            let text = std::fs::read_to_string(&curated)
                .map_err(|e| format!("read {}: {e}", curated.display()))?;
            return parse_sidecar_text(module, &text, curated, true).map(Some);
        }
    }

    load_curated_from_start(module, &filename, &start_dir)
}

/// Resolve only curated `contracts/python/sidecars/<module>.vox.pyi`, skipping local bindings.
pub fn load_curated_sidecar(
    module: &str,
    source_path: Option<&Path>,
) -> Result<Option<PySidecar>, String> {
    let filename = sanitize_sidecar_filename(module)?;
    let start_dir = resolve_start_dir(source_path);
    load_curated_from_start(module, &filename, &start_dir)
}

fn verify_sidecar_freshness(
    project_root: &Path,
    module: &str,
    sidecar: &PySidecar,
) -> Result<(), String> {
    let Some(expected_hash) = sidecar.lock_blake3.as_deref() else {
        return Ok(());
    };
    let uv_lock = project_root.join(".vox").join("python").join("uv.lock");
    if !uv_lock.is_file() {
        return Ok(());
    }
    let bytes = std::fs::read(&uv_lock).map_err(|e| format!("read {}: {e}", uv_lock.display()))?;
    let actual_hash = vox_crypto::hex_encode(&vox_crypto::secure_hash(&bytes));
    if !actual_hash.eq_ignore_ascii_case(expected_hash) {
        return Err(format!(
            "stale Python sidecar for `{module}` (uv.lock hash {actual_hash} != recorded {expected_hash}); run `vox py bind {module}` to refresh."
        ));
    }
    Ok(())
}

/// Format a hashed `python.escape_call` telemetry payload (never logs raw strings).
#[must_use]
pub fn format_escape_telemetry_event(module: &str, symbol: &str, tier: &str) -> serde_json::Value {
    let module_hash = vox_crypto::hex_encode(&vox_crypto::secure_hash(module.as_bytes()));
    let symbol_hash = vox_crypto::hex_encode(&vox_crypto::secure_hash(symbol.as_bytes()));
    serde_json::json!({
        "event": "python.escape_call",
        "module_hash": module_hash,
        "symbol_hash": symbol_hash,
        "tier": tier,
    })
}

/// Decode a `serde_json::Value` into a [`crate::eval::value::VoxValue`] honoring an optional
/// target Vox type name (`"int"`, `"float"`, `"str"`, `"bool"`, `"list[float]"`, etc.).
pub fn json_to_vox_value(
    raw: &serde_json::Value,
    target_hint: Option<&str>,
) -> Result<crate::eval::value::VoxValue, String> {
    use crate::eval::value::VoxValue;
    let hint = target_hint.map(str::trim);
    match hint {
        Some("int" | "Int" | "i64") => raw
            .as_i64()
            .or_else(|| raw.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64))
            .map(VoxValue::Int)
            .ok_or_else(|| format!("expected Python int, got {raw}")),
        Some("float" | "Float" | "f64" | "f32") => raw
            .as_f64()
            .or_else(|| raw.as_i64().map(|n| n as f64))
            .map(VoxValue::Float)
            .ok_or_else(|| format!("expected Python float, got {raw}")),
        Some("str" | "Str" | "String") => raw
            .as_str()
            .map(|s| VoxValue::Str(s.into()))
            .ok_or_else(|| format!("expected Python str, got {raw}")),
        Some("bool" | "Bool") => raw
            .as_bool()
            .map(VoxValue::Bool)
            .ok_or_else(|| format!("expected Python bool, got {raw}")),
        Some(other) if strip_bracket_type(other, &["list[", "List["]).is_some() => {
            let inner = strip_bracket_type(other, &["list[", "List["]).unwrap();
            let items = raw
                .as_array()
                .ok_or_else(|| format!("expected Python list, got {raw}"))?;
            let decoded = items
                .iter()
                .map(|v| json_to_vox_value(v, Some(inner)))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(VoxValue::list(decoded))
        }
        _ => json_any_to_vox_value(raw),
    }
}

fn json_any_to_vox_value(raw: &serde_json::Value) -> Result<crate::eval::value::VoxValue, String> {
    use crate::eval::value::VoxValue;
    match raw {
        serde_json::Value::Null => Ok(VoxValue::Null),
        serde_json::Value::Bool(b) => Ok(VoxValue::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(VoxValue::Int(i))
            } else if let Some(f) = n.as_f64() {
                Ok(VoxValue::Float(f))
            } else {
                Err(format!("unrepresentable Python number {n}"))
            }
        }
        serde_json::Value::String(s) => Ok(VoxValue::Str(s.as_str().into())),
        serde_json::Value::Array(items) => {
            let out = items
                .iter()
                .map(json_any_to_vox_value)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(VoxValue::list(out))
        }
        serde_json::Value::Object(map) => {
            let out = map
                .iter()
                .map(|(k, v)| Ok((k.clone(), json_any_to_vox_value(v)?)))
                .collect::<Result<Vec<_>, String>>()?;
            Ok(VoxValue::object(out))
        }
    }
}
