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

## Validate from an agent: vox_validate_diagram

`vox_validate_diagram` checks one diagram and returns everything needed to repair it in one round trip. It is deterministic, makes no model calls, and writes nothing. It is in the `lang` capability profile and is eligible for the read role.

Input:

| Field | Type | Meaning |
|-------|------|---------|
| `source` | string, required | The diagram text, or one Mermaid block when `from_mermaid` is true. |
| `from_mermaid` | bool, default false | Convert the Mermaid block first, then validate the conversion. |
| `emit` | `"svg"` or `"mermaid"`, optional | Also render a valid diagram. |

Unknown fields are refused.

Output (`schema_version` is always 1):

| Field | Present | Meaning |
|-------|---------|---------|
| `valid` | always | No error diagnostic remains (and, for Mermaid input, the import was complete). |
| `diagnostics` | always | Every diagnostic in source order: `code`, `severity`, `message`, `span` (line and column), and `fixes[]`, which is omitted when no mechanical fix exists. |
| `normalized_source` | always | The canonical `fmt` text once valid; `null` until then. |
| `stats` | always | `nodes`, `containers`, `edges`, `max_depth`. |
| `emitted` | when `emit` was requested and the diagram is valid | The SVG or Mermaid text. |
| `emit_error` | when an SVG could not be rendered | Why (for example a render cap). Never silently dropped. |
| `unsupported` | when `from_mermaid` | Mermaid lines the importer refused, as `{line, construct}`; empty when the import was complete. |
| `dropped` | when non-empty | Mermaid lines deliberately dropped (styling). |

After a Mermaid import, diagnostic spans point into `normalized_source`, the converted text, not into the original Mermaid.

Repair loop: apply every `fixes[]` entry (each is a byte span plus a replacement in the text you sent), call the tool again, and stop when `valid` is true. Use `normalized_source` as the final text.

## CLI: vox diagram

The command reads a file or stdin (`-`). See [cli.md](cli.md) for the full command surface.

| Subcommand | Role |
|------------|------|
| `vox diagram check <file\|->` | Print each diagnostic as `file:line:col: code: message` on stderr. |
| `vox diagram fmt <file\|-> [--check \| --write]` | Print the canonical form; `--check` exits 1 when not canonical; `--write` rewrites atomically (temp file and rename), only for a valid source. |
| `vox diagram render <file\|-> [-o FILE]` | Deterministic SVG. |
| `vox diagram export <file\|-> [-o FILE]` | Mermaid text. |
| `vox diagram import <file\|-> [-o FILE]` | One Mermaid block to canonical VoxDiagram. |

Exit codes: 0 valid (warnings allowed), 1 at least one error, non-canonical `fmt --check`, unsupported import lines or a failed render, 2 usage or I/O error. The global `--json` prints the same envelope as the MCP tool.

```text
vox diagram check diagram.txt
vox diagram fmt --check - < diagram.txt
vox --json diagram check diagram.txt
```

## Importing Mermaid

Import is fail-closed. It converts a block only when every line maps onto VoxDiagram; otherwise `source` stays empty and `unsupported` lists each refused line with its 1-based line number and a short construct name. Lines that carry only styling are listed in `dropped` and do not block the import. Run `vox diagram import <file|->` for a human-readable list, or call the tool with `from_mermaid: true`.

## Legacy Mermaid fences

Existing fences with the `mermaid` info string under `docs/src` are recorded per file in `contracts/documentation/legacy-mermaid-baseline.v1.yaml`. The baseline only shrinks: the doc lint rejects a new Mermaid fence. New diagrams use a `vox-diagram` fence, which the doc lint validates with the same checker as this tool. VoxDiagram has no Gantt kind, so Gantt charts stay Mermaid.
