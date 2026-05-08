# AGENTS — vox-mental-tracker

This directory is a **Vox-language** application. Automation lives in **`scripts/*.vox`** (`vox run ...`).

Platform SSOT remains in the main Vox repository — link to `docs/src/how-to/external-app-bootstrap.md` there for toolchain install.

Do **not** add cloud analytics, crash reporters, or passive network calls in product code — CI guards `dist/` for `fetch` / `XMLHttpRequest` / `WebSocket` (see `.github/workflows/ci.yml`).
