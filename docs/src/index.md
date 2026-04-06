---
title: "Vox: The AI-Native Programming Language"
description: "Vox is an AI-native full-stack language that eliminates hallucinations via compiler-enforced reality. Compiles to Rust and TypeScript with zero-null discipline."
category: "getting-started"
status: "current"
sort_order: 0
keywords: ["Vox programming language", "AI-native language", "Zero-hallucination", "Rust compiler", "MCP tools", "Durable workflows"]
last_updated: 2026-04-05
training_eligible: true
difficulty: "beginner"
---

<div class="vox-hero">
  <img src="assets/logo.png" alt="Vox Logo" class="logo" />
  <h1>Vox Programming Language</h1>
  <p class="subtitle">The AI-native language for compiler-enforced reality. Define data, server, and UI in a single source with Rust and TypeScript outputs.</p>
</div>

## The Full Stack in One File

Vox unites the whole stack through a safe, declarative compiler.

### 1. Data and Logic
```vox
{{#include ../examples/golden/index_showcase.vox:data}}
```

### 2. User Interfaces
```tsx
{{#include ../examples/golden/index_showcase.vox:ui}}
```

### 3. AI Agents & MCP
```vox
{{#include ../examples/golden/index_showcase.vox:mcp}}
```

## Why Vox?

- **One Source of Truth**: Application structure, backend codegen, and UI artifacts are all defined in a single language. No more duplicate type definitions across SQL, Rust, and TypeScript.
- **Compiler-Enforced Reality**: Prevent AI agent hallucinations by providing a statically checked boundary for agent interactions via MCP.
- **Zero-Null Discipline**: Explicit handling of absence with `Option[T]` and `Result[T, E]`. The compiler statically enforces exhaustive match checks.
- **Durable Orchestration**: Built-in support for distributed tracking, `actor` patterns, and resuming long-running `workflow` logic via interpreted execution.
- **AI-Native Tooling**: Model Context Protocol (MCP) tool schema generation out-of-the-box, allowing LLMs and agents to natively invoke your application's logic.

## Documentation

Vox uses the **Diátaxis** quadrant to structure its documentation.

| Need a Step-by-Step Lesson? | Need to Solve a Problem? |
| :-------------------------- | :----------------------- |
| **[Tutorials](tutorials/tut-getting-started.md)**<br/>Build the Task app, understand actors, and integrate UI. | **[How-To Guides](how-to/how-to-islands-and-pages.md)**<br/>Database ops, writing MCP tools, and deploying. |

| Need Broad Understanding? | Need Exact Details? |
| :------------------------- | :------------------ |
| **[Explanations](explanation/why-vox-for-ai.md)**<br/>Our compilation strategy, ML pipelines, and ADRs. | **[Reference](reference/ref-syntax.md)**<br/>Decorators, the Standard Library, and CLI bindings. |

## Quick Links
- **[Installation](reference/ref-installation.md)**: Install the `vox` CLI and toolchain constraints.
- **[Why Vox for AI?](explanation/why-vox-for-ai.md)**: Explore the architectural decisions behind preventing LLM hallucinations.
- **[Golden Examples](examples/golden.md)**: Learn through strictly verified snippets covering edge cases and syntax.
- **[GitHub Repository](https://github.com/vox-foundation/vox)**: For contribution guidelines and core development workflow (Apache-2.0).
