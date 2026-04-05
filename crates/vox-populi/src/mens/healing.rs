//! Healing loop: iterative LLM-based repair of `.vox` source until compile-clean.
//!
//! Architecture: `HealingLoop` wraps two closures — a *check* function (runs the
//! Vox compiler) and a *repair* function (calls the LLM with the grammar prompt +
//! error diagnostics). Each is synchronous from the healing loop's perspective;
//! async wrappers live at the call site.
//!
//! Pair logging: every successful repair is appended to
//! `~/.vox/corpus/heal_pairs.jsonl` for offline fine-tuning (W3-07).

use serde::{Deserialize, Serialize};

// ── Public types ─────────────────────────────────────────────────────────────

/// Result of a single compile-check pass on a source string.
#[derive(Debug, Clone)]
pub struct HealResult {
    /// `true` when the source passed compiler validation with no errors.
    pub ok: bool,
    /// Human-readable diagnostic messages (one per compiler error).
    pub diagnostics: Vec<String>,
}

/// A (failed → repaired) training pair collected during live healing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealPair {
    /// Natural-language task description that was used to generate the code.
    pub description: String,
    /// The source string that failed compilation.
    pub failed_source: String,
    /// Compiler diagnostics that accompanied the failure.
    pub diagnostics: Vec<String>,
    /// The successfully compiled source after repair.
    pub repaired_source: String,
    /// How many LLM calls were required before success.
    pub attempts: usize,
    /// Unix timestamp in milliseconds when the pair was collected.
    pub timestamp_unix_ms: u64,
}

/// Final outcome of a `HealingLoop::heal` call.
#[derive(Debug)]
pub enum HealOutcome {
    /// Source compiled cleanly within `max_attempts`.
    Success {
        source: String,
        /// Number of repair calls made (1 = first attempt was already valid).
        attempts: usize,
    },
    /// All attempts exhausted without producing a valid source.
    Exhausted {
        last_source: String,
        last_errors: Vec<String>,
    },
}

/// An iterative repair loop that drives an LLM to fix broken `.vox` source.
///
/// ## Example
/// ```rust,ignore
/// let loop_ = HealingLoop::new(
///     3,
///     |src| { /* run vox compiler */ HealResult { ok: true, diagnostics: vec![] } },
///     |src, errs| { /* call LLM */ Ok(repaired_src) },
/// );
/// match loop_.heal("A CRUD API for tasks", initial_source).await {
///     HealOutcome::Success { source, attempts } => println!("Healed in {attempts} attempt(s)"),
///     HealOutcome::Exhausted { .. } => eprintln!("Could not heal"),
/// }
/// ```
pub struct HealingLoop {
    /// Maximum number of LLM repair calls before giving up.
    max_attempts: usize,
    /// Synchronous check function: run the Vox compiler, return pass/fail + diagnostics.
    check_fn: Box<dyn Fn(&str) -> HealResult + Send + Sync>,
    /// Synchronous repair function: given failed source + diagnostics, return the next candidate.
    repair_fn: Box<dyn Fn(&str, &[String]) -> anyhow::Result<String> + Send + Sync>,
}

impl HealingLoop {
    /// Construct a new healing loop.
    ///
    /// - `max_attempts`: maximum LLM calls (default 3 is a good starting point).
    /// - `check_fn`: wraps `vox check` / the compiler pipeline.
    /// - `repair_fn`: wraps the LLM call with the grammar prompt + error context.
    pub fn new(
        max_attempts: usize,
        check_fn: impl Fn(&str) -> HealResult + Send + Sync + 'static,
        repair_fn: impl Fn(&str, &[String]) -> anyhow::Result<String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            max_attempts,
            check_fn: Box::new(check_fn),
            repair_fn: Box::new(repair_fn),
        }
    }

    /// Attempt to heal `source` until it compiles cleanly or `max_attempts` is exhausted.
    ///
    /// On success, appends a [`HealPair`] to the corpus file if `description` is provided.
    pub async fn heal(&self, description: &str, source: &str) -> HealOutcome {
        let mut current_source = source.to_string();

        for attempt in 1..=self.max_attempts {
            let result = (self.check_fn)(&current_source);
            if result.ok {
                if attempt > 1 {
                    // Only log when at least one repair call was made
                    let _ = self.log_pair(HealPair {
                        description: description.to_string(),
                        failed_source: source.to_string(),
                        diagnostics: result.diagnostics.clone(),
                        repaired_source: current_source.clone(),
                        attempts: attempt - 1,
                        timestamp_unix_ms: unix_ms_now(),
                    });
                }
                return HealOutcome::Success {
                    source: current_source,
                    attempts: attempt,
                };
            }

            if attempt == self.max_attempts {
                return HealOutcome::Exhausted {
                    last_source: current_source,
                    last_errors: result.diagnostics,
                };
            }

            // Ask the LLM to repair
            match (self.repair_fn)(&current_source, &result.diagnostics) {
                Ok(repaired) => current_source = repaired,
                Err(e) => {
                    tracing::warn!(
                        attempt,
                        %e,
                        "HealingLoop: repair_fn failed; abandoning"
                    );
                    return HealOutcome::Exhausted {
                        last_source: current_source,
                        last_errors: result.diagnostics,
                    };
                }
            }
        }

        // Unreachable — loop always returns inside the body.
        HealOutcome::Exhausted {
            last_source: current_source,
            last_errors: vec!["max_attempts loop fell through".to_string()],
        }
    }

    /// Append a [`HealPair`] to `~/.vox/corpus/heal_pairs.jsonl`.
    fn log_pair(&self, pair: HealPair) -> anyhow::Result<()> {
        let dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine home dir for corpus logging"))?
            .join(".vox")
            .join("corpus");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("heal_pairs.jsonl");
        let line = serde_json::to_string(&pair)?;
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        writeln!(f, "{line}")?;
        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn unix_ms_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
