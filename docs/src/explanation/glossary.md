---
title: "Glossary: Vox Terminology"
description: "A centralized registry of technical terms and concepts used within the Vox project."
category: "how-to"
last_updated: 2026-03-24
training_eligible: true
---
# Glossary: Vox Terminology

A centralized registry of technical terms and concepts used within the Vox project.

## A
### Actor
A stateful unit of concurrency that communicates via asynchronous message passing. Every Actor has a mailbox and an address.
### Activity
A retryable, idempotent step within a Vox workflow. Activities are designed to safely interact with external, non-durable systems.
### ADTs (Algebraic Data Types)
Types that can represent several distinct variants (e.g., enums in Rust).
### Affinity Group
A logical grouping of files or tasks used by the Orchestrator to route work to the agent with the most relevant file-system context.
### Arca
The internal storage engine and schema manager for Vox, powered by Turso/LibSQL. It handles code storage and agent state persistence.
### ARS (Automated Reasoning System)
The architecture handling multi-step workflow templates and dynamic dependencies in the Vox DEI Orchestrator.
### Axum
The Rust web framework Vox compiles HTTP endpoints and SSR components into.

## B
### Bincode
The binary serialization format Vox uses to pass messages and workflow state efficiently over the Mens.
### Budget Manager
The system that imposes token and time-based limits on A2A (Agent-to-Agent) orchestrator loops to prevent runaway execution.
### Burn
A native Rust deep learning framework originally used for Vox Mens, alongside Candle.

## C
### Candle
The minimalist ML framework Vox uses for QLoRA fine-tuning and inference.
### Checkpointing
The process of saving workflow state to persistent storage to allow recovery after a crash.
### Codex
The public-facing unified data API in Vox. In the Rust runtime, `VoxDb` is the implementation of Codex.
### CodeStore
The API structure connecting the agent orchestrator to the Arca database for persistent file and session states.
### CST (Concrete Syntax Tree)
A lossless representation of the source code that preserves every character, including whitespace and comments. Vox uses **Rowan** for its CST.

## D
### DEI (Distributed Execution Intelligence)
The core architecture powering Vox's multi-agent orchestrator and interactive control loops.
### Diátaxis
A framework for structuring technical documentation into four distinct pillars (Tutorials, How-To, Explanation, Reference).
### Durable Execution
A system guarantee that a program will eventually complete despite hardware or software failures. Workflows in Vox are durable by design.
### Decorator
A compile-time annotation (e.g., `@server`, `@table`) that modifies the behavior of a function or type during code generation.
### Discriminated Union
A type functionally equivalent to a Rust enum carrying data, replacing standard class inheritance.

## E
### E-E-A-T
Experience, Expertise, Authoritativeness, and Trustworthiness. A Google SEO standard applied to Vox documentation.
### Effect System
A type-level mechanism for tracking side effects (like I/O or network requests) within Vox functions.

## F
### Feature Flag
A mechanism for enabling new Vox compiler rules incrementally.
### Freshness Decay
The rate at which the Mens ML model prioritizes recent `.vox` syntax changes over older (potentially deprecated) code patterns.

## G
### Generics
Functions or ADTs parameterized over types (e.g., `Option[T]`).
### GEO (Generative Engine Optimization)
The practice of structuring documentation specifically for extraction by Google AI Overviews or Bing Copilot.
### GreenTree
The underlying tree structure for the Rowan CST. It is immutable and shared, allowing for efficient representation of syntax.

## H
### HF (Hugging Face)
A repository for ML models and tokenizer configurations. Vox Mens supports loading HF-compatible safetensor checkpoints.
### HIR (High-level Intermediate Representation)
A compiler representation that follows the AST but includes resolved names, types, and desugared constructs.
### HTMX
The primary interactivity layer for Vox frontend islands, rendering directly from server states.

## I
### Island
A highly interactive frontend component (`@island`) hydrated on the client-side within a standard server-rendered page.

