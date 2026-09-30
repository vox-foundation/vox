//! Task 14: `contracts/orchestration/model-defaults.v1.yaml` is the single
//! source of every default / fallback model id, and nothing else in scope
//! hard-codes a vendor model id.

use std::path::{Path, PathBuf};

use vox_config::model_defaults;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn contract() -> serde_yaml::Value {
    let raw =
        std::fs::read_to_string(repo_root().join("contracts/orchestration/model-defaults.v1.yaml"))
            .expect("model-defaults contract");
    serde_yaml::from_str(&raw).expect("valid yaml")
}

#[test]
fn every_contract_entry_is_exposed_and_matches() {
    let c = contract();
    let entries = c["defaults"].as_sequence().expect("defaults: [..]");
    assert!(!entries.is_empty());
    assert_eq!(
        entries.len(),
        model_defaults::all().len(),
        "one generated entry per contract entry"
    );
    for e in entries {
        let role = e["role"].as_str().expect("role");
        let d = model_defaults::all()
            .iter()
            .find(|d| d.role == role)
            .unwrap_or_else(|| panic!("role {role} not generated"));
        let models: Vec<&str> = match (&e["model"], &e["models"]) {
            (serde_yaml::Value::String(m), _) => vec![m.as_str()],
            (_, serde_yaml::Value::Sequence(ms)) => {
                ms.iter().map(|m| m.as_str().unwrap()).collect()
            }
            _ => panic!("role {role}: needs `model` or `models`"),
        };
        assert!(models.iter().all(|m| !m.trim().is_empty()), "{role}");
        assert_eq!(d.models, models.as_slice(), "{role}");
        assert!(
            e["rationale"].as_str().is_some_and(|r| !r.is_empty()),
            "{role}: rationale required"
        );
        assert!(
            e["verified_on"].as_str().is_some(),
            "{role}: verified_on required"
        );
        assert_eq!(model_defaults::default_for(role), Some(models[0]), "{role}");
    }
    assert_eq!(model_defaults::default_for("no-such-role"), None);
}

#[test]
fn bootstrap_fallbacks_are_the_contract_defaults() {
    use vox_config::bootstrap_inference as b;
    assert_eq!(b::RESEARCH_FLASH_FALLBACK, model_defaults::RESEARCH);
    assert_eq!(b::REVIEW_PREMIUM_FALLBACK, model_defaults::JUDGE);
    assert_eq!(b::NLI_FALLBACK, model_defaults::CLAIM_EXTRACTION);
    assert_eq!(b::REPAIR_LOOP_PREFERRED, model_defaults::CODE_REPAIR);
    assert_eq!(
        b::OPENROUTER_FREE_FALLBACK_MODELS,
        model_defaults::FREE_FLOOR
    );
}

#[test]
fn premium_aliases_and_classifier_come_from_the_contract() {
    let routing = vox_config::load_model_routing_config();
    let pins = vox_config::load_model_pins_config().expect("pins parse");
    for (alias, model) in model_defaults::premium_aliases() {
        assert_eq!(
            routing.premium_alias.get(alias).map(String::as_str),
            Some(model),
            "{alias}"
        );
        assert_eq!(
            pins.premium_alias.get(alias).map(String::as_str),
            Some(model),
            "{alias}"
        );
    }
    assert_eq!(
        routing.premium_alias.len(),
        model_defaults::premium_aliases().count()
    );
    assert_eq!(
        pins.classifier.primary.as_deref(),
        Some(model_defaults::CLASSIFIER_PRIMARY)
    );
    assert_eq!(
        pins.classifier.fallback.as_deref(),
        Some(model_defaults::CLASSIFIER_FALLBACK)
    );
}

/// Files whose job is to LIST models (catalogs, retirement lists) rather than
/// pick one — they legitimately name vendor ids.
/// (`model-pins.v1.yaml` `retired_ids` is handled by exempting exactly those ids.)
const LISTING_FILES: &[&str] = &["contracts/orchestration/model-catalog.bootstrap.v1.json"];

/// Files that name an id as an example or fixture, never as a default.
const EXAMPLE_FILES: &[&str] = &[
    // Lint rule's `minimal_repro` text shows a violating call.
    "crates/vox-code-audit/src/detectors/llm_provider_call.rs",
    // Test script: any cloud slug that needs a key; asserts it is refused.
    "scripts/axis-drive-openrouter-failure-honesty.vox",
];

/// Task 14 is split into 14a/14b/14c. These files are scheduled for 14b/14c;
/// each must still contain an offender (so the list only shrinks) and the list
/// must be empty when Task 14 is done.
const PENDING_14B_14C: &[&str] = &[
    // 14b
    "crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs",
    "crates/vox-orchestrator-mcp/src/chat_tools/mod.rs",
    "crates/vox-orchestrator-mcp/src/llm_bridge/providers/gemini.rs",
    // 14c
    "crates/vox-cli/src/commands/chat.rs",
    "crates/vox-cli/src/commands/status.rs",
    "crates/vox-corpus/src/synthetic_gen/bodies/_tool_pairs_body.rs",
    "crates/vox-gamify/src/ai/client/transport.rs",
    "crates/vox-gamify/src/ai/constants.rs",
    "crates/vox-gui/src/commands/harness_issues.rs",
    "crates/vox-scientia/src/evidence_assist.rs",
    "crates/vox-secrets/src/spec/registry/llm.rs",
];

