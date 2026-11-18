//! Vox-native execution adapter — persistent session REPL + tier-scoped capabilities.
//!
//! Calls `vox-compiler`'s pipeline to evaluate Vox source snippets and expressions of any type
//! (`Int`, `Float`, `Str`, `Bool`, `List`, `Map`, `Unit`, `Option`, `Result`) while persisting
//! declarations (`fn`, `type`, `import`, `@...`) and `let` bindings across terminal blocks.

use anyhow::{Result, anyhow};

use crate::host::TermSecurityLevel;

/// Returns `true` when `trimmed` explicitly begins with a Vox declaration or decorator keyword.
#[must_use]
pub fn is_explicit_vox_syntax(trimmed: &str) -> bool {
    trimmed.starts_with("fn ")
        || trimmed.starts_with("let ")
        || trimmed.starts_with("type ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("pub ")
        || trimmed.starts_with("async fn ")
        || trimmed.starts_with('@')
        || trimmed.starts_with("std.")
        || trimmed.starts_with("print(")
        || trimmed.starts_with("println(")
}

/// Heuristic filter avoiding speculative `vox-compiler` front-end work on obvious shell commands
/// (e.g. `echo hi`, `ls -la`, `cargo test`, `git status`), while admitting explicit Vox syntax,
/// literals/arithmetic expressions (`40 + 2`, `"hi"`), and symbols defined earlier in this session.
#[must_use]
pub fn should_try_vox_eval(
    trimmed: &str,
    known_decls: &[String],
    known_lets: &[(String, String)],
) -> bool {
    if trimmed.is_empty() {
        return false;
    }
    if is_explicit_vox_syntax(trimmed) {
        return true;
    }
    if trimmed.starts_with(['/', '.', '~', '-', '!', '?']) {
        return false;
    }
    if trimmed.starts_with(['"', '\'', '[', '(', '{'])
        || trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
        || matches!(trimmed, "true" | "false" | "null" | "Unit")
    {
        return true;
    }
    if let Some((head, _)) = trimmed.split_once(|c: char| c.is_whitespace() || c == '(') {
        if known_lets.iter().any(|(name, _)| name == head) {
            return true;
        }
        if known_decls
            .iter()
            .any(|d| d.contains(&format!("fn {head}(")) || d.contains(&format!("fn {head} ")))
        {
            return true;
        }
    }
    // Single expression with infix operator/call syntax and no POSIX pipeline/redirect characters
    !trimmed.contains(['|', '&', ';', '>', '<', '$', '`'])
        && (trimmed.contains(['+', '*', '/', '(', '=']) || trimmed.contains(" - "))
}

/// Persistent per-session `.vox` REPL state retaining declarations and `let` bindings across blocks.
#[derive(Debug, Default, Clone)]
pub struct VoxReplSession {
    decl_buffer: Vec<String>,
    let_buffer: Vec<(String, String)>,
}

impl VoxReplSession {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.decl_buffer.clear();
        self.let_buffer.clear();
    }

    #[must_use]
    pub fn decl_count(&self) -> usize {
        self.decl_buffer.len()
    }

    #[must_use]
    pub fn binding_count(&self) -> usize {
        self.let_buffer.len()
    }

    #[must_use]
    pub fn should_eval(&self, trimmed: &str) -> bool {
        should_try_vox_eval(trimmed, &self.decl_buffer, &self.let_buffer)
    }

    #[cfg(not(feature = "vox-lang"))]
    pub fn eval_in_session(
        &mut self,
        _src: &str,
        _level: TermSecurityLevel,
        _cwd: Option<&std::path::Path>,
    ) -> Result<String> {
        Err(anyhow!(
            "inline .vox evaluation requires `vox-terminal-core` feature `vox-lang`"
        ))
    }

    #[cfg(feature = "vox-lang")]
    pub fn eval_in_session(
        &mut self,
        src: &str,
        level: TermSecurityLevel,
        cwd: Option<&std::path::Path>,
    ) -> Result<String> {
        use vox_compiler::eval::Interpreter;
        use vox_compiler::eval::builtins::vox_value_display;
        use vox_compiler::eval::value::VoxValue;
        use vox_compiler::pipeline::{PipelineOptions, run_frontend_str_with_options};
        use vox_compiler::typeck::diagnostics::TypeckSeverity;

        let trimmed = src.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("empty Vox expression"));
        }

        let options = PipelineOptions {
            script_mode: true,
            ..PipelineOptions::default()
        };

        // Check whether this snippet is purely top-level declarations (`fn`, `type`, `import`, ...).
        let is_decl_only = trimmed.starts_with("fn ")
            || trimmed.starts_with("type ")
            || trimmed.starts_with("import ")
            || trimmed.starts_with("pub ")
            || trimmed.starts_with("async fn ")
            || trimmed.starts_with('@');

        // Detect simple `let <ident> = <rhs>` so we can evaluate and bind `<ident>` for future blocks.
        let let_binding_name = parse_simple_let_name(trimmed);

        let mut parts: Vec<String> = self.decl_buffer.clone();
        if !is_decl_only {
            let has_explicit_main = self.decl_buffer.iter().any(|d| d.contains("fn main"));
            if !has_explicit_main && !trimmed.contains("fn main") {
                for (_, stmt) in &self.let_buffer {
                    parts.push(stmt.clone());
                }
            }
        }

        if let Some(ref name) = let_binding_name {
            // Re-emit `<trimmed>\n<name>` in script mode so we verify types AND capture the bound value.
            parts.push(format!("{trimmed}\n{name}"));
        } else {
            parts.push(trimmed.to_string());
        }

        let combined = parts.join("\n");
        let res = run_frontend_str_with_options(&combined, "terminal.vox", &options)
            .map_err(|e| anyhow!("{e}"))?;

        let errors: Vec<String> = res
            .diagnostics
            .iter()
            .filter(|d| d.severity == TypeckSeverity::Error)
            .map(|d| d.message.clone())
            .collect();
        if !errors.is_empty() {
            return Err(anyhow!("{}", errors.join("; ")));
        }

        let mut interp = Interpreter::new(100_000);
        // Explicit capability assignment matching session `TermSecurityLevel` & `cwd`.
        interp.caps = capabilities_for_session(level, cwd);
        if let Some(dir) = cwd.filter(|d| d.is_dir()) {
            interp.source_path = Some(dir.join("terminal.vox"));
        }
        interp.run_module(&res.hir).map_err(|e| anyhow!("{e:?}"))?;

        let rendered = if res.hir.functions.iter().any(|f| f.name == "main") {
            let val = interp.call("main", vec![]).map_err(|e| anyhow!("{e:?}"))?;
            if let Some(name) = let_binding_name {
                let stored_stmt =
                    value_to_let_literal(&name, &val).unwrap_or_else(|| trimmed.to_string());
                if let Some(existing) = self.let_buffer.iter_mut().find(|(n, _)| n == &name) {
                    existing.1 = stored_stmt;
                } else {
                    self.let_buffer.push((name.clone(), stored_stmt));
                }
                if val == VoxValue::Null {
                    format!("{name} = ()")
                } else {
                    format!("{name} = {}", vox_value_display(&val))
                }
            } else if val == VoxValue::Null {
                "()".to_string()
            } else {
                vox_value_display(&val)
            }
        } else {
            if is_decl_only && !trimmed.contains("fn main") {
                self.decl_buffer.push(trimmed.to_string());
            }
            "ok".to_string()
        };

        Ok(rendered)
    }
}

