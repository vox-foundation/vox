use super::*;

/// Populi Mesh (`iroh` QUIC / A2A) dispatch envelope verified by the `PopuliMesh` spoke oracle (`SPOKE8-02`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopuliMeshDispatchEnvelope {
    /// Target mesh node identifier.
    #[serde(alias = "node_id", alias = "target_node")]
    pub target_node_id: String,
    /// Compute capability tier (`"gpu_cuda" | "gpu_rocm" | "metal_unified" | "npu_infer" | "cpu" | "metal" | "cuda" | "rocm" | "apple_silicon" | "mps"`).
    #[serde(alias = "tier", alias = "compute_tier", alias = "accelerator")]
    pub capability_tier: String,
    /// Minimum VRAM required in GiB (`1..=192`).
    #[serde(alias = "vram_gb", alias = "min_vram_gb")]
    pub vram_min_gb: u32,
    /// `iroh://` QUIC endpoint URI.
    #[serde(alias = "endpoint", alias = "quic_endpoint")]
    pub iroh_endpoint: String,
    /// Odd BFT/Raft verification quorum size (`1..=15`, `quorum_size % 2 == 1`).
    #[serde(alias = "quorum")]
    pub quorum_size: u32,
    /// Mesh payload kind (`"a2a_task_envelope" | "tensor_shard_sync" | "gradient_all_reduce"`).
    #[serde(alias = "payload", alias = "task_kind")]
    pub payload_kind: String,
    /// Dispatch timeout in milliseconds (`100..=120_000`).
    #[serde(default = "default_populi_timeout_ms")]
    pub timeout_ms: u64,
}

const fn default_populi_timeout_ms() -> u64 {
    5_000
}

/// Strip spoken ASR disfluencies (`um`, `uh`, `er`, `like`, `basically`, `you know`) case-insensitively
/// and normalize spoken punctuation (`open brace`, `close brace`, `open paren`, `close paren`,
/// `colon`, `arrow`, `plus`, `equals`, `comma`, `semicolon`) before compiling dictated Vox snippets
/// (`SPOKE8-02`, `SOTA-SPOKE-HARDEN-03`, `SOTA-SPOKE-UNSEEN-02`).
#[must_use]
pub fn normalize_asr_transcript_for_vox(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    loop {
        let lower = s.to_ascii_lowercase();
        let mut stripped_any = false;
        for filler in [
            "um, ",
            "uh, ",
            "er, ",
            "ah, ",
            "like, ",
            "so, ",
            "basically, ",
            "you know, ",
            "i mean, ",
            "um ",
            "uh ",
            "er ",
            "ah ",
        ] {
            if lower.starts_with(filler) {
                s = s[filler.len()..].trim_start().to_string();
                stripped_any = true;
                break;
            }
        }
        if !stripped_any {
            break;
        }
    }
    for mid_filler in [
        " um, ",
        " uh, ",
        " er, ",
        " like, ",
        " you know, ",
        " basically, ",
        " i mean, ",
        " Um, ",
        " Uh, ",
        " Like, ",
        " Basically, ",
        " You know, ",
        " um ",
        " uh ",
        " er ",
    ] {
        s = s.replace(mid_filler, " ");
    }
    if !s.contains('{') && (s.contains("open brace") || s.contains("close brace")) {
        for (spoken, sym) in [
            ("open brace", "{\n"),
            ("close brace", "\n}"),
            ("open paren", "("),
            ("close paren", ")"),
            (" colon ", ": "),
            (" arrow ", " to "),
            (" plus ", " + "),
            (" minus ", " - "),
            (" times ", " * "),
            (" equals ", " = "),
            (" equal ", " = "),
            (" comma ", ", "),
            (" semicolon ", "\n"),
        ] {
            s = s.replace(spoken, sym);
        }
    }
    s
}

/// Verify an Oratio spoken-transcript-to-Vox round-trip completion (`SPOKE8-02`).
#[must_use]
pub fn verify_speech_to_code_roundtrip(
    case_id: &str,
    spoken_transcript: &str,
    candidate_vox: &str,
) -> SpokeRoundtripReceipt {
    let cleaned_vox = normalize_asr_transcript_for_vox(candidate_vox);
    let mut base = verify_voxlang_roundtrip(case_id, spoken_transcript, &cleaned_vox);
    if spoken_transcript.trim().is_empty() {
        base.semantic_valid = false;
        base.execution_passed = false;
        base.pass_at_1 = false;
        base.admitted_record = None;
        base.oracle_receipt.passed = false;
        base.oracle_receipt.stage = "speech_transcript_empty".to_string();
        return base;
    }
    if let Some(ref mut rec) = base.admitted_record {
        rec.provenance.task_family = "speech_to_code:oratio_vox_roundtrip".to_string();
    }
    base
}

