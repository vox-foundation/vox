//! Doctor output formatting.

/// Print check results summary.
pub fn print_results(checks: &[super::common::Check], test_health: bool, json: bool) {
    if json {
        print_results_json(checks);
        return;
    }

    let mut failed = 0;
    for check in checks {
        if check.pass {
            println!("  ✓  {:25} {}", check.name, check.detail);
        } else {
            println!("  ✗  {:25} {}", check.name, check.detail);
            failed += 1;
        }
    }

    println!();
    if failed == 0 {
        if test_health {
            println!("✓ Test Health checks passed — automation is healthy!");
        } else {
            println!("✓ All checks passed — you're ready to build with Vox!");
        }
    } else {
        println!(
            "✗ {} check(s) failed — resolve the issues above before building.",
            failed
        );
    }
}

fn print_results_json(checks: &[super::common::Check]) {
    if let Ok(json) = serde_json::to_string_pretty(checks) {
        println!("{}", json);
    }
}
