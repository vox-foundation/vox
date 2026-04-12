---
title: "vox-dei"
description: "API reference for the new HITL vox-dei crate (Doubt/Resolution logic)."
category: "api-crate"
status: "current"
last_updated: 2026-04-10
training_eligible: true

schema_type: "TechArticle"
---

# vox-dei

This crate contains the focused Human-In-The-Loop (HITL) doubt and resolution logic. It replaces the old orchestrator architecture (which was renamed to `vox-orchestrator`).

## Key Components

- **`ResolutionAgent`**: Handles the audit flow and doubt resolution.
- **`FreeAiClient`**: Integration for managing AI assistance during the resolution.
- **`BudgetManager`**: Records the cost incurred during HITL doubt identification and resolution flows.

## Audit Flow

The end-to-end audit flow originates from a `doubt_task` MCP call. The `ResolutionAgent` then steps in to analyze the user's doubt, potentially interacting with the user or other agents to resolve it. A successful resolution, particularly when identifying AI obsequiousness, can trigger a Ludus reward for healthy skepticism.
