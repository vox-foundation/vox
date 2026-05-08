#!/usr/bin/env node
/**
 * Post-vox-build codegen fixups.
 *
 * Patches small codegen-TS gaps that aren't yet handled by the compiler:
 * - `.length()` method-call form → `.length` property access (strings/lists).
 *
 * Tracked compiler-side in
 * docs/superpowers/plans/2026-05-08-codegen-ts-bugs-blocking-tracker.md
 * (follow-up beyond the four bugs that already landed). Once the codegen
 * lowers `len()` calls properly this script becomes a no-op and can be
 * removed.
 *
 * Also: `Speech.transcribe_microphone()`, `str(...)`, `std.time.now_ms()`,
 * etc. resolve via globals installed in src/runtime.ts — no rewrite needed.
 */
import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

const distDir = "dist";
let touched = 0;

const files = await readdir(distDir);
for (const f of files) {
  if (!f.endsWith(".tsx") && !f.endsWith(".ts")) continue;
  const path = join(distDir, f);
  const original = await readFile(path, "utf8");
  const patched = original.replace(/\.length\(\)/g, ".length");
  if (patched !== original) {
    await writeFile(path, patched);
    touched++;
  }
}

console.log(`postbuild-fixup: patched .length() → .length in ${touched} file(s)`);
