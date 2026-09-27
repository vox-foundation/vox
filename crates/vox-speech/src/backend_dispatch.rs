//! Runtime backend selection for Oratio STT.
//!
//! Priority: `VOX_ORATIO_BACKEND` env → feature flags → Whisper via [`whisper_backend`]
//! (the host-registered `oratio` plugin transcriber; an actionable error when none is
//! registered).

use crate::backends::asr_backend::{AsrBackend, AsrOutput, TimedSegment};

/// Whisper transcriber supplied by the host: the `SpeechToText::transcribe` contract of
/// the `oratio` plugin (vox-plugin-speech). Takes mono f32 little-endian PCM bytes and a
/// `{"sample_rate", "language"}` config JSON; returns `{"text", "segments"}` JSON.
///
/// vox-speech cannot reach vox-plugin-host itself, so hosts that can (vox-gui,
/// vox-ml-cli's `vox oratio`) register one with [`register_whisper_transcriber`].
pub type ExternalWhisperTranscribe =
    fn(pcm_le_f32: &[u8], config_json: &str) -> Result<String, String>;

static WHISPER_TRANSCRIBER: std::sync::RwLock<Option<ExternalWhisperTranscribe>> =
    std::sync::RwLock::new(None);

/// Register the host's Whisper transcriber and drop the cached backend so the next
/// transcription rebuilds with it.
pub fn register_whisper_transcriber(f: ExternalWhisperTranscribe) {
    *WHISPER_TRANSCRIBER
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(f);
    invalidate_cache();
}

/// Whether a host registered a Whisper transcriber.
pub fn has_registered_whisper_transcriber() -> bool {
    registered_whisper_transcriber().is_some()
}

fn registered_whisper_transcriber() -> Option<ExternalWhisperTranscribe> {
    *WHISPER_TRANSCRIBER
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Test-only: forget the registered transcriber.
#[cfg(test)]
fn clear_whisper_transcriber_for_test() {
    *WHISPER_TRANSCRIBER
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    invalidate_cache();
}

/// [`AsrBackend`] over a host-registered [`ExternalWhisperTranscribe`].
struct ExternalWhisper(ExternalWhisperTranscribe);

impl AsrBackend for ExternalWhisper {
    fn name(&self) -> &'static str {
        "whisper (oratio plugin)"
    }

    fn transcribe_pcm(
        &self,
        pcm: &[f32],
        sample_rate: u32,
        language: Option<&str>,
    ) -> anyhow::Result<AsrOutput> {
        let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
        let config = serde_json::json!({ "sample_rate": sample_rate, "language": language });
        let json = (self.0)(&bytes, &config.to_string())
            .map_err(|e| anyhow::anyhow!("oratio plugin transcribe: {e}"))?;
        parse_plugin_transcription(&json)
    }
}

/// Map the plugin's `{"text", "segments"}` JSON to [`AsrOutput`]. `text` is required;
/// malformed segments are an error, never a panic.
fn parse_plugin_transcription(json: &str) -> anyhow::Result<AsrOutput> {
    #[derive(serde::Deserialize)]
    struct PluginTranscription {
        text: String,
        #[serde(default)]
        segments: Vec<TimedSegment>,
    }
    let parsed: PluginTranscription = serde_json::from_str(json)
        .map_err(|e| anyhow::anyhow!("oratio plugin returned invalid transcription JSON: {e}"))?;
    Ok(AsrOutput {
        raw_text: parsed.text,
        confidence: 0.85,
        n_best: Vec::new(),
        segments: parsed.segments,
    })
}

/// The Whisper backend: the host-registered transcriber, or an actionable error when
/// none is registered.
pub fn whisper_backend() -> anyhow::Result<Box<dyn AsrBackend>> {
    if let Some(f) = registered_whisper_transcriber() {
        return Ok(Box::new(ExternalWhisper(f)));
    }
    anyhow::bail!(
        "Candle Whisper STT runs in the `oratio` plugin (vox-plugin-speech) and this host \
         registered no Whisper transcriber; vox-gui and `vox oratio` register it at startup. \
         Install the plugin with `vox plugin install oratio`."
    )
}

