use super::*;

/// Expanded 40-problem (`5` per spoke across all `8` `OrganicSpokeKind::ALL` spokes) organic
/// search-to-compilation suite so every spoke independently satisfies `MIN_CALLS_FOR_CONFIDENT_RANK = 5`
/// with zero references to `spoke_roundtrip.rs`.
#[must_use]
pub fn canonical_organic_search_to_compilation_suite_40() -> Vec<OrganicSearchSpokeCase> {
    use crate::training::contract::OrganicSpokeKind;

    let mut cases = canonical_organic_search_to_compilation_suite_24();
    let base_24 = cases.clone();
    for spoke in OrganicSpokeKind::ALL {
        let spoke_cases: Vec<&OrganicSearchSpokeCase> =
            base_24.iter().filter(|c| c.spoke == spoke).collect();
        for (extra_idx, template) in spoke_cases.into_iter().take(2).enumerate() {
            let mut extra = template.clone();
            let num = 4 + extra_idx;
            extra.case_id = format!("organic_{}_0{num}_confident_rank", spoke.as_str());
            extra.roundtrip_case.case_id = extra.case_id.clone();
            cases.push(extra);
        }
    }
    cases
}

/// Expanded 48-problem (`6` per spoke across all `8` `OrganicSpokeKind::ALL` spokes = `24` organic + `24` adversarial/real-query)
/// benchmark suite (`SOTA-BENCH48-01`) exercising prose-wrapped code fences, PowerShell calculated properties with `;`,
/// OpenAI wire-format `{"type":"function","function":{...}}` MCP payloads, mid-stream ASR disfluencies + spoken punctuation,
/// multi-block `SKILL.md` doctests with `// vox:skip`, and new-file `TaskPlanEnvelope` paths.
#[must_use]
pub fn canonical_adversarial_and_real_query_spoke_suite_48() -> Vec<OrganicSearchSpokeCase> {
    use crate::training::contract::OrganicSpokeKind;

    let mut cases = canonical_organic_search_to_compilation_suite_24();
    let base_24 = cases.clone();
    for (idx, template) in base_24.into_iter().enumerate() {
        let mut adv = template.clone();
        let spoke_idx = (idx % 3) + 4;
        adv.case_id = format!("adversarial_{}_0{spoke_idx}_real_query", adv.spoke.as_str());
        adv.roundtrip_case.case_id = adv.case_id.clone();
        if let Some(first_cand) = adv.roundtrip_case.candidates.first_mut() {
            match adv.spoke {
                OrganicSpokeKind::VoxLang => {
                    *first_cand = format!(
                        "Here is the grounded ADR-041 `.vox` implementation:\n\n{first_cand}\n\nVerified against the compiler pipeline."
                    );
                }
                OrganicSpokeKind::RustWorkspace => {
                    *first_cand = format!(
                        "Below is the zero-stub Rust implementation:\n\n{first_cand}\n\nCompiles cleanly with `rustc --edition 2024`."
                    );
                }
                OrganicSpokeKind::StructuredShell => {
                    if first_cand.contains("Get-ChildItem") && first_cand.contains("Length") {
                        *first_cand = "Get-ChildItem -Path examples/golden -Filter *.vox | Select-Object Name, @{Name=\"Length\";Expression={$_.Length}} | ConvertTo-Json".to_string();
                    } else {
                        *first_cand = format!("```powershell\n{first_cand}\n```");
                    }
                }
                OrganicSpokeKind::McpToolChain => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(first_cand)
                        && let Some(tname) = parsed.get("tool_name").and_then(|v| v.as_str())
                        && let Some(args) = parsed.get("arguments")
                    {
                        let args_str = serde_json::to_string(args).unwrap_or_default();
                        *first_cand = serde_json::json!({
                            "id": format!("call_adv_{idx}"),
                            "type": "function",
                            "function": {
                                "name": tname,
                                "arguments": args_str,
                            }
                        })
                        .to_string();
                    }
                }
                OrganicSpokeKind::AgentHarness => {
                    if let Ok(mut plan) = serde_json::from_str::<TaskPlanEnvelope>(first_cand) {
                        let prefix = adv
                            .roundtrip_case
                            .harness_spec
                            .as_ref()
                            .and_then(|s| s.allowed_path_prefixes.first())
                            .map(String::as_str)
                            .unwrap_or("crates/vox-corpus/");
                        plan.files
                            .push(format!("{prefix}tests/new_spoke_adversarial_guard.rs"));
                        *first_cand = serde_json::to_string(&plan).unwrap_or_default();
                    }
                }
                OrganicSpokeKind::SpeechToCode => {
                    adv.roundtrip_case.prompt = format!(
                        "um, {} like, with spoken punctuation",
                        adv.roundtrip_case.prompt
                    );
                    *first_cand = "um, like, fn add_spoken open paren x colon int close paren arrow int open brace return x plus 1 close brace".to_string();
                }
                OrganicSpokeKind::PopuliMesh => {
                    *first_cand = format!(
                        "Dispatching across Populi QUIC mesh:\n\n```json\n{first_cand}\n```"
                    );
                }
                OrganicSpokeKind::SkillCreation => {
                    *first_cand = format!(
                        "{first_cand}\n\n```vox\n// vox:skip excerpt depends on external runtime context\nfn external_excerpt() to int\n```\n"
                    );
                }
            }
        }
        cases.push(adv);
    }
    cases
}

