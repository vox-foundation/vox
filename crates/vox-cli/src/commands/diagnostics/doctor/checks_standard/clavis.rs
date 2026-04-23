use super::super::common::Check;
use crate::commands::ci::run_body::run_body_helpers;

pub async fn run(_auto_heal: bool, checks: &mut Vec<Check>) {
    let cwd = std::env::current_dir().unwrap_or_default();

    // 1. Check Clavis Parity
    match run_body_helpers::run_clavis_parity(&cwd) {
        Ok(_) => {
            checks.push(Check::pass(
                "Clavis Parity",
                "contract (managed-env-names.v1.json) and docs are in sync with live code"
                    .to_string(),
            ));
        }
        Err(e) => {
            checks.push(Check::fail(
                "Clavis Parity",
                format!("{}. Run `vox ci clavis-contracts` or update docs.", e),
            ));
        }
    }

    // 2. Check Secret Env Guard (direct reads)
    // This is expensive if we scan all files, so we only do a shallow check or changed files
    // In doctor, we probably want to know if the current workspace state is "guard-compliant"
    match run_body_helpers::run_secret_env_guard(&cwd, false) {
        Ok(_) => {
            checks.push(Check::pass(
                "Secret Env Guard",
                "no direct secret env reads found in changed files".to_string(),
            ));
        }
        Err(e) => {
            checks.push(Check::fail(
                "Secret Env Guard",
                format!("{}. Migrate to clavis.resolve().", e),
            ));
        }
    }
}
