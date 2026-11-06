---
title: "Journey: Building Resilient AI Agents"
description: "How to use Vox's native MCP integration, vector memory, and type safety to build reliable, stateful AI agents without ductile LLM orchestration layers."
category: "Getting Started"
sort_order: 1

schema_type: "HowTo"
keywords: ["build resilient AI agents", "durable agentic workflows", "LLM programming guide", "MCP agent tools"]
---

# Journey: Building Resilient AI Agents

## The Broken Reality of Orchestrating LLMs

Building an intelligent AI agent generally involves duct-taping language models to your application state. This requires writing brittle Python scripts or complex TypeScript orchestrators like Langchain.

As soon as your agent needs to execute a tool reliably, parse JSON tool-call responses, retry failures, and maintain a stateful memory of the interaction, the infrastructure complexity explodes. LLMs hallucinate arguments, drop nested fields, and break your application logic.

## The Vox Paradigm: Built-In, Type-Safe Orchestration

Vox was explicitly designed as an AI-native programming language. You do not need an external orchestration library to build an agent, because Vox natively generates Model Context Protocol (MCP) tool schemas and natively coordinates stateful LLM queries.

In Vox, the chaos of generative models is bounded by the compiler's zero-null guarantees (`Result` and `Option`). You define the rigid boundaries; Vox handles the plumbing.

## Core Snippet: Creating an Agent Tool

By declaring a bare `tool`, Vox parses the docstring, the types, and the return structure, turning your function into a ready-to-execute schema for your LLM.

```vox
type SearchResult =
    | Found(text: str, score: int)
    | NotFound(query: str)

tool "search_knowledge: Search the knowledge base for documents matching the query" search_knowledge(query: str, max_results: int) to SearchResult {
    if max_results <= 0 {
        return NotFound(query)
    }
    return Found("Document matching " + query, 95)
}

server get_answer(user_question: str) to Result[str] {
    let hit = search_knowledge(user_question, 5)
    return match hit {
        Found(text, _score) => Ok(text)
        NotFound(q) => Error("No documents found for: " + q)
    }
}
```

## Running the Process

1. Save the above snippet into an entrypoint like `src/agent.vox`.
2. Compile and run:

   ```bash
   vox build src/agent.vox
   vox run src/agent.vox
   ```

3. Vox will start the development server. The endpoints become immediately queryable, and if running in MCP mode, your agent tools are automatically broadcasted for discovery.

## Maturity and limitations

- **Maturity:** `beta` for `tool`-shaped examples — compiler and MCP registry paths evolve; treat snippets as orientation, not a guarantee every field matches shipped schemas.
- **Limitation ids:** [L-001](../../../contracts/journeys/limitations.v1.yaml) (docs may oversell partial MCP surfaces), [L-023](../../../contracts/journeys/limitations.v1.yaml) (MCP tool registry parity is ongoing maintenance).

## Deep Dives

To truly scale out this pattern, see how Vox implements AI orchestration under the hood:

- **[How To: Build AI Agents & MCP Tools](../how-to/how-to-ai-agents.md)**: Explore more complex integration loops.
- **[MCP Exposure from the Vox Language](../archive/research-2026-q1/mcp-vox-language-exposure.md)**: SSOT explaining how decorators translate to the MCP JSON-Schema specification.
- **[Socrates Anti-Hallucination Protocol](../adr/005-socrates-anti-hallucination-ssot.md)**: How Vox evaluates and rejects incorrectly formed agent outputs before they hit your execution loop.