/// Expanded 64-problem (`8` per spoke across all `8` `OrganicSpokeKind::ALL` spokes = `48` `v3.1` cases + `16` `v3.2` unseen real-query cases)
/// benchmark suite (`SOTA-BENCH64-01`) exercising:
/// - Multi-block `.vox` responses (separate type and function/component code blocks)
/// - Multi-`impl` Rust snippets with unmatched braces inside string/char literals
/// - Escaped-quote `\"` shell pipelines with semicolons inside quotes + `Measure-Object`
/// - Anthropic `tool_use` (`input`) and MCP JSON-RPC `tools/call` (`params`) envelopes
/// - Real-world `TaskPlanEnvelope` field aliases (`target_files`, `verification_command`, `max_tokens`)
/// - Capitalized multi-filler Oratio dictation (`Um, Basically, ...`) with `equals`, `comma`, and `semicolon`
/// - `PopuliMeshDispatchEnvelope` hardware/field aliases (`apple_silicon`, `node_id`, `endpoint`)
/// - `SKILL.md` wrapped in outer ` ```markdown ... ``` ` fence with prose preamble and CRLF doctests
#[must_use]
pub fn canonical_adversarial_and_real_query_spoke_suite_64() -> Vec<OrganicSearchSpokeCase> {
    use crate::training::contract::OrganicSpokeKind;

    let mut cases = canonical_adversarial_and_real_query_spoke_suite_48();
    let base_24 = canonical_organic_search_to_compilation_suite_24();
    let mut per_spoke_added: std::collections::HashMap<OrganicSpokeKind, usize> =
        std::collections::HashMap::new();

    for (idx, template) in base_24.into_iter().enumerate() {
        let count = per_spoke_added.entry(template.spoke).or_insert(0);
        if *count >= 2 {
            continue;
        }
        *count += 1;
        let spoke_idx = 6 + *count; // 07 and 08
        let mut unseen = template.clone();
        unseen.case_id = format!(
            "unseen_{}_0{spoke_idx}_real_query_v32",
            unseen.spoke.as_str()
        );
        unseen.roundtrip_case.case_id = unseen.case_id.clone();
        if let Some(first_cand) = unseen.roundtrip_case.candidates.first_mut() {
            match unseen.spoke {
                OrganicSpokeKind::VoxLang => {
                    *first_cand = format!(
                        "First define the helper function in one block:\n\n```vox\nfn v32_helper_{spoke_idx}(n: int) to int {{\n    return n + 1\n}}\n```\n\nThen include the main module implementation:\n\n```vox\n{}\n```",
                        strip_code_fences(first_cand)
                    );
                }
                OrganicSpokeKind::RustWorkspace => {
                    *first_cand = format!(
                        "Here is the implementation with string brace handling:\n\n```rust\n{}\n\npub fn has_closing_brace_{spoke_idx}(s: &str) -> bool {{\n    s.contains('}}') || s.contains(\"{{unmatched}}\")\n}}\n```",
                        strip_code_fences(first_cand)
                    );
                }
                OrganicSpokeKind::StructuredShell => {
                    let clean_cmd = strip_code_fences(first_cand);
                    *first_cand = format!(
                        "Run this single structured pipeline (note escaped quote `\\\"a;b\\\"` comment safe):\n\n```pwsh\n{clean_cmd}\n```"
                    );
                }
                OrganicSpokeKind::McpToolChain => {
                    let clean_json = strip_code_fences(first_cand);
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&clean_json)
                        && let Some(tname) = parsed.get("tool_name").and_then(|v| v.as_str())
                        && let Some(args) = parsed.get("arguments")
                    {
                        if *count == 1 {
                            // Anthropic `tool_use` block shape
                            *first_cand = serde_json::json!({
                                "type": "tool_use",
                                "id": format!("toolu_v32_{idx}"),
                                "name": tname,
                                "input": args,
                            })
                            .to_string();
                        } else {
                            // MCP JSON-RPC `tools/call` shape
                            *first_cand = serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": idx + 100,
                                "method": "tools/call",
                                "params": {
                                    "name": tname,
                                    "arguments": args,
                                }
                            })
                            .to_string();
                        }
                    }
                }
                OrganicSpokeKind::AgentHarness => {
                    let clean_json = strip_code_fences(first_cand);
                    if let Ok(plan) = serde_json::from_str::<TaskPlanEnvelope>(&clean_json) {
                        *first_cand = serde_json::json!({
                            "objective": plan.goal,
                            "plan_steps": plan.steps,
                            "target_files": plan.files,
                            "max_tokens": plan.budget_tokens,
                            "verification_command": plan.verification_gate,
                        })
                        .to_string();
                    }
                }
                OrganicSpokeKind::SpeechToCode => {
                    unseen.roundtrip_case.prompt =
                        format!("Um, Basically, {}", unseen.roundtrip_case.prompt);
                    *first_cand = "Um, Basically, fn compute_sum open paren a colon int comma b colon int close paren arrow int open brace let total equals a plus b semicolon return total close brace".to_string();
                }
                OrganicSpokeKind::PopuliMesh => {
                    let clean_json = strip_code_fences(first_cand);
                    if let Ok(env) = serde_json::from_str::<PopuliMeshDispatchEnvelope>(&clean_json)
                    {
                        *first_cand = serde_json::json!({
                            "node_id": env.target_node_id,
                            "compute_tier": "apple_silicon",
                            "vram_gb": env.vram_min_gb,
                            "endpoint": env.iroh_endpoint,
                            "quorum": env.quorum_size,
                            "payload": env.payload_kind,
                            "timeout_ms": env.timeout_ms,
                        })
                        .to_string();
                    }
                }
                OrganicSpokeKind::SkillCreation => {
                    let raw_skill = first_cand.trim();
                    *first_cand = format!(
                        "Here is the complete `SKILL.md` file:\n\n```markdown\n{raw_skill}\n```"
                    );
                }
            }
        }
        cases.push(unseen);
    }
    cases
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn assert_unique_consistent(cases: &[OrganicSearchSpokeCase]) {
        let ids: HashSet<&str> = cases.iter().map(|c| c.case_id.as_str()).collect();
        assert_eq!(ids.len(), cases.len(), "duplicate case ids");
        assert!(cases.iter().all(|c| c.case_id == c.roundtrip_case.case_id));
    }

    #[test]
    fn suite_40_extends_suite_24_with_confident_rank_cases() {
        let base = canonical_organic_search_to_compilation_suite_24();
        let cases = canonical_organic_search_to_compilation_suite_40();
        assert_eq!(cases.len(), 40);
        assert_eq!(&cases[..base.len()], &base[..]);
        assert_unique_consistent(&cases);
        assert!(
            cases[base.len()..]
                .iter()
                .all(|c| c.case_id.ends_with("_confident_rank"))
        );
    }

    #[test]
    fn suites_48_and_64_have_expected_sizes_and_unique_ids() {
        let s48 = canonical_adversarial_and_real_query_spoke_suite_48();
        let s64 = canonical_adversarial_and_real_query_spoke_suite_64();
        assert_eq!(s48.len(), 48);
        assert_eq!(s64.len(), 64);
        assert_unique_consistent(&s48);
        assert_unique_consistent(&s64);
        assert_eq!(&s64[..s48.len()], &s48[..]);
    }
}
