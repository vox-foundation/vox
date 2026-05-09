use vox_code_audit::rules::{Finding, Severity};

pub fn print_terminal(findings: &[Finding], min_severity: Severity) {
    let filtered: Vec<_> = findings
        .iter()
        .filter(|f| f.severity >= min_severity)
        .collect();
    if filtered.is_empty() {
        println!("✓ No drift findings at {:?} level or above.", min_severity);
        return;
    }
    for f in &filtered {
        let icon = match f.severity {
            Severity::Info => "ℹ",
            Severity::Warning => "⚠",
            Severity::Error | Severity::Critical => "✗",
        };
        println!(
            "{} [{}] {}:{} — {}",
            icon,
            f.rule_id,
            f.file.display(),
            f.line,
            f.message
        );
        if let Some(s) = &f.suggestion {
            println!("  → {}", s);
        }
    }
    println!("\n{} finding(s).", filtered.len());
}

pub fn print_json(findings: &[Finding]) {
    println!(
        "{}",
        serde_json::to_string_pretty(findings).unwrap_or_default()
    );
}

pub fn exit_code(findings: &[Finding], fail_on: Severity) -> i32 {
    if findings.iter().any(|f| f.severity >= fail_on) {
        1
    } else {
        0
    }
}