fn scanned_files() -> Vec<PathBuf> {
    let root = repo_root();
    let mut out = Vec::new();
    let walk = |dir: &str, out: &mut Vec<PathBuf>, keep: &dyn Fn(&Path) -> bool| {
        for e in walkdir::WalkDir::new(root.join(dir))
            .into_iter()
            .filter_entry(|e| {
                let n = e.file_name().to_string_lossy();
                !matches!(
                    n.as_ref(),
                    "target" | "node_modules" | "tests" | "e2e" | "dist"
                )
            })
            .flatten()
        {
            if e.file_type().is_file() && keep(e.path()) {
                out.push(e.path().to_path_buf());
            }
        }
    };
    walk("crates", &mut out, &|p| {
        let n = p.file_name().unwrap().to_string_lossy();
        p.extension().is_some_and(|x| x == "rs")
            && p.to_string_lossy().contains("/src/")
            && n != "tests.rs"
            && !n.ends_with("_tests.rs")
            && !n.ends_with("_test.rs")
            && !n.starts_with("semcov")
    });
    walk("contracts/orchestration", &mut out, &|p| {
        p.extension().is_some_and(|x| x == "yaml" || x == "json")
    });
    walk("apps/editor/vox-vscode", &mut out, &|p| {
        let n = p.file_name().unwrap().to_string_lossy();
        n == "package.json" || (n.ends_with(".ts") && !n.ends_with(".test.ts"))
    });
    walk("scripts", &mut out, &|p| {
        p.extension().is_some_and(|x| x == "vox")
    });
    out
}

/// Source lines minus inline `#[cfg(test)] mod … { … }` blocks (in-file tests
/// keep their fixtures; brace-counted, so code after a mid-file test module is
/// still scanned) and minus comment lines (doc comments and examples stay).
fn scannable_lines(path: &Path, text: &str) -> Vec<(usize, String)> {
    let is_rs = path.extension().is_some_and(|x| x == "rs");
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut skip_depth: Option<i64> = None;
    let mut pending_test_attr = false;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim_start();
        if let Some(depth) = skip_depth.as_mut() {
            *depth += line.matches('{').count() as i64 - line.matches('}').count() as i64;
            if *depth <= 0 {
                skip_depth = None;
            }
            continue;
        }
        if is_rs && t.starts_with("#[cfg(") && t.contains("test") {
            pending_test_attr = true;
            continue;
        }
        if pending_test_attr {
            if t.is_empty() || t.starts_with("#[") || t.starts_with("//") {
                continue;
            }
            pending_test_attr = false;
            if (t.starts_with("mod ") || t.starts_with("pub(crate) mod ")) && t.ends_with('{') {
                skip_depth = Some(1);
                continue;
            }
        }
        if t.starts_with("//") || t.starts_with('#') || t.starts_with("* ") || t.starts_with("/*") {
            continue;
        }
        out.push((i + 1, line.to_string()));
    }
    out
}

#[test]
fn no_hardcoded_vendor_model_ids_outside_the_defaults_contract() {
    // Guards the class of bug in the 2026-09-20 `openrouter-free-slugs-churn`
    // note: a stale hardcoded id silently replaces the operator's intent.
    let id = regex::Regex::new(
        // A chat-LLM family after a vendor prefix (not URL paths like `openai/v1`
        // or non-chat assets like `openai/whisper-*`), or an unprefixed id.
        r#"(~?\b(google|anthropic|openai|deepseek|meta-llama|qwen|mistralai|microsoft|x-ai)/(gemini|gemma|claude|gpt|o[134]|deepseek|llama|qwen|mistral|mixtral|phi|grok)[a-z0-9.\-:]*)|["'`](gemini-[0-9]|gpt-[0-9]|claude-([0-9]|[a-z]+-[0-9])|o[134]-mini)"#,
    )
    .unwrap();
    let retired = vox_config::load_model_pins_config()
        .expect("pins parse")
        .retired_ids;
    let root = repo_root().canonicalize().unwrap();
    let mut offenders: Vec<String> = Vec::new();
    let mut files_with_offenders = std::collections::BTreeSet::new();
    for path in scanned_files() {
        let rel = path
            .canonicalize()
            .unwrap()
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if rel == "contracts/orchestration/model-defaults.v1.yaml"
            || LISTING_FILES.contains(&rel.as_str())
            || EXAMPLE_FILES.contains(&rel.as_str())
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        // Generated output copies its (scanned) source; drift is the generator's gate.
        if text
            .lines()
            .next()
            .is_some_and(|l| l.contains("@generated"))
        {
            continue;
        }
        for (n, line) in scannable_lines(&path, &text) {
            // Retired ids may be named (that is how they are retired).
            let hit = id.find_iter(&line).find(|m| {
                !retired
                    .iter()
                    .any(|r| m.as_str().trim_matches(['"', '\'', '`']) == r)
            });
            if let Some(m) = hit {
                files_with_offenders.insert(rel.clone());
                if !PENDING_14B_14C.contains(&rel.as_str()) {
                    offenders.push(format!("{rel}:{n}: {}", m.as_str()));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "hardcoded vendor model ids must live in contracts/orchestration/model-defaults.v1.yaml: {offenders:#?}"
    );
    let stale: Vec<_> = PENDING_14B_14C
        .iter()
        .filter(|p| !files_with_offenders.contains(**p))
        .collect();
    assert!(
        stale.is_empty(),
        "no longer pending — remove from PENDING_14B_14C: {stale:?}"
    );
}
