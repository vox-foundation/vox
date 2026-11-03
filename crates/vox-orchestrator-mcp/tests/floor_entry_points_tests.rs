//! Phase 12 Task 3: Source scan pinning every dispatcher and daemon client.
//!
//! Enforces that every in-process `handle_tool_call*` dispatcher and every
//! daemon `orch.tool_call` sender is explicitly classified as either a human
//! surface (permitted to resolve floored actions) or an agent entry point
//! (wrapped in `with_agent_origin` or flagged with `agent_origin: true`).

use std::path::{Path, PathBuf};

fn collect_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if file_name != "tests.rs" && !file_name.ends_with("_tests.rs") {
                files.push(path);
            }
        }
    }
}

/// Cuts lines at the first `#[cfg(test)]` whose next non-blank line starts with `mod ` and ends with `{`.
fn cut_test_modules(content: &str) -> Vec<&str> {
    let lines: Vec<&str> = content.lines().collect();
    let mut cut_idx = lines.len();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed == "#[cfg(test)]" {
            for next_line in &lines[i + 1..] {
                let next_trimmed = next_line.trim();
                if next_trimmed.is_empty() {
                    continue;
                }
                if next_trimmed.starts_with("mod ") && next_trimmed.ends_with('{') {
                    cut_idx = i;
                    break;
                }
                break;
            }
            if cut_idx < lines.len() {
                break;
            }
        }
    }
    lines[..cut_idx].to_vec()
}

fn is_fn_def(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("fn ")
        || t.starts_with("pub fn ")
        || t.starts_with("pub async fn ")
        || t.starts_with("pub(crate) fn ")
        || t.starts_with("pub(crate) async fn ")
        || t.starts_with("pub(super) fn ")
        || t.starts_with("pub(super) async fn ")
        || t.starts_with("async fn ")
}

#[test]
fn entry_point_callers_are_classified() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let crates_dir = manifest_dir
        .parent()
        .expect("CARGO_MANIFEST_DIR has parent");

    let mut rs_files = Vec::new();
    let entries = std::fs::read_dir(crates_dir).expect("read crates dir");
    for entry in entries.flatten() {
        let crate_path = entry.path();
        if crate_path.is_dir() {
            let src_dir = crate_path.join("src");
            if src_dir.is_dir() {
                collect_rs_files(&src_dir, &mut rs_files);
            }
        }
    }

    assert!(
        !rs_files.is_empty(),
        "expected to find rust source files under crates/*/src"
    );

    let allowed_part_a_files = [
        "vox-orchestrator-mcp/src/dispatch.rs",
        "vox-orchestrator-mcp/src/autonomous_tool_dispatch.rs",
        "vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs",
        "vox-orchestrator-mcp/src/http_gateway/rpc_tools.rs",
        "vox-orchestrator-mcp/src/daemon_extra.rs",
    ];

    let allowed_part_b_files = [
        "vox-orchestrator-mcp/src/daemon_route.rs",
        "vox-cli/src/commands/dei.rs",
        "vox-cli/src/commands/daemon_tool.rs",
        "vox-gui/src/commands/mcp.rs",
        "vox-gui/src/commands/browser.rs",
        "vox-gui/src/commands/chat.rs",
        "vox-gui/src/commands/chat_turn.rs",
        "vox-gui/src/commands/orchestrator.rs",
        "vox-gui/src/commands/harness.rs",
        "vox-gui/src/remote/calls.rs",
        "vox-gui/src/remote/gateway_client.rs",
    ];

    let mut part_a_count = 0;
    let mut part_b_count = 0;

    for file_path in &rs_files {
        let content = std::fs::read_to_string(file_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", file_path.display()));
        let norm_path = file_path.to_string_lossy().replace('\\', "/");
        let active_lines = cut_test_modules(&content);

        // Part (a): handle_tool_call callers
        for (idx, line) in active_lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || is_fn_def(line) {
                continue;
            }
            let is_handle_call = trimmed.contains("handle_tool_call(")
                || trimmed.contains("handle_tool_call_with_mode(")
                || trimmed.contains("handle_tool_call_with_receipt(");

            if is_handle_call {
                let matched = allowed_part_a_files
                    .iter()
                    .any(|allowed| norm_path.ends_with(allowed));
                assert!(
                    matched,
                    "Unclassified handle_tool_call site in {}: line {}: {}\nClassify it as a human surface or wrap with with_agent_origin",
                    norm_path,
                    idx + 1,
                    line
                );

                if norm_path.ends_with("autonomous_tool_dispatch.rs")
                    || norm_path.ends_with("chat_tools/chat/agent_loop.rs")
                    || norm_path.ends_with("http_gateway/rpc_tools.rs")
                {
                    let prev_has_origin =
                        idx > 0 && active_lines[idx - 1].contains("with_agent_origin(");
                    let line_has_origin = line.contains("with_agent_origin(");
                    assert!(
                        prev_has_origin || line_has_origin,
                        "Expected with_agent_origin wrapping call in {}: line {}: {}",
                        norm_path,
                        idx + 1,
                        line
                    );
                }

                part_a_count += 1;
            }
        }

        // Part (a) check for daemon_extra.rs
        if norm_path.ends_with("vox-orchestrator-mcp/src/daemon_extra.rs") {
            assert!(
                content.contains("get(\"agent_origin\")") && content.contains("with_agent_origin("),
                "daemon_extra.rs must contain both get(\"agent_origin\") and with_agent_origin("
            );
        }

        // Part (b): orch_daemon_method::TOOL_CALL or "orch.tool_call" senders
        if !norm_path.ends_with("vox-foundation/src/protocol.rs")
            && !norm_path.ends_with("vox-orchestrator-mcp/src/daemon_extra.rs")
        {
            for (idx, line) in active_lines.iter().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with("//") {
                    continue;
                }
                let is_tool_call_sender = trimmed.contains("orch_daemon_method::TOOL_CALL")
                    || trimmed.contains("\"orch.tool_call\"");

                if is_tool_call_sender {
                    let matched = allowed_part_b_files
                        .iter()
                        .any(|allowed| norm_path.ends_with(allowed));
                    assert!(
                        matched,
                        "Unclassified orch.tool_call sender in {}: line {}: {}\nClassify it as a human surface or mark with agent_origin: true",
                        norm_path,
                        idx + 1,
                        line
                    );

                    part_b_count += 1;
                }
            }
        }

        // Part (b) check for daemon_route.rs
        if norm_path.ends_with("vox-orchestrator-mcp/src/daemon_route.rs") {
            assert!(
                content.contains("\"agent_origin\": true"),
                "daemon_route.rs must contain \"agent_origin\": true"
            );
        }
    }

    assert!(
        part_a_count > 0,
        "expected to find part (a) handle_tool_call sites"
    );
    assert!(
        part_b_count > 0,
        "expected to find part (b) orch.tool_call sites"
    );

    // Part (c): server.rs contains daemon_route::call_tool_via_daemon(
    let server_rs = crates_dir.join("vox-orchestrator-mcp/src/server.rs");
    let server_content = std::fs::read_to_string(&server_rs)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", server_rs.display()));
    assert!(
        server_content.contains("daemon_route::call_tool_via_daemon("),
        "server.rs must contain daemon_route::call_tool_via_daemon("
    );
}
