<div align="center">
  <img src="docs/src/assets/logo.png" alt="Vox Logo" width="200" height="auto" />
  <h1>Vox Programming Language</h1>
  <p>Your backend, frontend, and database. One file. One binary. Zero nulls.</p>
  <p><strong><a href="https://vox-lang.org">vox-lang.org</a></strong></p>
</div>

<img src="docs/src/assets/hero_bg.png" alt="Vox Hero Banner" width="100%" />

> *"Is it a fact — or have I dreamt it — that, by means of electricity, the world of matter has become a great nerve, vibrating thousands of miles in a breathless point of time? Rather, the round globe is a vast head, a brain, instinct with intelligence! Or, shall we say, it is itself a thought, nothing but thought, and no longer the substance which we deemed it!"*
>
> — Nathaniel Hawthorne, *The House of the Seven Gables* (1851)

---

Vox is a unified, full-stack programming language designed to bridge the gap between high-level AI intent and low-level system performance. By compiling directly to **Rust** for backend durability and **TypeScript** for frontend reactivity, Vox enables developers to write their entire application stack in a single, LLM-friendly syntax.

## Quick Start

Get your first Vox app running and deployed locally in under 5 minutes.

### 1. Install the CLI

Ensure you have Rust installed, then install the Vox compiler CLI directly:

```bash
cargo install --locked --path crates/vox-cli
```

### 2. Initialize a Project

Use the CLI to scaffold a new project with the default TanStack template:

```bash
vox init my-app && cd my-app
```

### 3. Run Your Application

Start the development server, which hot-reloads both your Rust backend and TypeScript frontend:

```bash
vox run src/main.vox
```

---

## The CLI

The `vox` binary is the entrypoint for compile, run, package, and diagnostics.

Start with this first-time flow:

```text
vox commands --recommended   List the most important starter commands
vox doctor                   Why: verify toolchain and env before coding
vox build <file>             Why: compile and inspect generated output
vox check <file>             Why: fast type validation without full build
vox run <file>               Why: execute app locally end-to-end
vox bundle <file>            Why: produce deployable binary output
```

For full command coverage (including feature-gated surfaces), use:

```text
vox commands --format json --include-nested
vox --help
```

Canonical command reference: [`docs/src/reference/cli.md`](docs/src/reference/cli.md).

---

## Documentation & Resources

Want to dig deeper? We maintain a strictly standardized set of docs at [vox-lang.org](https://vox-lang.org):

1. **[What is Vox?](docs/src/index.md)** — Project narrative, core tenets, and language overview.
2. **[Frequently Asked Questions (FAQ)](docs/src/explanation/faq.md)** — Start here for deep answers on architecture, scaling, null safety, and AI integration.
3. **[First Full-Stack App](docs/src/how-to/first-full-stack-app.md)** — Step-by-step tutorial.
4. **[Contributing](CONTRIBUTING.md)** — First-PR checklist and onboarding links.
5. **[Agent & secret policy](AGENTS.md)** — Clavis / secret SSOT (required for API keys).
6. **[Syntax Reference](examples/STYLE.md)** — The 0.8.0 syntax standard.

---

## License

Apache-2.0. Source: [github.com/vox-foundation/vox](https://github.com/vox-foundation/vox).

Sponsorship links (GitHub Sponsors, Open Collective) will appear here when live.
