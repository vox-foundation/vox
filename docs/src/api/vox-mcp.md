---
title: "Crate API: vox-mcp"
description: "Internal MCP server crate: chat, codegen, and orchestrator bridges."
category: "api-crate"
status: "current"
last_updated: "2026-04-06"
training_eligible: true
---

# Crate API: vox-mcp

Embedded MCP (`vox-mcp`) talks to the workspace orchestrator for chat, routing telemetry, and codegen tools. See [Unified orchestration — SSOT](../reference/orchestration-unified.md) for contract boundaries.

<a id="llm-model-routing-modelstoml"></a>

## LLM model routing (`models.toml`)

Model registry and Ludus routing for MCP-backed chat and `vox_generate_code` are configured through the workspace model stack (including `models.toml` where present). Env overrides and cost telemetry hooks are documented in the orchestration SSOT and [env vars SSOT](../reference/env-vars.md).
