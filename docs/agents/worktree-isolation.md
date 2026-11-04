---
title: "Worktree isolation for agents"
description: "How each git worktree gets an isolated Vox home/config marker so agents cannot clobber the primary install."
category: "contributor"
status: "current"
---

# Worktree isolation for agents

## Why

Parallel agents sharing one userdata/home corrupt chat history, secrets resolution, and daemon sockets. Each `git worktree` must use its own stamped setup.

## Marker

After creating a worktree, run:

```bash
vox run scripts/worktree-setup.vox
```

This writes `.vox/worktree-setup.marker` (gitignored locally under the worktree) containing the worktree path hash and timestamp. `vox doctor` can report missing markers.

## Isolation rules

- Prefer worktree-local `.vox/` for cache/spool; never point `VOX_HOME` / userdata at the primary checkout's live home while developing in a secondary worktree.
- One `target/` per worktree (already enforced by `.cargo/config.toml`) — never symlink `target` across worktrees.
- GUI: build the sidecar once per worktree (`vox run scripts/gui-build.vox`) before `cargo build -p vox-gui`.

## Related

- Kill-safety triad: [kill-safety-and-hit-every-surface.md](kill-safety-and-hit-every-surface.md)
- Perennial bug: `vox-gui` sidecar missing in a fresh worktree (`AGENTS.md`)
