---
title: "Acta — App Architecture (2026)"
description: "Architecture for Acta, the first standalone external Vox project: a mobile-first, offline, voice-first universal logbook. One button records and timestamps anything; reminders cover the future; recordings cover the past. Built on the vox-mobile plugin (cdylib + host-shell FFI), vox-oratio (on-device STT + post-processing pipeline including intent extraction), Codex with bundled-sqlcipher (encrypted at rest), and Clavis-resolved keys (Android Keystore / iOS Keychain / Argon2id passphrase)."
category: "architecture"
status: "roadmap"
training_eligible: true
training_rationale: "Canonical architecture for Acta — Vox's first external project. Demonstrates the @table + @endpoint + @component + Vox.toml stack on mobile, with on-device oratio post-processing and intent extraction as a featured pattern."
---

# Acta — App Architecture (2026)

## Premise

**Acta** (Latin: *acta* — "things done; records; deeds") is a personal, offline-first, voice-first logbook for mobile. The bedrock loop is:

> **Hold one button, speak, release. The transcript is timestamped, classified, and saved to a permanent local log. Reminders are first-class entries that haven't fired yet.**

Past and future live in the same table. An "I went for a walk" entry and a "remind me to take meds at 9" entry are the same shape with different `fire_at_utc` and `transcript` semantics; the distinction is one boolean field. This unification is the architectural premise — most logging apps separate "journal" and "to-do" / "reminders," and that separation is what makes them feel like work.

Acta is **not a mental-health tracker, not a journal app, not a to-do app.** It is a *universal log*. Mental-health, expense tracking, workout journals, project notes — all are **presets** that ride on top of the universal layer (Phase 2), not the core.

This is the **first standalone external Vox project**. It lives in its own repository (`github.com/<user>/acta` — separate from the Vox monorepo) and consumes Vox as a published toolchain. Every architectural choice is biased toward exercising the Vox feature set realistically and surfacing rough edges that a real external project would hit.

Acta depends on the platform additions in [Vox Mobile Plugin Spec (2026)](vox-mobile-plugin-spec-2026.md) — specifically the `vox-mobile` plugin, `cargo-ndk` / `cargo-lipo` cdylib build target, host-shell FFI contract, mobile Clavis sources, Codex `bundled-sqlcipher`, and the table-based reminder runtime. It does **not** depend on any new Vox grammar.

## Non-goals

- **Not a productivity suite.** No projects, no kanban, no time-blocking. One log, one big button, reminders, search, export. That is the entire shape.
- **Not a sync product.** v1 is single-device. The data model and ULID-keyed schema are sync-ready (multi-device merge is a clean follow-on), but no sync server, no cloud, no account.
- **Not a clinical tool.** The mental-health preset is one optional overlay among many; Acta makes no clinical claims and surfaces no clinical scoring (PHQ-9 etc.) by default.
- **Not a transcription product.** Transcription is a means to the timestamped log entry; Acta is not a Whisper UI or a podcast-transcribe tool. The button-press → entry loop is the product.

## Core architectural premises

### 1. Every entry is a `LogEntry`

A single table holds past records and future reminders:

```vox
// vox:skip
@table type LogEntry {
    id:                ulid,
    created_at_utc:    datetime,    // when the row was written
    fire_at_utc:       Option<datetime>,  // None = past entry; Some = future reminder
    fired_at_utc:      Option<datetime>,  // when the alarm actually fired (for reminders)
    local_tz:          str,         // IANA name at write time
    source:            EntrySource, // voice | text | reminder_template
    transcript:        str,         // the words; for reminders, the title/body
    audio_path:        Option<str>, // PCM file, encrypted; nullable after retention
    duration_ms:       Option<int>,
    recurrence:        Option<str>, // RFC 5545 RRULE; reminders only
    attributes:        json,        // user-defined / preset structured fields
    tags:              Vec<str>,    // denormalized; canonical in EntryTag
    payload_json:      Option<str>, // intent-extracted structured data
    active:            bool,        // false = soft-deleted or expired
}
```

