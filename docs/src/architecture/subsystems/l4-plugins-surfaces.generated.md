---
title: "Layer 4 — Native Plugins & Terminal TUI (Generated)"
description: "Dynamically loaded skill/inference plugins and the ratatui-based vox-term TUI."
category: "Architecture SSOTs"
status: "current"
sort_order: 14
code_anchors:
  - "docs/src/architecture/layers.toml"
---

# Layer 4 — Native Plugins & Terminal TUI

Dynamically loaded skill/inference plugins and the ratatui-based vox-term TUI.

## Subsystem Topology

```vox-diagram
graph {
    direction: down
    node l4_plugins_surfaces: "Layer 4 — Native Plugins & Terminal TUI"
    node vox_plugin_browser: "vox-plugin-browser (plugin)"
    l4_plugins_surfaces -> vox_plugin_browser
    node vox_plugin_mens_candle_core: "vox-plugin-mens-candle-core (library)"
    l4_plugins_surfaces -> vox_plugin_mens_candle_core
    node vox_plugin_mens_candle_cuda: "vox-plugin-mens-candle-cuda (plugin)"
    l4_plugins_surfaces -> vox_plugin_mens_candle_cuda
    node vox_plugin_mens_candle_metal: "vox-plugin-mens-candle-metal (plugin)"
    l4_plugins_surfaces -> vox_plugin_mens_candle_metal
    node vox_plugin_nvml_probe: "vox-plugin-nvml-probe (plugin)"
    l4_plugins_surfaces -> vox_plugin_nvml_probe
    node vox_plugin_populi_mesh: "vox-plugin-populi-mesh (plugin)"
    l4_plugins_surfaces -> vox_plugin_populi_mesh
    node vox_plugin_publication: "vox-plugin-publication (plugin)"
    l4_plugins_surfaces -> vox_plugin_publication
    node vox_plugin_runtime_container: "vox-plugin-runtime-container (plugin)"
    l4_plugins_surfaces -> vox_plugin_runtime_container
    node vox_plugin_runtime_wasm: "vox-plugin-runtime-wasm (plugin)"
    l4_plugins_surfaces -> vox_plugin_runtime_wasm
    node vox_plugin_speech: "vox-plugin-speech (plugin)"
    l4_plugins_surfaces -> vox_plugin_speech
    node vox_plugin_webhook: "vox-plugin-webhook (plugin)"
    l4_plugins_surfaces -> vox_plugin_webhook
    node vox_term: "vox-term (binary)"
    l4_plugins_surfaces -> vox_term
}
```

## Crates in Layer

| Crate | Kind | Fan-In Cap | LoC Baseline |
|---|---|---|---|
| [`vox-plugin-browser`](../../api/vox-plugin-browser.generated.md) | `plugin` | — | — |
| [`vox-plugin-mens-candle-core`](../../api/vox-plugin-mens-candle-core.generated.md) | `library` | — | — |
| [`vox-plugin-mens-candle-cuda`](../../api/vox-plugin-mens-candle-cuda.generated.md) | `plugin` | — | — |
| [`vox-plugin-mens-candle-metal`](../../api/vox-plugin-mens-candle-metal.generated.md) | `plugin` | — | — |
| [`vox-plugin-nvml-probe`](../../api/vox-plugin-nvml-probe.generated.md) | `plugin` | — | — |
| [`vox-plugin-populi-mesh`](../../api/vox-plugin-populi-mesh.generated.md) | `plugin` | — | — |
| [`vox-plugin-publication`](../../api/vox-plugin-publication.generated.md) | `plugin` | — | — |
| [`vox-plugin-runtime-container`](../../api/vox-plugin-runtime-container.generated.md) | `plugin` | — | — |
| [`vox-plugin-runtime-wasm`](../../api/vox-plugin-runtime-wasm.generated.md) | `plugin` | — | — |
| [`vox-plugin-speech`](../../api/vox-plugin-speech.generated.md) | `plugin` | — | — |
| [`vox-plugin-webhook`](../../api/vox-plugin-webhook.generated.md) | `plugin` | — | — |
| [`vox-term`](../../api/vox-term.generated.md) | `binary` | — | 15000 |
