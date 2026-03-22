mod cache;
mod fix_pipeline;

use std::path::PathBuf;

use anyhow::Context;
use owo_colors::OwoColorize;
use vox_toestub::rules::{Language, Severity};
use vox_toestub::{Finding, OutputFormat, ToestubConfig, ToestubEngine};

use vox_db::{
    Codex, add_suppression, load_baseline as db_load_baseline, load_latest_task_queue,
    save_baseline as db_save_baseline, save_task_queue,
};
use vox_ludus::{LudusProfile, db};

/// Run the TOESTUB analysis.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    path: &std::path::Path,
    format: Option<&str>,
    severity: Option<&str>,
    suggest_fixes: bool,
    rules: Option<&str>,
    excludes: &[String],
    langs: Option<&str>,
    baseline: Option<&str>,
    save_baseline: Option<&str>,
    task_list: bool,
    import_suppressions: bool,
    ingest_findings: Option<&std::path::Path>,
    fix_pipeline: bool,
    fix_pipeline_apply: bool,
    gate: Option<&str>,
    gate_budget_path: Option<&std::path::Path>,
    _verify_impacted: bool,
    _max_escalation: u8,
    self_heal_safe_mode: bool,
) -> anyhow::Result<()> {
    // --task-list: show last saved queue from VoxDB and exit
    if task_list {
        if let Ok(db) = Codex::connect_default().await {
            let user_id = vox_db::paths::local_user_id();
            if let Ok(Some((total_findings, fix_suggestions_json))) =
                load_latest_task_queue(&db, &user_id).await
            {
                let fix_suggestions: Vec<vox_toestub::task_queue::FixSuggestion> =
                    serde_json::from_str(&fix_suggestions_json).unwrap_or_default();
                let queue = vox_toestub::TaskQueue {
                    total_findings: total_findings as usize,
                    fix_suggestions,
                };
                println!("{}", queue.to_markdown_checklist());
            } else {
                println!("No saved task queue found. Run a scan first.");
            }
            db.shutdown_for_drop();
        } else {
            println!("Could not connect to VoxDB. Run a scan to save a task queue.");
        }
        return Ok(());
    }

    // --import-suppressions: load toestub.toml and upsert into VoxDB, then exit
    if import_suppressions {
        let db = Codex::connect_default().await?;
        let toml_path = path.join("toestub.toml");
        let content = tokio::fs::read_to_string(&toml_path)
            .await
            .with_context(|| format!("Failed to read {}", toml_path.display()))?;
        let parsed: toml::Value = toml::from_str(&content)?;
        let count = if let Some(arr) = parsed.get("suppress").and_then(|s| s.as_array()) {
            let mut n = 0u32;
            for item in arr {
                if let Some(t) = item.as_table() {
                    let path_str = t
                        .get("path")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| anyhow::anyhow!("suppress entry missing 'path'"))?
                        .to_string();
                    let line = t
                        .get("line")
                        .and_then(|v| v.as_integer())
                        .ok_or_else(|| anyhow::anyhow!("suppress entry missing 'line'"))?;
                    let rule_id = t
                        .get("rule")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| anyhow::anyhow!("suppress entry missing 'rule'"))?
                        .to_string();
                    let reason = t.get("reason").and_then(|v| v.as_str());
                    add_suppression(&db, &path_str, line, &rule_id, reason).await?;
                    n += 1;
                }
            }
            n
        } else {
            0
        };
        println!(
            "Imported {} suppression(s) from toestub.toml into VoxDB.",
            count
        );
        db.shutdown_for_drop();
        return Ok(());
    }

    if let Some(ingest_path) = ingest_findings {
        let content = std::fs::read_to_string(ingest_path)
            .with_context(|| format!("Failed to read {}", ingest_path.display()))?;
        let findings: Vec<Finding> = serde_json::from_str(&content)?;
        let ingest_path = ingest_path.to_path_buf();
        let run_scope = path.to_string_lossy().to_string();
        let total = findings.len();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let r = (|| -> anyhow::Result<()> {
                let db = Codex::connect_default_sync()
                    .map_err(|e| anyhow::anyhow!("connect: {:?}", e))?;
                let user_id = vox_db::paths::local_user_id();
                let task_queue = vox_toestub::TaskQueue::from_findings(&findings);
                let fix_suggestions_json = serde_json::to_string(&task_queue.fix_suggestions)
                    .unwrap_or_else(|_| "[]".to_string());
                db.store()
                    .block_on(async {
                        save_task_queue(
                            &db,
                            &user_id,
                            &run_scope,
                            task_queue.total_findings as i64,
                            &fix_suggestions_json,
                        )
                        .await
                    })
                    .map_err(|e| anyhow::anyhow!("save_task_queue: {:?}", e))?;
                println!(
                    "Ingested {} findings from {} into VoxDB task queue (user_id={}).",
                    total,
                    ingest_path.display(),
                    user_id
                );
                db.shutdown_for_drop();
                Ok(())
            })();
            let _ = tx.send(r);
        });
        tokio::task::block_in_place(|| rx.recv())
            .map_err(|_| anyhow::anyhow!("ingest thread channel closed"))??;
        return Ok(());
    }

    // Connect to VoxDB early (for cache + baseline/task persistence)
    let db_opt = Codex::connect_default().await.ok().map(std::sync::Arc::new);

    let schema_path = tokio::task::spawn_blocking(|| std::fs::canonicalize("vox-schema.json").ok())
        .await
        .ok()
        .flatten();

    let config = ToestubConfig {
        roots: vec![PathBuf::from(path)],
        min_severity: match severity.unwrap_or("warning") {
            "info" => Severity::Info,
            "error" => Severity::Error,
            "critical" => Severity::Critical,
            _ => Severity::Warning,
        },
        format: OutputFormat::parse_format(format.unwrap_or("terminal")),
        suggest_fixes,
        languages: langs.map(|l| {
            l.split(',')
                .filter_map(|s| match s.trim() {
                    "rust" | "rs" => Some(Language::Rust),
                    "ts" | "typescript" => Some(Language::TypeScript),
                    "python" | "py" => Some(Language::Python),
                    "gdscript" | "gd" => Some(Language::GDScript),
                    "vox" => Some(Language::Vox),
                    _ => None,
                })
                .collect()
        }),
        excludes: excludes.to_vec(),
        rule_filter: rules.map(|r| r.split(',').map(|s| s.trim().to_string()).collect()),
        schema_path,
        ..Default::default()
    };

    // CPU-heavy scan: keep off the Tokio worker thread (nested async DB calls are not used here).
    let (result, output) = tokio::task::spawn_blocking(move || {
        let engine = ToestubEngine::new(config);
        engine.run_and_report()
    })
    .await
    .context("spawn_blocking join (stub-check engine)")?;

    // Print the formatted output
    println!("{}", output);

    // Print summary footer
    let summary = result.summary();
    if result.findings.is_empty() {
        println!(
            "{}",
            "🦶 TOESTUB: All clear — no issues found.".green().bold()
        );
    } else {
        println!(
            "{} Scanned {} files with {} rules, found {} issues.",
            "🦶 TOESTUB:".bold(),
            result.files_scanned,
            result.rules_applied,
            result.findings.len(),
        );
        if summary.critical > 0 || summary.error > 0 {
            println!(
                "{}",
                format!(
                    "   ⚠  {} critical, {} errors require attention.",
                    summary.critical, summary.error,
                )
                .red()
            );
        }
    }

    // If fix suggestions were requested, also dump the task queue
    if suggest_fixes && !result.task_queue.fix_suggestions.is_empty() {
        println!("\n{}", result.task_queue.to_markdown_checklist());
    }

    // ── Fix pipeline: staged passes for high-volume rules ──
    if fix_pipeline && !result.findings.is_empty() {
        let apply = fix_pipeline_apply && !self_heal_safe_mode;
        let findings = result.findings.clone();
        let path_buf = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            fix_pipeline::run_fix_pipeline(&findings, &path_buf, apply)
        })
        .await
        .context("fix pipeline task join")??;
    }

    // ── Save baseline / task queue to VoxDB ──
    if let Some(ref db) = db_opt {
        let user_id = vox_db::paths::local_user_id();
        let run_scope = path.to_string_lossy().to_string();

        if let Some(name) = save_baseline {
            let findings_json =
                serde_json::to_string(&result.findings).unwrap_or_else(|_| "[]".to_string());
            if db_save_baseline(db, name, &run_scope, &findings_json)
                .await
                .is_ok()
            {
                println!("\n  Saved baseline '{}' to VoxDB.", name);
            }
        }

        if suggest_fixes {
            let fix_suggestions_json = serde_json::to_string(&result.task_queue.fix_suggestions)
                .unwrap_or_else(|_| "[]".to_string());
            let _ = save_task_queue(
                db,
                &user_id,
                &run_scope,
                result.task_queue.total_findings as i64,
                &fix_suggestions_json,
            )
            .await;
        }
    }

    // ── Tiered CI gate: warning budget and ratchet ──
    if let (Some(gate_mode), Some(budget_path)) = (gate, gate_budget_path) {
        let by_rule: std::collections::HashMap<String, u32> =
            result
                .findings
                .iter()
                .fold(std::collections::HashMap::new(), |mut m, f| {
                    *m.entry(f.rule_id.clone()).or_insert(0) += 1;
                    m
                });
        match gate_mode.to_lowercase().as_str() {
            "ratchet" => {
                let arr: Vec<serde_json::Value> = by_rule
                    .into_iter()
                    .map(|(rule_id, count)| serde_json::json!({ "rule_id": rule_id, "count": count }))
                    .collect();
                let json = serde_json::to_string_pretty(&arr).unwrap_or_else(|_| "[]".to_string());
                if let Some(p) = budget_path.parent() {
                    let _ = tokio::fs::create_dir_all(p).await;
                }
                tokio::fs::write(budget_path, json).await?;
                println!(
                    "\n  Gate ratchet: saved budget to {}",
                    budget_path.display()
                );
            }
            "warnings" => {
                let content = tokio::fs::read_to_string(budget_path)
                    .await
                    .with_context(|| format!("Failed to read budget {}", budget_path.display()))?;
                let budget_arr: Vec<serde_json::Value> = serde_json::from_str(&content)?;
                let budget: std::collections::HashMap<String, u32> = budget_arr
                    .iter()
                    .filter_map(|o| {
                        let id = o.get("rule_id")?.as_str()?.to_string();
                        let c = o.get("count")?.as_u64()? as u32;
                        Some((id, c))
                    })
                    .collect();
                let gated_rules = ["stub/", "doc/missing-frontmatter", "unwired/module"]; // toestub-ignore(stub)
                let mut over = Vec::new();
                for (rule_id, &current) in &by_rule {
                    let budget_count = budget.get(rule_id).copied().unwrap_or(0);
                    if current <= budget_count {
                        continue;
                    }
                    if gated_rules
                        .iter()
                        .any(|p| rule_id == *p || rule_id.starts_with(*p))
                    {
                        over.push((rule_id.clone(), current, budget_count));
                    }
                }
                if !over.is_empty() {
                    for (id, cur, b) in &over {
                        eprintln!("  {}: current {} > budget {}", id, cur, b);
                    }
                    anyhow::bail!(
                        "Gate warnings: {} rule(s) exceed budget. Ratchet with --gate ratchet --gate-budget-path {}",
                        over.len(),
                        budget_path.display()
                    );
                }
            }
            _ => {}
        }
    }

    // ── Baseline diff: exit 1 on new or regressed findings ──
    if let Some(baseline_arg) = baseline {
        let baseline_map = if baseline_arg.contains('/')
            || baseline_arg.contains('\\')
            || baseline_arg.ends_with(".json")
        {
            let path = PathBuf::from(baseline_arg);
            let s = tokio::fs::read_to_string(&path).await?;
            cache::baseline_from_json(&s)?
        } else {
            let db = db_opt
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("VoxDB not available."))?;
            if let Some((_scope, findings_json)) =
                db_load_baseline(db.as_ref(), baseline_arg).await?
            {
                cache::baseline_from_json(&findings_json)?
            } else {
                anyhow::bail!("Baseline '{}' not found in VoxDB.", baseline_arg);
            }
        };

        let mut new = Vec::new();
        let mut regressed = Vec::new();
        for f in &result.findings {
            let key = fix_pipeline::norm_key(&f.file, f.line, &f.rule_id);
            match baseline_map.get(&key) {
                None => new.push(f.clone()),
                Some(sev) if f.severity > *sev => regressed.push((f.clone(), *sev)),
                _ => {}
            }
        }
        if !new.is_empty() || !regressed.is_empty() {
            if !new.is_empty() {
                eprintln!("{} new finding(s) not in baseline:", new.len());
                for f in new.iter().take(20) {
                    eprintln!("  {}:{} {}", f.file.display(), f.line, f.rule_id);
                }
                if new.len() > 20 {
                    eprintln!("  ... and {} more", new.len() - 20);
                }
            }
            if !regressed.is_empty() {
                eprintln!(
                    "{} regressed (severity increased) vs baseline:",
                    regressed.len()
                );
                for (f, _) in regressed.iter().take(20) {
                    eprintln!("  {}:{} {}", f.file.display(), f.line, f.rule_id);
                }
                if regressed.len() > 20 {
                    eprintln!("  ... and {} more", regressed.len() - 20);
                }
            }
            anyhow::bail!(
                "Baseline comparison failed: {} new, {} regressed.",
                new.len(),
                regressed.len()
            );
        }
    }

    // ── Gamification Auto-Rewards ──
    if result.findings.is_empty() {
        // Reward the user for a clean codebase!
        if let Some(ref db) = db_opt {
            let user_id = vox_db::paths::local_user_id();
            let mut profile = match db::get_profile(db, &user_id).await.unwrap_or(None) {
                Some(p) => p,
                None => {
                    let p = LudusProfile::new_default(&user_id);
                    db::upsert_profile(db.as_ref(), &p).await.ok();
                    p
                }
            };

            let mut xp_gain = 10;
            let mut crystal_gain = 5;

            if let Ok(Some(raw)) = db
                .store()
                .get_user_preference(&user_id, "gamify.clean_run_xp")
                .await
                && let Ok(val) = raw.parse::<u64>()
            {
                xp_gain = val;
            }
            if let Ok(Some(raw)) = db
                .as_ref()
                .store()
                .get_user_preference(&user_id, "gamify.clean_run_crystals")
                .await
                && let Ok(val) = raw.parse::<u64>()
            {
                crystal_gain = val;
            }

            let leveled_up = profile.add_xp(xp_gain);
            profile.add_crystals(crystal_gain);

            if db::upsert_profile(db.as_ref(), &profile).await.is_ok() {
                println!();
                println!("{}", "🎉 Gamification Rewards!".bright_yellow());
                println!("  +{} XP", xp_gain.to_string().bright_cyan());
                println!("  +{} Crystals", crystal_gain.to_string().bright_cyan());

                if leveled_up {
                    println!(
                        "  {} Level Up! You are now level {}",
                        "⭐".bright_yellow(),
                        profile.level.to_string().bright_white()
                    );
                }
            }
        }
    } else {
        println!(
            "\n  {} Want extra rewards? Run {} to fight these bugs in a battle.",
            "🔮".bright_magenta(),
            "vox ludus battle start".bright_green()
        );
    }

    if let Some(ref db) = db_opt {
        db.shutdown_for_drop();
    }

    if result.has_errors() {
        anyhow::bail!(
            "TOESTUB found {} error-level issues.",
            summary.error + summary.critical
        );
    }

    Ok(())
}
