//! Heldout benchmark prompt assembly for `vox mens eval-local`.
//!
//! Kept separate from the GPU-gated `eval_local` module so unit tests run under default features.

#![cfg_attr(not(feature = "gpu"), allow(dead_code))]

use std::path::Path;

use vox_bounded_fs::read_utf8_path_capped;

/// Default `eval-local --max-tokens`. The longest reference answer in
/// `mens/data/heldout_bench` is 316 Qwen3 tokens (median 126, p90 215), so 1024
/// fits a fenced answer with some prose about 3x over. Generation stops at EOS,
/// so the budget only costs time on runaway samples.
pub(crate) const HELDOUT_MAX_TOKENS: usize = 1024;

/// Prompt protocol, recorded in the report and the checkpoint header: the
/// Qwen3 non-thinking codegen prefix (`/no_think` plus a prefilled empty think
/// block) that QLoRA training supervises.
pub(crate) const HELDOUT_PROMPT_PROTOCOL: &str = "qwen3_non_thinking_codegen_v1";

/// One benchmark row after resolving paths and assembling the inference prompt.
pub(crate) struct PreparedBench {
    pub(crate) manifest_index: usize,
    pub(crate) id: String,
    pub(crate) file: String,
    pub(crate) category: String,
    pub(crate) description: String,
    pub(crate) context_files: Vec<String>,
    pub(crate) prompt: String,
    pub(crate) semantic_expected_contains: Vec<String>,
}

/// Sort prompts lexicographically so consecutive inferences share common prefixes (KV / prefix-cache friendly).
pub(crate) fn sort_prepared_benches_lexicographic(prepared: &mut [PreparedBench]) {
    prepared.sort_by(|a, b| {
        a.prompt
            .cmp(&b.prompt)
            .then_with(|| a.manifest_index.cmp(&b.manifest_index))
    });
}