/// Verify a `PopuliMesh` (`iroh` QUIC / A2A) dispatch envelope completion (`SPOKE8-02`).
#[must_use]
pub fn verify_populi_mesh_roundtrip(
    case_id: &str,
    prompt: &str,
    candidate_json: &str,
) -> SpokeRoundtripReceipt {
    let clean = strip_code_fences(candidate_json);
    let parsed: Result<PopuliMeshDispatchEnvelope, _> = serde_json::from_str(&clean);
    let Ok(env) = parsed else {
        return SpokeRoundtripReceipt {
            case_id: case_id.to_string(),
            spoke: SpokeKind::AgentHarness,
            syntax_valid: false,
            semantic_valid: false,
            execution_passed: false,
            pass_at_1: false,
            pass_at_k: false,
            stdout_or_result: String::new(),
            constructs_covered: Vec::new(),
            oracle_receipt: OracleReceipt {
                spoke: SpokeKind::AgentHarness,
                passed: false,
                stage: "populi_mesh_json_parse_failed".to_string(),
                diagnostics: vec!["invalid PopuliMeshDispatchEnvelope JSON".to_string()],
                exit_code: Some(1),
            },
            admitted_record: None,
        };
    };

    let valid_tier = matches!(
        env.capability_tier.as_str(),
        "gpu_cuda"
            | "gpu_rocm"
            | "metal_unified"
            | "npu_infer"
            | "cpu"
            | "metal"
            | "cuda"
            | "rocm"
            | "apple_silicon"
            | "mps"
            | "cuda_12"
            | "rocm_6"
    );
    let valid_vram = (1..=192).contains(&env.vram_min_gb);
    let valid_quorum = (1..=15).contains(&env.quorum_size) && env.quorum_size % 2 == 1;
    let valid_iroh = env.iroh_endpoint.starts_with("iroh://") && env.iroh_endpoint.len() >= 12;
    let valid_payload = matches!(
        env.payload_kind.as_str(),
        "a2a_task_envelope" | "tensor_shard_sync" | "gradient_all_reduce"
    );
    let valid_timeout = (100..=120_000).contains(&env.timeout_ms);
    let semantic_ok = !env.target_node_id.trim().is_empty()
        && valid_tier
        && valid_vram
        && valid_quorum
        && valid_iroh
        && valid_payload;
    let exec_ok = semantic_ok && valid_timeout;

    let receipt = OracleReceipt {
        spoke: SpokeKind::AgentHarness,
        passed: exec_ok,
        stage: if exec_ok {
            "populi_mesh_envelope+iroh_quorum_verified".to_string()
        } else {
            "populi_mesh_invariant_failed".to_string()
        },
        diagnostics: if exec_ok {
            Vec::new()
        } else {
            vec![
                "PopuliMeshDispatchEnvelope failed capability/quorum/iroh/payload invariants"
                    .to_string(),
            ]
        },
        exit_code: Some(if exec_ok { 0 } else { 1 }),
    };

    let admitted_record = if exec_ok {
        Some(TrainingRecordV2::new(
            SpokeKind::AgentHarness,
            prompt,
            "vox-populi-mesh",
            &clean,
            receipt.clone(),
            RecordProvenance {
                origin: "spoke_roundtrip".to_string(),
                source_id: case_id.to_string(),
                task_family: "populi_mesh".to_string(),
                difficulty: 5,
            },
        ))
    } else {
        None
    };

    SpokeRoundtripReceipt {
        case_id: case_id.to_string(),
        spoke: SpokeKind::AgentHarness,
        syntax_valid: true,
        semantic_valid: semantic_ok,
        execution_passed: exec_ok,
        pass_at_1: exec_ok,
        pass_at_k: exec_ok,
        stdout_or_result: format!(
            "populi://{}/{}?quorum={}",
            env.target_node_id, env.payload_kind, env.quorum_size
        ),
        constructs_covered: vec![env.capability_tier, env.payload_kind],
        oracle_receipt: receipt,
        admitted_record,
    }
}

