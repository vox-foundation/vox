---
title: "How-To: Build AI Agents and MCP Tools"
description: "Expand your Vox logic into reusable tools for AI assistants, wire agents into Vox's Federated Knowledge Base, and define MCP capabilities."
category: "How-To Guides"
status: "current"
topic: "mcp-tools"
domain: "orchestration"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
schema_type: "HowTo"
keywords: ["MCP tools Vox", "AI agents tutorial", "Model Context Protocol programming", "build AI agents Rust"]
---

# How-To: Build AI Agents and MCP Tools

<!-- kb:summary -->
Guides developers through exposing typed Vox functions as Model Context Protocol (MCP) tools using the `tool` keyword, composing agent orchestration with `@require` guardrails, verifying endpoints over HTTP/MCP, and connecting agents to Vox's 3-layer Federated Knowledge Base (`vox_kb_search`, `vox_kb_cite`, `vox_repo_query_text`) and self-healing OpenWiki pipelines.
<!-- /kb:summary -->

Vox is an AI-native language, meaning it bridges the gap between high-level business logic and the Model Context Protocol (MCP) without glue code. Any Vox function can become an MCP tool with a single keyword.

## 1. Creating MCP Tools

Any Vox function can be exported as an MCP tool using the `tool` keyword.

```vox
tool "Calculate the sum of two integers" sum(a: int, b: int) to int {
    return a + b
}
```

### Comparison to other approaches:
- **Type Safety**: If your function returns a `Result[T, E]`, Vox handles the MCP error response mapping for you.
- **Zero Configuration**: No manifests to maintain. The `tool` declaration is the manifest.
- **Auto-Discovery**: Tools are automatically discovered by the `vox-orchestrator` during development.

---

## 2. Defining Agent Roles

> [!NOTE]
> The `agent` declaration is tombstoned — it is not in the active grammar. Define agent behavior as a plain function, and expose capabilities via `tool`/`resource` declarations for MCP discovery instead.

```vox
fn assistant_greet(name: str) to str {
    return "Hello " + name + ", how can I assist you today?"
}
```

### Composing Agent Behavior
Since there is no dedicated agent construct, "handoffs" between agent-like functions are just ordinary function calls or MCP tool invocations orchestrated by `vox-orchestrator`.

---

## 3. Tool Discovery and Execution

To expose your tools to a local AI assistant (like Claude Desktop or Cursor):

1. **Run the MCP server**:
   ```bash
   vox run src/main.vox
   ```
2. **Observe Logs**: The orchestrator will list all registered tools and resources.
3. **Connect**: Add the generated endpoint to your `claude_desktop_config.json`.

---

## 4. Testing Your Tools

Never guess if a tool works. Verify tool registration and surface parity against the generated server using `vox ci mcp-vox-surface-parity`.

```bash
# Test the 'search_docs' endpoint manually using standard tools
curl -X POST http://localhost:8080/api/tools/search_docs -d '{"query": "actors"}'
```

---

## 5. Security and Bounds

By default, a `tool` has the same permissions as your compiled Vox binary. Use the `@require` decorator to add runtime guardrails:

```vox
table User {
    name: str
}

tool "Delete user data"
@require(auth.is_admin(caller))
delete_data(id: Id[User]) to Result[str] {
    db.User.delete(id)
    return Ok("deleted")
}
```

If the precondition fails, the MCP tool returns a "Tool execution failed" error to the model with the specific violation reason, preventing the LLM from attempting unauthorized actions.

---

## 6. Grounding Agents in Vox's Federated Knowledge Base & OpenWiki

Connected MCP clients automatically gain access to Vox's built-in retrieval and repository-intelligence tools (`vox_kb_search`, `vox_kb_cite`, `vox_kb_get`, `vox_repo_query_text`, and `vox_repo_status`), allowing agents to pull line-anchored `#Lstart-Lend` citations across repo-local `docs/wiki/` and `~/.vox/library/` before writing code.

See the hands-on walkthroughs:
- [Tutorial: Managing Knowledgebases, Self-Healing OpenWiki & the Doc Studio](../tutorials/tut-knowledgebase-and-openwiki.mdx)
- [Tutorial: Deep Research, Knowledge Preflight & Scientia Publication](../tutorials/tut-deep-research-and-scientia.mdx)

---

**Related Reference**:
- [MCP Protocol SSOT](../reference/secrets-ssot.md)
- [Agentic Loop Blueprint](../explanation/why-vox-for-ai.md)
- [CLI Reference: vox mcp](../reference/cli.md)


