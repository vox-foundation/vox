//! Remote-mode invoke gate: in remote mode, commands from `db`/`host`-class modules are rejected with
//! `local_only` before they run; `daemon`/`shell` modules and non-`commands` roots pass through.

use tauri::Manager;
use tauri::ipc::Invoke;

use super::backend::BackendState;
use super::error::AxisError;
use super::manifest::{self, ModuleClass};

pub type CommandTable = [(&'static str, &'static str, &'static str)];

pub fn local_only_denial(
    table: &CommandTable,
    command: &str,
    remote: bool,
    class_of: impl Fn(&str) -> Option<ModuleClass>,
) -> Option<AxisError> {
    if !remote {
        return None;
    }
    let (root, module, _) = table.iter().find(|(_, _, cmd)| *cmd == command)?;
    if *root != "commands" {
        return None;
    }
    let class = match class_of(module) {
        Some(ModuleClass::Daemon | ModuleClass::Shell) => return None,
        Some(c @ (ModuleClass::Db | ModuleClass::Host)) => c.as_str(),
        None => "unclassified",
    };
    Some(AxisError::LocalOnly {
        command: command.to_string(),
        module: module.to_string(),
        class,
    })
}

/// Pins `tauri::generate_handler!`'s closure to the Wry runtime so it can be wrapped.
pub fn typed_handler<F>(f: F) -> F
where
    F: Fn(Invoke<tauri::Wry>) -> bool + Send + Sync + 'static,
{
    f
}

// toestub-ignore(skeleton/untested-pub-api) — Tauri IPC adapter; the decision is `local_only_denial` (tested above) and the e2e spec drives it end to end
pub fn gate_invoke(
    table: &CommandTable,
    invoke: Invoke<tauri::Wry>,
    inner: &dyn Fn(Invoke<tauri::Wry>) -> bool,
) -> bool {
    let remote = invoke
        .message
        .webview()
        .state::<std::sync::Arc<BackendState>>()
        .is_remote();
    let command = invoke.message.command().to_string();
    if let Some(err) = local_only_denial(table, &command, remote, manifest::module_class) {
        invoke.resolver.reject(String::from(err));
        return true;
    }
    inner(invoke)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &CommandTable = &[
        ("commands", "pty", "pty_spawn"),
        ("commands", "models", "list_model_cards"),
        ("commands", "mcp", "invoke_mcp_tool"),
        ("commands", "build_info", "get_build_info"),
        ("commands", "brand_new", "brand_new_cmd"),
        ("drive", "bridge", "get_drive_mode"),
    ];

    fn class_of(module: &str) -> Option<ModuleClass> {
        match module {
            "pty" => Some(ModuleClass::Host),
            "models" => Some(ModuleClass::Db),
            "mcp" => Some(ModuleClass::Daemon),
            "build_info" => Some(ModuleClass::Shell),
            _ => None,
        }
    }

    #[test]
    fn local_mode_never_denies() {
        for (_, _, cmd) in TABLE {
            assert_eq!(
                local_only_denial(TABLE, cmd, false, class_of),
                None,
                "{cmd}"
            );
        }
    }

    #[test]
    fn remote_mode_denies_db_and_host_commands() {
        let err = local_only_denial(TABLE, "pty_spawn", true, class_of).expect("denied");
        assert_eq!(
            err,
            AxisError::LocalOnly {
                command: "pty_spawn".into(),
                module: "pty".into(),
                class: "host"
            }
        );
        let err = local_only_denial(TABLE, "list_model_cards", true, class_of).expect("denied");
        assert_eq!(err.code(), "local_only");
    }

    #[test]
    fn remote_mode_allows_daemon_shell_and_drive() {
        assert_eq!(
            local_only_denial(TABLE, "invoke_mcp_tool", true, class_of),
            None
        );
        assert_eq!(
            local_only_denial(TABLE, "get_build_info", true, class_of),
            None
        );
        assert_eq!(
            local_only_denial(TABLE, "get_drive_mode", true, class_of),
            None
        );
        assert_eq!(
            local_only_denial(TABLE, "not_registered", true, class_of),
            None
        );
    }

    #[test]
    fn unclassified_module_fails_closed_remotely() {
        let err = local_only_denial(TABLE, "brand_new_cmd", true, class_of).expect("denied");
        assert_eq!(
            err,
            AxisError::LocalOnly {
                command: "brand_new_cmd".into(),
                module: "brand_new".into(),
                class: "unclassified"
            }
        );
    }
}
