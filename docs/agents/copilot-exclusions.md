# GitHub Copilot Content Exclusions

GitHub Copilot's content exclusion cannot be configured via a file in the repository. Exclusions must be set in the GitHub web UI at the organization or repository level.

**Path:** GitHub.com → Settings → Copilot → Content exclusion

**Configured paths (last reviewed: 2026-04-11):**

```
.env
.env.*
secrets/**
credentials/**
*.pem
*.key
*.p12
populi/runs/**
mens/runs/**
*.db
*.db-wal
*.db-shm
```

> ⚠️ This file is the human-maintained record of what is configured in the GitHub UI. It is NOT machine-enforced. When you add an exclusion above, also configure it in GitHub Settings.

## Sync with `.voxignore`

The patterns above should be a subset of `.voxignore` (the SSOT for all AI context exclusion). When `.voxignore` is updated with a new sensitive path, also review whether it should be added here and configured in the GitHub UI.

See: [`docs/src/architecture/multi-repo-context-isolation-research-2026.md`](../src/architecture/multi-repo-context-isolation-research-2026.md) §3
