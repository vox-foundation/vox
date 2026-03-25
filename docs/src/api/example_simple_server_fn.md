---
title: "Example: Simple server function demo"
description: "Official documentation for Example: Simple server function demo for the Vox language. Detailed technical reference, architecture guides, "
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# Example: Simple server function demo

```vox
# Simple server function demo
# Shows @server generating both backend route and frontend client

type Greeting =
    | Hello(message: str)

@server fn greet(name: str) to Greeting:
    ret Hello("Welcome, " + name + "!")

@server fn add(a: int, b: int) to int:
    ret a + b

@component fn App() to Element:
    let result = use_state("")
    let handle_click = fn(_e):
        let greeting = greet("Alice")
        set_result(greeting)
    ret <div><button onClick={handle_click}>Greet</button><p>{result}</p></div>
```
