//! `vox diagram` — the VoxDiagram CLI over the shared validation envelope.
//!
//! The core is reached only through `vox_doc_pipeline::diagram` (the Phase 87 re-export), so
//! vox-cli needs no crate edge of its own. `--json` (the global flag) prints exactly the envelope
//! the `vox_validate_diagram` MCP tool returns.
//!
//! Exit codes: 0 valid, 1 diagnostics (any Error), 2 usage / I/O failure.

use std::io::{Read, Write};

use anyhow::Context;
use vox_doc_pipeline::diagram::{ValidateOpts, Validation, limits::MAX_SOURCE_BYTES, validate};

/// Subcommands of `vox diagram`.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum DiagramCmd {
    /// Check a diagram file or stdin and print each diagnostic as file:line:col with its code (exit 1 on any error)
    Check(InputArgs),
}

/// A diagram input.
#[derive(clap::Args, Debug, Clone)]
pub struct InputArgs {
    /// Diagram source file, or `-` for stdin.
    pub input: String,
}

/// Run `vox diagram`; exits the process with the command's exit code when it is non-zero.
pub fn run(cmd: DiagramCmd) -> anyhow::Result<()> {
    let (stdout, stderr) = (std::io::stdout(), std::io::stderr());
    let (mut out, mut err) = (stdout.lock(), stderr.lock());
    let json = crate::pipeline::global_json_enabled();
    let code = execute(&cmd, json, &mut std::io::stdin().lock(), &mut out, &mut err);
    let _ = out.flush();
    let _ = err.flush();
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

/// All behavior, over injected streams so it is testable without the binary.
fn execute(
    cmd: &DiagramCmd,
    json: bool,
    stdin: &mut dyn Read,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    match go(cmd, json, stdin, out, err) {
        Ok(code) => code,
        Err(e) => {
            let _ = writeln!(err, "{e:#}");
            2
        }
    }
}

fn go(
    cmd: &DiagramCmd,
    json: bool,
    stdin: &mut dyn Read,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> anyhow::Result<i32> {
    let DiagramCmd::Check(a) = cmd;
    let label = label(&a.input);
    let src = read_source(&a.input, stdin).with_context(|| label.to_owned())?;
    let v = validate(&src, ValidateOpts::default());
    if json {
        writeln!(out, "{}", serde_json::to_string_pretty(&v)?)?;
    } else {
        print_diagnostics(label, &v, err)?;
    }
    Ok(i32::from(!v.valid))
}

fn label(input: &str) -> &str {
    if input == "-" { "<stdin>" } else { input }
}

fn print_diagnostics(label: &str, v: &Validation, err: &mut dyn Write) -> std::io::Result<()> {
    for d in &v.diagnostics {
        writeln!(
            err,
            "{label}:{}:{}: {}: {}",
            d.span.line, d.span.col, d.code, d.message
        )?;
    }
    Ok(())
}

/// Read at most `MAX_SOURCE_BYTES + 1` bytes: enough for the core's size diagnostic, bounded memory.
fn read_source(input: &str, stdin: &mut dyn Read) -> anyhow::Result<String> {
    let limit = MAX_SOURCE_BYTES as u64 + 1;
    let mut buf = Vec::new();
    if input == "-" {
        stdin.take(limit).read_to_end(&mut buf)?;
    } else {
        std::fs::File::open(input)?
            .take(limit)
            .read_to_end(&mut buf)?;
    }
    if buf.len() > MAX_SOURCE_BYTES {
        // Rejected by the size limit whatever the bytes are; a cut multibyte tail must not mask that.
        return Ok(String::from_utf8_lossy(&buf).into_owned());
    }
    Ok(String::from_utf8(buf).context("input is not valid UTF-8")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BROKEN: &str = "graph {\n    node a: \"A\"\n    a -> b\n    node c: C\n}\n";

    fn check(src: &str) -> (i32, String, String) {
        let cmd = DiagramCmd::Check(InputArgs { input: "-".into() });
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = execute(&cmd, false, &mut src.as_bytes(), &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn broken_stdin_prints_every_error_and_exits_1() {
        let (code, out, err) = check(BROKEN);
        let want: Vec<String> = validate(BROKEN, ValidateOpts::default())
            .diagnostics
            .iter()
            .map(|d| {
                format!(
                    "<stdin>:{}:{}: {}: {}",
                    d.span.line, d.span.col, d.code, d.message
                )
            })
            .collect();
        assert!(want.len() >= 2);
        assert_eq!((code, out.as_str()), (1, ""));
        assert_eq!(err.lines().collect::<Vec<_>>(), want);
    }

    #[test]
    fn valid_source_exits_0_silently() {
        let (code, out, err) = check("graph {\n    node a: \"A\"\n}\n");
        assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    }

    #[test]
    fn non_utf8_input_exits_2() {
        let cmd = DiagramCmd::Check(InputArgs { input: "-".into() });
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = execute(&cmd, false, &mut &[0xff_u8, 0xfe][..], &mut out, &mut err);
        assert_eq!(code, 2);
        assert!(String::from_utf8(err).unwrap().starts_with("<stdin>: "));
    }

    #[test]
    fn oversized_input_gets_the_size_diagnostic_not_a_read_error() {
        let big = "é".repeat(MAX_SOURCE_BYTES);
        let (code, _, err) = check(&big);
        assert_eq!(code, 1);
        assert!(err.contains("limit-source"), "{err}");
    }
}
