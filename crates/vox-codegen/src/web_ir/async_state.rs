//! TypeScript / TSX emit for `Async[T]` view-arm expressions (GA-01).
//!
//! Wire-format-v1 encodes `Async[T]` as a `_tag`-discriminated union:
//! ```ts
//! type Async<T> =
//!   | { _tag: "fetching" }
//!   | { _tag: "empty" }
//!   | { _tag: "error"; error: unknown }
//!   | { _tag: "ok"; value: T };
//! ```
//!
//! The emitter narrows on `_tag` and renders the matching TSX subtree.
//! Generated output is a self-contained `(() => { … })()` IIFE so the
//! expression can be embedded anywhere JSX accepts an expression.

use vox_compiler::hir::nodes::async_view::HirAsyncView;

/// TypeScript type alias for `Async<T>` — emitted once per file that uses `Async[T]`.
pub const ASYNC_TYPE_ALIAS: &str = r#"/** Vox Async[T] — wire-format-v1 discriminated union. */
export type Async<T> =
  | { readonly _tag: "fetching" }
  | { readonly _tag: "empty" }
  | { readonly _tag: "error"; readonly error: unknown }
  | { readonly _tag: "ok"; readonly value: T };
"#;

/// Emit a TSX expression that narrows an `Async[T]` value and renders the matching arm.
///
/// `source_tsx` — already-emitted TSX expression for the `Async[T]` value.
/// `fetching_tsx` — JSX subtree for the `fetching` arm.
/// `empty_tsx` — JSX subtree for the `empty` arm.
/// `error_binding` — identifier to bind the error value in the error arm.
/// `error_tsx` — JSX subtree for the `error` arm.
/// `ok_binding` — identifier to bind the resolved value in the ok arm.
/// `ok_tsx` — JSX subtree for the `ok` arm.
#[must_use]
pub fn emit_async_view_tsx(
    source_tsx: &str,
    fetching_tsx: &str,
    empty_tsx: &str,
    error_binding: &str,
    error_tsx: &str,
    ok_binding: &str,
    ok_tsx: &str,
) -> String {
    format!(
        r#"(() => {{
  const _async = {source_tsx};
  if (_async._tag === "fetching") return ({fetching_tsx});
  if (_async._tag === "empty") return ({empty_tsx});
  if (_async._tag === "error") {{ const {error_binding} = _async.error; return ({error_tsx}); }}
  const {ok_binding} = _async.value; return ({ok_tsx});
}})()"#
    )
}

/// Validate that an `HirAsyncView` is exhaustive before emitting.
///
/// Returns `Err` with missing arm names if any arm is absent.
pub fn validate_async_view(view: &HirAsyncView) -> Result<(), Vec<&'static str>> {
    let missing = view.missing_arms();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

/// First-class `<WaitingNotice>` view contract (`RANK-02` / `A1-04`).
///
/// Replaces blank screens during rate limits, cooldowns, or external service
/// backoff with an explicit human-readable reason, next-resume timestamp, and
/// actionable user link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingNoticeSpec<'a> {
    pub reason: &'a str,
    pub resume_at: &'a str,
    pub action_href: &'a str,
    pub action_label: &'a str,
}

/// Validate and emit a `<WaitingNotice>` TSX fragment (`RANK-02`).
///
/// Rejects blank `reason`, `resume_at`, or `action_href` so rate-limit/cooldown
/// states never degrade into a blank screen.
pub fn emit_waiting_notice_tsx(spec: &WaitingNoticeSpec<'_>) -> Result<String, &'static str> {
    if spec.reason.trim().is_empty() {
        return Err("WaitingNotice requires a non-empty `reason`");
    }
    if spec.resume_at.trim().is_empty() {
        return Err("WaitingNotice requires a non-empty `resume_at` timestamp or duration");
    }
    if spec.action_href.trim().is_empty() {
        return Err("WaitingNotice requires a non-empty `action_href` recovery link");
    }
    let label = if spec.action_label.trim().is_empty() {
        "Manage schedule & limits"
    } else {
        spec.action_label
    };
    Ok(format!(
        r#"<aside role="status" aria-live="polite" data-vox-waiting-notice="true" data-testid="vox-waiting-notice" className="vox-waiting-notice">
  <p className="vox-waiting-reason">{reason}</p>
  <p className="vox-waiting-resume">Resumes at <time dateTime="{resume_at}">{resume_at}</time></p>
  <a className="vox-waiting-action" href="{action_href}">{label}</a>
</aside>"#,
        reason = spec.reason,
        resume_at = spec.resume_at,
        action_href = spec.action_href,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_iife_with_all_arms() {
        let tsx = emit_async_view_tsx(
            "userData",
            "<Spinner />",
            "<EmptyState />",
            "e",
            "<ErrorBanner error={e} />",
            "user",
            "<UserCard user={user} />",
        );
        assert!(tsx.contains("userData"));
        assert!(tsx.contains("_tag === \"fetching\""));
        assert!(tsx.contains("_tag === \"empty\""));
        assert!(tsx.contains("_tag === \"error\""));
        assert!(tsx.contains("const e = _async.error"));
        assert!(tsx.contains("const user = _async.value"));
        assert!(tsx.contains("<Spinner />"));
        assert!(tsx.contains("<UserCard user={user} />"));
    }

    #[test]
    fn async_type_alias_has_all_tags() {
        assert!(ASYNC_TYPE_ALIAS.contains("\"fetching\""));
        assert!(ASYNC_TYPE_ALIAS.contains("\"empty\""));
        assert!(ASYNC_TYPE_ALIAS.contains("\"error\""));
        assert!(ASYNC_TYPE_ALIAS.contains("\"ok\""));
    }

    #[test]
    fn wire_format_v1_tag_field_is_underscore_tag() {
        // Per wire-format-v1 SSOT: discriminant key is `_tag`, not `type` or `kind`.
        assert!(ASYNC_TYPE_ALIAS.contains("_tag"));
        assert!(!ASYNC_TYPE_ALIAS.contains("_kind"));
        assert!(!ASYNC_TYPE_ALIAS.contains(": \"type\""));
    }

    #[test]
    fn waiting_notice_tsx_requires_reason_resume_and_action() {
        let valid = WaitingNoticeSpec {
            reason: "Provider rate limit reached for synthesis tier",
            resume_at: "2026-10-15T01:15:00Z",
            action_href: "/settings/quotas",
            action_label: "Review quota settings",
        };
        let tsx = emit_waiting_notice_tsx(&valid).expect("valid waiting notice");
        assert!(tsx.contains("data-vox-waiting-notice=\"true\""));
        assert!(tsx.contains("Provider rate limit reached for synthesis tier"));
        assert!(tsx.contains("dateTime=\"2026-10-15T01:15:00Z\""));
        assert!(tsx.contains("href=\"/settings/quotas\""));

        let missing_reason = WaitingNoticeSpec {
            reason: " ",
            ..valid
        };
        assert!(emit_waiting_notice_tsx(&missing_reason).is_err());
    }
}
