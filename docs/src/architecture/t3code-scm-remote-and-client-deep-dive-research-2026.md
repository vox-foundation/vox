---
title: "T3 Code 6-Forge SCM, SSH Bootstrap, CloudLink Relay & Client/Desktop Deep-Dive (2026)"
description: "Source-anchored architecture of pingdotgg/t3code across all 6 source-control forge drivers, PR reactors, shared client-runtime PR helpers, zero-Node SSH archive bootstrap, Ed25519 CloudLink relay, 6-tier route racing, DesktopSnapShot, BrowserImport, and @pierre/highlights."
category: "Architecture SSOTs"
status: "research"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# T3 Code 6-Forge SCM, SSH Bootstrap, CloudLink Relay & Client/Desktop Deep-Dive (2026)

<!-- kb:summary -->
This document provides the file-and-symbol-anchored engineering reference for `pingdotgg/t3code` (`~/dev/t3code`, commit `80455376f01f19c10a06dc08b0629486c017bf79`) across its 6 source-control forge packages (`github`, `gitlab`, `gitcafe`, `forgejo`, `bitbucket`, `azure-devops`), `GitVcsDriverCore.ts` (4,565 LoC), background PR reactors, `packages/client-runtime` PR helpers, `packages/ssh` zero-Node remote archive bootstrap, `CloudLink.ts` Ed25519 relay, 6-tier staggered route racing, `DesktopSnapShot` (macOS, Windows, 6 Linux Wayland/X11 backends), `BrowserImport` cookie decryption, and `@pierre/highlights` WASM syntax highlighting.
<!-- /kb:summary -->

## 1. 6-Forge Source Control Architecture (`packages/source-control-*`)

### 1.1 Core Contracts & Discovery (`packages/source-control-core` & `packages/source-control-definitions`)
- **`SourceControlDefinition` (`packages/source-control-core/src/shared/definition.ts:201-304`):** Defines `kind: SourceControlProviderKind`, `publicHost: string | null`, `changeRequest: ChangeRequestTerminology`, `changeRequestActions: ReadonlySet<PullRequestAction>`, `recognizeRemote`, `parseChangeRequestUrl`, `mediaUrl`, `credentials`, and `settings`.
- **`PullRequestCapabilities` (`packages/contracts/src/pullRequest.ts:259-285`):** Attached to `PullRequestProviderApi.capabilities` with fields `diff`, `comment`, `actions`, `mergeMethods`, `deleteBranch`, `updateMethods`, `search`, `reactions`, `viewedFiles`, `review: { inlineComment, rangeComment, reply, resolve, verdicts }`, `reviewers: { request, listCandidates }`, `edit: { changeRequest, comment }`, `stacks`, `stackActions`, `labels`, `checksAlwaysReport`.
- **`PullRequestProviderApi` (`packages/source-control-core/src/server/PullRequestProvider.ts:264-547` — 35 methods):** `withVerifiedCredential?`, `getRoutingIdentity?`, `getViewer`, `listChangeRequests`, `listChangeRequestsAcross?`, `listChangeRequestStats?`, `getChangeRequestChecks?`, `getChangeRequest`, `getChangeRequestPreview?`, `getChangeRequestSummary?`, `getChangeRequestWatchFingerprint?`, `getChangeRequestStack?`, `getChangeRequestActivity`, `getReviewThreadComments?`, `getViewerPermissions`, `getDiff`, `getDiffFileContents?`, `getFilesViewed?`, `getFileStats?`, `setFilesViewed?`, `getFileRevisions?`, `runAction`, `updateChangeRequest?`, `comment`, `updateComment?`, `submitReview`, `listReviewerCandidates`, `setReviewerRequest`, `listLabelCandidates?`, `listRepositoryLabels?`, `searchRepositoryAuthors?`, `setLabels?`, `replyToThread`, `setReaction`, `setThreadResolution`.
- **`discovery.ts` (`packages/source-control-core/src/server/discovery.ts:77`) & `registry.ts` (`packages/source-control-definitions/src/registry.ts:16-35`):**
  - `BUILT_IN_SOURCE_CONTROL_DRIVER_LIST` order: `["github", "gitlab", "gitcafe", "forgejo", "bitbucket", "azure-devops"]`.
  - `recognizeRemotesIn` order: `["gitcafe", "forgejo", "github", "gitlab", "azure-devops", "bitbucket"]`; `claimChangeRequestUrlsFirst`: `["gitcafe"]`.
  - `DEFAULT_PROBE_TIMEOUT_MS = 5_000` (5s); `SourceControlProviderAuth["status"]` values: `"authenticated" | "unauthenticated" | "unknown"`.

