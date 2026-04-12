---
title: "Research: Planning Mode and Agentic Coding 2026"
description: "Synthesis of planning mode architectures for autonomous agentic coding workflows."
category: "architecture"
status: "research"
training_eligible: false

schema_type: "TechArticle"
---

# Agentic Coding Planning Mode 2026

## Overview
This document synthesizes findings and architectural design decisions for the Vox Agentic Planning Mode (V2). It outlines the pivot from naive LLM task listing to a verifiable, evidence-grounded planning state machine.

## Findings from Original Planning
- **Multi-pass planning**: A single zero-shot generation routinely hallucinates constraints. Separating the LLM into a planner and reviewer limits compounding errors.
- **Evidence-first approach**: The orchestrator must construct a structured factual landscape (`repo_facts`, `reference_docs`) before asking the model to propose solutions.
- **Structured output**: Bounding plan artifacts within formal JSON shapes enforces strict verification boundaries and eliminates vague, unmeasurable subtasks (e.g., "Review and refactor").
- **Verification criteria**: Every independent DAG node (task) must mandate explicit test commands or visual testing procedures.

## Tavily Architecture Inspiration
Tavily's design serves as an inspirational paradigm for our context assembly pipeline:
- **Sub-agent search isolation**: Decoupling the discovery actors from the execution actors ensures evidence collection isn't biased by prompt exhaustion.
- **Relevance-scored context packing**: Retrieving the top `N` memories and domain nodes based on their vector distance to the prompt, avoiding naive recency fallbacks.
- **Adaptive result truncation**: Applying semantic compression when the context limit is breached, prior to packing the token window.

## Vox-Specific Design Decisions
1. **SSOT Representation**: Local `.md` plan files are downgraded to read-only views. Canonical representation is durably stored in `Arca` DB via the `plan_sessions` and `plan_versions` domains.
2. **Versioned Replanning**: Plan iterations do not mutate steps destructively; they spawn a hierarchical lineage, enabling non-destructive rollback.
3. **Implicit Routing**: Task routing to specialized models (CodeGen vs InfraConfig) is intrinsically tied to `TaskCategory`, parsed natively from the structured planner schema.
4. **Tool Entrypoints**: State mutation is heavily centralized over `vox_plan`, `vox_replan`, and `vox_plan_status` directly through the MCP socket to support robust client interactions seamlessly.
