---
title: "Acta — User Journey (2026)"
description: "End-to-end user journeys for Acta — the mobile-first voice-first universal logbook. Covers first launch, the bedrock record/transcribe/log loop, future reminders, search, edits and corrections, exports, recovery from the encrypted backup, and the failure modes that determine whether the app actually gets used. Each journey ties to specific endpoints from the Acta architecture and FFI calls from the vox-mobile host-shell contract."
category: "architecture"
status: "roadmap"
training_eligible: true
training_rationale: "Canonical user-journey reference for Acta. Useful as a model for future Vox mobile apps because it shows the full flow from cold install through encrypted-backup recovery, including the platform-specific touchpoints (Android Keystore, iOS Keychain, AlarmManager.setAlarmClock, share sheet) that any voice-first offline app will hit."
---

# Acta — User Journey (2026)

This document is the *companion* to [Acta — App Architecture (2026)](vox-acta-app-architecture-2026.md). The architecture doc says what Acta is and what it's built from. This doc says what *using* Acta looks like, end to end. Anyone building, contributing to, designing for, or evaluating Acta should read this one first.

The journeys are written in present tense from the user's perspective, with bracketed notes pointing at the architecture-doc endpoint or the [vox-mobile host-shell contract](vox-mobile-plugin-spec-2026.md) function each step actually invokes.

## Persona

There is one persona — the user is the developer building this for themselves. Acta is single-user, single-device by design (sync is a future feature, not a v1 feature). The user is competent with technology, sideloads APKs, and would rather lose a feature than a privacy property. They want the app to disappear into the background between presses of the record button.

There is no secondary persona for a "viewer" or "collaborator." If a clinician, partner, or accountant needs to see Acta data, the user generates an export and hands it over (see *Journey 5: Export*). The app itself is solo.

---

## Journey 1 — First launch

**Goal:** the user can record a first entry in under 90 seconds from icon-tap.

**Steps:**

