//! Optional `--test-health` checks (compile + coverage tools).

use tokio::process::Command;

use super::super::common::Check;

/// Run test-health probes only. Returns `true` if the caller should stop (test-health mode).
pub async fn run(test_health: bool, checks: &mut Vec<Check>) -> bool {
    if !test_health {
        return false;
    }
    println!("Running test health analysis...");

    let test_compile = Command::new("cargo")
        .args(["test", "--workspace", "--no-run"])
        .output()
        .await;

    checks.push(match test_compile {
        Ok(o) if o.status.success() => Check {
            name: "Test Compilation".to_string(),
            pass: true,
            detail: "all tests compile successfully".to_string(),
        },
        Ok(o) => Check {
            name: "Test Compilation".to_string(),
            pass: false,
            detail: format!(
                "compilation failed:\n{}",
                String::from_utf8_lossy(&o.stderr)
            ),
        },
        Err(e) => Check {
            name: "Test Compilation".to_string(),
            pass: false,
            detail: format!("failed to invoke cargo: {}", e),
        },
    });

    let llvm_cov = Command::new("cargo")
        .arg("llvm-cov")
        .arg("--version")
        .output()
        .await;
    checks.push(match llvm_cov {
        Ok(o) if o.status.success() => Check {
            name: "cargo-llvm-cov".to_string(),
            pass: true,
            detail: "found".to_string(),
        },
        _ => Check {
            name: "cargo-llvm-cov".to_string(),
            pass: false,
            detail: "not found — suggested for coverage: cargo install cargo-llvm-cov".to_string(),
        },
    });

    let nextest = Command::new("cargo")
        .arg("nextest")
        .arg("--version")
        .output()
        .await;
    checks.push(match nextest {
        Ok(o) if o.status.success() => Check {
            name: "cargo-nextest".to_string(),
            pass: true,
            detail: "found".to_string(),
        },
        _ => Check {
            name: "cargo-nextest".to_string(),
            pass: false,
            detail: "not found — suggested for fast testing: cargo install cargo-nextest"
                .to_string(),
        },
    });
    true
}
