---
title: "Vox: The AI-Native Programming Language"
description: "Vox is a unified full-stack language that compiles to Rust and TypeScript. Zero null states, durable workflows, native MCP support, and a built-in ML training pipeline."
category: "getting-started"
sort_order: 0
keywords: ["Vox programming language", "AI-native language", "Rust compiler", "full-stack language"]
last_updated: 2026-03-24
training_eligible: true
difficulty: "beginner"
---

# Vox: The AI-Native Programming Language

**What is Vox?** 
Vox is a unified, full-stack programming language designed to bridge the gap between high-level AI intent and low-level system performance. By compiling directly to **Rust** for backend durability and **TypeScript** for frontend reactivity, Vox enables developers to write their entire application stack in a single, LLM-friendly syntax.

## Why Vox?

The software industry has fragmented into hundreds of specialized frameworks. Vox solves this by unifying the stack natively:
- **AI-Native Grammar**: The grammar is free of syntactical ambiguities, making it easier for Large Language Models (LLMs) to generate pristine Abstract Syntax Trees (AST).
- **Uniformity**: Frontend components, backend services, and database schemas live together in one `.vox` file. Define a `@table` and you get the schema, the CRUD API, and the React types for free.
- **Durable Execution**: Workflows survive machine failures. If a server goes down during a multi-step `workflow`, Vox automatically resumes exactly where it left off upon restart.
- **Zero Null States**: Null references are completely banned from the language. All absence of value must be represented by `Option[T]` or `Result`, eliminating the most common source of runtime crashes.
- **Native ML Pipeline**: Integrated training with **Populi** allows you to perform Quantized Low-Rank Adaptation (QLoRA) directly within the Vox ecosystem.
- **First-Class AI Agents**: Adding `@mcp.tool` to any function instantly exposes it as a Model Context Protocol generic tool for external AI agents.

## Quick Start
Get your first Vox app running and deployed locally in under 5 minutes:

### 1. Install the CLI
Ensure you have Rust installed, then install the Vox compiler CLI directly:
```bash
cargo install --path crates/vox-cli
```

### 2. Initialize a Project
Use the CLI to scaffold a new project with the default TanStack template:
```bash
vox init my-app && cd my-app
```

### 3. Run Your Application
Start the development server, which hot-reloads both your Rust backend and TypeScript frontend:
```bash
vox run src/main.vox
```

## Language at a Glance

Vox seamlessly mixes different programming paradigms based on the intent of the block.

**Type-Safe APIs and Database Tables:**
```vox
@table type User { id: int, name: str }

@query
fn get_user(id: int) to str {
    ret db.User.find(id).name
}
```

**Durable State Machines (Actors):**
```vox
actor Counter {
    state count: int = 0
    on Increment() to int {
        count = count + 1
        ret count
    }
}
```

**Interactive Frontend Components:**
```vox
@island
fn DashboardView() to Element {
    ret <div className="dashboard">
        <h1>Overview</h1>
    </div>
}
```

## Navigating the Documentation

Vox utilizes the **Diátaxis** documentation framework. Choose your path based on what you need to achieve:

### 🚀 Tutorials (Learning)
Guided, step-by-step lessons to learn the platform.
- [Tutorial: Your First Full-Stack App](tutorials/tut-first-app.md)
- [Tutorial: Actor Basics](tutorials/tut-actor-basics.md)
- [Tutorial: Workflow Durability](tutorials/tut-workflow-durability.md)

### 🛠️ How-To Guides (Problem Solving)
Goal-oriented recipes for common tasks.
- [How To: Deploy to Production](how-to/how-to-deploy.md)
- [How To: Build AI Agents & MCP Tools](how-to/how-to-ai-agents.md)
- [How To: Train Populi Models](how-to/how-to-train-populi-4080.md)

### 📚 Reference (Information)
Technical descriptions of language machinery and syntax.
- [Language Syntax Guide](reference/ref-language.md)
- [Type System Reference](reference/ref-type-system.md)
- [CLI Commands Reference](reference/cli.md)
- [Decorator Registry](reference/ref-decorators.md)

### 🧠 Explanations (Understanding)
Deep dives into the architecture and theory.
- [Compiler Lowering Phases](explanation/expl-compiler-lowering.md)
- [The Durable Execution Model](explanation/expl-durable-execution.md)
- [The Vox Runtime Architecture](explanation/expl-runtime.md)

## Join the Foundation

Vox is developed by the **Vox Foundation** under the Apache-2.0 license. We are building a future where software is declarative, distributed, and naturally understood by both humans and context-aware AIs.

- Read the source code on [GitHub](https://github.com/vox-foundation/vox)
- Review our [Architecture Decision Records (ADR)](adr/README.md)
- Check out [Golden Examples](examples/golden.md) for compiled, working snippet code.