This is *intentionally* one wide table. Splitting "entry" and "reminder" into separate tables is the most common temptation when designing this kind of app and the most common mistake — it turns "show me everything that happened or will happen this week" into a UNION query and forces every UI surface to remember which table it's in. One table, one query, one mental model.

The reminder runtime described in [vox-mobile-plugin-spec §Phase 5](vox-mobile-plugin-spec-2026.md#phase-5--reminder-runtime-vox-stdlibreminder) is configured via `Vox.toml` to watch `LogEntry` rows where `fire_at_utc IS NOT NULL AND fired_at_utc IS NULL AND active = true`.

### 2. Universal offline-resilient timestamps

Every `LogEntry` carries three pieces that together guarantee correct interpretation on any device, in any timezone, after any duration of offline operation, after any export:

| Field | Format | Why |
|---|---|---|
| `id` (ULID) | 128-bit, 26 chars Crockford base32 | First 48 bits = ms since epoch → lexicographic sort = chronological sort. Generated on-device, no central counter, monotonic per ms. Two devices writing offline never collide ([ULID spec](https://github.com/ulid/spec)). |
| `created_at_utc` | RFC 3339 / ISO 8601 in UTC | Ground truth for ordering and queries. Never displayed raw. |
| `local_tz` | IANA tz name (e.g. `"America/Los_Angeles"`) | Lets the entry be re-displayed correctly years later, even if the user has moved. Matches the recommendation in the [external frontend interop wire-format SSOT](wire-format-v1-ssot.md). |

Display formatting always derives from these three. There is no "last-modified" timestamp on the device-side schema — edits are append-only (a corrected transcript creates a new `LogEntry` linked back via `attributes.corrects = <id>`), preserving the audit trail.

### 3. Voice-first means oratio post-processing, not just STT

Acta uses [vox-oratio](../../../crates/vox-oratio/Cargo.toml) for on-device transcription, but the value is in the **post-processing pipeline** — getting from raw Whisper output to a useful structured log entry. The pipeline (in dispatch order) is:

1. **[`acoustic_preprocess`](../../../crates/vox-oratio/src/acoustic_preprocess.rs)** — denoise + normalize the captured PCM before it hits the model. Improves accuracy on phone microphones.
2. **[`vad/`](../../../crates/vox-oratio/src/vad/)** — voice activity detection trims leading/trailing silence and skips dead air mid-recording. Faster STT, cleaner transcripts.
3. **[`backends/sherpa_onnx`](../../../crates/vox-oratio/src/backends/sherpa_onnx.rs)** — Whisper-Tiny.en via sherpa-onnx (51× faster than whisper.cpp on Android per the [VoicePing 2026 benchmark](https://voiceping.net/en/blog/research-offline-speech-transcription-benchmark/)). Returns top-N hypotheses, not just the best one.
4. **[`transcript_rerank`](../../../crates/vox-oratio/src/transcript_rerank.rs)** — picks the best hypothesis, optionally Vox-compiler-aware (irrelevant for Acta but free).
5. **[`speech_lexicon`](../../../crates/vox-oratio/src/speech_lexicon.rs) + [`contextual_bias`](../../../crates/vox-oratio/src/contextual_bias.rs)** — bias toward the user's vocabulary. Acta builds this lexicon automatically by counting words across the user's existing log: if you say "Sarah" 50 times, the model is biased to hear "Sarah" not "Sara."
6. **[`speech_normalize`](../../../crates/vox-oratio/src/speech_normalize.rs)** — number/date/unit normalization ("nine a m tomorrow" → "9am tomorrow"; "two hundred milligrams" → "200mg").
7. **[`refine/rules.rs`](../../../crates/vox-oratio/src/refine/rules.rs)** — capitalization, punctuation, common-ASR-error fixups. Free, fast, no model.
8. **[`refine/llm_correction_prompt.rs`](../../../crates/vox-oratio/src/refine/llm_correction_prompt.rs)** — opt-in heavier LLM cleanup. Off by default (battery cost; needs a small on-device LLM via vox-mens). Flag in settings: "Polish my transcripts (slower, better)."
9. **[`speech_intent`](../../../crates/vox-oratio/src/speech_intent.rs)** — the killer feature. Extracts intent from the transcript and produces a structured envelope. "Remind me to call mom tomorrow at 6" → `SpeechIntent::CreateReminder { target: "call mom", fire_at: <resolved>, body: "..." }`. Acta routes this envelope through a small dispatch table (Phase 1: only `CreateReminder` and `CreateLogEntry` intents; Phase 2: preset-specific intents).

The pipeline is *configurable per app* via `Vox.toml`:

```toml
[oratio]
backend = "sherpa-onnx"
model = "whisper-tiny-en"
preprocess = ["acoustic", "vad"]
postprocess = ["rerank", "lexicon", "bias", "normalize", "refine.rules", "intent"]
# postprocess.refine.llm = false      # opt-in, default off
```

### 4. Encryption is invisible to the app

Per [vox-mobile-plugin-spec §Phase 4](vox-mobile-plugin-spec-2026.md#phase-4--codex-bundled-sqlcipher-and-storage-encryption), encryption is configured once in `Vox.toml` and never appears in app code:

```toml
[storage]
encryption = { source = "clavis:vox.mobile.db_key", required = true }
```

The Clavis source resolves the key from Android Keystore (StrongBox-backed when available), iOS Keychain, or a user passphrase via Argon2id (for "remember me off" mode). Codex opens the SQLite connection with `PRAGMA key`, and every `@table` insert/query is transparently encrypted.

Audio files are stored separately under `<config_dir>/audio/<entry_id>.opus.enc`, encrypted with a per-file random key wrapped under the same DB master key (chacha20-poly1305 via `vox-crypto`). They are *not* in the SQL database — keeping audio out of SQLite avoids unbounded DB growth and lets the retention sweep be a directory walk.

## Repository shape

Acta lives in `github.com/<user>/acta`. Layout:

```
acta/
├── Vox.toml                    # target = "mobile"; oratio + storage configs
├── README.md
├── src/
│   ├── main.vox                # routes block; init handler
│   ├── schema.vox              # LogEntry + EntryTag + Habit + UserPreset
│   ├── endpoints/
│   │   ├── record.vox          # POST /record/start, /record/finish
│   │   ├── entries.vox         # GET /entries, /entries/:id, PATCH, DELETE
│   │   ├── reminders.vox       # POST /reminders, GET /reminders/active
│   │   ├── search.vox          # GET /search?q=&from=&to=&tag=
│   │   ├── presets.vox         # POST /presets/:name/enable, GET /presets
│   │   └── export.vox          # GET /export/{csv|markdown|json|encrypted-db}
│   ├── pipeline/
│   │   ├── transcribe.vox      # wires the oratio pipeline; called by record_finish
│   │   ├── classify.vox        # tier-1 rules + tier-2 MiniLM
│   │   ├── intent_dispatch.vox # routes SpeechIntent → CreateReminder / CreateEntry
│   │   └── lexicon_builder.vox # rebuilds user vocab from existing entries
│   └── presets/                # Phase 2 — see appendix
│       ├── mental_health.vox
│       ├── expense.vox
│       └── workout.vox
├── components/                 # Vox @component → React/TSX → Capacitor WebView
│   ├── RecordButton.vox
│   ├── Timeline.vox            # past + future, one scroll
│   ├── EntryDetail.vox
│   ├── SearchBar.vox
│   ├── Settings.vox
│   └── PresetGallery.vox
├── shell-android/              # generated by `vox mobile init` from host-shell.v1
└── shell-ios/                  # generated by `vox mobile init` from host-shell.v1
```

The `shell-android/` and `shell-ios/` directories are **generated artifacts** in the [external frontend interop plan](external-frontend-interop-plan-2026.md) sense — `vox mobile bindgen` regenerates them; hand-edits go in delimited `// vox:user-edit` zones.

## Components in detail

### Schema (full)

```vox
// vox:skip
enum EntrySource { voice, text, reminder_template, intent_extracted }
enum TagSource   { user, rule, embedding, llm, intent }
enum TagKind     { system, user_defined, preset }

@table type LogEntry {
    id:                ulid,
    created_at_utc:    datetime,
    fire_at_utc:       Option<datetime>,
    fired_at_utc:      Option<datetime>,
    local_tz:          str,
    source:            EntrySource,
    transcript:        str,
    audio_path:        Option<str>,
    duration_ms:       Option<int>,
    recurrence:        Option<str>,
    attributes:        json,                   // freeform, preset-defined
    tags:              Vec<str>,               // denormalized for fast list rendering
    payload_json:      Option<str>,            // SpeechIntent envelope for intent-extracted entries
    corrects:          Option<ulid>,           // points to the entry this one supersedes
    active:            bool,
}

@table type Tag {
    id:    ulid,
    name:  str unique,
    kind:  TagKind,
}

@table type EntryTag {
    entry_id:    ulid references LogEntry(id) on_delete: cascade,
    tag_id:      ulid references Tag(id),
    confidence:  f32,                          // 0.0–1.0; 1.0 = user-applied
    applied_by:  TagSource,
    primary key: (entry_id, tag_id),
}

@table type Lexicon {                          // for oratio contextual_bias
    word:        str unique,
    count:       int,
    user_added:  bool,
    last_seen:   datetime,
}

@table type Preset {                           // user-toggleable overlays
    name:         str unique,                  // 'mental_health', 'expense', 'workout', etc.
    enabled:      bool,
    config_json:  json,                        // preset-specific tuning
    installed_at: datetime,
}

// FTS5 over LogEntry transcripts and tags. Vox does not yet have @virtual table
// grammar; the FTS5 virtual table is created via vox-db's schema_extensions
// machinery (see crates/vox-db/src/schema_extensions.rs for the pattern used by
// knowledge_nodes_fts). Acta registers an extension that emits:
//
//   CREATE VIRTUAL TABLE IF NOT EXISTS log_entry_fts
//   USING fts5(transcript, tags, content='LogEntry', content_rowid='rowid',
//              tokenize='porter unicode61');
//
// A future Vox proposal could add @fts5 as a decorator on @table fields
// (out of scope for Acta v1).
```

### Endpoints (full)

```vox
// vox:skip
@endpoint(method: POST, path: "/record/start")
fn record_start() -> RecordSession

@endpoint(method: POST, path: "/record/finish")
fn record_finish(session_id: ulid, pcm_path: str, sample_rate_hz: int) -> LogEntry

@endpoint(method: POST, path: "/entries/text")
fn create_text_entry(text: str, attributes: Option<json>) -> LogEntry

@endpoint(method: GET, path: "/entries")
fn list_entries(
    from: Option<datetime>,
    to: Option<datetime>,
    direction: Direction,                      // past | future | both
    tag: Option<str>,
    limit: int,
) -> Vec<LogEntry>

@endpoint(method: GET, path: "/entries/:id")
fn get_entry(id: ulid) -> Result<LogEntry>

@endpoint(method: PATCH, path: "/entries/:id")
fn edit_entry(id: ulid, transcript: Option<str>, attributes: Option<json>, tags: Option<Vec<str>>) -> LogEntry

@endpoint(method: DELETE, path: "/entries/:id")
fn delete_entry(id: ulid) -> Result<()>      // soft-delete: active = false

@endpoint(method: POST, path: "/reminders")
fn create_reminder(
    transcript: str,
    fire_at_utc: datetime,
    recurrence: Option<str>,
    attributes: Option<json>,
) -> LogEntry

@endpoint(method: GET, path: "/reminders/active")
fn list_active_reminders() -> Vec<LogEntry>

@endpoint(method: GET, path: "/search")
fn search(q: str, limit: int) -> Vec<LogEntry>      // FTS5 over LogEntryFts

@endpoint(method: GET, path: "/export/csv")
fn export_csv(from: datetime, to: datetime) -> str

@endpoint(method: GET, path: "/export/markdown")
fn export_markdown(from: datetime, to: datetime) -> Vec<MarkdownFile>

@endpoint(method: GET, path: "/export/json")
fn export_json(from: datetime, to: datetime) -> json

@endpoint(method: GET, path: "/export/encrypted-db")
fn export_encrypted_db() -> Bytes               // .vox-vault file via Codex export-encrypted

fn on_reminder_fired(r: LogEntry) {              // dispatched by reminder runtime
    show_notification(r.transcript, r.attributes.get("body").unwrap_or(""));
    set_fired_at_now(r.id);
    if r.recurrence.is_some() {
        schedule_next_occurrence(r);            // computes next fire_at via RRULE; updates row
    }
}
```

### The transcribe pipeline (Vox)

```vox
// vox:skip
import std.oratio
import std.intent

fn transcribe(pcm_path: str, sample_rate_hz: int) -> TranscriptResult {
    let audio = oratio.acoustic_preprocess(pcm_path);
    let trimmed = oratio.vad(audio);
    let candidates = oratio.transcribe_n(trimmed, n: 3);
    let lexicon = lexicon_for_user();
    let best = oratio.transcript_rerank(candidates, lexicon);
    let normalized = oratio.speech_normalize(best);
    let refined = oratio.refine_rules(normalized);

    let intent_envelope = oratio.speech_intent(refined);
    return TranscriptResult {
        text: refined,
        intent: intent_envelope,
        confidence: best.confidence,
    };
}

fn handle_record_finish(session_id: ulid, pcm_path: str, sample_rate_hz: int) -> LogEntry {
    let result = transcribe(pcm_path, sample_rate_hz);
    return intent_dispatch.route(session_id, pcm_path, result);
}
```

### Intent dispatch

```vox
// vox:skip
fn route(session_id: ulid, pcm_path: str, t: TranscriptResult) -> LogEntry {
    match t.intent.action {
        SpeechIntentAction::CreateReminder { target, fire_at, body } => {
            create_reminder(
                transcript: target,
                fire_at_utc: fire_at,
                recurrence: None,
                attributes: Some(json::object! {
                    "body": body,
                    "extracted_from_audio": pcm_path,
                    "intent_confidence": t.intent.confidence,
                }),
            )
        }
        SpeechIntentAction::None => {
            create_log_entry_from_audio(session_id, pcm_path, t)
        }
        // Phase 2: preset-specific intents
        _ => create_log_entry_from_audio(session_id, pcm_path, t),
    }
}
```

### UI (Vox `@component`)

The UI is a Vox `component` tree lowered to React/TSX and hosted in Capacitor's WebView. The home screen is intentionally minimal:

- **`RecordButton`** — full-bleed circular button. Press-and-hold to record. Visual amplitude indicator. Releases trigger the record/finish flow. Lives in the **host shell**, not the WebView, so mic capture has zero WebView indirection.
- **`Timeline`** — an infinite-scroll list of `LogEntry` rows, ordered by `coalesce(fire_at_utc, created_at_utc)` descending for past, ascending for future. The "now" line is a sticky divider. Future entries (reminders) render with an alarm-clock icon; past entries with a microphone icon (voice) or pen icon (text) or magic-wand icon (intent-extracted).
- **`EntryDetail`** — single entry view: full transcript, audio playback (if `audio_path` not yet retention-purged), tags, attributes, edit / delete / "this is a reminder" / "this is a log" / "supersede with new transcript".
- **`SearchBar`** — FTS5 over transcript+tags, with date-range and tag chips.
- **`Settings`** — passphrase change, retention period for audio (default 7 days), opt-in LLM polish, oratio model picker, lexicon manager, preset gallery, export controls.
- **`PresetGallery`** — Phase 2; lists installable presets (mental_health, expense, workout, etc.).

## Sequencing and dependencies

```
[vox-mobile plugin Phases 1–6]
            │
            ▼
Phase A — Bedrock loop (record → transcribe → log → display)
            │
            ▼
Phase B — Reminders (table-watching → host alarm → on_reminder_fired)
            │
            ▼
Phase C — Search + export (FTS5; CSV / Markdown / JSON / encrypted DB)
            │
            ▼
Phase D — Oratio post-processing pipeline (lexicon, bias, intent dispatch)
            │
            ▼
Phase E — Recovery (encrypted DB import; passphrase reset flow)
            │
            ▼
Phase F (Phase 2 of the app) — Presets and structured overlays
```

Phases A–E are MVP. Each is independently shippable to a test device. Phase F is post-MVP — see appendix.

## Effort estimate (assuming Doc 1 is in flight or done)

- Phase A: ~3 days (record/finish endpoint, naive transcribe, single-entry display)
- Phase B: ~2 days (reminder table-watcher wiring + on_reminder_fired)
- Phase C: ~3 days (search + four exporters + share intent)
- Phase D: ~3 days (oratio pipeline configuration + intent dispatch + lexicon builder)
- Phase E: ~2 days (encrypted DB export/import + passphrase reset)

**Total MVP: ~13 working days** — plus app-store-style polish (icon, splash, onboarding copy). Phases F (presets) is open-ended depending on how many overlays you want to ship.

## Risks and mitigations

- **STT quality on cheap phones.** Whisper-Tiny.en is the floor; quality may be unacceptable. Mitigation: settings-toggle to download Whisper-Base.en (~80MB, slower but better). Lexicon + bias close most of the gap on user-specific vocab.
- **Intent extraction false positives.** "I should remind myself to..." in a journal entry could be misclassified as `CreateReminder`. Mitigation: the transcribe pipeline returns the intent envelope but the dispatch always saves the entry; if intent is `CreateReminder`, the entry is the reminder, otherwise it's a log entry. Either way the user sees what was created and can convert with one tap.
- **Audio retention vs. backup.** Audio purged after 7 days is gone forever; the encrypted-DB export captures only DB rows, not the audio directory. Mitigation: `export-encrypted-db` is augmented with `--include-audio` to bundle the audio dir in the archive.
- **Universal timestamp + DST transitions.** A reminder set for "9am tomorrow" the day before DST falls back will fire at 8am or 10am depending on naive computation. Mitigation: `fire_at_utc` is computed by anchoring to the user's `local_tz` + wall time, *re-resolved* via tz-aware library at fire time. Documented behavior.
- **One-table design over-flexes.** A `LogEntry` row could become a kitchen sink as presets accumulate. Mitigation: the `attributes: json` column absorbs preset-specific fields; the core columns are frozen; presets that need their own searchable indexes get a satellite table that joins on `entry_id`.

## Privacy posture

- Encryption-at-rest via Clavis-resolved key; Argon2id passphrase as recovery.
- Audio files: 7-day default retention, configurable to 0 (immediate purge after transcribe). Retention sweep is itself a `LogEntry` with `recurrence = "FREQ=DAILY;BYHOUR=3"` whose handler is `audio_retention_sweep` — eating its own dogfood.
- **No `INTERNET` permission.** Manifest declares `<uses-permission android:name="android.permission.INTERNET" tools:node="remove"/>`. iOS app marks no `NSAppTransportSecurity` exceptions. Verifiable trust signal.
- No telemetry of any kind from the device. The `vox-mobile` plugin's build-time telemetry is workstation-side only.
- No accounts, no cloud, no server. v1 is fully self-contained.

## Distribution

- **Android:** sideload APK (debug); release builds via `vox mobile package --release` produce a signed APK suitable for IzzyOnDroid. Official F-Droid is a follow-on.
- **iOS:** TestFlight via `vox mobile package --platform=ios --release`, then App Store if the user wants. App Store review will likely flag `RECORD_AUDIO` and require a privacy disclosure ("Voice recordings are stored only on this device and never transmitted").

---

## Appendix A — Phase 2 presets (Acta v2)

The original input architecture described a mental-health tracker. That design becomes a **preset** in Acta v2:

```vox
// vox:skip
// presets/mental_health.vox
preset mental_health {
    extends: LogEntry,
    attributes: {
        mood_1_to_10:   Option<int>,
        energy_1_to_10: Option<int>,
        sleep_hours:    Option<f32>,
        meds_taken:     Option<bool>,
    },
    intents: {
        "i feel (?P<feeling>.+)" => RecordMood { feeling: $feeling },
        "i slept (?P<hours>\d+) hours?" => RecordSleep { hours: $hours },
    },
    exports: {
        "weekly_chart.pdf": render_mental_health_weekly_chart,
        "therapist_handoff.pdf": render_therapist_pdf,
    },
    ui: {
        timeline_badge: { mood_1_to_10 -> color_scale("#ef4444", "#22c55e") },
        entry_detail_addons: [MoodSlider, EnergySlider, SleepInput, MedsCheckbox],
    },
}
```

Other presets (sketches, not specified):

- **expense** — `attributes: { amount, currency, category, vendor }`; intent `"spent (?P<amt>...) on (?P<vendor>.+)"`; exports CSV per month.
- **workout** — `attributes: { exercise_name, sets, reps, weight, duration_min }`; intent `"did (?P<sets>...) sets of (?P<exercise>.+)"`; exports stacked bar chart per muscle group.
- **family-events** — `attributes: { who, location }`; reminders for birthdays / anniversaries via recurrence rules.

The preset machinery itself is a Phase F deliverable: a `preset` keyword (or decorator on a module) that registers attribute extensions, intent regex extensions, exporter functions, and UI add-ons. Out of scope for the MVP but the schema and intent dispatch are designed to accommodate it without breaking changes.

## Appendix B — Why Vox earns its keep on this project

A skeptic will ask: this app could be built faster in Kotlin or React Native. Why Vox?

- **One source, three deployments.** The same `.vox` source can be rebuilt with `--target=server` (Acta as a self-hosted personal sync server later), `--target=fullstack` (Acta as a desktop/web companion), and `--target=mobile` (the phone app). The schema, endpoints, classifier, and exporters are written once.
- **Type-safe wire format.** The [Wire Format v1 SSOT](wire-format-v1-ssot.md) means a future sync server and the mobile client never disagree on date encoding, enum tagging, or BigInt handling. ULIDs, RRULE strings, and the `attributes: json` column are wire-format-codified.
- **Free OpenAPI and TypeScript clients.** Once the [interop plan Phase 2](external-frontend-interop-plan-2026.md#phase-2--wire-format-ssot-and-standards-based-schema-emit) lands, `vox emit openapi` produces a spec the user could publish for third-party Acta clients without writing a line of additional code.
- **Encryption + secret management as platform features.** Clavis + Codex `bundled-sqlcipher` mean Acta's encryption story is a single `Vox.toml` line, not 200 lines of `EncryptedSharedPreferences` boilerplate.
- **Forces dogfooding.** Building Acta surfaces real gaps in `vox-mobile` (the plugin spec) that benefit every future Vox app.

If after Phase A the build experience is painful enough to threaten the project, the fallback is unchanged from earlier guidance: port the schema and pipeline to Kotlin / Swift directly. The schema is small enough (one table, three indexes, one virtual FTS5 table) that no work is wasted.
