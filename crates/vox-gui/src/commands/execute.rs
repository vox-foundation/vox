use serde::Serialize;
use serde_json::Value;
use tauri_plugin_shell::ShellExt;

const VOX_SIDECAR_NAME: &str = "vox";

#[derive(Serialize)]
pub struct ExecuteOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Run a `vox` CLI command through the bundled sidecar.
///
/// D-03 confirmation gate: an invocation whose effective argv walks to a destructive index row is
/// refused with `vox_cli::command_catalog::CONFIRMATION_REQUIRED_PREFIX` + row id unless the
/// caller passes `confirmed: true`. `confirmed` is optional, so existing callers that omit it keep
/// working for every non-destructive op. All argv and gate logic lives in `vox-cli`.
///
/// Output is buffered whole (`.output()`), with no cap or timeout: a long or chatty verb returns
/// only when it exits. The UI bounds what it renders (`RunOutputDialog`) and guards against
/// concurrent duplicate runs; streaming/capping here is a separate backend change.
#[tauri::command]
pub async fn execute_command(
    app: tauri::AppHandle,
    path: Vec<String>,
    args: Value,
    confirmed: Option<bool>,
) -> Result<ExecuteOutput, String> {
    // The only way to an argv: assembly and the D-03 gate are one vox-cli call (WR-02).
    let shell_args = vox_cli::command_catalog::prepare_execution(path, &args, confirmed)?;

    let output = app
        .shell()
        .sidecar(VOX_SIDECAR_NAME)
        .map_err(|e| e.to_string())?
        .args(shell_args)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    Ok(ExecuteOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use vox_cli::command_catalog::{CONFIRMATION_REQUIRED_PREFIX, prepare_execution};

    /// Behaviour of the step `execute_command` performs before the sidecar spawn.
    #[test]
    fn destructive_argv_is_gated_before_spawn() {
        let args = serde_json::json!({"__argv": ["remove", "demo"]});
        let err = prepare_execution(vec!["plugin".to_string()], &args, None)
            .expect_err("must refuse without confirmation");
        assert!(err.starts_with(CONFIRMATION_REQUIRED_PREFIX), "{err}");
        assert!(prepare_execution(vec!["plugin".to_string()], &args, Some(true)).is_ok());
    }
}