/// Verify an `agentskills.io`-compliant `SKILL.md` document completion (`SPOKE8-02`).
#[must_use]
pub fn verify_skill_creation_roundtrip(
    case_id: &str,
    prompt: &str,
    candidate_skill_md: &str,
) -> SpokeRoundtripReceipt {
    let mut normalized = strip_code_fences(candidate_skill_md);
    if !normalized.starts_with("---\n")
        && !normalized.starts_with("---\r\n")
        && let Some(idx) = normalized
            .find("\n---\n")
            .or_else(|| normalized.find("\n---\r\n"))
    {
        normalized = normalized[idx + 1..].to_string();
    }
    let trimmed = normalized.trim();
    let has_frontmatter = trimmed.starts_with("---\n") || trimmed.starts_with("---\r\n");
    if !has_frontmatter {
        return SpokeRoundtripReceipt {
            case_id: case_id.to_string(),
            spoke: SpokeKind::McpToolChain,
            syntax_valid: false,
            semantic_valid: false,
            execution_passed: false,
            pass_at_1: false,
            pass_at_k: false,
            stdout_or_result: String::new(),
            constructs_covered: Vec::new(),
            oracle_receipt: OracleReceipt {
                spoke: SpokeKind::McpToolChain,
                passed: false,
                stage: "skill_frontmatter_missing".to_string(),
                diagnostics: vec!["SKILL.md must begin with YAML frontmatter `---`".to_string()],
                exit_code: Some(1),
            },
            admitted_record: None,
        };
    }

    let after_open = &trimmed[4..];
    let Some(close_idx) = after_open.find("\n---") else {
        return SpokeRoundtripReceipt {
            case_id: case_id.to_string(),
            spoke: SpokeKind::McpToolChain,
            syntax_valid: false,
            semantic_valid: false,
            execution_passed: false,
            pass_at_1: false,
            pass_at_k: false,
            stdout_or_result: String::new(),
            constructs_covered: Vec::new(),
            oracle_receipt: OracleReceipt {
                spoke: SpokeKind::McpToolChain,
                passed: false,
                stage: "skill_frontmatter_unclosed".to_string(),
                diagnostics: vec!["unclosed YAML frontmatter block in SKILL.md".to_string()],
                exit_code: Some(1),
            },
            admitted_record: None,
        };
    };

    let yaml_block = &after_open[..close_idx];
    let body_block = after_open[close_idx + 4..].trim();

    let mut skill_name = String::new();
    let mut skill_desc = String::new();
    let mut has_vox_meta = false;
    let mut has_vox_tools = false;

    for line in yaml_block.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("name:") {
            skill_name = rest.trim().trim_matches('"').trim_matches('\'').to_string();
        } else if let Some(rest) = l.strip_prefix("description:") {
            skill_desc = rest.trim().trim_matches('"').trim_matches('\'').to_string();
        } else if l.starts_with("vox-category:") || l.starts_with("vox-id:") {
            has_vox_meta = true;
        } else if l.starts_with("vox-tools:") {
            has_vox_tools = !l.strip_prefix("vox-tools:").unwrap_or("").trim().is_empty();
        }
    }

    let valid_name = !skill_name.is_empty()
        && skill_name.len() <= 64
        && skill_name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    let valid_desc = (10..=1024).contains(&skill_desc.len());
    let valid_body = body_block.len() >= 20 && body_block.contains("# ");

    let mut doctest_ok = true;
    let mut search_rest = body_block;
    while let Some(vox_start) = search_rest.find("```vox") {
        let after_tag = &search_rest[vox_start + 6..];
        let Some(nl_idx) = after_tag.find('\n') else {
            doctest_ok = false;
            break;
        };
        if !after_tag[..nl_idx].trim().is_empty() {
            search_rest = &after_tag[nl_idx + 1..];
            continue;
        }
        let rest = &after_tag[nl_idx + 1..];
        let Some(vox_end) = rest.find("```") else {
            doctest_ok = false;
            break;
        };
        let vox_code = &rest[..vox_end];
        search_rest = &rest[vox_end + 3..];
        if vox_code.trim_start().starts_with("// vox:skip") {
            continue;
        }
        let rule_violations =
            crate::corpus::corpus_readiness::check_vox_retired_and_enforcement_rules(vox_code);
        let diags = vox_compiler::pipeline::check_file(vox_code, "skill_doctest.vox");
        let err_count = diags
            .iter()
            .filter(|d| d.severity == vox_compiler::typeck::diagnostics::TypeckSeverity::Error)
            .count();
        if err_count > 0 || !rule_violations.is_empty() {
            doctest_ok = false;
            break;
        }
    }

    let semantic_ok = valid_name && valid_desc && has_vox_meta && has_vox_tools;
    let exec_ok = semantic_ok && valid_body && doctest_ok;

    let receipt = OracleReceipt {
        spoke: SpokeKind::McpToolChain,
        passed: exec_ok,
        stage: if exec_ok {
            format!("agentskills_yaml_frontmatter+vox_doctest_verified(name={skill_name})")
        } else {
            "agentskills_compliance_failed".to_string()
        },
        diagnostics: if exec_ok {
            Vec::new()
        } else {
            vec![
                "SKILL.md failed agentskills.io frontmatter/metadata/doctest compliance"
                    .to_string(),
            ]
        },
        exit_code: Some(if exec_ok { 0 } else { 1 }),
    };

    let admitted_record = if exec_ok {
        Some(TrainingRecordV2::new(
            SpokeKind::McpToolChain,
            prompt,
            "vox-skills",
            trimmed,
            receipt.clone(),
            RecordProvenance {
                origin: "spoke_roundtrip".to_string(),
                source_id: case_id.to_string(),
                task_family: "skill_creation".to_string(),
                difficulty: 4,
            },
        ))
    } else {
        None
    };

    SpokeRoundtripReceipt {
        case_id: case_id.to_string(),
        spoke: SpokeKind::McpToolChain,
        syntax_valid: true,
        semantic_valid: semantic_ok,
        execution_passed: exec_ok,
        pass_at_1: exec_ok,
        pass_at_k: exec_ok,
        stdout_or_result: format!("skill://{skill_name}"),
        constructs_covered: vec!["agentskills_yaml_frontmatter".to_string()],
        oracle_receipt: receipt,
        admitted_record,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POPULI_OK: &str = r#"{"target_node_id":"node-1","capability_tier":"gpu_cuda","vram_min_gb":24,"iroh_endpoint":"iroh://node-1:4433","quorum_size":3,"payload_kind":"a2a_task_envelope","timeout_ms":5000}"#;

    #[test]
    fn asr_normalization_strips_fillers_and_expands_spoken_punctuation() {
        assert_eq!(
            normalize_asr_transcript_for_vox("  Um, Basically, fn f"),
            "fn f"
        );
        let spoken = normalize_asr_transcript_for_vox(
            "fn add open paren a close paren open brace return a close brace",
        );
        assert!(spoken.contains('(') && spoken.contains(')'));
        assert!(spoken.contains("{\n") && spoken.contains("\n}"));
        assert!(!spoken.contains("open brace"));
        // Already-symbolic source is left alone.
        assert_eq!(normalize_asr_transcript_for_vox("fn a() {}"), "fn a() {}");
    }

    #[test]
    fn speech_roundtrip_tags_provenance_and_rejects_empty_transcript() {
        let code = "```vox\nfn greet(x: int) to int { return x + 1 }\n```";
        let ok = verify_speech_to_code_roundtrip("sp_ok", "um dictate a function", code);
        assert!(ok.execution_passed);
        assert_eq!(
            ok.admitted_record.expect("admitted").provenance.task_family,
            "speech_to_code:oratio_vox_roundtrip"
        );
        let empty = verify_speech_to_code_roundtrip("sp_empty", "   ", code);
        assert!(!empty.execution_passed);
        assert_eq!(empty.oracle_receipt.stage, "speech_transcript_empty");
        assert!(empty.admitted_record.is_none());
    }

    #[test]
    fn populi_roundtrip_accepts_valid_envelope_and_rejects_bad_ones() {
        let ok = verify_populi_mesh_roundtrip("pop_ok", "dispatch", POPULI_OK);
        assert!(ok.execution_passed);
        assert_eq!(
            ok.stdout_or_result,
            "populi://node-1/a2a_task_envelope?quorum=3"
        );

        let even = POPULI_OK.replace("\"quorum_size\":3", "\"quorum_size\":4");
        let r = verify_populi_mesh_roundtrip("pop_even", "dispatch", &even);
        assert!(r.syntax_valid && !r.execution_passed);
        assert_eq!(r.oracle_receipt.stage, "populi_mesh_invariant_failed");

        let garbage = verify_populi_mesh_roundtrip("pop_json", "dispatch", "not json");
        assert!(!garbage.syntax_valid);
        assert_eq!(
            garbage.oracle_receipt.stage,
            "populi_mesh_json_parse_failed"
        );
    }

    #[test]
    fn skill_roundtrip_requires_frontmatter() {
        let r = verify_skill_creation_roundtrip("sk_none", "author", "# Just a heading\n");
        assert!(!r.execution_passed);
        assert_eq!(r.oracle_receipt.stage, "skill_frontmatter_missing");
    }
}
