---
title: "Vox FAQ: Frequently Asked Questions"
description: "Answers to common questions about the Vox programming language: setup, concurrency, durability, and AI integration."
category: "how-to"
last_updated: 2026-03-24
training_eligible: true
---

# Vox Frequently Asked Questions (FAQ)

## Language Basics

### What is Vox?
Vox is an AI-native, full-stack programming language that unifies frontend, backend, and infrastructure into a single, LLM-friendly syntax. It compiles to Rust (backend) and TypeScript (frontend) — it does not replace either, but orchestrates both from one codebase.

### Is Vox statically typed?
Yes. Vox uses bidirectional type inference: you rarely need explicit types inside function bodies, but all signatures are validated at compile time.

### How does Vox handle null?
Null is completely banned. Absent values use `Option[T]` (`Some(value)` or `None`); fallible operations use `Result[T, E]` (`Ok(value)` or `Error(e)`). Both must be explicitly handled — the compiler rejects unhandled cases. See [Type System Reference](../reference/ref-type-system.md) for details.

## Installation & Toolchain

### How do I install and update Vox?
Build from source with `cargo install --path crates/vox-cli` (or use the install scripts in the repo).  
To discover what your currently installed binary supports, run `vox commands --recommended` and `vox commands --include-nested`.

### What does `vox build` do?
`vox build` lexes, parses, and type-checks your `.vox` file, then generates Rust and TypeScript output.  
Why use it: it gives you a deterministic compile artifact you can inspect before running or bundling.

### Can I use existing Rust or NPM libraries?
Yes. Use `@rust.import` for Rust crates and standard NPM imports in frontend blocks.

## Architecture & Runtime

### What are Actors and Workflows?
- **Actor** — a stateful unit of concurrency with a private mailbox. Processes one message at a time; no shared-state races.
- **Workflow** — a durable state machine that coordinates long-running work. If your server crashes mid-execution, the workflow resumes exactly where it left off on restart.

### What is the Mens?
Vox's distributed compute layer. Nodes across regions communicate and route actor messages natively — no Redis or RabbitMQ setup required. The Mens BaaS layer handles Codex (Turso) database connections and actor routing automatically.

### How is Vox different from Go or Erlang/Elixir?
Go's goroutines and Erlang's processes are ephemeral — a crash loses their state. Vox Workflows are durably persisted. Vox also adds static typing (Rust codegen) and a unified UI layer (React/TypeScript), which neither Go nor Erlang/Elixir provide out of the box.

## AI & ML Integration

### How does Vox support AI agents?
Vox has native [Model Context Protocol (MCP)](https://modelcontextprotocol.io) support. Add `@mcp.tool` to any function and Vox automatically generates a standard MCP JSON schema — your app instantly becomes a tool or data source for external agents (Claude, OpenAI, etc.).

### What is Mens, and how do I fine-tune a model?
Mens is Vox's native ML pipeline for QLoRA (Quantized Low-Rank Adaptation) fine-tuning of LLMs on your codebase — entirely in Rust, no Python required. Run:

```bash
vox mens train --backend qlora
```

It crawls files marked `training_eligible: true`, builds tensors, and runs the Candle training backend. See [How To: Train Mens Models](../how-to/how-to-train-mens-4080.md).

### What is the Socrates Protocol?
An anti-hallucination layer built into the orchestrator. Before generating or approving code, Vox asks the underlying LLM to self-evaluate confidence and structure its reasoning — reducing spurious output.

## Deployment & Community

### How do I deploy a Vox app?
Vox compiles to a single statically-linked binary. Deploy it anywhere — AWS, Render, a Raspberry Pi, or the Vox Mens.

### Is Vox open source? How do I contribute?
Yes, Apache-2.0 licensed. Submit PRs on GitHub, follow `STYLE.md`, and run `vox ci manifest` plus `vox ci command-compliance` before pushing.