### 1.2 Per-Forge Implementation Matrix

| Forge Package | Key Modules & Mechanics |
|---|---|
| `packages/source-control-github` | `GitHubSourceControlProvider.ts:376-381,506-515` (`HEAD_LOOKUPS_PER_DOCUMENT = 50`, `HEAD_LOOKUP_BATCH_WINDOW = "50 millis"`, `BACKGROUND_HEAD_LOOKUPS_PER_DOCUMENT = 25`, `BACKGROUND_HEAD_LOOKUP_BATCH_WINDOW = "500 millis"`), `GitHubPullRequestApi.ts:390-392,1640-1706` (`STAT_ALIASES_PER_REQUEST = 25`, `SUMMARY_BATCH_WINDOW = "10 millis"`), `GitHubCredentials.ts:26-29,108-112` (`GH_PROCESS_CONCURRENCY = { key: "gh", limit: 4 }`, `TOKEN_TTL = Duration.minutes(5)`, `MISSING_TTL = Duration.seconds(10)`, `GH_ENTERPRISE_TOKEN` / `GITHUB_ENTERPRISE_TOKEN` when `env.GH_HOST === host`), `gitHubGraphiteStack.ts` (Graphite comment parser + branch parent chain builder). |
| `packages/source-control-gitlab` | `GitLabPullRequestProvider.ts` (REST v4 `/api/v4/projects/:id/merge_requests`), `GitLabCli.ts` (`glab auth status` + `~/.config/glab-cli/config.yml`), `gitlabDiffPosition.ts` (maps unified diff line ranges to `{ base_sha, start_sha, head_sha, old_line, new_line, line_range }`). |
| `packages/source-control-bitbucket` | `BitbucketPullRequestProvider.ts` + `BitbucketPullRequestApi.ts:25,517` (REST 2.0; imports `parseDiffFileRevisions` from `packages/source-control-core/src/server/diffFileRevisions.ts:113`), `BitbucketCredentials.ts` (`BITBUCKET_TOKEN` Bearer or `BITBUCKET_USERNAME` + `BITBUCKET_APP_PASSWORD` Basic auth). |
| `packages/source-control-azure-devops` | `AzureDevOpsPullRequestProvider.ts:39-42` (REST v7.1; bounds concurrent Git blob fetches via `DIFF_FILE_CONCURRENCY = 4` and `MAX_DIFF_SPAWNS = 8`), `azureDevOpsDiff.ts` (`buildUnifiedFileDiff`, `buildUnifiedPatch`), `AzureDevOpsIdentity.ts`. |
| `packages/source-control-forgejo` | `ForgejoPullRequestProvider.ts` (Gitea/Forgejo/Codeberg `/api/v1` REST), `ForgejoCli.ts:66-101` (`forgejoKeysPaths` reads `$XDG_DATA_HOME/forgejo-cli/keys.json` or `~/.local/share/forgejo-cli/keys.json` on Linux, `~/Library/Application Support/{forgejo-cli,Cyborus}.forgejo-cli/keys.json` on macOS, and `%APPDATA%\{forgejo-cli,Cyborus}\forgejo-cli\data\keys.json` on Windows). |
| `packages/source-control-gitcafe` | `GitCafePullRequestProvider.ts` (`https://git.cafe/api/gitcafe` REST API, stack reconstruction via `parentPullRequestNumber` / `childPullRequestNumbers`), `definition.ts:109` (`canonicalizeGitCafePullRequestUrl` normalizes `/-\/pull/<n>` to `/pull/<n>`). |