## J
### jj / Jujutsu
A version control interface capable of acting as an outer history layer for Vox's internal agent modifications.
### Journal
An append-only log of all operations and state changes performed within a durable workflow.
### JSONL
JSON Lines format, previously used for agent transcripts before the transition to Codex.

## K
### K-Complexity
The cognitive and structural complexity metric Vox uses to govern the size of its syntax and compiler passes.

## L
### Lowering
The process of transforming code from a higher-level representation to a lower-level one (e.g., AST -> HIR -> Codegen).
### LIR (Low-level Intermediate Representation)
A target-specific representation optimized for final code generation.
### LoRA / QLoRA
Quantized Low-Rank Adaptation—the technique enabling fine-tuning of multi-billion parameter LLMs on a single consumer GPU (like an RTX 4080).
### Ludus
The gamification and reward subsystem for AI agents within the Vox DEI orchestrator.

## M
### MCP (Model Context Protocol)
An open protocol that enables AI models to interact with local tools and data. Vox generates MCP tool schemas via `@mcp.tool`.
### Mens
The distributed compute layer in Vox, allowing multiple nodes to coordinate and execute tasks across the network.

## N
### Name Resolution
The compiler phase mapping identifiers in the CST to specific semantic definitions in the `TypeEnv`.
### NF4 (NormalFloat 4)
The specific 4-bit quantization format used by Vox Mens to compress base model weights during training.

## O
### OpenClaw
The underlying specification standard for AI agent skills and dependency resolution.
### Option[T]
The safe alternative to null in Vox. A container that either holds a value (`Some(T)`) or nothing (`None`).
### Oratio
The voice and speech synthesis subsystem in Vox.
### Orchestrator
The `vox-dei` sub-engine that routes tasks between human inputs, automated agents, and durable workflows.

## P
### Pattern Matching
The `match` keyword allows exhaustive checking across union types and ADT variants, guaranteeing all states are handled.
### Mens
The native machine learning and training subsystem in Vox, optimized for QLoRA fine-tuning of LLMs.
### Prestige
A Ludus mechanic for agents completing extreme workflow chains, granting permanent capability boosts.

## R
### Reduction Budget
A fairness mechanism in the scheduler that limits how long a single process can run before yielding.
### Replay
The process of recreating an actor or workflow's state by re-executing its journaled operations from a stable starting point.
### Result[T]
A type used for error handling that either holds a successful value or an error.
### Rune
The reactive primitive in the Vox frontend inspired by Svelte 5 logic tracking.

## S
### Schema.org
The JSON-LD standard embedded into Vox MdBook headers to structure technical documentation SEO.
### Socrates
The anti-hallucination and confidence-scoring protocol used by the Vox orchestrator to validate agent outputs.
### Spawn
The keyword used to create a new instance of an actor.
### SSOT (Single Source of Truth)
The architectural principle that every piece of data or configuration should live in exactly one authoritative location.

## T
### TanStack Router
The foundation for Vox's client-side navigation model.
### Tokio
The async Rust runtime undergirding Vox Actors and Workflows.
### TOESTUB
An architectural enforcement standard for detecting AI-coding anti-patterns during agent execution.
### Training Weight
The multiplier applied to a `.vox` file during Mens QLoRA preflight to emphasize syntactically modern examples.

## U
### Unification
The constraint-solving technique used by the `Typeck` phase to infer missing types across expressions.
### Unit
The literal type `()`, equivalent to `void`, representing actions that return no data.

## V
### Vox Foundation
The governing entity and canonical open-source home for the language.
### VoxDb
The Rust structural equivalent implementing the Codex data and persistence model.

## W
### WASM (WebAssembly)
The compile target for Vox frontend code and isolated plugin logic.
### wgpu
The underlying cross-platform API used for hardware-accelerated Burn / Candle tensor math.
### Workflow
A long-running, durable state machine that coordinates activities and actors.
