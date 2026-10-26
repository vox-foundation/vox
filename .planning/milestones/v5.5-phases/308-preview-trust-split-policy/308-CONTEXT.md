# Phase 308 Context: Preview/Privileged Workflow Trust-Split Policy

**Requirements:** `T3-PREVIEW-01..04`

**Intent:** Author `docs/src/ci/privileged-preview-trust-split.md` capturing t3code relay pattern: deploy may run PR code only under environment approval; cleanup uses `pull_request_target` + default-branch checkout and never executes PR code.

**Sources:** see store `docs/t3-deeper-extraction-v5.5.md`.