---

## 2. `GitVcsDriverCore.ts`, PR Reactors & Shared `client-runtime` PR Helpers

### 2.1 `GitVcsDriverCore.ts` (`apps/server/src/vcs/GitVcsDriverCore.ts` — 4,565 LoC)
- **Worktree Creation (`createWorktree`):** Runs `git worktree add`, symlinks/copies untracked `.env*` files from the root repo (`copyWorktreeEnvFiles`), and runs project setup scripts.
- **Restore Points (`T3CODE_RESTORE_POINT_PREFIX = "t3code-restore-point"`):** Creates non-destructive git stash commits (`git stash create` + `git stash store`) before destructive branch resets or checkouts.
- **Hidden Git Checkpoints (`VcsCheckpointOps`):** Stores turn snapshots under `refs/t3/checkpoints/` using `git write-tree` and `git commit-tree` with an isolated temporary `GIT_INDEX_FILE`.

### 2.2 Background PR Reactors (`PullRequestSyncReactor.ts` & `PullRequestWatchReactor.ts`)
- **`PullRequestSyncReactor.ts:36-38,276-346,553`:**
  - Runs on `Schedule.spaced("1 minute")` with adaptive per-thread cadence (`QUIET_AFTER_MS = 30 * 60 * 1_000`, `QUIET_SYNC_INTERVAL_MS = 5 * 60 * 1_000`, `SLOW_SYNC_INTERVAL_MS = 15 * 60 * 1_000`).
  - `syncStackPullRequestsForThread` walks `fetchedStack?.stack?.layers`, dispatches `thread.pull-request.link` with `source: "stack"`, and respects tombstones in `thread.pullRequests` (`unlinkedAt: IsoDateTime | null`, filtered via `visibleThreadPullRequests`).
- **`PullRequestWatchReactor.ts:295-367` & `pullRequestWatch.ts:11-13`:**
  - `readRemarks` paginates `pullRequests.threadComments` until `cursor === null` with a cycle guard (`cursors.has(cursor)`).
  - Enforces `PULL_REQUEST_WATCH_WAKE_LIMIT = 10` (consecutive comment-only wakes before auto-disabling `watchRemarks`) and `PULL_REQUEST_WATCH_NO_CHECKS_MS = 15 * 60_000` (grace period before clearing `watchChecks` when a PR reports zero checks and `!capabilities.checksAlwaysReport`).

### 2.3 Shared Client-Runtime PR Helpers (`packages/client-runtime/src/pullRequest*.ts`)
- **`pullRequestEditing.ts:17-44`:** `canEditPullRequestChangeRequest`, `canEditPullRequestComment` (`"issue-comment" | "review-comment"`), and `canResolvePullRequestThread`.
- **`pullRequestReactions.ts:3-97`:** `PULL_REQUEST_REACTION_ORDER` (`thumbs-up`, `thumbs-down`, `laugh`, `hooray`, `confused`, `heart`, `rocket`, `eyes`), `pullRequestReactionTooltip` (`NAMED_ACTOR_LIMIT = 3`), and optimistic overlay `applyPendingPullRequestReactions`.
- **`pullRequestChecks.ts:3-30`:** `resolvePullRequestDetailChecks` (compares `observedAt`, preserving `"merged"`) and `pullRequestChecksRefreshIntervalMs` (`45_000` ms when pending/empty, else `60_000` ms).
- **`pullRequestLinkInput.ts:8-123`:** `parsePullRequestReference`, `pullRequestLinkProject`, and `resolveLinkPullRequestInput`.

---

## 3. SSH Zero-Node Bootstrap, CloudLink Relay & 6-Tier Route Racing

