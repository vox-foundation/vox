//! Slash-command registry for the terminal input router.
//!
//! Commands registered here are surfaced in the command palette and dispatched
//! by `Session::submit` when the input intent is `InputIntent::SlashCommand`.

use std::collections::HashMap;

/// A registered slash-command handler (pure function, no side-effects on registry).
pub type CommandFn = fn(args: &str) -> CommandResult;

/// Outcome of executing a slash-command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult {
    /// Display a text response to the user.
    Output(String),
    /// No output; the command handled the side-effect itself.
    Silent,
    /// The command name was not recognised.
    NotFound,
}

/// A registered command entry.
#[derive(Clone)]
pub struct CommandEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub handler: CommandFn,
}

/// Registry mapping `/name` → [`CommandEntry`].
pub struct CommandRegistry {
    entries: HashMap<&'static str, CommandEntry>,
}

impl CommandRegistry {
    /// Creates a new, empty registry.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Registers a command. Panics if a duplicate name is inserted (config error).
    pub fn register(&mut self, entry: CommandEntry) {
        assert!(
            self.entries.insert(entry.name, entry).is_none(),
            "duplicate command registration"
        );
    }

    /// Dispatches `/name [args]` to the matching handler.
    pub fn dispatch(&self, input: &str) -> CommandResult {
        let input = input.trim_start_matches('/');
        let (name, args) = input.split_once(' ').unwrap_or((input, ""));
        let key: &str = name;
        match self.entries.get(key) {
            Some(entry) => (entry.handler)(args),
            None => CommandResult::NotFound,
        }
    }

    /// Returns all entries, sorted by name.
    pub fn entries(&self) -> Vec<&CommandEntry> {
        let mut v: Vec<&CommandEntry> = self.entries.values().collect();
        v.sort_by_key(|e| e.name);
        v
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        let mut r = Self::new();
        r.register(CommandEntry {
            name: "help",
            description: "List available slash-commands",
            handler: |_| {
                CommandResult::Output(
                    "Available commands: /help /model /budget /skills /memory /context /sh /nu /ai"
                        .into(),
                )
            },
        });
        r.register(CommandEntry {
            name: "nu",
            description: "Run a structured Nushell pipeline (JSON output) or show Nushell hints",
            handler: |args| {
                let trimmed = args.trim();
                if trimmed.is_empty() || trimmed == "help" || trimmed == "--help" {
                    CommandResult::Output(
                        "Usage: /nu <pipeline> | !nu <pipeline> — e.g. `/nu ls | where size > 1kb | to json`\n\
                         POSIX-to-Nushell Quick Reference:\n  \
                         • &&     → ; or and\n  \
                         • ||     → or\n  \
                         • 2>&1   → out+err>\n  \
                         • $VAR   → $env.VAR"
                            .into(),
                    )
                } else {
                    let (code, stdout, stderr) = crate::pty::run_nu_structured_capture(trimmed);
                    if code == 0 {
                        CommandResult::Output(stdout)
                    } else {
                        CommandResult::Output(format!("exit {code}: {stderr}{stdout}"))
                    }
                }
            },
        });
        r.register(CommandEntry {
            name: "model",
            description: "Switch, list, or intelligently route AI models",
            handler: |args| {
                let trimmed = args.trim();
                if trimmed.is_empty() {
                    CommandResult::Output(
                        "Usage: /model <name> | /model route <prompt>  — e.g. /model claude-sonnet-4-6"
                            .into(),
                    )
                } else if let Some(rest) = trimmed
                    .strip_prefix("route ")
                    .or_else(|| trimmed.strip_prefix("explain "))
                {
                    let routed = crate::agent::route_agent_prompt(rest);
                    CommandResult::Output(routed.summary())
                } else {
                    CommandResult::Output(format!("Model set to: {trimmed}"))
                }
            },
        });
        r.register(CommandEntry {
            name: "budget",
            description: "Show or set token budget",
            handler: |args| {
                if args.is_empty() {
                    CommandResult::Output("Token budget: unlimited".into())
                } else {
                    CommandResult::Output(format!("Budget set to: {args} tokens"))
                }
            },
        });
        r.register(CommandEntry {
            name: "skills",
            description: "List available skills",
            handler: |_| {
                CommandResult::Output("Skill registry: use `vox skill list` for details.".into())
            },
        });
        r.register(CommandEntry {
            name: "memory",
            description: "Open memory editor",
            handler: |_| {
                CommandResult::Output("Memory editor: use `vox memory search` for details.".into())
            },
        });
        r.register(CommandEntry {
            name: "context",
            description: "Edit context window",
            handler: |_| {
                CommandResult::Output("Context window: use `vox llm context` for details.".into())
            },
        });
        r.register(CommandEntry {
            name: "sh",
            description: "Run a raw shell command",
            handler: |args| {
                if args.is_empty() {
                    CommandResult::Output("Usage: /sh <command>".into())
                } else {
                    CommandResult::Output(format!("Shell: {args}"))
                }
            },
        });
        r.register(CommandEntry {
            name: "ai",
            description: "Send a prompt to the active agent",
            handler: |args| {
                if args.is_empty() {
                    CommandResult::Output("Usage: /ai <prompt>".into())
                } else {
                    CommandResult::Output(format!("Sending to agent: {args}"))
                }
            },
        });
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_command_returns_output() {
        let reg = CommandRegistry::default();
        assert!(matches!(reg.dispatch("/help"), CommandResult::Output(_)));
    }

    #[test]
    fn unknown_command_returns_not_found() {
        let reg = CommandRegistry::default();
        assert_eq!(reg.dispatch("/xyzzy"), CommandResult::NotFound);
    }

    #[test]
    fn model_with_arg() {
        let reg = CommandRegistry::default();
        let result = reg.dispatch("/model claude-haiku-4-5");
        assert_eq!(
            result,
            CommandResult::Output("Model set to: claude-haiku-4-5".into())
        );
    }

    #[test]
    fn entries_sorted() {
        let reg = CommandRegistry::default();
        let names: Vec<&str> = reg.entries().iter().map(|e| e.name).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
    }

    #[test]
    fn custom_command_registration() {
        let mut reg = CommandRegistry::new();
        reg.register(CommandEntry {
            name: "ping",
            description: "Ping",
            handler: |_| CommandResult::Output("pong".into()),
        });
        assert_eq!(reg.dispatch("/ping"), CommandResult::Output("pong".into()));
    }
}
