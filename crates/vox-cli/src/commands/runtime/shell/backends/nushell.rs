//! Nushell backend (optional): `nu -c` when installed.

use std::process::Command;

/// Runs a Nushell expression via `nu -c`. Returns (stdout, stderr, exit_code).
pub fn run_nu_capture(script: &str) -> std::io::Result<(String, String, Option<i32>)> {
    let output = Command::new("nu").args(["-c", script]).output()?;
    let code = output.status.code();
    Ok((
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        code,
    ))
}

/// Diagnose POSIX-isms in Nushell input and provide actionable Nushell syntax hints.
pub fn diagnose_nu_posix_isms(script: &str) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if script.contains("&&") {
        diagnostics
            .push("POSIX '&&' is not supported in Nushell; use ';' or 'and' instead".to_string());
    }
    if script.contains("||") {
        diagnostics.push("POSIX '||' is not supported in Nushell; use 'or' instead".to_string());
    }
    if script.contains("2>&1") {
        diagnostics.push(
            "POSIX '2>&1' is not supported in Nushell; use 'out+err>' or 'e>|' instead".to_string(),
        );
    }
    for var in ["$HOME", "$PATH", "$USER"] {
        if script.contains(var) {
            diagnostics.push(format!(
                "POSIX '{var}' should be referenced as '$env.{}' in Nushell",
                &var[1..]
            ));
        }
    }
    diagnostics
}

/// Runs a Nushell expression via `nu --no-config-file -c "do { <script> } | to json --raw"`,
/// returning `(stdout, stderr, exit_code, structured_data, syntax_hints)`.
pub fn run_nu_structured(
    script: &str,
) -> std::io::Result<(
    String,
    String,
    Option<i32>,
    Option<serde_json::Value>,
    Vec<String>,
)> {
    let hints = diagnose_nu_posix_isms(script);
    let wrapped = format!("do {{ {script} }} | to json --raw");
    let output = Command::new("nu")
        .args(["--no-config-file", "-c", &wrapped])
        .output()?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let parsed = serde_json::from_str::<serde_json::Value>(stdout.trim()).ok();
        return Ok((stdout, stderr, output.status.code(), parsed, hints));
    }
    let (stdout, stderr, code) = run_nu_capture(script)?;
    let parsed = serde_json::from_str::<serde_json::Value>(stdout.trim()).ok();
    Ok((stdout, stderr, code, parsed, hints))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nu_version_smoke() {
        if Command::new("nu").arg("--version").status().is_err() {
            return;
        }
        let (out, err, code) = run_nu_capture("version").expect("nu");
        assert!(code == Some(0), "stderr={err}");
        assert!(!out.is_empty());
    }

    #[test]
    fn posix_hints_detected() {
        let hints = diagnose_nu_posix_isms("ls && echo $HOME 2>&1 || true");
        assert_eq!(hints.len(), 4);
    }
}
