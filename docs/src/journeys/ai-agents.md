---
title: "Journey: Building Resilient AI Agents"
description: "How to use Vox's native MCP integration, vector memory, and type safety to build reliable, stateful AI agents without ductile LLM orchestration layers."
category: "getting-started"
sort_order: 1
---

# Journey: Building Resilient AI Agents

## The Broken Reality of Orchestrating LLMs

Building an intelligent AI agent generally involves duct-taping language models to your application state. This requires writing brittle Python scripts or complex TypeScript orchestrators like Langchain. 

As soon as your agent needs to execute a tool reliably, parse JSON tool-call responses, retry failures, and maintain a stateful memory of the interaction, the infrastructure complexity explodes. LLMs hallucinate arguments, drop nested fields, and break your application logic.

## The Vox Paradigm: Built-In, Type-Safe Orchestration

Vox was explicitly designed as an AI-native programming language. You do not need an external orchestration library to build an agent, because Vox natively generates Model Context Protocol (MCP) tool schemas and natively coordinates stateful LLM queries.

In Vox, the chaos of generative models is bounded by the compiler's zero-null guarantees (`Result` and `Option`). You define the rigid boundaries; Vox handles the plumbing.

## Core Snippet: Creating an Agent Tool

By adding a single decorator—`@mcp.tool`— Vox parses the docstring, the types, and the return structure, turning your server function into a ready-to-execute schema for your LLM.

```vox
# Skip-Test: ui-only

> [!WARNING]
> This feature is partially implemented. The syntax below is accepted by the parser
> but runtime behavior may differ from what is described. Features like `vector_search` and `agent.query` are aspirational.

type SearchResult =
    | Found({text: str, score: int})
    | NotFound({query: str})

@mcp.tool "Search the knowledge base for documents matching the query"
fn search_knowledge(query: str, max_results: int) to SearchResult {
    let hits = db.vector_search(query, max_results)
    if hits.len() == 0 {
        ret NotFound({query: query})
    }
    ret Found({text: hits[0].text, score: hits[0].score})
}

@server 
fn get_answer(user_question: str) to Result[str] {
    // Vox triggers the AI provider, gives it access to `search_knowledge`,
    // and returns a strongly-typed string result or an explicitly handled error.
    let answer = agent.query(user_question, { tools: [search_knowledge] })
    ret Ok(answer)
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

## Deep Dives

To truly scale out this pattern, see how Vox implements AI orchestration under the hood:

- **[How To: Build AI Agents & MCP Tools](../how-to/how-to-ai-agents.md)**: Explore more complex integration loops.
- **[MCP Exposure from the Vox Language](../architecture/mcp-vox-language-exposure.md)**: SSOT explaining how decorators translate to the MCP JSON-Schema specification.
- **[Socrates Anti-Hallucination Protocol](../adr/005-socrates-anti-hallucination-ssot.md)**: How Vox evaluates and rejects incorrectly formed agent outputs before they hit your execution loop.
