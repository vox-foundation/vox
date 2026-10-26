# Phase 310 Context: TOESTUB Test-Discipline Detectors (Wall-Clock, Real-Wait, Loop, Frame)

**Requirements:** `T3-TEST-DET-01..04`

**Intent:** Extend `test_clock_and_skip` (or new detector) to flag wall-clock reads in tests: `Instant::now`, `SystemTime::now`, `Date.now`, `performance.now` (t3code `no-wall-clock-in-tests`).

**Sources:** see store `docs/t3-deeper-extraction-v5.5.md`.
