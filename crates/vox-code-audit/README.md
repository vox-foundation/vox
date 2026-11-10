# vox-code-audit (`toestub`)

AI code quality stub detector — finds stubs, magic values, empty bodies, missing references, premature victory claims, and DRY violations.

**T**odo, **O**mitted wiring, **E**mpty bodies, **S**tub functions, **T**oo-early victory, **U**nresolved references, **B**roken DRY — detector.

`toestub` mechanically detects AI coding anti-patterns and subtle incomplete implementations that are common when using AI code generation tools.

## What It Detects

| Anti-Pattern | Description |
|-------------|-------------|
| `todo!()` / `unimplemented!()` | Unimplemented stubs left in place across Rust, TypeScript, Python, and Vox |
| Empty function bodies | Functions with no implementation body |
| Hardcoded secrets & magic values | Hardcoded IP addresses, ports, connection strings, API tokens |
| Premature victory claims | Comments like `// TODO: implement later` or `// done` masking incomplete work |
| Heuristic unwrap/panic calls | `.unwrap()` or `panic!` calls outside tests in production code paths |
| Stringly-typed enums | String fields documented with pipe-separated ad-hoc enum lists |

## CLI Binary (`toestub`)

Run the standalone CLI binary on any codebase:

```bash
cargo run -p vox-code-audit --bin toestub -- [OPTIONS] [ROOTS]...
```

### Options

```text
Arguments:
  [ROOTS]...  Root directories or files to scan [default: .]

Options:
  -f, --format <FORMAT>        Output format: terminal, json, markdown [default: terminal]
  -s, --min-severity <LEVEL>   Minimum severity: info, warning, error, critical [default: info]
  -l, --lang <LANG>            Filter by language: rust, typescript, python, gdscript, vox
  -m, --mode <MODE>            Run mode: audit, fix, explain [default: audit]
  -h, --help                   Print help
  -V, --version                Print version
```

## Library Usage

```rust
use vox_code_audit::engine::{ToestubConfig, ToestubEngine, ToestubRunMode};
use vox_code_audit::report::OutputFormat;
use vox_code_audit::rules::Severity;
use std::path::PathBuf;

let config = ToestubConfig {
    roots: vec![PathBuf::from(".")],
    min_severity: Severity::Warning,
    format: OutputFormat::Json,
    run_mode: ToestubRunMode::Audit,
    ..ToestubConfig::default()
};

let engine = ToestubEngine::new(config);
let snapshot = engine.run();

for finding in &snapshot.findings {
    println!("{}:{}: [{}] {}", finding.file.display(), finding.line, finding.rule_id, finding.message);
}
```

## License

Licensed under the Apache License, Version 2.0.
