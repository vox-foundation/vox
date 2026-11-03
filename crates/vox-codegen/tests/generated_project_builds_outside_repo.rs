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

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

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

static NESTED_CARGO: Mutex<()> = Mutex::new(());
static PKG_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Unique per call: a shared target dir can hand one path package's stale
/// artifacts to another with the same name (see `emit_compile_harness.rs`).
fn unique_pkg_name(base: &str) -> String {
    let n = PKG_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{base}_{}_{n}", std::process::id())
}

/// Writes the generated crate under the OS temp dir, outside the vox checkout,
/// and runs `cargo check` on it. The target dir is stable and shared (also
/// outside the checkout), so only the first run compiles the vox runtime's
/// dependency tree instead of every run starting cold.
fn cargo_check_outside_repo(pkg_name: &str, files: &HashMap<String, String>) -> Output {
    let scratch = std::env::temp_dir().join(format!("vox_codegen_outside_repo_{pkg_name}"));
    let pkg = scratch.join(pkg_name);
    fs::create_dir_all(pkg.join("src")).expect("mkdir");
    let _cleanup = CleanupScratch(scratch.clone());
    for (rel, contents) in files {
        let path = pkg.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir parent");
        }
        fs::write(path, contents).expect("write");
    }

    // `$CARGO` is whatever `cargo test` was invoked with (the build-broker
    // shim when installed), never a hardcoded resolved path.
    let cargo_bin = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let _guard = NESTED_CARGO.lock().unwrap_or_else(|e| e.into_inner());
    Command::new(cargo_bin)
        .current_dir(&pkg)
        .args(["check", "-q"])
        .env(
            "CARGO_TARGET_DIR",
            std::env::temp_dir().join("vox-codegen-outside-repo-target"),
        )
        .output()
        .expect("spawn cargo check")
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

    let pkg_name = unique_pkg_name("notes_app_gen");
    let out = generate(&hir, &pkg_name, RustAppShell::AxumLocalServer).expect("generate");

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

    // Outside the vox checkout entirely, so the historical `../../crates/...`
    // relative fallback cannot resolve. If `resolve_vox_repo_root` regresses,
    // `cargo check` fails with a "failed to read .../crates/vox-db/Cargo.toml".
    let output = cargo_check_outside_repo(&pkg_name, &out.files);

    assert!(
        output.status.success(),
        "cargo check failed for generated notes-app crate outside the vox checkout:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

/// Vox types every `db.T.<op>` as a `Result`, so `db.T.insert(...)?` is valid
/// Vox. Codegen already unwraps the op itself (inner `.await?` in mutations,
/// `.expect` elsewhere), so an extra `?` from the Vox-level `Try` was applied
/// to `()` / `usize` / `Option<Row>` and failed with E0277. The `id: int`
/// params also cover handler params binding as their declared types.
#[test]
#[ignore = "slow; runs nested cargo check outside the repo (~30s+); owner: codegen sunset: never; use --include-slow or CI"]
fn generated_db_write_mutations_with_try_compile() {
    let src = r#"
        table Note {
            title: str
            body: str
        }

        mutation add_note(title: str, body: str) to Result[int] {
            let id = db.Note.insert({ title: title, body: body })?
            return Ok(id)
        }

        mutation add_note_discard(title: str, body: str) to Result[str] {
            db.Note.insert({ title: title, body: body })?
            return Ok("added")
        }

        mutation rename_note(id: int, title: str) to Result[str] {
            db.Note.update(id, { title: title, body: "" })?
            return Ok("renamed")
        }

        mutation remove_note(id: int) to Result[str] {
            db.Note.delete(id)?
            return Ok("removed")
        }

        mutation note_exists(id: int) to Result[bool] {
            let found = db.Note.get(id)?
            return Ok(found.is_some())
        }
    "#;
    let ast = parse(lex(src)).expect("parse");
    let hir = lower_module(&ast);
    let pkg_name = unique_pkg_name("notes_try_gen");
    let out = generate(&hir, &pkg_name, RustAppShell::AxumLocalServer).expect("generate");
    let output = cargo_check_outside_repo(&pkg_name, &out.files);
    assert!(
        output.status.success(),
        "cargo check failed for generated db-write mutations:\nsrc/main.rs:\n{}\nstderr:\n{}",
        out.files
            .get("src/main.rs")
            .map(String::as_str)
            .unwrap_or("<missing>"),
        String::from_utf8_lossy(&output.stderr),
    );
}
