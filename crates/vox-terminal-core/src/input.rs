#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputIntent {
    VoxNative(String),
    Shell(String),
    StructuredShell(String),
    Agent(String),
    Command { name: String, args: String },
}

pub fn classify(raw: &str) -> InputIntent {
    let t = raw.trim();
    if let Some(rest) = t.strip_prefix("!nu ") {
        return InputIntent::StructuredShell(rest.trim().to_string());
    }
    if t == "!nu" {
        return InputIntent::StructuredShell(String::new());
    }
    if let Some(rest) = t.strip_prefix('!') {
        return InputIntent::Shell(rest.trim().to_string());
    }
    if let Some(rest) = t.strip_prefix('?') {
        let prompt = rest.trim();
        if !prompt.is_empty() {
            return InputIntent::Agent(prompt.to_string());
        }
    }
    if let Some(rest) = t.strip_prefix('/') {
        let rest = rest.trim_start();
        if rest.is_empty() {
            return InputIntent::VoxNative(t.to_string());
        }
        let (name, args) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        return match name {
            "nu" => {
                let trimmed_args = args.trim();
                if trimmed_args.is_empty() || trimmed_args == "help" || trimmed_args == "--help" {
                    InputIntent::Command {
                        name: name.to_string(),
                        args: trimmed_args.to_string(),
                    }
                } else {
                    InputIntent::StructuredShell(trimmed_args.to_string())
                }
            }
            "sh" | "shell" => InputIntent::Shell(args.trim().to_string()),
            "ai" | "agent" => InputIntent::Agent(args.trim().to_string()),
            _ => InputIntent::Command {
                name: name.to_string(),
                args: args.trim().to_string(),
            },
        };
    }
    InputIntent::VoxNative(t.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_nu_structured_shell_and_help() {
        assert_eq!(
            classify("!nu ls | to json"),
            InputIntent::StructuredShell("ls | to json".to_string())
        );
        assert_eq!(
            classify("/nu ls | where size > 0"),
            InputIntent::StructuredShell("ls | where size > 0".to_string())
        );
        assert_eq!(
            classify("/nu --help"),
            InputIntent::Command {
                name: "nu".to_string(),
                args: "--help".to_string()
            }
        );
    }
}
