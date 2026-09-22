//! `vox repl` — interactive read-eval-print loop for Vox expressions.
//!
//! Session state (see [`ReplSession`]) persists across lines: a `fn`/`type`/etc.
//! declared on one line stays callable on later lines, and the interpreter
//! keeps its scope across calls. Bare top-level statements (`let x = 5`,
//! `sq(4)`) are *not* persisted as declarations — see [`ReplSession::eval_line`].

use anyhow::Result;
use std::io::{self, Write};
use vox_compiler::eval::Interpreter;
use vox_compiler::eval::value::VoxValue;
use vox_compiler::pipeline::{PipelineOptions, run_frontend_str_with_options};
use vox_compiler::typeck::diagnostics::TypeckSeverity;

/// Outcome of evaluating one REPL line.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplOutcome {
    /// A non-null value was produced; display-formatted and ready to print.
    Value(String),
    /// The line ran with nothing to show (declaration-only line, or a `Null` result).
    Silent,
    /// One error message per line, already formatted for `eprintln!`.
    Errors(Vec<String>),
}

/// Persistent REPL session: an interpreter whose scope accumulates bindings
/// across lines, plus a buffer of source text for every line that was itself
/// a top-level declaration (`fn`, `type`, `actor`, …).
///
/// Each line is typechecked and lowered against `decl_buffer + line`, so a
/// function declared on an earlier line is in scope for a later line's call —
/// fixing the "declare then call in the next line" case that a bare
/// isolated-per-line pipeline can't see.
///
/// Deferred: a top-level `let` binding does not persist to later lines. Script
/// mode lowers bare statements into a synthetic `fn main() { .. }`, and `let`
/// inside that body is a local binding scoped to that one call to `main` —
/// not a global. Persisting it would need the interpreter to distinguish
/// module-level `let`s from ordinary local ones, which is a larger change
/// than this fix warrants; only declarations (`fn`/`type`/…) persist today.
pub struct ReplSession {
    interp: Interpreter,
    decl_buffer: String,
}

impl ReplSession {
    pub fn new() -> Self {
        let mut interp = Interpreter::new(100_000);
        // Explicit, not the constructor's default: an interactive REPL is a
        // trusted, local session — grant every gated namespace.
        interp.caps = vox_compiler::eval::caps::CapabilitySet::developer_default();
        Self {
            interp,
            decl_buffer: String::new(),
        }
    }

    /// Evaluate one line against the accumulated session state.
    pub fn eval_line(&mut self, line: &str) -> ReplOutcome {
        let options = PipelineOptions {
            script_mode: true,
            ..PipelineOptions::default()
        };

        // Parsing alone (ignoring typecheck outcome) tells us whether this
        // line, on its own, is a top-level declaration rather than a bare
        // statement/expression — that's what decides whether it joins
        // `decl_buffer` for future lines.
        let line_is_decl = run_frontend_str_with_options(line, "repl.vox", &options)
            .map(|r| !r.module.declarations.is_empty())
            .unwrap_or(false);

        let combined = if self.decl_buffer.is_empty() {
            line.to_string()
        } else {
            format!("{}\n{line}", self.decl_buffer)
        };

        let res = match run_frontend_str_with_options(&combined, "repl.vox", &options) {
            Ok(res) => res,
            Err(e) => return ReplOutcome::Errors(vec![format!("compiler error: {e}")]),
        };

        if res.has_errors() {
            let errors = res
                .diagnostics
                .into_iter()
                .filter(|d| d.severity == TypeckSeverity::Error)
                .map(|d| format!("error: {}", d.message))
                .collect();
            return ReplOutcome::Errors(errors);
        }

        if let Err(e) = self.interp.run_module(&res.hir) {
            return ReplOutcome::Errors(vec![format!("lowering error: {e:?}")]);
        }

        let outcome = if res.hir.functions.iter().any(|f| f.name == "main") {
            match self.interp.call("main", vec![]) {
                Ok(val) if val != VoxValue::Null => {
                    ReplOutcome::Value(vox_compiler::eval::builtins::vox_value_display(&val))
                }
                Ok(_) => ReplOutcome::Silent,
                Err(e) => ReplOutcome::Errors(vec![format!("eval error: {e:?}")]),
            }
        } else {
            ReplOutcome::Silent
        };

        // Only persist the line as a declaration once it has successfully
        // typechecked and run — a failed line shouldn't poison future lines.
        if line_is_decl && !matches!(outcome, ReplOutcome::Errors(_)) {
            if self.decl_buffer.is_empty() {
                self.decl_buffer = line.to_string();
            } else {
                self.decl_buffer.push('\n');
                self.decl_buffer.push_str(line);
            }
        }

        outcome
    }
}

impl Default for ReplSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Start an interactive REPL: parse and evaluate one line at a time, keeping
/// declarations and interpreter state across lines for the whole session.
pub async fn run() -> Result<()> {
    println!("Vox REPL");
    println!("Enter expressions or declarations. Type `exit` or press Ctrl+D to quit.");

    let mut session = ReplSession::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut input = String::new();

    loop {
        print!("vox> ");
        stdout.flush()?;
        input.clear();
        if stdin.read_line(&mut input)? == 0 {
            println!();
            break;
        }
        let line = input.trim();
        if line.is_empty() {
            continue;
        }
        if matches!(line, "exit" | "quit") {
            break;
        }

        match session.eval_line(line) {
            ReplOutcome::Value(s) => println!("{s}"),
            ReplOutcome::Silent => {}
            ReplOutcome::Errors(errors) => {
                for e in errors {
                    eprintln!("{e}");
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bug this fixes: a function declared on one line must be callable
    /// on the next, and the result must print via the display formatter
    /// (`16`), not `Debug` (`Int(16)`).
    #[test]
    fn function_declared_then_called_across_lines() {
        let mut session = ReplSession::new();
        let decl = session.eval_line("fn sq(x: int) to int { return x*x }");
        assert_eq!(
            decl,
            ReplOutcome::Silent,
            "declaration alone prints nothing"
        );

        let call = session.eval_line("sq(4)");
        assert_eq!(call, ReplOutcome::Value("16".to_string()));
    }

    #[test]
    fn calling_before_declaration_still_errors() {
        let mut session = ReplSession::new();
        let call = session.eval_line("sq(4)");
        assert!(matches!(call, ReplOutcome::Errors(_)));
    }

    #[test]
    fn multiple_declarations_all_persist() {
        let mut session = ReplSession::new();
        assert_eq!(
            session.eval_line("fn add(a: int, b: int) to int { return a + b }"),
            ReplOutcome::Silent
        );
        assert_eq!(
            session.eval_line("fn double(x: int) to int { return add(x, x) }"),
            ReplOutcome::Silent
        );
        assert_eq!(
            session.eval_line("double(5)"),
            ReplOutcome::Value("10".to_string())
        );
    }

    #[test]
    fn same_line_expression_still_works() {
        let mut session = ReplSession::new();
        assert_eq!(
            session.eval_line("1 + 2"),
            ReplOutcome::Value("3".to_string())
        );
    }
}
