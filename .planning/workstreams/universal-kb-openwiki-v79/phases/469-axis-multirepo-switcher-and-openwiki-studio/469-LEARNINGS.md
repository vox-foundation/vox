# Phase 469 Learnings

1. **Test Mocks Must Respect Unswallowed Errors:** Removing silent `.catch(() => {})` fallbacks in shared canvases surfaces real `role="alert"` errors to operators and requires legacy sibling unit tests to route mock responses by `payload?.tool` instead of blanket stubs.
2. **Portal Auditing Under jsdom + Real Themes:** Explicitly opening Radix dialogs (`bottom-status-bar-repo-popover`) and multi-panel studio tabs (`openwiki-heal-studio`) inside multi-viewport × multi-theme Vitest convergence harnesses catches z-index, token, and native-control regressions early without Playwright browser launch overhead on CI/local loops.