#[cfg(feature = "vox-lang")]
fn parse_simple_let_name(trimmed: &str) -> Option<String> {
    if trimmed.contains('\n') {
        return None;
    }
    let rest = trimmed.strip_prefix("let ")?.trim_start();
    let rest = rest.strip_prefix("mut ").unwrap_or(rest).trim_start();
    let (lhs, _rhs) = rest.split_once('=')?;
    let ident = lhs.split(':').next()?.trim();
    if ident.is_empty()
        || !ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        || ident.chars().next().is_some_and(|c| c.is_ascii_digit())
    {
        return None;
    }
    Some(ident.to_string())
}

#[cfg(feature = "vox-lang")]
fn value_to_let_literal(name: &str, val: &vox_compiler::eval::value::VoxValue) -> Option<String> {
    use vox_compiler::eval::value::VoxValue;
    match val {
        VoxValue::Int(n) => Some(format!("let {name} = {n}")),
        VoxValue::Float(f) if f.is_finite() => Some(format!("let {name} = {f:?}")),
        VoxValue::Bool(b) => Some(format!("let {name} = {b}")),
        VoxValue::Str(s) => Some(format!("let {name} = {:?}", s.as_ref())),
        VoxValue::List(items) => {
            let mut rendered = Vec::with_capacity(items.len());
            for item in items.iter() {
                match item {
                    VoxValue::Int(n) => rendered.push(n.to_string()),
                    VoxValue::Float(f) if f.is_finite() => rendered.push(format!("{f:?}")),
                    VoxValue::Bool(b) => rendered.push(b.to_string()),
                    VoxValue::Str(s) => rendered.push(format!("{:?}", s.as_ref())),
                    _ => return None,
                }
            }
            Some(format!("let {name} = [{}]", rendered.join(", ")))
        }
        _ => None,
    }
}

