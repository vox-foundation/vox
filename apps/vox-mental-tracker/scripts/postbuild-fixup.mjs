#!/usr/bin/env node
/**
 * Post-vox-build codegen fixups.
 *
 * All codegen gaps previously patched here have landed in the compiler:
 * - §1.A.3 `.length()` → `.length`: fixed in vox-compiler (EmitCtx refactor, 2026-05-08).
 * - §1.A.1/§1.A.2 handler body invocation + async await: fixed in vox-compiler (2026-05-08).
 *
 * This script is now a no-op. It remains in place as a hook point for future
 * emergency patches; once we confirm the app is fully compiler-driven, it can
 * be removed alongside the `postbuild` npm script entry.
 *
 * Other globals (`Speech.transcribe_microphone`, `str(...)`, `std.time.now_ms()`, etc.)
 * resolve via globals installed in src/runtime.ts — tracked as §1.B.1.
 */
console.log("postbuild-fixup: no patches needed (all gaps closed in compiler)");
