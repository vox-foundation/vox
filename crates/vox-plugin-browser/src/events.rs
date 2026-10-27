//! Per-page CDP event capture: console messages, uncaught exceptions, network
//! requests and JavaScript dialogs. Buffers are bounded so a chatty page cannot
//! grow plugin memory without limit.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use chromiumoxide::Page;
use chromiumoxide_cdp::cdp::browser_protocol::network::{
    EventLoadingFailed, EventRequestWillBeSent, EventResponseReceived,
};
use chromiumoxide_cdp::cdp::browser_protocol::page::{
    EventJavascriptDialogClosed, EventJavascriptDialogOpening, HandleJavaScriptDialogParams,
};
use chromiumoxide_cdp::cdp::js_protocol::runtime::{
    EventConsoleApiCalled, EventExceptionThrown, RemoteObject,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

use crate::engine::BrowserEngine;

const MAX_CONSOLE_ENTRIES: usize = 200;
const MAX_NETWORK_ENTRIES: usize = 300;
const MAX_TEXT_CHARS: usize = 2_000;
const DEFAULT_QUERY_LIMIT: usize = 50;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ConsoleEntry {
    pub(crate) seq: u64,
    pub(crate) level: String,
    pub(crate) text: String,
    /// `console` for `console.*` calls, `exception` for uncaught errors.
    pub(crate) source: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct NetworkEntry {
    pub(crate) seq: u64,
    pub(crate) request_id: String,
    pub(crate) method: String,
    pub(crate) url: String,
    pub(crate) resource_type: Option<String>,
    pub(crate) status: Option<i64>,
    pub(crate) mime_type: Option<String>,
    pub(crate) failed: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct DialogInfo {
    pub(crate) kind: String,
    pub(crate) message: String,
    pub(crate) default_prompt: Option<String>,
    pub(crate) url: String,
}

#[derive(Debug, Default)]
pub(crate) struct PageEventLog {
    next_seq: u64,
    console: VecDeque<ConsoleEntry>,
    network: VecDeque<NetworkEntry>,
    dialog: Option<DialogInfo>,
}

/// `page_events` options. Empty JSON returns the latest console and network
/// entries plus any open dialog.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct EventQuery {
    /// Only entries with `seq > since_seq`.
    pub(crate) since_seq: u64,
    /// Max entries per kind (newest kept); 0 means the default of 50.
    pub(crate) limit: usize,
    /// Subset of `console`, `network`; empty means both.
    pub(crate) kinds: Vec<String>,
    /// Console level filter, e.g. `error`.
    pub(crate) level: Option<String>,
    /// Case-insensitive URL substring filter for network entries.
    pub(crate) url_contains: Option<String>,
    /// Drop buffered entries after reading.
    pub(crate) clear: bool,
}

fn truncate(text: &str) -> String {
    if text.chars().count() <= MAX_TEXT_CHARS {
        return text.to_string();
    }
    let mut out: String = text.chars().take(MAX_TEXT_CHARS - 1).collect();
    out.push('…');
    out
}

fn push_bounded<T>(buf: &mut VecDeque<T>, item: T, cap: usize) {
    if buf.len() == cap {
        buf.pop_front();
    }
    buf.push_back(item);
}

fn enum_label<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(s)) => s,
        Ok(other) => other.to_string(),
        Err(_) => "unknown".to_string(),
    }
}