/// Derive Vox interpreter `CapabilitySet` from the session security level and tracked `cwd`.
#[cfg(feature = "vox-lang")]
pub fn capabilities_for_session(
    level: TermSecurityLevel,
    cwd: Option<&std::path::Path>,
) -> vox_compiler::eval::caps::CapabilitySet {
    use vox_compiler::eval::caps::CapabilitySet;
    let existing_cwd = cwd.filter(|d| d.is_dir()).map(std::path::Path::to_path_buf);
    match level {
        TermSecurityLevel::L1ReadOnly => {
            if let Some(dir) = existing_cwd
                && let Ok(caps) =
                    CapabilitySet::from_roots(vec![dir], vec![], &["time:real", "env:ro"])
            {
                return caps;
            }
            CapabilitySet::parse("time:real,env:ro").unwrap_or_else(|_| {
                CapabilitySet::parse("").expect("empty capability spec is valid")
            })
        }
        TermSecurityLevel::L2WorkspaceBuild | TermSecurityLevel::L3StructuredDefault => {
            if let Some(dir) = existing_cwd
                && let Ok(caps) = CapabilitySet::from_roots(
                    vec![],
                    vec![dir],
                    &["time:real", "env:ro", "process:allow"],
                )
            {
                return caps;
            }
            CapabilitySet::parse("time:real,env:ro,process:allow").unwrap_or_else(|_| {
                CapabilitySet::parse("").expect("empty capability spec is valid")
            })
        }
        TermSecurityLevel::L4HostGated | TermSecurityLevel::L5AutopilotFloor => {
            CapabilitySet::developer_default()
        }
    }
}

/// Evaluate a single stateless Vox snippet or expression (any type) under restrictive caps.
#[cfg(feature = "vox-lang")]
pub fn eval_line(src: &str) -> Result<String> {
    let mut repl = VoxReplSession::new();
    repl.eval_in_session(src, TermSecurityLevel::L1ReadOnly, None)
}

/// Fallback diagnostic when `vox-terminal-core` is built without feature `vox-lang`.
#[cfg(not(feature = "vox-lang"))]
pub fn eval_line(_src: &str) -> Result<String> {
    Err(anyhow!(
        "inline .vox evaluation requires `vox-terminal-core` feature `vox-lang`"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vox_syntax_filter_and_repl_state_behave_cleanly() {
        assert!(is_explicit_vox_syntax("let x = 1"));
        assert!(is_explicit_vox_syntax("std.frame.summary()"));
        assert!(!is_explicit_vox_syntax("echo hi"));

        let lets = vec![("x".to_string(), "let x = 10".to_string())];
        let decls = vec!["fn add(a: int) to int { return a + 1 }".to_string()];
        assert!(should_try_vox_eval("x + 2", &decls, &lets));
        assert!(should_try_vox_eval("add(3)", &decls, &lets));
        assert!(!should_try_vox_eval("cargo test -p foo", &decls, &lets));
        assert!(!should_try_vox_eval("/usr/bin/env", &decls, &lets));

        let mut repl = VoxReplSession::new();
        assert_eq!(repl.decl_count(), 0);
        assert_eq!(repl.binding_count(), 0);
        assert!(repl.should_eval("40 + 2"));
        repl.reset();
        assert_eq!(repl.decl_count(), 0);
        assert_eq!(repl.binding_count(), 0);
    }

    #[cfg(feature = "vox-lang")]
    #[test]
    fn repl_evaluates_multi_type_and_persists_state_until_reset() {
        let mut repl = VoxReplSession::new();
        let out1 = repl
            .eval_in_session("let count = 7", TermSecurityLevel::L1ReadOnly, None)
            .unwrap();
        assert!(out1.contains("count = 7"), "got {out1:?}");
        assert_eq!(repl.binding_count(), 1);
        let out2 = repl
            .eval_in_session("count * 6", TermSecurityLevel::L1ReadOnly, None)
            .unwrap();
        assert_eq!(out2, "42");
        let b_out = repl
            .eval_in_session("true", TermSecurityLevel::L1ReadOnly, None)
            .unwrap();
        assert_eq!(b_out, "true");
        repl.reset();
        assert_eq!(repl.binding_count(), 0);
    }
}
