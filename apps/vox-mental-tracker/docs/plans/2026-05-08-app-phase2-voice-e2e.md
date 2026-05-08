# App Phase 2 — Voice E2E (parser/confirm/edit/save) — Implementation Plan

> **For agentic workers:** Use superpowers:test-driven-development per task; superpowers:verification-before-completion before claiming any task done.

**Goal:** Make the voice flow on `/voice` an authoritative single loop: transcribe → parse (real extraction) → confirm/edit → save (with raw_transcript persistence linked to the event).

**Why:** Today `VoicePage` shows a parsed JSON preview but offers no save action. `preview_voice_parse` returns hardcoded payloads instead of extracting values. The TS-side `intent_parser.ts` already does real extraction; bring Vox to parity using `std.regex` (now landed) and tie the parsed output into `record_raw_transcript` + `record_event` so corrections (`record_correction`) work end-to-end.

**Architecture:**
- Replace string-substring classification in `preview_voice_parse` with regex-based extraction matching `intent_parser.ts` semantics.
- Introduce `type ParsedVoice { kind: str, payload_json: str, confidence: float }` (now possible — struct types landed in #73).
- Add `parse_voice(transcript) to ParsedVoice` returning the structured result; keep `preview_voice_parse` as a thin string wrapper that re-stringifies for backward compat.
- Wire `VoicePage` Save action: call `record_raw_transcript(transcript, parser_decisions_json, confidence)` → on Ok(tid), call `record_event(kind, payload_json, now_ms, "voice", tid, "", tz, offset)`.
- Add an Edit step: per-field state vars the user can tweak before save (mood score override, meal description override, exercise duration override).
- Test parity via shared fixtures in `tests/fixtures/parser_cases.json`.

**Tech Stack:** Vox (struct types, `std.regex`, `std.json` from landed plans), TypeScript (vitest fixtures), Playwright for the UI flow.

**Out of scope:**
- Sherpa-onnx replacement of SpeechRecognizer (Phase 3 — blocked on PR #68).
- Multi-turn dialog ("yes/no" confirmation via voice).
- Time-of-day extraction from utterances ("this morning") — naive `now_ms()` for now.

---

## Tasks

### Task A — Vox parser parity (regex)

- [ ] **A1.** In `apps/vox-mental-tracker/src/main.vox`, define `type ParsedVoice { kind: str, payload_json: str, confidence: float }`.
- [ ] **A2.** Add `@endpoint(kind: query) fn parse_voice(transcript: str) to ParsedVoice` mirroring `intent_parser.ts` rules:
  - mood: regex `(?:mood|feeling).*?(\d)`; clamp 1-5; payload `{"mood_score":N}`; conf 0.75
  - meal: regex `(?:ate|eating|had)\s+(.+)`; payload `{"description":"…"}`; conf 0.65
  - exercise: regex `(?:ran|run|jogged|walked)\s+(?:for\s+)?(\d+)\s*(?:min|minutes)`; payload `{"activity":"cardio","duration_minutes":N}`; conf 0.7
  - sleep: substring on `bed|sleep|slept|nap`; payload `{}`; conf 0.55
  - default: `note_recorded` with full transcript as body; conf 0.4
- [ ] **A3.** Refactor `preview_voice_parse(transcript) to str` to call `parse_voice(transcript)` and emit `{"kind":...,"payload":<payload>,"confidence":...}` for the existing UI consumer.
- [ ] **A4.** Run `vox check apps/vox-mental-tracker/src/main.vox` — must pass.

### Task B — Shared fixture suite

- [ ] **B1.** Read existing `tests/fixtures/parser_cases.json`. Each entry has `{transcript, expected_kind, expected_payload?, expected_confidence?}`.
- [ ] **B2.** Add cases covering each of the 5 kinds, plus tricky variants (mood "I feel like a 4 today", meal "had pasta for lunch", exercise "ran for 30 minutes", sleep "going to bed", note "remember to call mom"). Aim for at least 10 cases.
- [ ] **B3.** Update `tests/intent_parser.test.ts` to drive every fixture case through `parseIntent` and assert kind + payload.
- [ ] **B4.** Add a TS-side parity check that simulates the Vox parser (or just run `vox` to emit and compare via integration step in B6).
- [ ] **B5.** Run `pnpm test` — must pass.
- [ ] **B6.** Optional: emit `parse_voice` results from the compiled Vox endpoint and assert match in a `parser_parity.test.ts`. Skip if too much wiring.

### Task C — VoicePage UI: confirm + edit + save

In `apps/vox-mental-tracker/src/main.vox`, replace the `VoicePage` component body:

- [ ] **C1.** State vars:
  - `transcript_raw: str = ""`
  - `parsed_kind: str = ""`
  - `parsed_payload: str = ""`
  - `parsed_confidence: float = 0.0`
  - `edit_mood_override: int = 0` (0 means no override)
  - `edit_meal_description: str = ""` (empty means use parsed)
  - `edit_exercise_minutes: int = 0` (0 means use parsed)
  - `status: str = "Tap Transcribe to start."`
  - `last_saved_event_id: str = ""`

- [ ] **C2.** Buttons:
  1. **Transcribe** — calls `Speech.transcribe_microphone()`; on Ok sets `transcript_raw` and clears parsed/edit vars; on Error sets status.
  2. **Parse** — calls `parse_voice(transcript_raw)` and assigns the three parsed_* vars; status becomes "Confirm or edit before saving".
  3. **Save** — see C3 logic.
  4. **Reset** — clears everything to initial state.

- [ ] **C3.** Save handler logic (use match-arm statement bodies + std.json — both landed):
  ```
  let payload = effective_payload(parsed_kind, parsed_payload, edit_mood_override, edit_meal_description, edit_exercise_minutes)
  match record_raw_transcript(transcript_raw, parsed_payload, parsed_confidence) {
      Ok(tid) => match record_event(parsed_kind, payload, str(std.time.now_ms()), "voice", tid, "", "UTC", 0) {
          Ok(eid) => {
              last_saved_event_id = eid
              status = "Saved as " + parsed_kind + " (id " + eid + ")"
          }
          Error(e) => status = "save failed: " + e
      }
      Error(e) => status = "transcript persist failed: " + e
  }
  ```
- [ ] **C4.** Add a private fn `effective_payload(kind, parsed_payload_json, mood_override, meal_override, exercise_override) to str` that uses `std.json.parse` to read the parsed payload and applies any non-default override before re-stringifying.
- [ ] **C5.** Render edit fields conditionally on `parsed_kind` (mood number input, meal text input, exercise minutes input).
- [ ] **C6.** Display the parsed JSON, confidence, last saved event id.
- [ ] **C7.** `vox check` passes.

### Task D — Playwright E2E

- [ ] **D1.** Add `tests/e2e/voice_flow.spec.ts`:
  1. Navigate to `/voice`.
  2. Mock or stub `Speech.transcribe_microphone` to return "I feel like a 3" (for browser lane — native STT not available).
  3. Click Parse; assert kind == "mood_recorded" and payload contains `mood_score":3`.
  4. Click Save; assert status shows the saved event id.
  5. Navigate to `/timeline`; assert the count incremented.
- [ ] **D2.** `pnpm exec playwright install` (one-time setup) then `pnpm e2e` — must pass.

### Task E — Verification

- [ ] **E1.** `vox check apps/vox-mental-tracker/src/main.vox` — passes with 0 errors.
- [ ] **E2.** `pnpm test` (vitest) — all tests pass.
- [ ] **E3.** `pnpm e2e` — all Playwright tests pass.
- [ ] **E4.** Manual: open a dev server (`pnpm dev`), exercise the full flow on `/voice`, check that `/timeline` reflects the new event.
