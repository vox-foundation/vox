---
title: "Reference: VoxDiagram"
description: "Reference for VoxDiagram, the checked diagram language for docs: the agent grammar card, graph and sequence examples, the vox_validate_diagram MCP tool, the vox diagram CLI and the Mermaid policy."
category: "Language Reference"
status: "current"
topic: "language-tooling"
domain: "language"
authorship: "llm-assisted"
authorship_model: "claude-sonnet-5-5"
---

# Reference: VoxDiagram

<!-- kb:summary -->
VoxDiagram is the checked diagram language for Vox documentation. This page publishes the grammar card that agents receive with the validation tool, shows a graph and a sequence example that the doc lint validates, and documents the vox_validate_diagram MCP envelope, the vox diagram command line with its exit codes, strict Mermaid import, and the shrink-only policy for legacy Mermaid fences in docs.
<!-- /kb:summary -->

## Grammar card

This is the exact text appended to the `vox_validate_diagram` tool description. A test fails if this copy and the shipped constant diverge.

```text
VoxDiagram: graph or sequence. Put it in a vox-diagram fenced block. EVERY label is "double quoted".

graph {
    direction: right // down | right | up | left (optional, default down)

    node client: "Desktop Client"
    node db: "Task DB" { shape: cylinder } // rect (default) rounded diamond cylinder start end

    container backend: "Orchestrator" { // containers nest, max depth 4
        node gateway: "Gateway"
        node engine: "Agent Runtime"
        gateway -> engine: "dispatch" // edge label optional
        engine -> engine: "retry" // self-loop ok
    }

    client -> backend.gateway: "TLS" // dotted path reaches inside a container
    backend.engine -> db
}

sequence {
    actor ui: "Desktop Client"
    actor api: "Gateway"
    ui -> api: "POST /run"
    note over api: "validates input"
    reply api -> ui: "202 Accepted" // reply is the dashed return
}

Rules
1. Declare every node: node id: "Label". An edge never creates a node.
2. The arrow is exactly ->. Endpoints are nodes, never containers (use container.node).
3. Ids: letters, digits, _ and -; start with a letter or _. Reserved: graph sequence direction node container shape
4. One statement per line, one arrow per statement. Comments are // only. Escapes in labels: \" \\ \n.
5. Sequences: declare every actor first. a -> b: "text" is a message, reply a -> b: "text" a dashed return, note over a[, b]: "text" a note.
6. Limits: 64 KiB, 500 nodes, 1000 edges, depth 4, labels 200 chars.

Do NOT write Mermaid (each is an error):
  A[Label] --> B           => node a: "Label" then a -> b
  subgraph X ... end       => container x: "X" { ... }
  graph TD                 => graph { direction: down }
  flowchart LR             => graph { direction: right }
  a -->|text| b            => a -> b: "text"
  a -- text --> b          => a -> b: "text"
  a & b --> c              => a -> c and b -> c on two lines
  %% comment               => // comment
  node a: "A";             => no semicolon
  style a fill:red         => styling is not supported

Run the validator; apply every fixes[] entry; repeat until valid.
```

## Examples

A graph with a nested container:

```vox-diagram
graph {
    direction: right

    node client: "Desktop Client"
    node db: "Task DB" { shape: cylinder }

    container backend: "Orchestrator" {
        node gateway: "Gateway"
        node engine: "Agent Runtime"
        gateway -> engine: "dispatch"
    }

    client -> backend.gateway: "TLS"
    backend.engine -> db
}
```

A sequence with a note and a dashed reply:

```vox-diagram
sequence {
    actor ui: "Desktop Client"
    actor api: "Gateway"
    ui -> api: "POST /run"
    note over api: "validates input"
    reply api -> ui: "202 Accepted"
}
```
