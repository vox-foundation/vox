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

# Vox Programming Language

<div class="vox-hero">
    <img src="assets/logo.png" alt="Vox logo" class="logo" />
    <h1>The AI-Native Programming Language</h1>
    <p class="subtitle">One language. Any model. 100% type-safe tool execution.</p>
    <div class="vox-cta-container">
        <a href="tutorials/tut-getting-started.md" class="vox-cta primary">Get Started</a>
        <a href="reference/ref-syntax.md" class="vox-cta secondary">Syntax Reference</a>
    </div>
</div>

Vox integrates the entire stack into a single, compiled boundary. It generates your database schema, server logic, React islands, and MCP tools from a single source file.

## Why Vox?

- **Compiled Agent Boundaries**: Prevent LLM hallucinations by providing a statically checked interface for agent tool calls via MCP.
- **Unified Type Safety**: Define types once. The compiler generates synchronized Rust (backend) and TypeScript (frontend) definitions automatically.
- **Durable by Design**: Build resilient background tasks using native `workflow` and `actor` primitives that survive process restarts.
- **Zero-Null Discipline**: Explicit handling of absence with `Option[T]` and `Result[T, E]`. The compiler enforces exhaustive branching.
- **Built-in AI Tooling**: Every function can be exported as an MCP tool, allowing agents to natively discover and invoke your application's logic.

## Showcase: The Full Stack in One File

### 1. Data and Logic
```vox
{{#include ../../examples/golden/index_showcase.vox:data}}
```

### 2. User Interfaces
```tsx
{{#include ../../examples/golden/index_showcase.vox:ui}}
```

### 3. AI Agents & MCP
```vox
{{#include ../../examples/golden/index_showcase.vox:mcp}}
```

## Documentation Structure

Vox uses the **Diátaxis** framework to organize knowledge by user intent.

<div class="quadrant-container" style="display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; margin: 2rem 0;">
    <div style="border: 1px solid var(--table-border-color); padding: 1rem; border-radius: 8px;">
        <h3>Learning Oriented</h3>
        <p><strong>[Tutorials](tutorials/tut-getting-started.md)</strong></p>
        <p>Step-by-step lessons to build applications and understand core concepts.</p>
    </div>
    <div style="border: 1px solid var(--table-border-color); padding: 1rem; border-radius: 8px;">
        <h3>Problem Oriented</h3>
        <p><strong>[How-To Guides](how-to/how-to-islands-and-pages.md)</strong></p>
        <p>Practical recipes for specific tasks like deployment or database scaling.</p>
    </div>
    <div style="border: 1px solid var(--table-border-color); padding: 1rem; border-radius: 8px;">
        <h3>Understanding Oriented</h3>
        <p><strong>[Explanations](explanation/why-vox-for-ai.md)</strong></p>
        <p>High-level overviews of the compiler architecture and design philosophy.</p>
    </div>
    <div style="border: 1px solid var(--table-border-color); padding: 1rem; border-radius: 8px;">
        <h3>Information Oriented</h3>
        <p><strong>[Reference](reference/ref-syntax.md)</strong></p>
        <p>Technical specifications for keywords, decorators, and the standard library.</p>
    </div>
</div>

## Quick Links
- **[Installation Guide](tutorials/tut-getting-started.md)**: Set up the `vox` toolchain on your machine.
- **[Golden Examples](examples/golden.md)**: Scannable, verified code snippets for common patterns.
- **[Internal Architecture](architecture/architecture-index.md)**: Deep dives into the compiler and runtime internals.
- **[GitHub Repository](https://github.com/vox-foundation/vox)**: Core source code and contributor space (Apache-2.0).
