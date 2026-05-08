# Language: Regex stdlib (`std.regex`) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans.

**Goal:** Add `std.regex` with `compile(pattern: str) to Result[Regex]`, `Regex.matches(text: str) to bool`, `Regex.find(text: str) to Optional[Match]`, `Regex.find_all(text: str) to List[Match]`, and `Match.group(idx: int) to Optional[str]`.

**Why now:** The vox-mental-tracker's TS-side `intent_parser` uses regex to extract the actual mood score, exercise duration, and meal description from a transcript. The Vox-side `preview_voice_parse` falls back to substring `contains` and hardcodes the extracted values — semantic regression. Bringing Vox to parity needs regex; without it the parser logic must live only in TS, which keeps the language unable to host on-device intent parsing across both runtimes.

Also useful for: `vox-cli` argument validators, route param extraction, contract field shape checks (most of which currently leave Vox and call into Rust crates).

**Architecture:**
- Add `Regex` and `Match` as named opaque types.
- TS codegen: thin wrapper around the native `RegExp` global. Patterns convert from Vox-flavored regex (assume PCRE-like — same as JS for an initial cut) directly.
- Rust codegen: backed by the `regex` crate (already in workspace deps for vox-compiler tests; promote to runtime if needed).
- `std.regex.compile` returns `Result` so invalid patterns produce a typed error rather than panicking at parse time.

**Tech Stack:** Rust (`regex`), TS native `RegExp`.

**Out of scope:**
- Capture group naming.
- Replace / split helpers (separate plan, since they're independently useful).
- Compile-time pattern validation (compiler-side syntax checking of the pattern literal).

---

## File Structure

**Created:**
- `examples/golden/regex_stdlib.vox` covering compile + match + capture group.

**Modified:**
- `crates/vox-compiler/src/typeck/builtins.rs` — add `std.regex` namespace, `Regex` and `Match` types with methods.
- `crates/vox-compiler/src/builtin_registry/**` — add entries.
- `crates/vox-compiler/src/codegen_ts/**` — lower to `new RegExp(pat)`, `pat.test(s)`, `pat.exec(s)`.
- `crates/vox-compiler/src/codegen_rust/**` — lower to `regex::Regex`, `is_match`, `captures`.

---

## Tasks

- [ ] **A1.** Register `Regex` and `Match` named types with their methods.
- [ ] **A2.** Register `std.regex.compile` returning `Result[Regex]`.
- [ ] **A3.** TS codegen — wrap native `RegExp`. Document any pattern-syntax incompatibilities (e.g., Rust `regex` crate doesn't support backreferences; either constrain Vox patterns or use a different Rust crate; recommend just documenting "POSIX-ish, no backreferences" in the reference doc).
- [ ] **A4.** Rust codegen — `regex` crate. If runtime-side: add to the runtime crate's `Cargo.toml`.
- [ ] **A5.** Golden example + snapshot.
- [ ] **A6.** User-facing reference: `docs/src/reference/stdlib-regex.md`.

---

## Verification

- [ ] `cargo nextest run -p vox-compiler` passes.
- [ ] `vox check examples/golden/regex_stdlib.vox` passes.
- [ ] `vox build examples/golden/regex_stdlib.vox` produces TS that runs in node and Rust that compiles.

---

## Integration

After landing, vox-mental-tracker's `preview_voice_parse` can extract real values:
```
let mood_re = std.regex.compile("(?:mood|feeling).*?(\\d)")?
match mood_re.find(t) {
    Some(m) => match m.group(1) {
        Some(s) => /* parse to int, cap to 5, build payload */
        None => /* fall through */
    }
    None => /* fall through */
}
```
At which point the TS `intent_parser` and Vox `preview_voice_parse` can share a fixture-driven test corpus rather than two diverging implementations.
