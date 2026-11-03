---
title: "Generate a Python client from Vox OpenAPI"
description: "Emit an OpenAPI 3.1 document from a Vox module and generate a Python client with openapi-python-client."
category: "How-To Guides"
status: "current"
topic: "code-generation"
domain: "language"
authorship: "llm-assisted"
authorship_model: "grok-4.7"
training_eligible: false
---

# Generate a Python client from Vox OpenAPI

<!-- kb:summary -->
Shows how to produce an OpenAPI 3.1 document from a Vox module and turn it into a Python client with openapi-python-client. A default fullstack build and a client-target build write dist/openapi.json when the module declares user types or query, mutation, or server endpoints. vox emit openapi writes the same document to openapi.json, or to the path given by --out. The server build target emits Rust only and skips this file. The page gives the commands and does not vendor a Python package.
<!-- /kb:summary -->

Vox emits OpenAPI 3.1 from the same generator the TypeScript client uses
(`vox_codegen_ts::openapi_emit::generate_openapi`). Point
[openapi-python-client](https://github.com/openapi-generators/openapi-python-client)
at that JSON. No Python package is committed with the app.

## Emit the spec

A module emits `openapi.json` when it declares user types or any `query`, `mutation`,
or `server` endpoint.

Default `vox build` (fullstack) and `vox build --target client` write the file under
the output directory (`dist` unless you pass `--out-dir`):

```bash
vox build src/main.vox
vox build src/main.vox --target client
```

Both commands write `dist/openapi.json`.

`vox build --target server` emits the Rust backend only and does not write
`openapi.json`. Use `vox emit openapi` when you want the spec without a TypeScript
build. The default path is `openapi.json` in the working directory; `--out` overrides it:

```bash
vox emit openapi src/main.vox --out dist/openapi.json
```

## Generate the Python client

Install `openapi-python-client` in the Python environment you already use, then
generate from the spec:

```bash
openapi-python-client generate --path dist/openapi.json
```

The generator writes a Python package next to the current directory. Re-run it
after the Vox API changes and you rebuild the spec. This client talks to the HTTP
surface described in the spec; it does not open the app database.