/// Test-only instrumentation: counts invocations of `create_backend()`'s body,
/// used to assert that `with_cached_backend` constructs the backend once.
#[cfg(test)]
pub(crate) static CREATE_BACKEND_CALL_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Instantiate the configured STT backend.
///
/// # Env
/// - `VOX_ORATIO_BACKEND=auto` (default) — picks Sherpa if compiled in, else Whisper
/// - `VOX_ORATIO_BACKEND=whisper` — always Whisper ([`whisper_backend`])
/// - `VOX_ORATIO_BACKEND=sherpa` — always Sherpa (returns error if feature not compiled)
pub fn create_backend() -> anyhow::Result<Box<dyn AsrBackend>> {
    #[cfg(test)]
    {
        CREATE_BACKEND_CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if std::env::var("VOX_TEST_FORCE_BACKEND_FAIL").is_ok() {
            anyhow::bail!("forced failure for test (VOX_TEST_FORCE_BACKEND_FAIL set)");
        }
    }

    let backend_env = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxOratioBackend)
        .expose()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "auto".to_string());
    let backend_env = backend_env.trim().to_ascii_lowercase();

    match backend_env.as_str() {
        "auto" | "" => {
            #[cfg(feature = "stt-sherpa")]
            {
                match crate::backends::sherpa_onnx::SherpaOnnxBackend::new() {
                    Ok(backend) => Ok(Box::new(backend) as Box<dyn AsrBackend>),
                    Err(e) => {
                        tracing::warn!(
                            target: "vox_oratio_backend",
                            event = "sherpa_init_failed_falling_back",
                            error = %e,
                            "Sherpa-ONNX (Parakeet) init failed; falling back to Whisper"
                        );
                        whisper_backend().map_err(|w| {
                            anyhow::anyhow!(
                                "Sherpa-ONNX init failed: {e:#}; Whisper fallback failed: {w:#}"
                            )
                        })
                    }
                }
            }
            #[cfg(not(feature = "stt-sherpa"))]
            {
                whisper_backend()
            }
        }
        "whisper" | "candle" => whisper_backend(),
        "sherpa" => {
            #[cfg(feature = "stt-sherpa")]
            return Ok(Box::new(
                crate::backends::sherpa_onnx::SherpaOnnxBackend::new()?,
            ));
            #[cfg(not(feature = "stt-sherpa"))]
            anyhow::bail!("Backend 'sherpa' selected but `stt-sherpa` feature not compiled.");
        }
        other => anyhow::bail!("Unknown VOX_ORATIO_BACKEND value: {other:?}"),
    }
}

/// Process-lifetime cache for the ASR backend instance.
///
/// `create_backend()` can be expensive (model resolution, ONNX Runtime
/// session construction) — for Sherpa-ONNX/Parakeet this includes loading a
/// ~671MB model. Without caching, calling `create_backend()` per utterance
/// (as the file-transcription path in `traits.rs` does) would pay that full
/// cost on every dictation stop. This cache constructs the backend once and
/// reuses it for the lifetime of the process.
///
/// A construction failure leaves the slot `None` so the *next* call retries
/// `create_backend()` from scratch, instead of latching into a permanent
/// "no backend" state until restart.
///
/// Stored as `Arc`, not `Box`: `with_cached_backend` only needs the mutex
/// held long enough to construct-once-and-clone the handle, not for the
/// duration of the (potentially slow) inference call `f` makes with it —
/// holding a `MutexGuard` across `f` would serialize every concurrent
/// transcription in the process on ASR inference time, not just backend
/// construction, which nothing about this cache is meant to require.
static BACKEND: std::sync::Mutex<Option<std::sync::Arc<dyn AsrBackend>>> =
    std::sync::Mutex::new(None);

/// Clear the cached backend so the *next* call to [`with_cached_backend`]
/// reconstructs it via `create_backend()`, re-reading `VOX_ORATIO_BACKEND`.
///
/// Needed because changing the backend setting (e.g. via the GUI's Settings
/// panel) only updates the env var / secret resolution the *next*
/// `create_backend()` call reads — without this, a Settings change would
/// update the displayed value but silently keep using the previously
/// constructed (now stale) engine for the rest of the process's life,
/// since `with_cached_backend` has no other way to know the setting changed.
pub fn invalidate_cache() {
    let mut guard = BACKEND
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = None;
}

/// Run `f` against the cached ASR backend, constructing it on first
/// successful call. Safe to call from multiple threads: the backend is
/// constructed at most once (barring a failed attempt, which is retried on
/// the next call). The lock is released before `f` runs, so concurrent
/// transcriptions don't serialize on each other — only on the one-time
/// construction.
pub fn with_cached_backend<F, R>(f: F) -> anyhow::Result<R>
where
    F: FnOnce(&dyn AsrBackend) -> anyhow::Result<R>,
{
    let backend = {
        let mut guard = BACKEND
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.is_none() {
            *guard = Some(std::sync::Arc::from(create_backend()?));
        }
        guard.as_ref().expect("just populated").clone()
    };
    f(backend.as_ref())
}

/// Test-only: clears the cache slot so each test starts from a known state.
/// `BACKEND` is a process-wide static shared by every test in this binary;
/// without this, whichever test runs first "wins" the cache for the rest.
/// Delegates to [`invalidate_cache`] — same operation, test-facing name.
#[cfg(test)]
fn reset_cache_for_test() {
    invalidate_cache();
}

