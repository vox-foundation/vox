//! Opens a search hit's locator on the GUI host (editor spawn / browser open). Host-class:
//! the file or URL is opened on the machine running Axis even when search ran remotely.

use super::process_util::quiet_command;

pub use vox_orchestrator_mcp::gui_ops::dto::knowledge::OpenLocatorDto;

/// Outcome of an open_locator call.
#[derive(Debug, serde::Serialize)]
pub struct OpenOutcomeDto {
    pub action: String, // "spawned" | "opened" | "focus_chat" | "focus_command"
}

/// Returns the (program, args) needed to open a URL on the given OS.
pub(crate) fn url_open_command(os: &str, url: &str) -> (String, Vec<String>) {
    match os {
        "windows" => (
            "cmd".to_string(),
            vec![
                "/C".to_string(),
                "start".to_string(),
                String::new(),
                url.to_string(),
            ],
        ),
        "macos" => ("open".to_string(), vec![url.to_string()]),
        _ => ("xdg-open".to_string(), vec![url.to_string()]),
    }
}

/// Focus actions that need no process spawn; `None` for kinds that open something.
fn focus_action(kind: &str) -> Option<&'static str> {
    match kind {
        "chat" => Some("focus_chat"),
        "command" => Some("focus_command"),
        _ => None,
    }
}

/// Open a locator in the appropriate application.
#[tauri::command]
pub async fn open_locator(locator: OpenLocatorDto) -> Result<OpenOutcomeDto, String> {
    let OpenLocatorDto { kind, value } = locator;
    if let Some(action) = focus_action(&kind) {
        return Ok(OpenOutcomeDto {
            action: action.to_string(),
        });
    }
    match kind.as_str() {
        "file" => {
            let editor = std::env::var("VOX_EDITOR").unwrap_or_else(|_| "code".into());
            quiet_command(&editor)
                .arg(&value)
                .spawn()
                .map_err(|e| format!("failed to spawn editor: {e}"))?;
            Ok(OpenOutcomeDto {
                action: "spawned".to_string(),
            })
        }
        "web" => {
            let (prog, args) = url_open_command(std::env::consts::OS, &value);
            quiet_command(&prog)
                .args(&args)
                .spawn()
                .map_err(|e| format!("failed to open URL: {e}"))?;
            Ok(OpenOutcomeDto {
                action: "opened".to_string(),
            })
        }
        _ => Err("not openable".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_open_windows() {
        let (prog, args) = url_open_command("windows", "https://example.com");
        assert_eq!(prog, "cmd");
        assert!(args.contains(&"/C".to_string()));
        assert!(args.contains(&"start".to_string()));
        assert!(args.contains(&"https://example.com".to_string()));
    }

    #[test]
    fn url_open_macos() {
        let (prog, args) = url_open_command("macos", "https://example.com");
        assert_eq!(prog, "open");
        assert_eq!(args, vec!["https://example.com"]);
    }

    #[test]
    fn url_open_linux() {
        let (prog, args) = url_open_command("linux", "https://example.com");
        assert_eq!(prog, "xdg-open");
        assert_eq!(args, vec!["https://example.com"]);
    }

    #[tokio::test]
    async fn focus_kinds_answer_without_spawning_and_unknown_kinds_refuse() {
        let chat = open_locator(OpenLocatorDto {
            kind: "chat".into(),
            value: "s1".into(),
        })
        .await
        .unwrap();
        assert_eq!(chat.action, "focus_chat");
        let cmd = open_locator(OpenLocatorDto {
            kind: "command".into(),
            value: "x".into(),
        })
        .await
        .unwrap();
        assert_eq!(cmd.action, "focus_command");
        let none = open_locator(OpenLocatorDto {
            kind: "none".into(),
            value: String::new(),
        })
        .await;
        assert_eq!(none.unwrap_err(), "not openable");
    }
}
