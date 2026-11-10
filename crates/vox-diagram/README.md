# vox-diagram

Pure VoxDiagram core: graph DSL lexer, error-recovering parser, resolved IR, diagnostics with machine-applicable fixes, canonical formatter, markdown fence scanner, dagre graph layout, and a deterministic SVG renderer from the [Vox](https://github.com/vox-foundation/vox) project.

## Features

- **Lexer & Parser:** Error-recovering parser producing typed diagram IR (flowcharts, sequence diagrams, state machines, class diagrams, entity-relationship diagrams).
- **Diagnostics:** Spanned compiler errors with machine-applicable code fixes.
- **Formatter:** Canonical code formatting for `.vdg` diagrams.
- **Layout & Rendering:** Built-in dagre-based graph layout engine with deterministic SVG rendering. Zero system font or C library dependencies.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
vox-diagram = "0.6"
```

## License

Apache-2.0
