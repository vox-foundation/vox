use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::lex;
use vox_compiler::parser::parse;
use vox_codegen::syntax_k::{SyntaxKInput, canonical_web_ir_bytes, measure_syntax_k_event};
use vox_codegen::web_ir::lower::lower_hir_to_web_ir;

#[derive(Debug, Serialize, Deserialize, Default)]
struct ComplexityBudget {
    #[serde(default)]
    fixtures: HashMap<String, usize>,
}

pub(crate) fn run_k_complexity_budget(root: &Path, tolerance: f64, update: bool) -> Result<()> {
    let budget_path = root.join("contracts/eval/complexity-budget.v1.json");
    let mut budget = if budget_path.exists() {
        let content = fs::read_to_string(&budget_path)?;
        serde_json::from_str::<ComplexityBudget>(&content)?
    } else {
        ComplexityBudget::default()
    };

    let golden_dir = root.join("examples/golden");
    if !golden_dir.is_dir() {
        return Err(anyhow!("examples/golden directory not found"));
    }

    let mut failures = Vec::new();
    let mut new_budgets = HashMap::new();

    // Scan for .vox files in golden dir
    for entry in fs::read_dir(golden_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("vox") {
            let fixture_id = path.file_stem().unwrap().to_str().unwrap().to_string();
            let source = fs::read_to_string(&path)?;

            // Measure K-complexity of WebIR
            let tokens = lex(&source);
            let module =
                parse(tokens).map_err(|e| anyhow!("Failed to parse {}: {:?}", fixture_id, e))?;
            let hir = lower_module(&module);
            let web_ir = lower_hir_to_web_ir(&hir);
            let ir_bytes = canonical_web_ir_bytes(&web_ir)
                .map_err(|e| anyhow!("Failed to serialize IR {}: {:?}", fixture_id, e))?;

            let input = SyntaxKInput {
                fixture_id: &fixture_id,
                target_kind: "web_ir",
                bytes: &ir_bytes,
                source_hash: None,
                web_ir_hash: None,
                baseline_bytes: None,
                support_metrics: None,
            };

            let event = measure_syntax_k_event(input)
                .map_err(|e| anyhow!("Failed to measure K-complexity {}: {:?}", fixture_id, e))?;
            let current_k = event.k_est_bytes;

            new_budgets.insert(fixture_id.clone(), current_k);

            if let Some(&allowed) = budget.fixtures.get(&fixture_id) {
                let limit = (allowed as f64 * (1.0 + tolerance / 100.0)).ceil() as usize;
                if current_k > limit {
                    failures.push(format!(
                        "Fixture '{}' exceeded budget: {} > {} (allowed: {}, tolerance: {}%)",
                        fixture_id, current_k, limit, allowed, tolerance
                    ));
                }
            } else if !update {
                eprintln!("Warning: Fixture '{}' has no budget defined.", fixture_id);
            }
        }
    }

    let total_fixtures = new_budgets.len();
    if update {
        budget.fixtures = new_budgets;
        let content = serde_json::to_string_pretty(&budget)?;
        if let Some(parent) = budget_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&budget_path, content)?;
        println!(
            "Updated complexity budget baseline: {}",
            budget_path.display()
        );
    }

    if !failures.is_empty() {
        for f in &failures {
            eprintln!("  [K-Complexity] ERROR: {}", f);
        }
        anyhow::bail!(
            "K-complexity budget audit failed ({} violations)",
            failures.len()
        );
    }

    println!(
        "K-complexity budget OK ({} fixtures validated)",
        total_fixtures
    );
    Ok(())
}
