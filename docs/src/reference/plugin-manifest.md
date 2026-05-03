---
title: "Plugin Manifest (Plugin.toml)"
description: "Schema for the Plugin.toml file every Vox plugin ships."
category: "reference"
status: "current"
training_eligible: true
---

# Plugin Manifest (Plugin.toml)

Every Vox plugin ships a `Plugin.toml` in its install directory describing what the plugin is, what it provides, and what the host needs to know to load it. This page is the schema reference. For the design rationale see [Plugin System Redesign (2026)](../architecture/plugin-system-redesign-2026.md).

## Common header

```toml
[plugin]
id = "<short-hyphenated-id>"
name = "<human-readable name>"
version = "<semver>"
description = "<one-line description>"
authors = ["..."]
license = "<SPDX identifier>"
homepage = "<url>"

[plugin.host]
min-vox-version = "<semver>"
```

## Payload kinds

The `[plugin.payload]` block discriminates on `kind`:

- `code` — ships a `cdylib` per OS/arch and provides one or more code extension points.
- `skill` — ships a `SKILL.md` and registers MCP tools.
- `composite` — ships both.

### Code payload

```toml
[plugin.payload]
kind = "code"
abi-version = 1

[plugin.payload.provides]
extension-points = ["MlBackend"]

[plugin.payload.requires]
os = ["windows", "linux"]
arch = ["x86_64"]
native-libs = [
    { name = "cudart", min-version = "12.0" },
    { name = "cublas" },
]

[plugin.payload.artifacts]
"windows-x86_64" = "vox_plugin_<id>.dll"
"linux-x86_64"   = "libvox_plugin_<id>.so"
"macos-aarch64"  = "libvox_plugin_<id>.dylib"
```

### Skill payload

```toml
[plugin.payload]
kind = "skill"
format-version = 1
skill-md = "<filename>.skill.md"

[plugin.payload.tools]
exposes = ["vox_tool_one", "vox_tool_two"]
```

### Composite payload

```toml
[plugin.payload]
kind = "composite"

[plugin.payload.code]
abi-version = 1
provides.extension-points = ["MeshDriver"]
artifacts."linux-x86_64" = "libvox_plugin_<id>.so"

[plugin.payload.skill]
format-version = 1
skill-md = "<filename>.skill.md"
tools.exposes = ["vox_tool_one"]
```

## Validation

The manifest is parsed at host startup and validated against this schema. Failures are reported by `vox plugin doctor` with the offending field path.