// The tests below select Whisper (`VOX_ORATIO_BACKEND=whisper`, or the "auto"
// fallback) and register `fake_transcriber` as the host's Whisper transcriber,
// so they run on the default feature set.
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    /// Serializes tests in this module: they share the process-wide `BACKEND`
    /// cache and registered transcriber, and mutate `VOX_ORATIO_BACKEND` /
    /// `VOX_TEST_FORCE_BACKEND_FAIL` env vars, so they cannot run concurrently.
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Stands in for the `oratio` plugin: decodes the f32 LE bytes and the
    /// config JSON back, and reports what it received in the plugin's output shape.
    fn fake_transcriber(pcm_le_f32: &[u8], config_json: &str) -> Result<String, String> {
        let samples: Vec<f32> = pcm_le_f32
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c))
            .collect();
        let cfg: serde_json::Value =
            serde_json::from_str(config_json).map_err(|e| e.to_string())?;
        let sr = cfg["sample_rate"].as_u64().ok_or("sample_rate missing")?;
        let lang = cfg["language"].as_str().unwrap_or("none");
        Ok(serde_json::json!({
            "text": format!("n={} sr={sr} lang={lang}", samples.len()),
            "segments": [{"start_ms": 0, "end_ms": 10, "text": "x"}],
        })
        .to_string())
    }

    #[test]
    fn registered_transcriber_serves_whisper_selection() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        register_whisper_transcriber(fake_transcriber);
        // SAFETY: test-only env mutation, serialized by TEST_LOCK.
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("VOX_TEST_FORCE_BACKEND_FAIL");
            std::env::set_var("VOX_ORATIO_BACKEND", "whisper");
        }

        let out = with_cached_backend(|b| b.transcribe_pcm(&[0.5, -0.25, 1.0], 16000, Some("en")));

        // SAFETY: test-only env mutation, serialized by TEST_LOCK.
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("VOX_ORATIO_BACKEND");
        }
        clear_whisper_transcriber_for_test();

        let out = out.expect("registered transcriber should serve the whisper selection");
        assert_eq!(out.raw_text, "n=3 sr=16000 lang=en");
        assert_eq!(
            out.segments,
            vec![TimedSegment {
                start_ms: 0,
                end_ms: 10,
                text: "x".to_string()
            }]
        );
        assert_eq!(out.confidence, 0.85);
        assert!(out.n_best.is_empty());
    }

    #[test]
    fn parse_plugin_transcription_contract() {
        let text_only = parse_plugin_transcription(r#"{"text":"hi","language":"en"}"#)
            .expect("text-only output is valid");
        assert_eq!(text_only.raw_text, "hi");
        assert!(text_only.segments.is_empty());

        assert!(
            parse_plugin_transcription(r#"{"segments":[]}"#).is_err(),
            "missing text"
        );
        assert!(
            parse_plugin_transcription("not json").is_err(),
            "invalid JSON"
        );
        assert!(
            parse_plugin_transcription(r#"{"text":"hi","segments":[{"start_ms":"a"}]}"#).is_err(),
            "malformed segment"
        );
    }

    #[test]
    fn unregistered_whisper_is_an_actionable_error() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        clear_whisper_transcriber_for_test();
        let err = whisper_backend()
            .err()
            .expect("no transcriber registered: must error");
        assert!(
            format!("{err:#}").contains("vox plugin install oratio"),
            "error should say how to install the plugin: {err:#}"
        );
    }

    #[test]
    fn with_cached_backend_constructs_once() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        register_whisper_transcriber(fake_transcriber);
        reset_cache_for_test();
        // SAFETY: test-only env mutation, serialized by TEST_LOCK.
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("VOX_TEST_FORCE_BACKEND_FAIL");
            std::env::set_var("VOX_ORATIO_BACKEND", "whisper");
        }

        let before = CREATE_BACKEND_CALL_COUNT.load(Ordering::SeqCst);
        with_cached_backend(|_backend| Ok(())).expect("first call should construct backend");
        with_cached_backend(|_backend| Ok(())).expect("second call should reuse cached backend");
        let after = CREATE_BACKEND_CALL_COUNT.load(Ordering::SeqCst);

        // SAFETY: test-only env mutation, serialized by TEST_LOCK.
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("VOX_ORATIO_BACKEND");
        }
        clear_whisper_transcriber_for_test();

        assert_eq!(
            after - before,
            1,
            "create_backend() should run exactly once across two with_cached_backend calls"
        );
    }

    #[test]
    fn with_cached_backend_retries_after_failure() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        register_whisper_transcriber(fake_transcriber);
        reset_cache_for_test();
        // SAFETY: test-only env mutation, serialized by TEST_LOCK.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("VOX_ORATIO_BACKEND", "whisper");
            std::env::set_var("VOX_TEST_FORCE_BACKEND_FAIL", "1");
        }

        let first = with_cached_backend(|_backend| Ok(()));
        assert!(first.is_err(), "forced failure should propagate as Err");

        // "Fix" the condition that caused the failure.
        // SAFETY: test-only env mutation, serialized by TEST_LOCK.
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("VOX_TEST_FORCE_BACKEND_FAIL");
        }

        let second = with_cached_backend(|_backend| Ok(()));

        // SAFETY: test-only env mutation, serialized by TEST_LOCK.
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("VOX_ORATIO_BACKEND");
        }
        clear_whisper_transcriber_for_test();

        assert!(
            second.is_ok(),
            "a failed construction must not permanently latch the cache into a failure \
             state; the next call should retry create_backend() from scratch: {:?}",
            second.err()
        );
    }

    /// Regression test for the bug the `Box` -> `Arc` cache conversion fixed:
    /// `with_cached_backend` must release its lock before running `f`, so two
    /// concurrent calls don't serialize on each other's (potentially slow)
    /// inference. Proven via a deadlock construction: both threads' closures
    /// must reach a 2-party barrier to complete. If the lock were still held
    /// across `f` (the old `Box`-based bug), the second thread could not even
    /// *enter* its closure until the first thread's closure returns — but the
    /// first thread's closure is itself blocked on the barrier waiting for the
    /// second thread to arrive. That's a genuine deadlock, not just slowness,
    /// so a regression here fails by hanging rather than by a flaky timing
    /// assumption. Bounded by a channel + `recv_timeout` so CI fails fast
    /// (as a normal test failure) instead of hanging indefinitely.
    #[test]
    fn with_cached_backend_does_not_serialize_concurrent_calls() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        register_whisper_transcriber(fake_transcriber);
        reset_cache_for_test();
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("VOX_TEST_FORCE_BACKEND_FAIL");
            std::env::set_var("VOX_ORATIO_BACKEND", "whisper");
        }
        // Warm the cache first so both threads below hit the "already
        // constructed" fast path, isolating the property under test (lock
        // scope around `f`) from construction itself.
        with_cached_backend(|_backend| Ok(())).expect("warm the cache");

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let (tx, rx) = std::sync::mpsc::channel();
        for _ in 0..2 {
            let barrier = barrier.clone();
            let tx = tx.clone();
            std::thread::spawn(move || {
                let result = with_cached_backend(|_backend| {
                    barrier.wait();
                    Ok(())
                });
                let _ = tx.send(result.is_ok());
            });
        }

        for _ in 0..2 {
            let ok = rx.recv_timeout(vox_config::timeouts::D_5S).expect(
                "both with_cached_backend calls should complete within 5s; a hang here \
                     means the lock is being held across `f` again (the bug the Arc \
                     conversion fixed), deadlocking both threads on the shared barrier",
            );
            assert!(ok, "with_cached_backend call failed unexpectedly");
        }

        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("VOX_ORATIO_BACKEND");
        }
        clear_whisper_transcriber_for_test();
    }

    // Asserts the "auto" mode falls back to the registered Whisper transcriber
    // when Sherpa init fails — the Sherpa arm only exists with `stt-sherpa`.
    #[cfg(feature = "stt-sherpa")]
    #[test]
    fn create_backend_auto_falls_back_to_whisper_when_sherpa_init_fails() {
        // Held for the duration of the test: see `crate::env_test_lock` (Task
        // 4 Step 1) — this env var is also mutated by sherpa_model_config's test.
        let _sherpa_env_guard = crate::env_test_lock::SHERPA_MODEL_DIR_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _guard = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        register_whisper_transcriber(fake_transcriber);

        let dir = tempfile::tempdir().expect("tempdir");
        // Deliberately empty: no real ONNX model files, so Sherpa-ONNX init
        // fails fast and offline. VOX_ORATIO_SHERPA_MODEL_DIR being set means
        // `resolve_sherpa_transducer_model_paths` short-circuits before the
        // HF Hub network branch entirely (see Task 4) — no network I/O here.
        // SAFETY: test-only env mutation, serialized by the locks above.
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("VOX_ORATIO_BACKEND");
            std::env::set_var("VOX_ORATIO_SHERPA_MODEL_DIR", dir.path());
        }
        let result = create_backend();
        // SAFETY: test-only env mutation, serialized by the locks above.
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("VOX_ORATIO_SHERPA_MODEL_DIR");
        }
        clear_whisper_transcriber_for_test();
        match result {
            Ok(backend) => assert_eq!(backend.name(), "whisper (oratio plugin)"),
            Err(e) => panic!(
                "auto mode must fall back to the registered Whisper transcriber when \
                 Sherpa-ONNX init fails (empty model dir), not propagate the Sherpa error: {e:#}"
            ),
        }
    }
}
