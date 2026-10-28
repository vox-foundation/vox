//! B2.1 — Corpus readiness gate & `.vox` deterministic compiler/interpreter admission (`CORPUS-02`).
//!
//! Checks whether a corpus has enough rows AND sufficient semantic diversity
//! before any GPU training spend is authorized, and gates positive `.vox` training
//! records through `lex -> parse -> HIR -> typeck -> interp`.

/// Minimum thresholds a corpus must meet before training may proceed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MinReadiness {
    /// Minimum number of training rows required.
    pub min_rows: usize,
    /// Minimum AST/semantic diversity score (0.0–1.0).
    pub min_ast_diversity: f64,
}

/// Report produced by [`assess_corpus_readiness`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReadinessReport {
    /// Number of rows found in the corpus.
    pub rows: usize,
    /// Computed AST diversity score (0.0–1.0).
    pub ast_diversity: f64,
    /// Whether the row count meets the minimum.
    pub rows_ok: bool,
    /// Whether the diversity score meets the minimum.
    pub diversity_ok: bool,
    /// True only when both `rows_ok` and `diversity_ok` are true.
    pub ready: bool,
}

/// Assess whether a corpus (as a slice of JSONL rows) meets the minimum
/// readiness thresholds before GPU training spend is authorized.
///
/// Diversity is measured via [`vox_eval::eval_semantic_entropy`] over all
/// `"task"`, `"output"`, `"response"`, and `"vox_code"` strings found in the
/// rows.  When none are present the score is 0.0.
pub fn assess_corpus_readiness(rows: &[serde_json::Value], min: &MinReadiness) -> ReadinessReport {
    let rows_ok = rows.len() >= min.min_rows;

    // Collect candidate strings for diversity analysis.
    let mut strings: Vec<String> = Vec::with_capacity(rows.len());
    for row in rows {
        for field in &["task", "output", "response", "vox_code"] {
            if let Some(s) = row.get(field).and_then(|v| v.as_str()) {
                strings.push(s.to_owned());
                break; // one string per row is enough for diversity
            }
        }
    }

    let entropy = vox_eval::eval_semantic_entropy(&strings, min.min_ast_diversity);
    let ast_diversity = entropy.ast_diversity;
    let diversity_ok = ast_diversity >= min.min_ast_diversity;
    let ready = rows_ok && diversity_ok;

    ReadinessReport {
        rows: rows.len(),
        ast_diversity,
        rows_ok,
        diversity_ok,
        ready,
    }
}

/// Deterministic `.vox` compiler + interpreter admission gate (`CORPUS-02`).
///
/// Runs `vox_compiler::pipeline::run_frontend_str` (`lex -> parse -> HIR -> typeck`),
/// rejects retired decorator syntax (`@table`, `@query fn`, `@mutation fn`, `@component fn`, `@mcp.tool`),
/// and runs an in-process `Interpreter::run_module` smoke check on the lowered HIR.
#[must_use]
pub fn verify_vox_admission(source: &str) -> crate::training::OracleReceipt {
    use crate::training::{OracleReceipt, SpokeKind};

    let trimmed = source.trim();
    if trimmed.is_empty() {
        return OracleReceipt {
            spoke: SpokeKind::VoxLang,
            passed: false,
            stage: "empty_check".to_string(),
            diagnostics: vec!["Empty .vox snippet rejected".to_string()],
            exit_code: Some(1),
        };
    }

    for retired in [
        "@table",
        "@query fn",
        "@mutation fn",
        "@component fn",
        "@mcp.tool",
        "@server fn",
        "@endpoint",
    ] {
        if trimmed.contains(retired) {
            return OracleReceipt {
                spoke: SpokeKind::VoxLang,
                passed: false,
                stage: "grammar_unification_gate".to_string(),
                diagnostics: vec![format!("Retired decorator syntax `{retired}` is forbidden")],
                exit_code: Some(1),
            };
        }
    }

    match vox_compiler::pipeline::run_frontend_str(trimmed, "<corpus_admission.vox>") {
        Ok(res) => {
            let errors: Vec<String> = res
                .diagnostics
                .iter()
                .filter(|d| d.severity == vox_compiler::typeck::diagnostics::TypeckSeverity::Error)
                .map(|d| d.message.clone())
                .collect();
            if !errors.is_empty() {
                return OracleReceipt {
                    spoke: SpokeKind::VoxLang,
                    passed: false,
                    stage: "typeck".to_string(),
                    diagnostics: errors,
                    exit_code: Some(1),
                };
            }

            let mut interp = vox_compiler::eval::Interpreter::new(10_000);
            if let Err(e) = interp.run_module(&res.hir) {
                return OracleReceipt {
                    spoke: SpokeKind::VoxLang,
                    passed: false,
                    stage: "interp_smoke".to_string(),
                    diagnostics: vec![format!("Interpreter module registration failed: {e:?}")],
                    exit_code: Some(1),
                };
            }

            OracleReceipt {
                spoke: SpokeKind::VoxLang,
                passed: true,
                stage: "lex+parse+hir+typeck+interp".to_string(),
                diagnostics: Vec::new(),
                exit_code: Some(0),
            }
        }
        Err(err) => OracleReceipt {
            spoke: SpokeKind::VoxLang,
            passed: false,
            stage: "parse".to_string(),
            diagnostics: vec![err.to_string()],
            exit_code: Some(1),
        },
    }
}

