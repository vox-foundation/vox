---
title: "Claude Code Overlay"
description: "Claude Code-specific instructions and behavior narrowing."
category: "contributor"
status: "current"
training_eligible: true
training_rationale: "Defines Claude-specific rules for interacting with the Vox codebase."
---
# Claude Code Overlay

This project uses `AGENTS.md` as the cross-tool policy surface (required reading first).

## Claude-specific additions

- If you open a `.vox` file, treat it as Vox language source — not Rust, not TypeScript.
- Honor `// vox:skip` annotations in code blocks; do not validate those against the compiler.
- Do not store project-specific research in your IDE memory; write to `docs/src/architecture/` instead.
- **Automation scripts are `.vox` files.** Do not generate `.ps1`, `.sh`, or `.py` scripts for project automation. Use `vox run scripts/foo.vox` instead. See [`AGENTS.md §VoxScript-First Glue Code`](AGENTS.md).

See: [AGENTS.md](AGENTS.md) for full policy.

See [agent-instruction-architecture.md](docs/src/contributors/agent-instruction-architecture.md) for the instruction layering model.
