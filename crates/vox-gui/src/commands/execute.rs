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
#[tauri::command]
pub async fn execute_command(
    app: tauri::AppHandle,
    path: Vec<String>,
    args: Value,
    confirmed: Option<bool>,
) -> Result<ExecuteOutput, String> {
    let shell_args = vox_cli::command_catalog::form_argv(path, &args);
    vox_cli::command_catalog::gate_execution(&shell_args, confirmed)?;

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
    use vox_cli::command_catalog::{CONFIRMATION_REQUIRED_PREFIX, form_argv, gate_execution};

    /// The wiring `execute_command` performs before the sidecar spawn: a destructive subcommand
    /// smuggled through `__argv` is refused unless `confirmed: true`.
    #[test]
    fn destructive_argv_is_gated_before_spawn() {
        let args = serde_json::json!({"__argv": ["remove", "demo"]});
        let argv = form_argv(vec!["plugin".to_string()], &args);
        let err = gate_execution(&argv, None).expect_err("must refuse without confirmation");
        assert!(err.starts_with(CONFIRMATION_REQUIRED_PREFIX), "{err}");
        assert!(gate_execution(&argv, Some(true)).is_ok());
    }
}