1. User installs the APK (sideload) or `.ipa` (TestFlight). Taps the icon.
2. **Welcome screen.** One paragraph: *"Acta keeps a private log of anything you record, on this device only. No accounts, no cloud, no internet permission."* Two buttons: *Continue* and *Restore from backup*. (The latter is *Journey 6.*)
3. **Set passphrase.** *"Pick a passphrase. We use it to encrypt your log. Write it down somewhere safe — there is no recovery if you lose it."*
   - Two text inputs (passphrase + confirm).
   - Strength meter (zxcvbn-style, library bundled).
   - On submit: cdylib derives the master key with Argon2id (parameters pinned in the [Clavis spec](vox-mobile-plugin-spec-2026.md#phase-3--mobile-clavis-sources)), wraps it with the platform keystore, stores both. The Argon2id salt is written to `<config_dir>/db_key.salt`.
   - Codex opens the SQLite DB with `PRAGMA key`; the schema migrations from `src/schema.vox` run.
4. **Permission requests** (one screen, three rows, each with explanation):
   - **Microphone** (foreground only) — *"Acta records audio only when you hold the record button."*
   - **Notifications** — *"For reminders to fire."*
   - **Battery whitelist** (Android) — *"For reminders to fire on time during deep sleep. Tap Open Settings."* This invokes `request_battery_whitelist_prompt` on the [host-shell contract](vox-mobile-plugin-spec-2026.md#phase-2--host-shell-contract-and-bindgen), which deep-links to `Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS`.
5. **Tutorial — three swipeable cards.** No skipping; tap-to-dismiss each.
   - *"Hold the button. Speak. Release. That's the whole app."*
   - *"Say 'remind me to ___ at ___' to schedule something for later."*
   - *"Everything you say stays on this device. Forever, or until you delete it."*
6. **Land on the Record screen.** Big button. Status bar: *"Tap and hold to log something."*

**Total time budget:** 60–90 seconds. If the user can't be recording within that window, the onboarding has failed.

**Backend touches:** Clavis spec resolution (`vox.mobile.db_key`), Codex `with_encryption_key().open()`, host-shell `request_battery_whitelist_prompt`. No network. No analytics event fires anywhere — Acta does not have analytics.

---

## Journey 2 — The bedrock loop (record a log entry)

**Goal:** every record-and-release cycle produces a timestamped entry without making the user think.

**Steps:**

1. User presses-and-holds the giant `RecordButton` on the home screen. Visual amplitude indicator pulses; haptic tick on press.
2. Host shell starts `MediaRecorder` (Android) / `AVAudioRecorder` (iOS) at 16 kHz mono PCM into a temp file. (Pure platform code — no FFI traffic during capture.)
3. User releases. Haptic tick.
4. Host shell calls `vox_mobile_record_pcm(temp_pcm_path, 16000)` on the cdylib.
5. Inside the cdylib, the [transcribe pipeline](vox-acta-app-architecture-2026.md#the-transcribe-pipeline-vox) runs — `acoustic_preprocess → vad → sherpa_onnx (top-3 hypotheses) → transcript_rerank (with user lexicon) → speech_normalize → refine.rules → speech_intent`.
6. Intent dispatch routes the result. Two outcomes:

   - **`SpeechIntent::None`** (the common case) — the function `create_log_entry_from_audio` is called. New `LogEntry` row written: `id = ulid()`, `created_at_utc = now()`, `local_tz = system_tz()`, `source = voice`, `transcript = refined_text`, `audio_path = <encrypted_path>`, `fire_at_utc = None`, `tags = classifier_suggested_tags`.
   - **`SpeechIntent::CreateReminder { target, fire_at, body }`** (the magic case) — the function `create_reminder` is called. New `LogEntry` row written with `fire_at_utc = fire_at`, `transcript = target`, `attributes.body = body`, `attributes.intent_confidence = 0.87` (or whatever).
7. The cdylib returns the new `LogEntry` to the host shell as JSON. The WebView re-renders the Timeline; the new row animates in from the top.
8. **Confirmation surface.** Below the new row, a short toast: *"Logged at 5:32 PM"* (past) or *"Reminder set for tomorrow 9:00 AM"* (future). Toast has an *Undo* button for 5 seconds (sets `active = false`) and an *Edit* button.

**Total user time:** 0 (button press) + speech duration + ~3–5 seconds for transcribe-and-render on a midrange phone. Acceptable up to ~10s; degrades discoverability above that.

**What the user sees that they wouldn't see in a worse design:**
- The toast distinguishes past from future. The user immediately knows which kind of entry was created.
- Intent-extracted reminders are *not silently confirmed* — the user sees the time and can tap to correct. False-positive intent extraction is recoverable in one tap.

---

## Journey 3 — Reminder fires

**Goal:** the reminder fires on time. Tapping it returns the user to context.

**Steps:**

1. At the scheduled time, Android's `AlarmManager` (configured via `setAlarmClock`, Doze-exempt) fires. iOS's `UNUserNotificationCenter` analogue fires. The host shell receives the alarm.
2. Host shell calls `vox_mobile_reminder_fired(reminder_id)` on the cdylib.
3. The runtime loads the row, sets `fired_at_utc = now()`, calls the user-defined `on_reminder_fired(r: LogEntry)` ([architecture doc](vox-acta-app-architecture-2026.md#endpoints-full)). The default implementation calls `show_notification(r.transcript, r.attributes.get("body"))` via the [host-shell callback](vox-mobile-plugin-spec-2026.md#phase-2--host-shell-contract-and-bindgen).
4. If `recurrence` is set, the runtime computes the next occurrence (RFC 5545 RRULE), updates `fire_at_utc` and clears `fired_at_utc`, and re-issues `request_alarm` for the new time.
5. User sees a notification: *"Take your morning walk"* / *"How did you sleep?"* / *"Call mom"*.
6. **Tap notification.** Host shell deep-links to `EntryDetail/<reminder_id>` (or to a custom `deeplink_path` from the callback). User can:
   - **Mark done** — adds an `attributes.completed = true`, leaves the row.
   - **Snooze** — picker for +5 min / +1 hour / +tomorrow; updates `fire_at_utc` and re-issues `request_alarm`.
   - **Voice-respond** — the `RecordButton` is rendered in-context with the reminder's transcript pre-filled as the title; the recorded entry's `attributes.responds_to = <reminder_id>` is set automatically.
   - **Delete** — soft-delete the reminder.
7. **Missed reminders.** If the device was off at fire time, on next boot the host shell's `BOOT_COMPLETED` receiver calls `vox_mobile_init` which reconciles all alarms (per [Phase 5 of the plugin spec](vox-mobile-plugin-spec-2026.md#phase-5--reminder-runtime-vox-stdlibreminder)) and immediately fires any reminders whose `fire_at_utc < now() AND fired_at_utc IS NULL`.

**Failure modes & handling:**
- **OEM aggressive battery management.** Mitigated by the battery-whitelist prompt in [Journey 1](#journey-1--first-launch). If the user declined, settings has a *"Reminders not firing? Re-check battery permission"* link.
- **DST transition.** `fire_at_utc` is recomputed from the user's `local_tz` + wall-clock intent ("9 AM tomorrow") at fire time, not stored as a frozen UTC moment. Documented in settings.
- **Reminder stuck "active" after device clock change.** The reconciler always re-derives the next fire from the row's stored intent, so `now()` jumps don't desync.

---

## Journey 4 — Find something later (search)

**Goal:** the user remembers something happened ~last month and wants the entry.

**Steps:**

1. User taps the search icon in the Timeline header.
2. `SearchBar` appears. Three filter chips below: *Past / Future / Both* (default Both), date-range pill (default *All time*), tag chip (default *Any*).
3. User types `"sarah"`. As they type, FTS5 query fires (debounced 200ms): `GET /search?q=sarah&limit=50`. Hits the `LogEntryFts` virtual table on `transcript` and `tags`.
4. Results list renders, each row showing the matched fragment with `<mark>` highlights, plus the entry's date in the user's local format ("April 18, 2026, 3:41 PM, 2 weeks ago").
5. User taps a result → `EntryDetail` view → can play audio (if not retention-purged), edit, tag, or convert to a reminder.

**Edge cases the search must handle without the user thinking about them:**
- **Search across timezones.** The user might have written the entry while traveling. The displayed date is in the entry's `local_tz`; search uses `created_at_utc`. Both are right.
- **Stemming.** SQLite FTS5 with `tokenize = "porter unicode61"`. "Walking" matches "walks" matches "walk."
- **Tag-only search.** Tapping a tag chip without text returns all entries with that tag.

---

## Journey 5 — Export (hand off to another tool or person)

**Goal:** get the data out, in a format the other party (clinician, accountant, future-self) will understand.

Four exports, all generated by Vox endpoints, all dispatched through the host-shell `request_share` callback:

| Export | Endpoint | Use |
|---|---|---|
| **CSV** | `GET /export/csv?from=&to=` | Spreadsheet analysis. One row per entry; ISO-8601 timestamps; tags semicolon-joined. |
| **Markdown bundle** | `GET /export/markdown?from=&to=` | Plain-text archive, one file per day, headers per period (morning/afternoon/evening). Embeddable in Obsidian. Survives Acta's existence. |
| **JSON** | `GET /export/json?from=&to=` | For another Vox app or a script. Schema follows the [Wire Format v1 SSOT](wire-format-v1-ssot.md). |
| **Encrypted DB** | `GET /export/encrypted-db` | Full-fidelity backup. A `.vox-vault` file (the SQLCipher DB + manifest with backup-KEK-wrapped key). Decrypted only by Acta's restore flow with the same passphrase. *Not* human-readable. |

**Steps for each:**
1. *Settings → Export.*
2. Pick format. Pick range (Last 7 / 30 / 90 / 365 days / All time / Custom).
3. Tap *Export*. The cdylib generates the file under `<config_dir>/exports/`. (The encrypted-DB export uses Codex's `export-encrypted` subcommand from [Phase 4 of the plugin spec](vox-mobile-plugin-spec-2026.md#phase-4--codex-bundled-sqlcipher-and-storage-encryption).)
4. Cdylib calls `request_share(file_path, mime_type)`. Host shell opens the platform share sheet.
5. User picks the recipient — email, Signal, Drive, AirDrop, whatever. Acta itself never auto-sends.

**Privacy invariants:**
- The CSV / Markdown / JSON exports are **plaintext**. The user is warned ("This file is unencrypted. Anyone with the file can read it.") with a one-time confirm before generating.
- The encrypted-DB export is portable but useless without the passphrase. The user is *encouraged* to print or write down the passphrase separately when they generate the first encrypted backup.

---

## Journey 6 — Recovery (new device, restored from backup)

**Goal:** the user moves to a new phone or factory-resets, and recovers all their data.

**Steps:**

1. User installs Acta on the new device. Taps icon.
2. Welcome screen. Taps *Restore from backup*.
3. File picker opens. User locates a `.vox-vault` file on the device (transferred via cable, AirDrop, Drive download, etc.).
4. *"Enter the passphrase you used when creating this backup."* Text input.
5. cdylib derives the candidate key with Argon2id from passphrase + the salt embedded in the vault's manifest. Attempts to unwrap the backup-KEK-wrapped DB master key.
   - On success: replaces the local `<config_dir>/data.db` with the decrypted contents; rewraps the master key with the *new* device's keystore for at-rest storage; re-runs reconciler to re-issue all active reminders to the new device's `AlarmManager`.
   - On failure: *"Wrong passphrase. Try again, or restore a different backup."*
6. User lands on Timeline. All entries present. All future reminders re-armed.

**Failure modes:**
- **Lost passphrase.** No recovery. *Documented prominently.* The trade-off is intentional — the encryption is meaningful only if the key is genuinely user-held. Acta's onboarding warns about this; the encrypted-DB export warns again.
- **Corrupt backup.** Codex import surfaces `EncryptionKeyMismatch` vs `CorruptDb` distinctly. Corrupt-DB shows a recovery hint: "Restore an older backup."
- **Backup from a newer Acta version with schema changes.** Codex's existing migration framework handles forward migrations; rejected only if the backup is *newer* than the installed app (suggests "Update Acta first").

---

## Journey 7 — Edits and corrections

**Goal:** transcripts have errors. Reminders move. Tags change. The user expects ergonomic correction without losing the audit trail.

**Steps for transcript correction:**

1. User taps an entry → `EntryDetail`. Reads transcript. Spots an error ("nine a m" should be "9 AM").
2. Taps *Edit transcript*. Text editor opens.
3. Saves. **Behind the scenes:** the original `LogEntry` row is *not mutated*. Instead, a new `LogEntry` row is written with `corrects = <original_id>`, the new transcript, and `created_at_utc = now()`. The original's `active = false`. The Timeline only shows the latest in a `corrects` chain by default; "show history" toggles the rest.
4. **Why.** Append-only history makes the export trustworthy and makes any future sync trivially conflict-free (last-writer-wins on `corrects` chains).

**Steps for tag correction:**

1. `EntryDetail` shows tag chips with confidence dots (small dots: low confidence; filled circles: user-applied).
2. Tap a tag → *Remove* / *Promote to user-applied*.
3. Long-press empty tag area → *Add tag*. Auto-complete from existing `Tag` table.
4. Tag changes update `EntryTag` rows directly (no `corrects` chain — tags are metadata, not content).

**Steps for reminder reschedule:**

1. `EntryDetail` for a future reminder shows time picker.
2. Tap *Change time*. Picker. Save.
3. Behind the scenes: `UPDATE LogEntry SET fire_at_utc = ?` triggers the reminder runtime's change hook, which issues `cancel_alarm(old_id)` then `request_alarm(reminder_id, new_fire_at_utc, ...)`. No app code involvement.

---

## Journey 8 — Settings the user actually opens

The full settings tree is intentionally short. Things the user will touch:

- **Passphrase:** Change passphrase. (Re-derives key, re-wraps master key, re-reads salt. Does NOT re-encrypt the DB — `PRAGMA rekey` is invoked.)
- **Audio retention:** *Keep audio for: [0 / 7 / 30 / 90 / forever] days.* Default 7.
- **Transcription quality:** *Whisper model: [Tiny.en (default, 40MB) / Base.en (80MB, slower, better)]*. Choosing Base.en triggers a download.
- **LLM polish:** *Polish my transcripts (slower, requires vox-mens):* Off / On. Off by default.
- **Lexicon:** Browse and edit the auto-built user vocabulary list. Add, remove, set custom pronunciations (forwarded to oratio's `speech_lexicon`).
- **Reminders:** Master enable / disable. *Test reminder* button (fires in 30 seconds — used to debug battery-whitelist issues).
- **Export:** the four exports from Journey 5.
- **Backup:** *"Create encrypted backup now"* — same as the encrypted-DB export, with a *"Save to..."* share-sheet step.
- **About:** Version, Vox version, oratio model versions, on-disk sizes, *"Show licenses"*.
- **Reset:** *"Delete all my data on this device."* Two confirm steps. Wipes `<config_dir>` recursively and removes the keystore-wrapped key. Does NOT touch backups the user previously exported.

Settings the user should NOT see: anything telemetry-related (there is none), any account UI (there are no accounts), any "share usage data" toggle (there is no usage data).

---

## Journey 9 — Adding a preset (Phase 2 / Acta v2)

**Goal:** the user wants to track moods alongside their general log, without rebuilding the whole app.

**Steps:**

1. Settings → *Preset gallery*. Scrolls list of available presets.
2. Taps *Mental Health Tracker → Install*.
3. Behind the scenes: `POST /presets/mental_health/enable` writes a `Preset` row with `enabled = true` and the preset's default `config_json`. The preset's `attributes` extension (`mood_1_to_10`, `energy_1_to_10`, …) is registered with the `LogEntry`'s json schema. Intent regexes are added to the dispatch table. Exporters are registered.
4. From now on:
   - The `RecordButton` long-press shows a quick-action wheel including *"Quick mood"* (mood slider only, no audio).
   - Voice intent like *"I feel anxious"* is matched by `mental_health`'s `RecordMood` regex and creates an entry with `attributes.mood = "anxious"`.
   - `EntryDetail` shows the mood / energy / sleep add-on widgets.
   - Settings → *Export* gains a new *Therapist PDF* option.
   - Timeline rows acquire a colored badge (red→green) reflecting `mood_1_to_10` if set.

**To uninstall:** *Settings → Preset gallery → Mental Health Tracker → Disable*. The preset's `attributes` data on existing entries is preserved (just hidden); re-enabling restores the UI.

---

## Failure modes worth naming explicitly

- **Transcription returns empty / garbage.** Toast: *"I didn't catch that. Try again, or type instead."* The audio is kept (subject to retention) so the user can replay and re-transcribe later.
- **STT crashes the cdylib.** The host shell catches the panic, restarts the runtime, surfaces *"Acta hit an internal error and restarted. Your last recording is saved as audio-only — replay to retry transcription."*
- **Disk full.** Insert/update returns `Codex::Full`. Modal: *"Not enough storage. Free up space and try again, or delete old audio in Settings → Audio retention."*
- **Reminder didn't fire.** *Settings → Reminders → "Show missed reminders"* lists reminders where `fire_at_utc < now() - 5min AND fired_at_utc IS NULL`. One-tap *"Reschedule for now + 5 min"* per row.
- **App backgrounded mid-recording.** The host shell's foreground service flag keeps the recorder running for up to 30s after backgrounding so the user can swipe between apps without losing the entry.

---

## What is *not* in any of these journeys

These are explicit non-features for v1, listed so reviewers don't propose them:

- No login screen.
- No password reset email.
- No "share with my partner" link.
- No Apple Health / Google Fit / Strava sync.
- No widgets on the home screen (Phase 2 candidate).
- No multi-device sync (Phase 2 candidate; the schema is sync-ready).
- No web companion (Phase 2 candidate via `--target=fullstack`).
- No subscription, no upsell, no purchase flow of any kind. Acta is a free piece of personal infrastructure.

The shape of Acta is *the user, their phone, the button, and the log*. Anything that would push past that shape lives outside v1.
