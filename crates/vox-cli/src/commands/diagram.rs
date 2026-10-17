//! `vox diagram` — the VoxDiagram CLI over the shared validation envelope.
//!
//! The core is reached only through `vox_doc_pipeline::diagram` (the Phase 87 re-export), so
//! vox-cli needs no crate edge of its own. `--json` (the global flag) prints exactly the envelope
//! the `vox_validate_diagram` MCP tool returns.
//!
//! Exit codes: 0 valid, 1 diagnostics (any Error, non-canonical `fmt --check`, unsupported import
//! lines, a failed render), 2 usage / I/O failure.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use vox_doc_pipeline::diagram::{
    Emit, ValidateOpts, Validation, limits::MAX_SOURCE_BYTES, validate,
};

/// Subcommands of `vox diagram`.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum DiagramCmd {
    /// Check a diagram file or stdin and print each diagnostic as file:line:col with its code (exit 1 on any error)
    Check(InputArgs),
    /// Print the canonical form of a diagram, or verify (--check) or rewrite (--write) the file
    Fmt(FmtArgs),
    /// Render a diagram as deterministic SVG to stdout or -o FILE
    Render(OutputArgs),
    /// Export a diagram as Mermaid to stdout or -o FILE
    Export(OutputArgs),
    /// Convert a Mermaid block to canonical VoxDiagram and list unsupported lines (exit 1)
    Import(OutputArgs),
}

/// A diagram input.
#[derive(clap::Args, Debug, Clone)]
pub struct InputArgs {
    /// Diagram source file, or `-` for stdin.
    pub input: String,
}

/// Arguments of `vox diagram fmt`.
#[derive(clap::Args, Debug, Clone)]
pub struct FmtArgs {
    #[command(flatten)]
    pub input: InputArgs,
    /// Exit 1 when the file is not already canonical; write nothing.
    #[arg(long, conflicts_with = "write")]
    pub check: bool,
    /// Rewrite the file in place with its canonical form.
    #[arg(long)]
    pub write: bool,
}

/// Arguments of the commands that produce a document.
#[derive(clap::Args, Debug, Clone)]
pub struct OutputArgs {
    #[command(flatten)]
    pub input: InputArgs,
    /// Write the result to FILE instead of stdout.
    #[arg(short = 'o', long, value_name = "FILE")]
    pub output: Option<PathBuf>,
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
    let (input, opts, dest, fmt) = match cmd {
        DiagramCmd::Check(a) => (&a.input, ValidateOpts::default(), None, None),
        DiagramCmd::Fmt(f) => (&f.input.input, ValidateOpts::default(), None, Some(f)),
        DiagramCmd::Render(o) => (&o.input.input, emit(Emit::Svg), o.output.as_deref(), None),
        DiagramCmd::Export(o) => (
            &o.input.input,
            emit(Emit::Mermaid),
            o.output.as_deref(),
            None,
        ),
        DiagramCmd::Import(o) => (
            &o.input.input,
            ValidateOpts {
                from_mermaid: true,
                emit: None,
            },
            o.output.as_deref(),
            None,
        ),
    };
    let (check, write) = fmt.map_or((false, false), |f| (f.check, f.write));
    if json && (dest.is_some() || check || write) {
        bail!("--json prints the envelope only; it cannot be combined with -o, --write or --check");
    }
    if write && input == "-" {
        bail!("fmt --write needs a file, not stdin");
    }
    let label = if input == "-" { "<stdin>" } else { input };
    let src = read_source(input, stdin).with_context(|| label.to_owned())?;
    let v = validate(&src, opts);
    let failed = i32::from(!v.valid || v.emit_error.is_some());
    if json {
        writeln!(out, "{}", serde_json::to_string_pretty(&v)?)?;
        return Ok(failed);
    }
    print_findings(label, &v, err)?;
    if failed == 1 {
        return Ok(1);
    }
    let payload = match cmd {
        DiagramCmd::Check(_) => return Ok(0),
        DiagramCmd::Fmt(_) | DiagramCmd::Import(_) => v.normalized_source.as_deref(),
        DiagramCmd::Render(_) | DiagramCmd::Export(_) => v.emitted.as_deref(),
    }
    .context("internal error: a valid diagram produced no output")?;
    if check {
        if src == payload {
            return Ok(0);
        }
        writeln!(
            err,
            "{label}: not canonical; run `vox diagram fmt --write {label}`"
        )?;
        return Ok(1);
    }
    if write {
        if src != payload {
            write_atomic(Path::new(input), payload)?;
        }
        return Ok(0);
    }
    match dest {
        Some(path) => std::fs::write(path, payload).with_context(|| path.display().to_string())?,
        None => out.write_all(payload.as_bytes())?,
    }
    Ok(0)
}

fn emit(e: Emit) -> ValidateOpts {
    ValidateOpts {
        from_mermaid: false,
        emit: Some(e),
    }
}

/// Diagnostics, importer refusals and drops, then a render failure, all as `label:...` lines.
fn print_findings(label: &str, v: &Validation, err: &mut dyn Write) -> std::io::Result<()> {
    for d in &v.diagnostics {
        writeln!(
            err,
            "{label}:{}:{}: {}: {}",
            d.span.line, d.span.col, d.code, d.message
        )?;
    }
    for u in v.unsupported.iter().flatten() {
        writeln!(err, "{label}:{}: unsupported: {}", u.line, u.construct)?;
    }
    for u in &v.dropped {
        writeln!(err, "{label}:{}: dropped: {}", u.line, u.construct)?;
    }
    if let Some(e) = &v.emit_error {
        writeln!(err, "{label}: {e}")?;
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

/// Sibling temp file + rename, so a failed write never truncates the original.
/// ponytail: a symlinked target is replaced by a regular file; resolve it first if that matters.
fn write_atomic(path: &Path, data: &str) -> anyhow::Result<()> {
    let name = path
        .file_name()
        .context("not a file path")?
        .to_string_lossy();
    let tmp = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    let result = (|| {
        let perms = std::fs::metadata(path)?.permissions();
        std::fs::write(&tmp, data)?;
        std::fs::set_permissions(&tmp, perms)?;
        std::fs::rename(&tmp, path)
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(e).with_context(|| path.display().to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BROKEN: &str = "graph {\n    node a: \"A\"\n    a -> b\n    node c: C\n}\n";

    fn run_cmd(cmd: &DiagramCmd, json: bool, src: &str) -> (i32, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = execute(cmd, json, &mut src.as_bytes(), &mut out, &mut err);
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    fn check(src: &str) -> (i32, String, String) {
        run_cmd(
            &DiagramCmd::Check(InputArgs { input: "-".into() }),
            false,
            src,
        )
    }

    fn render(output: Option<&str>) -> DiagramCmd {
        DiagramCmd::Render(OutputArgs {
            input: InputArgs { input: "-".into() },
            output: output.map(PathBuf::from),
        })
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

    #[test]
    fn json_with_output_is_refused_before_reading() {
        let (code, out, err) = run_cmd(&render(Some("x.svg")), true, "not read");
        assert_eq!((code, out.as_str()), (2, ""));
        assert!(err.contains("--json"), "{err}");
    }

    #[test]
    fn fmt_write_refuses_stdin() {
        let cmd = DiagramCmd::Fmt(FmtArgs {
            input: InputArgs { input: "-".into() },
            check: false,
            write: true,
        });
        let (code, _, err) = run_cmd(&cmd, false, "graph {\n}\n");
        assert_eq!(code, 2);
        assert!(err.contains("--write"), "{err}");
    }
}
