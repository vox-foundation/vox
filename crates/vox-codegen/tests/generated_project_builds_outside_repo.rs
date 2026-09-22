//! Regression test for the bug reported 2026-09-21: `vox build`'s generated
//! `Cargo.toml` hardcoded relative path deps on vox's own runtime crates
//! (`vox-db = { path = "../../crates/vox-db" }`), which only resolve when
//! the generated project happens to sit at a fixed depth *inside* a vox
//! checkout. Scaffolding a project anywhere else (the normal case — `vox
//! init`/`vox build` are meant to work on user code outside this repo) made
//! `cargo check` fail immediately with "No such file or directory".
//!
//! Unlike `ai_fixture_bundle_compiles.rs` (which writes its scratch crate
//! under `<workspace_root>/_bundle_.../gen_pkg` — still *inside* the repo,
//! so the old relative paths happened to resolve there and the bug never
//! showed up), this test writes into the OS temp dir, well outside
//! `~/dev/vox`, to actually exercise the "outside a vox checkout" path. See
//! `resolve_vox_repo_root` in `src/codegen_rust/emit/mod.rs` and
//! `docs/src/architecture/generated-project-runtime-deps.md`.
//!
//! Also covers the companion "trim deps" fix: a table-only, non-AI fixture
//! must not list `vox-orchestrator` / `vox-speech` in the generated
//! `Cargo.toml` — neither is referenced by the generated code for this
//! fixture.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use vox_codegen::codegen_rust::RustAppShell;
use vox_codegen::codegen_rust::emit::generate;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;

struct CleanupScratch(PathBuf);

impl Drop for CleanupScratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Slow: writes a generated crate outside the repo and runs `cargo check`
/// against it (~25s+, first-time crate compilation). Excluded from default
/// local `vox ci pre-push --full`; use `--include-slow` or rely on CI.
#[test]
#[ignore = "slow; runs nested cargo check outside the repo (~30s+); owner: codegen sunset: never; use --include-slow or CI"]
fn generated_notes_app_builds_outside_vox_checkout() {
    // A trivial "notes app": one table, one query, one mutation. No `@ai`,
    // `@subagent`, `@durable`/`workflow`/`activity`/`actor`, `@scheduled`,
    // or `Speech.transcribe` — the exact "doesn't need any of the heavy AI
    // subsystem crates" shape from the bug report.
    let src = r#"
        table Note {
            title: str
            body: str
        }

        query note_count() to int {
            return len(db.Note.all())
        }

        mutation touch() to Result[str] {
            return Ok("ok")
        }
    "#;
    let ast = parse(lex(src)).expect("parse");
    let hir = lower_module(&ast);

    let out = generate(&hir, "notes_app_gen", RustAppShell::AxumLocalServer).expect("generate");

    let cargo_toml = out.files.get("Cargo.toml").expect("Cargo.toml emitted");
    assert!(
        !cargo_toml.contains("vox-orchestrator"),
        "a table-only fixture with no @ai subagent/memory-search fixture must not \
         depend on vox-orchestrator:\n{cargo_toml}"
    );
    assert!(
        !cargo_toml.contains("vox-speech"),
        "a fixture with no Speech.transcribe(...) call must not depend on \
         vox-speech:\n{cargo_toml}"
    );

    // Outside the vox checkout entirely — NOT under the workspace root, so
    // the historical `../../crates/...` relative fallback cannot resolve
    // here. If `resolve_vox_repo_root` regresses, `cargo check` below fails
    // with a "failed to read .../crates/vox-db/Cargo.toml" manifest error.
    let uniq = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let scratch = std::env::temp_dir().join(format!("vox_codegen_outside_repo_test_{uniq}"));
    let pkg = scratch.join("notes_app_gen");
    fs::create_dir_all(pkg.join("src")).expect("mkdir");
    let _cleanup = CleanupScratch(scratch.clone());

    for (rel, contents) in &out.files {
        let path = pkg.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir parent");
        }
        fs::write(path, contents).expect("write");
    }

    // Route through whatever `cargo` `cargo test` itself was invoked with
    // (`$CARGO`), same as `ai_fixture_bundle_compiles.rs` — on this machine
    // that's the build-broker shim when installed, or plain `cargo` on PATH
    // otherwise; never a hardcoded resolved path.
    let cargo_bin = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo_bin)
        .current_dir(&pkg)
        .args(["check", "-q"])
        .output()
        .expect("spawn cargo check");

    assert!(
        output.status.success(),
        "cargo check failed for generated notes-app crate outside the vox checkout \
         (under {}):\nstdout:\n{}\nstderr:\n{}",
        pkg.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
