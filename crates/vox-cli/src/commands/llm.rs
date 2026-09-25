use clap::Subcommand;
use owo_colors::OwoColorize;
use std::io::IsTerminal;

// Vox snippets printed by `vox llm prompt`. GOLDEN_ROUTE and GOLDEN_MUTATION
// are copied VERBATIM from examples/golden/crud_api.vox:19-21 and :29-32,
// which the compiler verifies. The SYNTAX_* and SCHEMA_* constants are
// hand-written shapes, guarded instead by the printed_snippets test below.
//
// Do NOT hand-write Vox here. This subcommand exists to teach an LLM the
// language; a wrong snippet is a training defect shipped as a feature. It
// previously printed `@query`/`@mutation` (hard parse errors since
// 2026-06-30, cd7cc96874) labelled "Golden Example", alongside `pub fn`,
// `u64`, `->`, `String` and `Result<(), Error>` -- none of which are Vox.
const SYNTAX_ROUTE: &str = "query user_count() to int {\n    // ...\n}";
const GOLDEN_ROUTE: &str = "query user_count() to int {\n    return len(db.User.all())\n}";
const SCHEMA_ROUTE: &str = "{ \"type\": \"route\", \"keyword\": \"query\" }";
const SYNTAX_MUTATION: &str = "mutation seed_user(name: str) to str {\n    // ...\n}";
const GOLDEN_MUTATION: &str = "mutation seed_user(name: str) to str {\n    db.User.insert({ name: name, active: true })\n    return \"created\"\n}";
const SCHEMA_MUTATION: &str = "{ \"type\": \"mutation\", \"keyword\": \"mutation\" }";

#[derive(Subcommand)]
pub enum LlmCmd {
    /// Print relevant vox-language-surface.v1.json context + golden examples to stdout for use with any LLM.
    Prompt {
        /// The task you want help with (e.g., 'web-route', 'server-fn').
        task: String,
    },
}

/// Whether stdout should carry ANSI colour: respects `NO_COLOR`
/// (https://no-color.org/, any non-empty value disables colour) and only
/// colours when stdout is a tty (never when piped/redirected).
fn stdout_color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

pub async fn run(cmd: LlmCmd) -> anyhow::Result<()> {
    match cmd {
        LlmCmd::Prompt { task } => {
            let color = stdout_color_enabled();
            let heading = format!("Generating LLM prompt context for task: {}", task);
            if color {
                println!("{}", heading.bright_cyan());
            } else {
                println!("{heading}");
            }

            let section = |title: &str| {
                if color {
                    println!("{}", title.bright_yellow());
                } else {
                    println!("{title}");
                }
            };

            let mut found = false;
            let task_lower = task.to_lowercase();

            if task_lower == "web-route" || task_lower == "route" || task_lower == "@query" {
                section("--- Route Declaration Syntax ---");
                println!("{SYNTAX_ROUTE}");
                println!();
                section("--- Golden Example ---");
                println!("{GOLDEN_ROUTE}");
                println!();
                section("--- MCP Schema Excerpt ---");
                println!("{SCHEMA_ROUTE}");
                found = true;
            } else if task_lower == "server-fn"
                || task_lower == "mutation"
                || task_lower == "@mutation"
            {
                section("--- Mutation Declaration Syntax ---");
                println!("{SYNTAX_MUTATION}");
                println!();
                section("--- Golden Example ---");
                println!("{GOLDEN_MUTATION}");
                println!();
                section("--- MCP Schema Excerpt ---");
                println!("{SCHEMA_MUTATION}");
                found = true;
            }

            if !found {
                println!(
                    "No specific golden found for task: '{}'. Please refer to `docs/agents/vox-language-surface.v1.json`.",
                    task
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(unsafe_code)] // test-only std::env::{set_var, remove_var} (unsafe since edition 2024)
mod tests {
    use super::*;

    /// `NO_COLOR` (https://no-color.org/) must disable colour regardless of
    /// tty state — the actual bug: `vox llm prompt` emitted ANSI escapes even
    /// with `NO_COLOR=1` set and stdout piped.
    #[test]
    fn stdout_color_enabled_respects_no_color() {
        // SAFETY: test-only env mutation; no other test in this process reads NO_COLOR.
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }
        assert!(
            !stdout_color_enabled(),
            "NO_COLOR=1 must disable colour output"
        );
        unsafe {
            std::env::remove_var("NO_COLOR");
        }
    }

    /// Under `cargo test`, stdout is captured (not a tty), so colour must be
    /// disabled even with `NO_COLOR` unset — colour is opt-in to a real terminal.
    #[test]
    fn stdout_color_enabled_disabled_when_not_a_tty() {
        // SAFETY: test-only env mutation.
        unsafe {
            std::env::remove_var("NO_COLOR");
        }
        assert!(
            !stdout_color_enabled(),
            "non-tty stdout (captured by the test harness) must not emit colour"
        );
    }

    /// This subcommand's whole purpose is telling an LLM how to write Vox, so
    /// a snippet containing non-Vox syntax is a defect shipped as a feature.
    /// Guards every printed snippet, not just the ones labelled "Golden" --
    /// the "Syntax" lines were wrong too.
    #[test]
    fn printed_snippets_contain_no_non_vox_syntax() {
        for (label, text) in [
            ("route syntax", SYNTAX_ROUTE),
            ("route golden", GOLDEN_ROUTE),
            ("route schema", SCHEMA_ROUTE),
            ("mutation syntax", SYNTAX_MUTATION),
            ("mutation golden", GOLDEN_MUTATION),
            ("mutation schema", SCHEMA_MUTATION),
        ] {
            // Retired at-prefixed data-layer decorators: hard parse errors
            // since 2026-06-30 (cd7cc96874).
            for retired in ["@query", "@mutation", "@server", "@table", "@tool"] {
                assert!(
                    !text.contains(retired),
                    "{label} contains retired decorator {retired}: {text}"
                );
            }
            // Rust-isms that are not Vox: `to` is the return arrow, `int`/`str`
            // are the types, and there is no `fn` keyword in the bare form.
            for non_vox in ["pub fn", "->", "u64", "String", "Result<"] {
                assert!(
                    !text.contains(non_vox),
                    "{label} contains {non_vox:?}, which is not Vox syntax: {text}"
                );
            }
        }
    }
}