### 3.1 Zero-Node SSH Remote Bootstrap (`packages/ssh/src/RemoteBootstrap.ts`)
- Inspects remote OS/arch via `uname -s -m` over SSH (`ControlMaster=auto`, `ControlPersist=60s`).
- Streams the platform-matched standalone server archive (`t3-server-<os>-<arch>.tar.gz` bundling the runtime + `t3` entrypoint) to `~/.t3/remote-server/<version>/`, extracts it, starts `t3 serve --port 0 --print-connect-json`, and forwards the remote port to `127.0.0.1:<localPort>`.

### 3.2 CloudLink Ed25519 Relay (`apps/server/src/cloud/CloudLink.ts` & `config.ts`)
- Connects outbound to `wss://api.t3.codes/ws/environment`, signs the relay challenge with the host's Ed25519 private key, and verifies relay-minted client access tokens against `CLOUD_MINT_PUBLIC_KEY` (`apps/server/src/cloud/config.ts`, imported at `CloudLink.ts:86`).

### 3.3 6-Tier Staggered Route Racing (`packages/client-runtime/src/connection/supervisor.ts:51-52`)
- Races candidate routes with a `150ms` staggered head start in priority order: `local` (IPC/loopback) → `lan` → `tailscale` → `ssh` → `cloud` → `public`.
- Background probe (`BETTER_ROUTE_CHECK_INTERVAL = "60 seconds"`, `BETTER_ROUTE_COOLDOWN_MS = 300_000`) hot-swaps active WebSocket sessions from `cloud` to `tailscale`/`lan` when a lower-latency route becomes reachable.
- **Server PTY Manager (`apps/server/src/terminal/Manager.ts:95-104`):** Maintains `DEFAULT_MAX_RETAINED_INACTIVE_SESSIONS = 128`, `DEFAULT_HISTORY_LINE_LIMIT = 5_000`, and `DEFAULT_HISTORY_BYTE_LIMIT = 8 * 1024 * 1024`.

---

## 4. Desktop Native Helpers & Web Client Micro-Engines

1. **Cross-Platform Screenshot & Region Picker (`apps/desktop/src/snapshot/`):** macOS (`screencapture -i`), Windows (`DesktopSnapShot.ps1` DPI-aware WinForms overlay), and 6 Linux Wayland/X11 backends (`hyprshot`, `grim` + `slurp`, `spectacle`, `gnome-screenshot`, `scrot`, `import`).
2. **Multi-Browser Cookie Decryption (`apps/desktop/src/preview/BrowserImport/`):** Imports cookies into the preview browser from Chromium (`AES-128-CBC` / `AES-256-GCM` via OS keychain / DPAPI), Firefox (`cookies.sqlite` + `recovery.jsonlz4` `mozLz4`), and Safari (`Cookies.binarycookies` big-endian binary parser).
3. **WASM Syntax Highlighting & Tiptap Code-Block Engine (`apps/web/src/`):**
   - `apps/web/src/lib/syntaxHighlighting.ts:15-36` sets `PREFERRED_HIGHLIGHTER: HighlighterTypes = "highlights"` via `@pierre/diffs` (`getSharedHighlighter`), replacing Shiki with `@pierre/highlights` WASM lexers (commit `d228f1f2`).
   - `apps/web/src/components/composerCodeBlockHighlight.ts:18-22` bounds live ProseMirror highlighting via `MAX_HIGHLIGHTED_CODE_BLOCK_LENGTH = 20_000`, `DEFERRED_HIGHLIGHT_CODE_BLOCK_LENGTH = 2_000`, and `DEFERRED_HIGHLIGHT_DELAY_MS = 300`.
   - `apps/web/src/composer-code-block.ts:28-190` implements `indentedNewlineInCodeBlock`, `exitCodeBlockOnTrailingBlankLines`, `exitCodeBlockOnClosingFence`, and `indentCodeBlock`.
