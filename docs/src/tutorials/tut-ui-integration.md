---
title: "Tutorial: Building UI with Islands"
description: "Learn how to build modern, reactive user interfaces with Vox using islands."
category: "tutorials"
status: "current"
last_updated: "2026-04-06"
training_eligible: true
---
# Tutorial: Building UI with Islands

Learn how to build modern, reactive user interfaces with Vox. This tutorial covers the `@island` decorator, JSX-like syntax, and binding UI state to backend logic.

> [!NOTE]
> The `@island` decorator was updated in v0.3 to use standard brace syntax and return arrows (`->`). 

## 1. The `@island` Decorator

Vox interactive UI components are defined with the `@island` decorator. They look and feel like React components but are compiled and hydrated for maximum performance.

```vox
// vox:skip
@island
fn Profile(name: str, bio: str) -> Element {
    <div class="p-6 bg-white shadow rounded-lg">
        <h2 class="text-xl font-bold">{name}</h2>
        <p class="text-gray-600">{bio}</p>
    </div>
}
```

## 2. Server vs. Client

You can mix lightweight server-rendered HTML routes with rich client-side islands. 

```vox
// vox:skip
http get "/profile" -> Element {
    // This renders purely on the server
    <html>
        <body>
            <h1>"User Profile"</h1>
            // The island mounts on the client
            <Profile name="Alice" bio="Developer" />
        </body>
    </html>
}
```

## 3. JSX in Vox

Vox supports a JSX-like syntax directly in `.vox` files. You can embed variables using braces, map over collections, and conditionally render elements.

```vox
// vox:skip
@island
fn UserList(users: list[str]) -> Element {
    <ul class="divide-y">
        {users.map(fn(user) {
            <li class="py-2">{user}</li>
        })}
    </ul>
}
```

## 4. Binding to Backend Logic

The true power of Vox lies in its technical unification. You can call `@mutation` or `@server fn` functions directly from your UI event handlers. Use standard React-like `onChange` or `onClick` attributes.

```vox
{{#include ../../../examples/golden/getting_started.vox:ui}}
```

## 5. Routing

You map a route to your island or server handler through the global `routes { }` block.

```vox
// vox:skip
routes {
    "/" -> NewsletterForm
}
```

---

**Next Steps**:
- [Language Syntax](../reference/ref-syntax.md) — Detailed JSX specification.
- [First App](tut-first-app.md) — Apply these UI patterns to a collaborative task list.
