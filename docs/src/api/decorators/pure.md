---
title: "@pure"
description: "Vox @pure decorator: marks side-effect-free functions for optimization and tooling contracts."
category: "api-decorator"
status: current
last_updated: 2026-04-06

schema_type: "TechArticle"
---

# @pure

Verified golden (machine-checked in CI):

{{#include ../../../../examples/golden/ref_effects.vox:pure_region}}

## Shallow compile-time lint

The compiler runs an **AST-only** hint on `@pure` functions (warning severity, not a proof of purity). It flags obvious calls that are almost never pure in our stdlib surface: **`print`**, **`sleep`**, and **`spawn`**. It does **not** attempt to detect database access, HTTP, environment reads, or other effects.

Diagnostic code: **`lint.pure_shallow_violation`**.