pub(crate) fn prepare_bench_item(
    bench: &Path,
    manifest_index: usize,
    bench_item: &serde_json::Value,
) -> PreparedBench {
    use owo_colors::OwoColorize;

    let id = bench_item["id"].as_str().unwrap_or("?").to_string();
    let file = bench_item["file"].as_str().unwrap_or("").to_string();
    let category = bench_item["category"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let description = bench_item["description"].as_str().unwrap_or("").to_string();
    let sample_path = bench.join(&file);
    let context_files: Vec<String> = bench_item["context_files"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let mut context_blob = String::new();
    if !context_files.is_empty() {
        let mut blocks: Vec<String> = Vec::new();
        for rel in &context_files {
            let p = bench.join(rel);
            match read_utf8_path_capped(&p) {
                Ok(text) => {
                    blocks.push(format!("### {rel}\n{text}"));
                }
                Err(e) => {
                    eprintln!(
                        "  {} Context file not read {}: {}",
                        "⚠".yellow(),
                        p.display(),
                        e
                    );
                }
            }
        }
        if !blocks.is_empty() {
            context_blob = format!(
                "You are given the following Vox source files as context (read-only):\n\n{}\n\n---\n\n",
                blocks.join("\n\n")
            );
        }
    }

    let primary_text = if sample_path.exists() {
        read_utf8_path_capped(&sample_path).unwrap_or_default()
    } else {
        String::new()
    };

    let body = if !description.is_empty() {
        let task = format!("Write a Vox program that: {}", description);
        if primary_text.trim().is_empty() {
            task
        } else {
            format!(
                "{}\n\nPrimary file `{}` (starter or spec you may replace):\n```vox\n{}\n```",
                task, file, primary_text
            )
        }
    } else if !primary_text.is_empty() {
        primary_text
    } else {
        eprintln!(
            "  {} Sample file not found: {}",
            "⚠".yellow(),
            sample_path.display()
        );
        String::new()
    };

    let raw_prompt = if context_blob.is_empty() {
        body
    } else {
        format!("{context_blob}{body}")
    };
    let prompt = if raw_prompt.trim().is_empty() {
        raw_prompt
    } else {
        use vox_eval::qwen_reasoning::{Lane, assistant_opener_for_lane, user_for_lane};
        let sys_prompt = vox_corpus::training::generate_training_system_prompt();
        // Byte-identical to the codegen training prefix (`chatml_prefix_for_lane`,
        // Lane::Codegen): `/no_think` on the user turn and an empty think block
        // prefilled, so Qwen3 answers directly instead of opening `<think>`.
        let user = user_for_lane(&raw_prompt, Lane::Codegen);
        let opener = assistant_opener_for_lane(Lane::Codegen);
        format!(
            "<|im_start|>system\n{sys_prompt}<|im_end|>\n<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n{opener}"
        )
    };
    let semantic_expected_contains: Vec<String> = bench_item["semantic_expected_contains"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    PreparedBench {
        manifest_index,
        id,
        file,
        category,
        description,
        context_files,
        prompt,
        semantic_expected_contains,
    }
}

#[cfg(test)]
mod tests {
    use super::{PreparedBench, prepare_bench_item, sort_prepared_benches_lexicographic};
    use serde_json::json;

    #[test]
    fn prepare_includes_context_files_and_primary_template() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("lib.vox"), "fn helper() to int { ret 1 }\n")
            .expect("write lib");
        std::fs::write(dir.path().join("app.vox"), "fn main() to int { ret 0 }\n")
            .expect("write app");
        let item = json!({
            "id": "t1",
            "file": "app.vox",
            "context_files": ["lib.vox"],
            "category": "grammar",
            "description": "Complete main."
        });
        let p = prepare_bench_item(dir.path(), 0, &item);
        assert_eq!(p.manifest_index, 0);
        assert_eq!(p.id, "t1");
        assert!(
            p.prompt
                .contains("You are given the following Vox source files")
        );
        assert!(p.prompt.contains("### lib.vox"));
        assert!(p.prompt.contains("fn helper()"));
        assert!(
            p.prompt
                .contains("Write a Vox program that: Complete main.")
        );
        assert!(p.prompt.contains("Primary file `app.vox`"));
        assert!(p.prompt.contains("fn main()"));
    }

    #[test]
    fn prepare_description_only_omits_primary_block_when_file_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let item = json!({
            "id": "t2",
            "file": "missing.vox",
            "description": "Hello."
        });
        let p = prepare_bench_item(dir.path(), 3, &item);
        assert_eq!(p.manifest_index, 3);
        assert!(p.prompt.contains("Write a Vox program that: Hello."));
        assert!(!p.prompt.contains("Primary file"));
    }

    #[test]
    fn sort_prepared_lexicographic_by_prompt() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.vox"), "a").expect("a");
        std::fs::write(dir.path().join("b.vox"), "b").expect("b");
        let mut prepared: Vec<PreparedBench> = vec![
            prepare_bench_item(
                dir.path(),
                0,
                &json!({"id":"z","file":"b.vox","description":""}),
            ),
            prepare_bench_item(
                dir.path(),
                1,
                &json!({"id":"y","file":"a.vox","description":""}),
            ),
        ];
        sort_prepared_benches_lexicographic(&mut prepared);
        assert_eq!(prepared[0].id, "y");
        assert_eq!(prepared[1].id, "z");
    }

    /// Parse-oracle smoke on a stratified golden set (actors, workflows, UI, MCP, types, HTTP, fullstack).
    #[test]
    fn golden_vox_parse_for_eval_oracle_smoke() {
        let golden = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/golden");
        if !golden.is_dir() {
            return;
        }
        const REL_PATHS: &[&str] = &[
            "hello.vox",
            "counter_actor.vox",
            "checkout_workflow.vox",
            "dashboard_ui.vox",
            "mcp_tools.vox",
            "ref_syntax.vox",
            "type_system.vox",
            "web_routing_fullstack.vox",
            "blog_fullstack.vox",
        ];
        for rel in REL_PATHS {
            let p = golden.join(rel);
            assert!(
                p.is_file(),
                "expected golden file {} (update REL_PATHS if renamed)",
                p.display()
            );
            let src = std::fs::read_to_string(&p).expect("read golden");
            let tokens = vox_compiler::lexer::lex(&src);
            let ok = vox_compiler::parser::parse(tokens).is_ok();
            assert!(
                ok,
                "golden {} must parse (Mens / eval prompt-matrix syntax oracle)",
                p.display()
            );
        }
        let mut total_vox = 0usize;
        for ent in walkdir::WalkDir::new(&golden)
            .into_iter()
            .filter_map(Result::ok)
        {
            if ent.file_type().is_file()
                && ent.path().extension().and_then(|e| e.to_str()) == Some("vox")
            {
                total_vox += 1;
            }
        }
        assert!(
            total_vox >= REL_PATHS.len(),
            "expected more goldens than the smoke set (drift if equal — add new .vox or shrink REL_PATHS)"
        );
    }

    /// Qwen3 thinks by default and is still thinking at any small budget, so the
    /// held-out prompt must be the non-thinking codegen prefix training
    /// supervises: `/no_think` on the user turn and an empty think block
    /// prefilled after `assistant\n`.
    #[test]
    fn the_heldout_prompt_is_non_thinking() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = prepare_bench_item(
            dir.path(),
            0,
            &json!({"id": "t", "file": "x.vox", "description": "Hello."}),
        );
        assert!(
            p.prompt
                .ends_with("<|im_start|>assistant\n<think>\n\n</think>\n\n"),
            "assistant turn must prefill the empty think block: {:?}",
            p.prompt
        );
        assert!(
            p.prompt.contains("Hello. /no_think<|im_end|>"),
            "user turn must end in /no_think: {:?}",
            p.prompt
        );
    }

    /// The longest reference answer in `mens/data/heldout_bench` is 316 Qwen3
    /// tokens (median 126), so 128 truncated half the bench even without think.
    #[test]
    fn the_heldout_budget_fits_a_full_answer() {
        assert_eq!(super::HELDOUT_MAX_TOKENS, 1024);
    }
}
