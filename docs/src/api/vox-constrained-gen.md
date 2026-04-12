---
title: "vox-constrained-gen"
description: "API reference for vox-constrained-gen, the experimental grammar-constrained decoding crate."
category: "api-crate"
status: "current"
last_updated: 2026-04-10
training_eligible: true

schema_type: "TechArticle"
---

# vox-constrained-gen

This crate provides experimental grammar-constrained decoding logic for the Vox ecosystem. It enforces structural constraints during generation, ensuring outputs conform to specified schemas or grammars.

## Relationship to Research
This crate implements the findings described in [`research-grammar-constrained-decoding-2026.md`](../architecture/research-grammar-constrained-decoding-2026.md). It is currently under active experimentation and its surfaces are subject to change.

## Enabling
This feature is considered experimental. It is typically enabled internally by the Orchestrator when required by the active policy or explicitly requested by grammar-constrained prompts.
