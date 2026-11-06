---
title: "Layer 1 — Primitives, Crypto, Secrets & Transport (Generated)"
description: "Foundational primitives including vox-crypto, Clavis secrets, iroh QUIC mesh transport, and bounded FS."
category: "Architecture SSOTs"
status: "current"
sort_order: 11
code_anchors:
  - "docs/src/architecture/layers.toml"
---

# Layer 1 — Primitives, Crypto, Secrets & Transport

Foundational primitives including vox-crypto, Clavis secrets, iroh QUIC mesh transport, and bounded FS.

## Subsystem Topology

```vox-diagram
graph {
    direction: down
    node l1_primitives: "Layer 1 — Primitives, Crypto, Secrets & Transport"
    node vox_bounded_fs: "vox-bounded-fs (library)"
    l1_primitives -> vox_bounded_fs
    node vox_build_queue: "vox-build-queue (library)"
    l1_primitives -> vox_build_queue
    node vox_crypto: "vox-crypto (library)"
    l1_primitives -> vox_crypto
    node vox_foundation: "vox-foundation (library)"
    l1_primitives -> vox_foundation
    node vox_grammar_export: "vox-grammar-export (library)"
    l1_primitives -> vox_grammar_export
    node vox_graph_reader: "vox-graph-reader (library)"
    l1_primitives -> vox_graph_reader
    node vox_hf_layout: "vox-hf-layout (library)"
    l1_primitives -> vox_hf_layout
    node vox_http_client: "vox-http-client (library)"
    l1_primitives -> vox_http_client
    node vox_identity: "vox-identity (library)"
    l1_primitives -> vox_identity
    node vox_jsonschema_util: "vox-jsonschema-util (library)"
    l1_primitives -> vox_jsonschema_util
    node vox_language_surface: "vox-language-surface (library)"
    l1_primitives -> vox_language_surface
    node vox_mesh_transport: "vox-mesh-transport (library)"
    l1_primitives -> vox_mesh_transport
    node vox_openai: "vox-openai (library)"
    l1_primitives -> vox_openai
    node vox_package_types: "vox-package-types (library)"
    l1_primitives -> vox_package_types
    node vox_plugin_api: "vox-plugin-api (library)"
    l1_primitives -> vox_plugin_api
    node vox_plugin_sdk: "vox-plugin-sdk (library)"
    l1_primitives -> vox_plugin_sdk
    node vox_plugin_types: "vox-plugin-types (library)"
    l1_primitives -> vox_plugin_types
    node vox_redact: "vox-redact (library)"
    l1_primitives -> vox_redact
    node vox_research_events: "vox-research-events (library)"
    l1_primitives -> vox_research_events
    node vox_rule_pack: "vox-rule-pack (library)"
    l1_primitives -> vox_rule_pack
}
```

## Crates in Layer

| Crate | Kind | Fan-In Cap | LoC Baseline |
|---|---|---|---|
| [`vox-bounded-fs`](../../api/vox-bounded-fs.generated.md) | `library` | 25 | — |
| [`vox-build-queue`](../../api/vox-build-queue.generated.md) | `library` | — | — |
| [`vox-crypto`](../../api/vox-crypto.generated.md) | `library` | — | — |
| [`vox-foundation`](../../api/vox-foundation.generated.md) | `library` | 18 | — |
| [`vox-grammar-export`](../../api/vox-grammar-export.generated.md) | `library` | — | — |
| [`vox-graph-reader`](../../api/vox-graph-reader.generated.md) | `library` | — | — |
| [`vox-hf-layout`](../../api/vox-hf-layout.generated.md) | `library` | — | 6000 |
| [`vox-http-client`](../../api/vox-http-client.generated.md) | `library` | 25 | — |
| [`vox-identity`](../../api/vox-identity.generated.md) | `library` | — | — |
| [`vox-jsonschema-util`](../../api/vox-jsonschema-util.generated.md) | `library` | — | — |
| [`vox-language-surface`](../../api/vox-language-surface.generated.md) | `library` | — | — |
| [`vox-mesh-transport`](../../api/vox-mesh-transport.generated.md) | `library` | — | — |
| [`vox-openai`](../../api/vox-openai.generated.md) | `library` | — | — |
| [`vox-package-types`](../../api/vox-package-types.generated.md) | `library` | — | — |
| [`vox-plugin-api`](../../api/vox-plugin-api.generated.md) | `library` | — | — |
| [`vox-plugin-sdk`](../../api/vox-plugin-sdk.generated.md) | `library` | — | — |
| [`vox-plugin-types`](../../api/vox-plugin-types.generated.md) | `library` | — | — |
| [`vox-redact`](../../api/vox-redact.generated.md) | `library` | — | — |
| [`vox-research-events`](../../api/vox-research-events.generated.md) | `library` | — | — |
| [`vox-rule-pack`](../../api/vox-rule-pack.generated.md) | `library` | — | — |
| [`vox-scaling-policy`](../../api/vox-scaling-policy.generated.md) | `library` | — | — |
| [`vox-secrets`](../../api/vox-secrets.generated.md) | `library` | 34 | — |
| [`vox-telemetry`](../../api/vox-telemetry.generated.md) | `library` | — | — |