fn remote_object_text(obj: &RemoteObject) -> String {
    match &obj.value {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(value) => value.to_string(),
        None => obj
            .description
            .clone()
            .unwrap_or_else(|| enum_label(&obj.r#type)),
    }
}

impl PageEventLog {
    fn bump(&mut self) -> u64 {
        self.next_seq += 1;
        self.next_seq
    }

    pub(crate) fn push_console(&mut self, level: &str, text: &str, source: &'static str) {
        let seq = self.bump();
        let entry = ConsoleEntry {
            seq,
            level: level.to_string(),
            text: truncate(text),
            source,
        };
        push_bounded(&mut self.console, entry, MAX_CONSOLE_ENTRIES);
    }

    pub(crate) fn on_request(
        &mut self,
        request_id: &str,
        method: &str,
        url: &str,
        resource_type: Option<String>,
    ) {
        let seq = self.bump();
        let entry = NetworkEntry {
            seq,
            request_id: request_id.to_string(),
            method: method.to_string(),
            url: truncate(url),
            resource_type,
            status: None,
            mime_type: None,
            failed: None,
        };
        push_bounded(&mut self.network, entry, MAX_NETWORK_ENTRIES);
    }

    fn latest_request_mut(&mut self, request_id: &str) -> Option<&mut NetworkEntry> {
        self.network
            .iter_mut()
            .rev()
            .find(|entry| entry.request_id == request_id)
    }

    pub(crate) fn on_response(&mut self, request_id: &str, status: i64, mime_type: &str) {
        if let Some(entry) = self.latest_request_mut(request_id) {
            entry.status = Some(status);
            entry.mime_type = Some(mime_type.to_string());
        }
    }

    pub(crate) fn on_failed(&mut self, request_id: &str, error_text: &str) {
        if let Some(entry) = self.latest_request_mut(request_id) {
            entry.failed = Some(truncate(error_text));
        }
    }

    pub(crate) fn set_dialog(&mut self, dialog: Option<DialogInfo>) {
        self.dialog = dialog;
    }

    pub(crate) fn dialog(&self) -> Option<DialogInfo> {
        self.dialog.clone()
    }

    pub(crate) fn query(&mut self, q: &EventQuery) -> serde_json::Value {
        let wants = |kind: &str| q.kinds.is_empty() || q.kinds.iter().any(|k| k == kind);
        let limit = if q.limit == 0 {
            DEFAULT_QUERY_LIMIT
        } else {
            q.limit
        };
        let console: Vec<&ConsoleEntry> = if wants("console") {
            let level = q.level.as_deref().map(str::to_ascii_lowercase);
            let matched: Vec<_> = self
                .console
                .iter()
                .filter(|e| e.seq > q.since_seq)
                .filter(|e| level.as_deref().is_none_or(|l| e.level == l))
                .collect();
            matched[matched.len().saturating_sub(limit)..].to_vec()
        } else {
            Vec::new()
        };
        let network: Vec<&NetworkEntry> = if wants("network") {
            let needle = q.url_contains.as_deref().map(str::to_ascii_lowercase);
            let matched: Vec<_> = self
                .network
                .iter()
                .filter(|e| e.seq > q.since_seq)
                .filter(|e| {
                    needle
                        .as_deref()
                        .is_none_or(|n| e.url.to_ascii_lowercase().contains(n))
                })
                .collect();
            matched[matched.len().saturating_sub(limit)..].to_vec()
        } else {
            Vec::new()
        };
        let out = serde_json::json!({
            "console": console,
            "network": network,
            "dialog": self.dialog,
            "next_seq": self.next_seq,
        });
        if q.clear {
            self.console.clear();
            self.network.clear();
        }
        out
    }
}

/// Event log plus the listener tasks feeding it; dropping it stops the tasks.
pub(crate) struct PageEvents {
    pub(crate) log: Arc<Mutex<PageEventLog>>,
    tasks: Vec<JoinHandle<()>>,
}

impl Drop for PageEvents {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

fn with_log(log: &Arc<Mutex<PageEventLog>>, f: impl FnOnce(&mut PageEventLog)) {
    if let Ok(mut guard) = log.lock() {
        f(&mut guard);
    }
}

impl PageEvents {
    /// Subscribe to the page's console, exception, network and dialog events.
    /// Must run inside the plugin's Tokio runtime.
    pub(crate) async fn attach(page: &Page) -> Result<Self, String> {
        let log = Arc::new(Mutex::new(PageEventLog::default()));
        let err = |e: chromiumoxide::error::CdpError| format!("event listener: {e}");
        let mut tasks = Vec::new();

        let mut console = page
            .event_listener::<EventConsoleApiCalled>()
            .await
            .map_err(err)?;
        let l = log.clone();
        tasks.push(tokio::spawn(async move {
            while let Some(ev) = console.next().await {
                let text = ev
                    .args
                    .iter()
                    .map(remote_object_text)
                    .collect::<Vec<_>>()
                    .join(" ");
                let level = enum_label(&ev.r#type);
                with_log(&l, |log| log.push_console(&level, &text, "console"));
            }
        }));

        let mut exceptions = page
            .event_listener::<EventExceptionThrown>()
            .await
            .map_err(err)?;
        let l = log.clone();
        tasks.push(tokio::spawn(async move {
            while let Some(ev) = exceptions.next().await {
                let details = &ev.exception_details;
                let text = details
                    .exception
                    .as_ref()
                    .and_then(|obj| obj.description.clone())
                    .unwrap_or_else(|| details.text.clone());
                with_log(&l, |log| log.push_console("error", &text, "exception"));
            }
        }));

        let mut requests = page
            .event_listener::<EventRequestWillBeSent>()
            .await
            .map_err(err)?;
        let l = log.clone();
        tasks.push(tokio::spawn(async move {
            while let Some(ev) = requests.next().await {
                let kind = ev.r#type.as_ref().map(enum_label);
                with_log(&l, |log| {
                    log.on_request(
                        ev.request_id.inner(),
                        &ev.request.method,
                        &ev.request.url,
                        kind,
                    )
                });
            }
        }));

        let mut responses = page
            .event_listener::<EventResponseReceived>()
            .await
            .map_err(err)?;
        let l = log.clone();
        tasks.push(tokio::spawn(async move {
            while let Some(ev) = responses.next().await {
                with_log(&l, |log| {
                    log.on_response(
                        ev.request_id.inner(),
                        ev.response.status,
                        &ev.response.mime_type,
                    )
                });
            }
        }));

        let mut failures = page
            .event_listener::<EventLoadingFailed>()
            .await
            .map_err(err)?;
        let l = log.clone();
        tasks.push(tokio::spawn(async move {
            while let Some(ev) = failures.next().await {
                with_log(&l, |log| {
                    log.on_failed(ev.request_id.inner(), &ev.error_text)
                });
            }
        }));

        let mut dialogs = page
            .event_listener::<EventJavascriptDialogOpening>()
            .await
            .map_err(err)?;
        let l = log.clone();
        tasks.push(tokio::spawn(async move {
            while let Some(ev) = dialogs.next().await {
                let info = DialogInfo {
                    kind: enum_label(&ev.r#type),
                    message: truncate(&ev.message),
                    default_prompt: ev.default_prompt.clone(),
                    url: truncate(&ev.url),
                };
                with_log(&l, |log| log.set_dialog(Some(info)));
            }
        }));

        let mut closed = page
            .event_listener::<EventJavascriptDialogClosed>()
            .await
            .map_err(err)?;
        let l = log.clone();
        tasks.push(tokio::spawn(async move {
            while closed.next().await.is_some() {
                with_log(&l, |log| log.set_dialog(None));
            }
        }));

        Ok(Self { log, tasks })
    }
}

/// Response for actions refused because a JavaScript dialog blocks the page.
pub(crate) fn dialog_open_response(dialog: &DialogInfo) -> serde_json::Value {
    serde_json::json!({
        "ok": false,
        "reason": "dialog_open",
        "dialog": dialog,
        "hint": "call handle_dialog to accept or dismiss it first",
    })
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct DialogAction {
    pub(crate) accept: bool,
    pub(crate) prompt_text: Option<String>,
}

impl BrowserEngine {
    fn event_log(
        &self,
        guard: &crate::hosts::HostRegistry,
        page_id: &str,
    ) -> Result<Arc<Mutex<PageEventLog>>, String> {
        guard
            .host_for_page(page_id)?
            .page_events
            .get(page_id)
            .map(|events| events.log.clone())
            .ok_or_else(|| format!("no event log for page_id {page_id:?}"))
    }

    pub async fn page_events(
        &self,
        page_id: &str,
        query: &EventQuery,
    ) -> Result<serde_json::Value, String> {
        let log = {
            let guard = self.host.lock().await;
            self.event_log(&guard, page_id)?
        };
        let mut log = log.lock().map_err(|_| "event log poisoned".to_string())?;
        Ok(log.query(query))
    }

    pub(crate) async fn pending_dialog(&self, page_id: &str) -> Option<DialogInfo> {
        let guard = self.host.lock().await;
        let log = self.event_log(&guard, page_id).ok()?;
        let dialog = log.lock().ok()?.dialog();
        dialog
    }

    pub async fn handle_dialog(
        &self,
        page_id: &str,
        action: &DialogAction,
    ) -> Result<serde_json::Value, String> {
        let Some(dialog) = self.pending_dialog(page_id).await else {
            return Ok(serde_json::json!({ "ok": false, "reason": "no_dialog" }));
        };
        let page = self.page_ref(page_id).await?;
        let mut params = HandleJavaScriptDialogParams::new(action.accept);
        params.prompt_text = action.prompt_text.clone();
        page.execute(params).await.map_err(Self::map_page_err)?;
        {
            let guard = self.host.lock().await;
            if let Ok(log) = self.event_log(&guard, page_id) {
                with_log(&log, |log| log.set_dialog(None));
            }
        }
        Ok(serde_json::json!({ "ok": true, "accepted": action.accept, "dialog": dialog }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_buffer_is_bounded_and_keeps_newest() {
        let mut log = PageEventLog::default();
        for i in 0..(MAX_CONSOLE_ENTRIES + 5) {
            log.push_console("log", &format!("m{i}"), "console");
        }
        assert_eq!(log.console.len(), MAX_CONSOLE_ENTRIES);
        assert_eq!(log.console.front().map(|e| e.text.as_str()), Some("m5"));
    }

    #[test]
    fn long_text_is_truncated_by_chars() {
        let mut log = PageEventLog::default();
        log.push_console("log", &"é".repeat(MAX_TEXT_CHARS + 10), "console");
        let text = &log.console[0].text;
        assert_eq!(text.chars().count(), MAX_TEXT_CHARS);
        assert!(text.ends_with('…'));
    }

    #[test]
    fn response_and_failure_attach_to_latest_request_with_same_id() {
        let mut log = PageEventLog::default();
        log.on_request("r1", "GET", "https://a.test/redirect", None);
        log.on_request("r1", "GET", "https://a.test/final", Some("Document".into()));
        log.on_request("r2", "POST", "https://a.test/api", Some("Fetch".into()));
        log.on_response("r1", 200, "text/html");
        log.on_failed("r2", "net::ERR_FAILED");
        assert_eq!(log.network[0].status, None);
        assert_eq!(log.network[1].status, Some(200));
        assert_eq!(log.network[2].failed.as_deref(), Some("net::ERR_FAILED"));
    }

    #[test]
    fn query_filters_by_seq_level_url_and_limit() {
        let mut log = PageEventLog::default();
        log.push_console("log", "hello", "console");
        log.push_console("error", "boom", "exception");
        log.on_request("r1", "GET", "https://cdn.test/app.js", None);
        log.on_request("r2", "GET", "https://api.test/users", None);

        let errors = log.query(&EventQuery {
            level: Some("ERROR".into()),
            kinds: vec!["console".into()],
            ..Default::default()
        });
        assert_eq!(errors["console"].as_array().map(Vec::len), Some(1));
        assert_eq!(errors["network"].as_array().map(Vec::len), Some(0));

        let api = log.query(&EventQuery {
            url_contains: Some("API.test".into()),
            ..Default::default()
        });
        assert_eq!(api["network"][0]["request_id"], "r2");

        let newer = log.query(&EventQuery {
            since_seq: 3,
            ..Default::default()
        });
        assert_eq!(newer["network"].as_array().map(Vec::len), Some(1));
        assert_eq!(newer["next_seq"], 4);

        let last = log.query(&EventQuery {
            limit: 1,
            ..Default::default()
        });
        assert_eq!(last["console"][0]["text"], "boom");
    }

    #[test]
    fn clear_drops_entries_but_keeps_sequence_and_dialog() {
        let mut log = PageEventLog::default();
        log.push_console("log", "a", "console");
        log.set_dialog(Some(DialogInfo {
            kind: "alert".into(),
            message: "hi".into(),
            default_prompt: None,
            url: "about:blank".into(),
        }));
        log.query(&EventQuery {
            clear: true,
            ..Default::default()
        });
        let after = log.query(&EventQuery::default());
        assert_eq!(after["console"].as_array().map(Vec::len), Some(0));
        assert_eq!(after["next_seq"], 1);
        assert_eq!(after["dialog"]["kind"], "alert");
    }

    #[test]
    fn dialog_open_response_names_the_dialog() {
        let dialog = DialogInfo {
            kind: "confirm".into(),
            message: "Leave?".into(),
            default_prompt: None,
            url: "https://a.test".into(),
        };
        let out = dialog_open_response(&dialog);
        assert_eq!(out["reason"], "dialog_open");
        assert_eq!(out["dialog"]["message"], "Leave?");
    }
}
