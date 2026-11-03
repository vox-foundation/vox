//! Shell execution probes normalized toward ACI `execution_probe` shapes.

use serde_json::json;

use super::backends::{nushell, powershell};

/// Which structured shell backend to use for a probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellBackendKind {
    PowerShell,
    Nushell,
}

/// Stdout/stderr/exit_code tuple aligned with `contracts/aci/agent-computer-interface.v1.schema.json` probe fields.
#[derive(Debug, Clone)]
pub struct ShellExecutionProbe {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub structured_data: Option<serde_json::Value>,
    pub syntax_hints: Vec<String>,
}

impl ShellExecutionProbe {
    /// JSON object suitable for merging into `aci.execution_probe`.
    pub fn to_json_value(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        obj.insert("stdout".into(), json!(self.stdout));
        obj.insert("stderr".into(), json!(self.stderr));
        obj.insert("exit_code".into(), json!(self.exit_code));
        if let Some(ref data) = self.structured_data {
            obj.insert("structured_data".into(), data.clone());
        }
        if !self.syntax_hints.is_empty() {
            obj.insert("syntax_hints".into(), json!(self.syntax_hints));
        }
        serde_json::Value::Object(obj)
    }
}

/// Runs `script` on the chosen backend (best-effort; surfaces stderr on spawn failure).
pub fn run_shell_probe(kind: ShellBackendKind, script: &str) -> ShellExecutionProbe {
    match kind {
        ShellBackendKind::PowerShell => match powershell::run_pwsh_capture(script) {
            Ok((stdout, stderr, exit_code)) => {
                let structured_data = serde_json::from_str::<serde_json::Value>(stdout.trim()).ok();
                ShellExecutionProbe {
                    stdout,
                    stderr,
                    exit_code,
                    structured_data,
                    syntax_hints: Vec::new(),
                }
            }
            Err(e) => ShellExecutionProbe {
                stdout: String::new(),
                stderr: format!("spawn error: {e}"),
                exit_code: None,
                structured_data: None,
                syntax_hints: Vec::new(),
            },
        },
        ShellBackendKind::Nushell => match nushell::run_nu_structured(script) {
            Ok((stdout, stderr, exit_code, structured_data, syntax_hints)) => ShellExecutionProbe {
                stdout,
                stderr,
                exit_code,
                structured_data,
                syntax_hints,
            },
            Err(e) => ShellExecutionProbe {
                stdout: String::new(),
                stderr: format!("spawn error: {e}"),
                exit_code: None,
                structured_data: None,
                syntax_hints: nushell::diagnose_nu_posix_isms(script),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_serializes() {
        let p = ShellExecutionProbe {
            stdout: "a".into(),
            stderr: "".into(),
            exit_code: Some(0),
            structured_data: Some(json!({"rows": 3})),
            syntax_hints: vec!["use ; instead of &&".into()],
        };
        let val = p.to_json_value();
        assert_eq!(val["stdout"].as_str(), Some("a"));
        assert_eq!(val["structured_data"]["rows"], 3);
        assert_eq!(val["syntax_hints"][0], "use ; instead of &&");
    }
}
