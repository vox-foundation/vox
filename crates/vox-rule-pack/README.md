# vox-rule-pack

Declarative rule pack loader and validator for code audit detectors and static heuristics from the [Vox](https://github.com/vox-foundation/vox) project.

## Features

- **Rule Declarations:** Structured YAML/JSON rule formats defining regex, structural patterns, severity, and remediation advice.
- **Rule Engine:** High-performance rule execution and filtering without heavy dependencies.
- **Auditing Integration:** Powers static analysis detectors across repositories.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
vox-rule-pack = "0.6"
```

## License

Apache-2.0
