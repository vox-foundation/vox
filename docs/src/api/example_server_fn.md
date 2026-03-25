---
title: "Example: Server function demo"
description: "Official documentation for Example: Server function demo for the Vox language. Detailed technical reference, architecture guides, and imp"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# Example: Server function demo

```vox
# Server function demo
# Shows how @server generates both API route + typed fetch wrapper

type Greeting =
    | Hello(message: str)

@server fn greet(name: str) to Greeting:
    ret Hello("Welcome, " + name + "!")

@server fn add(a: int, b: int) to int:
    ret a + b
```
