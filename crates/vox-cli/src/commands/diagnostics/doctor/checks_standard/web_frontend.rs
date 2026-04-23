//! Node / pnpm / shadcn scaffold checks for the React interop stack (WS12).

use tokio::process::Command;

use crate::frontend::pnpm_executable;

use super::super::common::Check;

/// Optional readiness checks for Vite + pnpm + shadcn-style `components.json`.
pub async fn run(checks: &mut Vec<Check>) {
    let pnpm = pnpm_executable();
    let pnpm_out = Command::new(pnpm).arg("--version").output().await;
    let (pnpm_ok, pnpm_detail) = match pnpm_out {
        Ok(o) if o.status.success() => {
            let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
            (
                true,
                if ver.is_empty() {
                    format!("{pnpm} — ok")
                } else {
                    format!("{pnpm} {ver}")
                },
            )
        }
        Ok(o) => (
            false,
            format!(
                "{pnpm} exited {} — install Node + pnpm for `vox run` / Vite",
                o.status
            ),
        ),
        Err(_) => (
            false,
            format!("{pnpm} not found — required for frontend installs/builds"),
        ),
    };
    checks.push(Check {
        name: "pnpm (frontend)".to_string(),
        pass: pnpm_ok,
        detail: pnpm_detail,
    });

    let node_bin = if cfg!(windows) { "node.exe" } else { "node" };
    let node_out = Command::new(node_bin).arg("--version").output().await;
    let (node_ok, node_detail) = match node_out {
        Ok(o) if o.status.success() => (
            true,
            format!("{} {}", node_bin, String::from_utf8_lossy(&o.stdout).trim()),
        ),
        _ => (
            false,
            "node not found — required for Vite / React builds".to_string(),
        ),
    };
    checks.push(Check {
        name: "node (frontend)".to_string(),
        pass: node_ok,
        detail: node_detail,
    });

    let mut cj_path = std::path::PathBuf::from("app/components.json");
    let mut found = tokio::fs::try_exists(&cj_path).await.unwrap_or(false);
    if !found {
        cj_path = std::path::PathBuf::from("components.json");
        found = tokio::fs::try_exists(&cj_path).await.unwrap_or(false);
    }

    if found {
        let raw = match tokio::fs::read_to_string(&cj_path).await {
            Ok(s) => s,
            Err(e) => {
                checks.push(Check {
                    name: "components.json (shadcn)".to_string(),
                    pass: false,
                    detail: format!("read {}: {e}", cj_path.display()),
                });
                return;
            }
        };
        let v: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                checks.push(Check {
                    name: "components.json (shadcn)".to_string(),
                    pass: false,
                    detail: format!("invalid JSON: {e}"),
                });
                return;
            }
        };
        let rsc = v.get("rsc").and_then(|x| x.as_bool()).unwrap_or(true);
        let pass = !rsc;
        checks.push(Check {
            name: "components.json (shadcn)".to_string(),
            pass,
            detail: if pass {
                format!(
                    "{} has \"rsc\": false — compatible with v0 / client components",
                    cj_path.display()
                )
            } else {
                format!(
                    "{}: set \"rsc\": false for Vox + v0 / shadcn client-mode interop",
                    cj_path.display()
                )
            },
        });
    } else {
        checks.push(Check {
            name: "components.json (shadcn)".to_string(),
            pass: true,
            detail: "not present — optional; run `vox build --scaffold` or `VOX_WEB_EMIT_SCAFFOLD=1` to seed shadcn schema + app shell"
                .to_string(),
        });
    }

    // Tailwind v4 entry — `@import "tailwindcss"` in app/globals.css (scaffold SSOT).
    let mut tw_path = std::path::PathBuf::from("app/globals.css");
    let mut tw_exists = tokio::fs::try_exists(&tw_path).await.unwrap_or(false);
    if !tw_exists {
        tw_path = std::path::PathBuf::from("globals.css");
        tw_exists = tokio::fs::try_exists(&tw_path).await.unwrap_or(false);
    }
    if tw_exists {
        match tokio::fs::read_to_string(&tw_path).await {
            Ok(raw) => {
                let ok = raw.contains("@import \"tailwindcss\"")
                    || raw.contains("@import 'tailwindcss'");
                checks.push(Check {
                    name: "tailwindcss v4 entry (globals.css)".to_string(),
                    pass: ok,
                    detail: if ok {
                        format!("{} includes @import \"tailwindcss\"", tw_path.display())
                    } else {
                        format!(
                            "{}: add `@import \"tailwindcss\";` for Tailwind v4 + @tailwindcss/vite",
                            tw_path.display()
                        )
                    },
                });
            }
            Err(e) => checks.push(Check {
                name: "tailwindcss v4 entry (globals.css)".to_string(),
                pass: false,
                detail: format!("read {}: {e}", tw_path.display()),
            }),
        }
    } else {
        checks.push(Check {
            name: "tailwindcss v4 entry (globals.css)".to_string(),
            pass: true,
            detail: "app/globals.css not found — optional until `vox build --scaffold`".to_string(),
        });
    }
}