/// Gate a candidate `(prompt, response)` `.vox` pair and construct a verified [`crate::training::TrainingRecordV2`]
/// if and only if [`verify_vox_admission`] passes (`CORPUS-02`).
#[must_use]
pub fn admit_vox_record(
    prompt: &str,
    response: &str,
    origin: &str,
    source_id: &str,
    task_family: &str,
) -> Option<crate::training::TrainingRecordV2> {
    let receipt = verify_vox_admission(response);
    if !receipt.passed {
        return None;
    }
    let provenance = crate::training::RecordProvenance {
        origin: origin.to_string(),
        source_id: source_id.to_string(),
        task_family: task_family.to_string(),
        difficulty: crate::training::construct_difficulty(task_family, "vox_source"),
    };
    Some(crate::training::TrainingRecordV2::new(
        crate::training::SpokeKind::VoxLang,
        prompt,
        crate::training::builtin_system_prompt(),
        response,
        receipt,
        provenance,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_when_above_thresholds() {
        let templates: Vec<&str> = vec![
            "fn alpha() -> Result { ok() }",
            "if ready { process() } else { skip() }",
            "match mode { A => op_a() _ => op_b() }",
            "for item in batch { handle(item) }",
            "let val = fetch().await?; val",
            "while running { tick() sleep() }",
            "pub struct Config { field: Type }",
            "impl Trait for Foo { fn method(&self) -> R { body() } }",
            "use crate::module::Thing;",
            "pub enum Kind { Alpha Beta Gamma }",
            "async fn serve(ctx: Ctx) -> Response { ctx.run().await }",
            "return Err(anyhow::anyhow!(msg))",
            "let _ = spawn(async move { worker().await });",
            "tracing::info!(target: msg)",
            "assert_eq!(left right msg)",
            "vec![alpha beta gamma delta]",
            "Box::new(handler(config))",
            "Arc::clone(&shared_state)",
            "serde_json::from_str::<T>(raw)?",
            "tokio::select! { a = fut_a => handle_a(a) b = fut_b => handle_b(b) }",
        ];
        let rows: Vec<_> = (0..200)
            .map(|i| {
                serde_json::json!({
                    "task": templates[i % templates.len()],
                    "lane": "test"
                })
            })
            .collect();
        let r = assess_corpus_readiness(
            &rows,
            &MinReadiness {
                min_rows: 100,
                min_ast_diversity: 0.1,
            },
        );
        assert!(r.rows_ok, "200 rows should meet min_rows=100");
        assert!(
            r.ready,
            "should be ready with sufficient rows and diversity (got ast_diversity={})",
            r.ast_diversity
        );
    }

    #[test]
    fn not_ready_when_too_few_rows() {
        let rows: Vec<_> = (0..5)
            .map(|i| serde_json::json!({"task": format!("t{i}")}))
            .collect();
        let r = assess_corpus_readiness(
            &rows,
            &MinReadiness {
                min_rows: 100,
                min_ast_diversity: 0.0,
            },
        );
        assert!(!r.rows_ok, "5 rows should not meet min_rows=100");
        assert!(!r.ready, "not ready with too few rows");
    }

    #[test]
    fn empty_corpus_is_not_ready() {
        let r = assess_corpus_readiness(
            &[],
            &MinReadiness {
                min_rows: 1,
                min_ast_diversity: 0.0,
            },
        );
        assert!(!r.ready);
        assert_eq!(r.rows, 0);
        assert_eq!(r.ast_diversity, 0.0);
    }

    #[test]
    fn diversity_gate_can_block_even_with_enough_rows() {
        let rows: Vec<_> = (0..200)
            .map(|_| serde_json::json!({"task": "same task every time"}))
            .collect();
        let r = assess_corpus_readiness(
            &rows,
            &MinReadiness {
                min_rows: 100,
                min_ast_diversity: 0.9,
            },
        );
        assert!(r.rows_ok, "200 rows meet min_rows");
        assert!(
            !r.diversity_ok,
            "identical rows should fail high diversity threshold"
        );
        assert!(!r.ready);
    }

    #[test]
    fn verify_vox_admission_accepts_valid_vox_and_rejects_broken_or_retired() {
        let valid = "fn add(a: int, b: int) to int { return a + b }";
        let receipt = verify_vox_admission(valid);
        assert!(receipt.passed, "valid vox must pass: {:?}", receipt);
        assert_eq!(receipt.stage, "lex+parse+hir+typeck+interp");

        let rec = admit_vox_record(
            "Write an add function in Vox",
            valid,
            "repo_source",
            "examples/add.vox",
            "vox_fn",
        )
        .expect("admit valid vox record");
        assert!(rec.verify_integrity());

        let retired = "@table type User { name: str }";
        let retired_receipt = verify_vox_admission(retired);
        assert!(!retired_receipt.passed);
        assert_eq!(retired_receipt.stage, "grammar_unification_gate");

        let broken = "fn broken(a: int to int { return a }";
        let broken_receipt = verify_vox_admission(broken);
        assert!(!broken_receipt.passed);
        assert!(admit_vox_record("broken", broken, "synthetic", "broken.vox", "vox_fn").is_none());
    }
}
