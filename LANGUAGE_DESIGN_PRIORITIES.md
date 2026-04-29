---
title: "LANGUAGE_DESIGN_PRIORITIES.md"
description: "The authoritative priority stack for Vox language, type system, and library design decisions."
category: "architecture"
status: "current"
training_eligible: true
training_rationale: "Defines the design priorities that govern every other architecture decision in the repository."
---

# Vox Language Design Priorities

Vox is a programming language designed as a substrate for LLM-generated code. The design target is the statistical machinery of transformer-class models, not any specific model's current capabilities. Vox is the hand inside the glove that future model-makers train against.

## Priority stack

When two design choices conflict, the higher priority wins. This ordering is load-bearing. It is the first thing every contributor — human or agent — should read.

**P0. Make wrong programs structurally unrepresentable wherever possible.** This is the ceiling of what the language can do for correctness. If a class of bug can be encoded as a type, a parsing rule, or a structural invariant such that programs containing the bug cannot be expressed, encode it that way. Examples: contrast violations refused by the color type system; layout overflow refused by the geometry system; focus traps refused by the navigation system.

**P1. Minimize the number of independent decisions per unit of correct code.** This is the everyday discipline. Each independent decision a model makes during sampling is a multiplicative point of failure. Where a decision carries no semantic content, eliminate the choice. Where a concept has multiple expressions, pick one and remove the others. One canonical shape per concept.

**P2. Distinctive surface syntax that resists Programming Language Confusion drift.** Vox should be visually unmistakable as Vox at the token level. Looking like Python is a correctness regression — under sampling pressure, models drift to languages they recognize. Distinctive keywords, distinctive operators, distinctive structural markers. This is not aesthetics; it is mitigation against a documented failure mode.

**P3. Locality of correctness.** A token's correctness should be determinable from its local context. Long-range coupling — where a token written here is correct or incorrect because of a declaration thousands of tokens earlier — is the worst case for autoregressive sampling. Where global state is unavoidable, surface it locally (visible at the token's call site, in the type, or in a required preceding declaration).

**P4. Human ergonomics, subordinate to P0–P3.** Vox should be writable and readable by humans. When ergonomics conflict with the priorities above, the priorities win. This is a deliberate inversion of how most languages are designed. Humans are now the secondary writer of Vox code; agents are the primary.

**P5. Familiarity to existing models' training distribution is a tiebreaker only.** When two designs are equivalent under P0–P4, prefer the one that fewer existing models will get wrong. Never use familiarity as a primary argument. Future models will be trained on Vox; the design horizon is the substrate, not the present-day distribution.

## Operational corollaries

**C1. The fine-tuning pipeline is part of the language design surface.** Vox's correctness story has two halves: the language structurally prevents what it can, and the fine-tuned model produces idiomatic Vox for what the language cannot prevent. Design choices must be evaluable against the pipeline. Adding a feature whose correct use cannot be taught via fine-tuning is a P0 violation in waiting.

**C2. The GUI paradigm is the wedge.** GUI code is where current models fail most visibly: contrast, touch target size, focus management, geometry, occlusion, ARIA semantics. These error classes are well-documented and substantially preventable at the language level. Demonstrating this is the proof-of-thesis. Other domains (data, workflow, agents) are downstream; if the GUI demonstration succeeds, the framework generalizes.

**C3. Constraint goes where errors happen; expressiveness stays where reasoning happens.** Type systems and parsing rules constrain output at the points models most often go wrong. Body-of-function reasoning is left expressive enough that the model can think in Vox. Over-constraining everything degrades reasoning ability; under-constraining lets known error classes through. The line between these is drawn by where empirical errors cluster.

**C4. One primitive per concept, no cosmetic alternatives.** If there are two ways to express the same thing with no semantic difference, that is a bug, not a feature. Remove one. This applies to syntax, type construction, control flow, and standard library shape. Models forced to choose between semantically equivalent expressions waste a decision and drift toward the training-distribution default.

**C5. Distinctive idioms must be load-bearing.** A surface feature that distinguishes Vox from Python only cosmetically (different keyword for the same thing) does not resist PLC drift effectively. Distinctive features must do real work, so the model that learns Vox cannot collapse them back to Python without breaking semantics.

## Anti-patterns this document forbids

- **Z-index-style global ordering.** A token whose correctness depends on the cumulative effect of every ancestor's stacking-context creation is the canonical violation of P3. Layer systems must be local: a declaration's layer is determined by its kind, not by an integer competing with every other integer in the document.

- **Two ways to express the same data shape.** If `Person { name: str }` and `record Person(name: str)` both produce equivalent values, only one of them exists in Vox.

- **Optional decorators that change semantics.** A decorator that may or may not be present, where the version without it silently defaults to a different behavior, forces a decision and hides it. Either the decorator is required, or it carries no semantic load and is informational only.

- **Names borrowed from Python/JavaScript for the same concept.** Where a concept has a clear name in those languages, Vox uses a different name *if and only if* the concept's behavior in Vox differs in a way users must know about. Otherwise we use the same name. Distinctiveness is for distinct semantics, not for novelty.

## How to use this document

Every design decision — language feature, syntax change, type system extension, library API — should cite which priority it advances and which it costs. Decisions that advance a higher priority at the expense of a lower one are correct. Decisions that advance a lower priority at the expense of a higher one are wrong, regardless of how appealing they look in isolation.

When the next handoff is written, this document is read first.
