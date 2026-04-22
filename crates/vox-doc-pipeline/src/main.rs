//! Dynamic mdBook `SUMMARY.md` generator and documentation linter for Vox.
//!
//! ## Modes
//!
//! - Default: regenerates `docs/src/SUMMARY.md` and runs the doc linter.
//! - `--check`: validates that `SUMMARY.md` is up-to-date and all markdown docs are structurally clean; exits non-zero on failure.
//! - `--lint-only`: runs the linter without regenerating `SUMMARY.md`.
//! - `--paths <p1,p2,...>`: lint a subset of `docs/src` paths for faster iteration.
//! - `--fix`: apply safe doc auto-fixes (`status: draft` -> `status: roadmap`) before linting.
//!
//! ## Lint checks performed on every `.md` file in `docs/src/`
//!
//! 1. **Code-fence balance**: every opening ` ``` ` must have a matching closing ` ``` `.
//! 2. **Frontmatter presence**: files without a `---` YAML block are flagged as warnings.
//! 3. **Generic descriptions**: template `"Official documentation for ... in the Vox programming language ecosystem."` are errors.

fn main() {
    vox_doc_pipeline::pipeline::run();
}
