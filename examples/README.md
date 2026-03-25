# Vox Examples

Working code examples demonstrating Vox language features. Each `.vox` file is a complete, self-contained program.

## Golden Examples

The `golden/` directory contains validated, current-syntax examples suitable for learning and for the **Mens** ML training corpus. Every file in `golden/` passes `vox check` in CI.

| Example | Description | Difficulty |
|---------|-------------|------------|
| [hello.vox](golden/hello.vox) | Minimal Vox function | Beginner |
| [crud_api.vox](golden/crud_api.vox) | Server functions with database CRUD (`@server`, `@table`) | Beginner |
| [counter_actor.vox](golden/counter_actor.vox) | Persistent actor with state | Intermediate |
| [checkout_workflow.vox](golden/checkout_workflow.vox) | Durable workflow with retries | Intermediate |
| [dashboard_ui.vox](golden/dashboard_ui.vox) | UI components and routing | Intermediate |
| [mcp_tools.vox](golden/mcp_tools.vox) | MCP tool generation with `@mcp.tool` | Intermediate |
| [agent_pipeline.vox](golden/agent_pipeline.vox) | AI agent pipeline with actors | Advanced |
| [test_suite.vox](golden/test_suite.vox) | Unit testing with `@test` | Beginner |
| [type_system.vox](golden/type_system.vox) | ADTs, `Option[T]`, `Result[T]` | Intermediate |
| [config_deploy.vox](golden/config_deploy.vox) | Configuration and deployment patterns | Advanced |

## Example Lifecycle

```
golden → deprecated → archived
```

1. **Golden**: Current, validated syntax. CI must pass. Eligible for Mens training.
2. **Deprecated**: Syntax is outdated. Kept for historical reference. Not in training corpus.
3. **Archived**: Moved to `archive/`. Not shown in docs. Not in training.

When the Vox syntax changes:
1. Create a new `golden/` example with the updated syntax.
2. Set `superseded_by` in the old file's header and mark `status: deprecated`.
3. Move the deprecated file to `archive/`.
4. CI will update `PARSE_STATUS.md` automatically.

## Parse Status

See [PARSE_STATUS.md](PARSE_STATUS.md) for the CI-generated parse result matrix.

## Style Guide

See [STYLE.md](STYLE.md) for coding conventions and frontmatter requirements.
