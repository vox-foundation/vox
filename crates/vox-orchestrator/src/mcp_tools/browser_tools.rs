//! Chromium-backed browser MCP tools (`vox_browser_*`).
//!
//! Uses [`vox_browser`] (CDP / `chromiumoxide`). Playwright is not required.

use crate::mcp_tools::llm_bridge::call_llm;
use crate::mcp_tools::params::{
    BrowserActParams, BrowserExtractJsonParams, BrowserExtractParams, BrowserFillParams,
    BrowserGotoParams, BrowserHtmlParams, BrowserOpenParams, BrowserPageParams,
    BrowserScreenshotParams, BrowserTargetParams, BrowserWaitParams, ToolResult,
};
use crate::mcp_tools::server_state::ServerState;
use serde::Deserialize;
use vox_browser::global_engine;

fn summary_max_chars() -> usize {
    vox_clavis::resolve_secret(vox_clavis::SecretId::VoxBrowserLlmContextChars).expose()
        .and_then(|s| s.parse().ok())
        .unwrap_or(24_000)
}

pub async fn browser_open(_state: &ServerState, p: BrowserOpenParams) -> String {
    let eng = global_engine();
    match eng.open(&p.url, p.headless).await {
        Ok(page_id) => ToolResult::ok(serde_json::json!({
            "page_id": page_id,
            "url": p.url,
        }))
        .to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err_with_remediation(
            e,
            "Install Chromium/Chrome or set VOX_CHROME_EXECUTABLE; for containers try VOX_BROWSER_NO_SANDBOX=1.",
        )
        .to_json(),
    }
}

pub async fn browser_close(_state: &ServerState, p: BrowserPageParams) -> String {
    let eng = global_engine();
    match eng.close(&p.page_id).await {
        Ok(()) => ToolResult::ok(serde_json::json!({ "closed": true })).to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e).to_json(),
    }
}

pub async fn browser_goto(_state: &ServerState, p: BrowserGotoParams) -> String {
    let eng = global_engine();
    match eng.goto(&p.page_id, &p.url).await {
        Ok(()) => ToolResult::ok(serde_json::json!({ "ok": true })).to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e).to_json(),
    }
}

pub async fn browser_click(_state: &ServerState, p: BrowserTargetParams) -> String {
    let eng = global_engine();
    match eng.click(&p.page_id, &p.target).await {
        Ok(()) => ToolResult::ok(serde_json::json!({ "ok": true })).to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e).to_json(),
    }
}

pub async fn browser_fill(_state: &ServerState, p: BrowserFillParams) -> String {
    let eng = global_engine();
    match eng.fill(&p.page_id, &p.target, &p.value).await {
        Ok(()) => ToolResult::ok(serde_json::json!({ "ok": true })).to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e).to_json(),
    }
}

pub async fn browser_wait_for(_state: &ServerState, p: BrowserWaitParams) -> String {
    let eng = global_engine();
    match eng.wait_for(&p.page_id, &p.target, p.timeout_secs).await {
        Ok(()) => ToolResult::ok(serde_json::json!({ "ok": true })).to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e).to_json(),
    }
}

pub async fn browser_text(_state: &ServerState, p: BrowserTargetParams) -> String {
    let eng = global_engine();
    match eng.text(&p.page_id, &p.target).await {
        Ok(text) => ToolResult::ok(serde_json::json!({ "text": text })).to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e).to_json(),
    }
}

pub async fn browser_html(_state: &ServerState, p: BrowserHtmlParams) -> String {
    let eng = global_engine();
    match eng.html(&p.page_id, &p.target).await {
        Ok(html) => ToolResult::ok(serde_json::json!({ "html": html })).to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e).to_json(),
    }
}

pub async fn browser_screenshot(_state: &ServerState, p: BrowserScreenshotParams) -> String {
    let eng = global_engine();
    match eng.screenshot(&p.page_id, &p.path).await {
        Ok(path) => ToolResult::ok(serde_json::json!({ "path": path })).to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e).to_json(),
    }
}

pub async fn browser_extract(state: &ServerState, p: BrowserExtractParams) -> String {
    let eng = global_engine();
    let summary = match eng
        .visible_text_summary(&p.page_id, summary_max_chars())
        .await
    {
        Ok(s) => s,
        Err(e) => return ToolResult::<serde_json::Value>::err(e).to_json(),
    };
    let sys = "You help automate web pages. Answer ONLY with the extracted content requested — no preamble.";
    let user = format!(
        "Instruction:\n{}\n\nVisible page text (truncated):\n{}",
        p.instruction, summary
    );
    match call_llm(state, sys, &user, None).await {
        Ok((text, model, _)) => ToolResult::ok(serde_json::json!({
            "extraction": text,
            "model": model,
            "execution_mode": "assisted",
        }))
        .to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err_with_remediation(
            e,
            "Configure MCP chat model (`vox_list_models` / `vox_set_active_model`) or provider API keys via Clavis.",
        )
        .to_json(),
    }
}

pub async fn browser_extract_json(state: &ServerState, p: BrowserExtractJsonParams) -> String {
    let eng = global_engine();
    let summary = match eng
        .visible_text_summary(&p.page_id, summary_max_chars())
        .await
    {
        Ok(s) => s,
        Err(e) => return ToolResult::<serde_json::Value>::err(e).to_json(),
    };
    let sys = "Reply with a single JSON object only (no markdown fences). The object MUST validate informally against the schema description given.";
    let user = format!(
        "Schema (JSON Schema):\n{}\n\nTask:\n{}\n\nVisible page text:\n{}",
        p.schema_json, p.instruction, summary
    );
    match call_llm(state, sys, &user, None).await {
        Ok((text, model, _)) => {
            let trimmed = text.trim();
            let val: Result<serde_json::Value, _> = serde_json::from_str(trimmed);
            match val {
                Ok(v) => ToolResult::ok(serde_json::json!({
                    "data": v,
                    "model": model,
                    "execution_mode": "assisted",
                }))
                .to_json(),
                Err(e) => ToolResult::<serde_json::Value>::err(format!(
                    "model returned non-JSON: {e}; raw={trimmed:?}"
                ))
                .to_json(),
            }
        }
        Err(e) => ToolResult::<serde_json::Value>::err_with_remediation(
            e,
            "Configure MCP chat model and provider keys.",
        )
        .to_json(),
    }
}

#[derive(Debug, Deserialize)]
struct ActJson {
    #[allow(dead_code)]
    action: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

pub async fn browser_act(state: &ServerState, p: BrowserActParams) -> String {
    let eng = global_engine();
    let summary = match eng
        .visible_text_summary(&p.page_id, summary_max_chars())
        .await
    {
        Ok(s) => s,
        Err(e) => return ToolResult::<serde_json::Value>::err(e).to_json(),
    };
    let sys = r#"Reply with ONE JSON object only, no markdown. Shape:
{"action":"click"|"fill"|"goto"|"wait"|"noop","target":"css or xpath:... optional","value":"optional","url":"optional"}.
Use xpath: prefix in target for XPath. Choose the best next step for the instruction."#;
    let user = format!(
        "Goal:\n{}\n\nVisible page text:\n{}",
        p.instruction, summary
    );
    let Ok((text, model, _)) = call_llm(state, sys, &user, None).await else {
        return ToolResult::<serde_json::Value>::err_with_remediation(
            "LLM call failed (check model / keys)",
            "Configure MCP chat model (`vox_set_active_model`) and Clavis secrets.",
        )
        .to_json();
    };
    let trimmed = text.trim();
    let act: ActJson = match serde_json::from_str(trimmed) {
        Ok(a) => a,
        Err(e) => {
            return ToolResult::<serde_json::Value>::err(format!(
                "act: model JSON parse {e}; raw={trimmed:?}"
            ))
            .to_json();
        }
    };
    let action = act.action.to_lowercase();
    let res = match action.as_str() {
        "noop" => Ok(()),
        "goto" => {
            let Some(url) = act.url.as_deref().filter(|u| !u.is_empty()) else {
                return ToolResult::<serde_json::Value>::err("act goto requires url".to_string())
                    .to_json();
            };
            eng.goto(&p.page_id, url).await
        }
        "wait" => {
            let Some(t) = act.target.as_deref().filter(|s| !s.is_empty()) else {
                return ToolResult::<serde_json::Value>::err(
                    "act wait requires target".to_string(),
                )
                .to_json();
            };
            eng.wait_for(&p.page_id, t, 30).await
        }
        "click" => {
            let Some(t) = act.target.as_deref().filter(|s| !s.is_empty()) else {
                return ToolResult::<serde_json::Value>::err(
                    "act click requires target".to_string(),
                )
                .to_json();
            };
            eng.click(&p.page_id, t).await
        }
        "fill" => {
            let (Some(t), Some(v)) = (
                act.target.as_deref().filter(|s| !s.is_empty()),
                act.value.as_deref(),
            ) else {
                return ToolResult::<serde_json::Value>::err(
                    "act fill requires target and value".to_string(),
                )
                .to_json();
            };
            eng.fill(&p.page_id, t, v).await
        }
        _ => {
            return ToolResult::<serde_json::Value>::err(format!("unknown action {action:?}"))
                .to_json();
        }
    };
    match res {
        Ok(()) => ToolResult::ok(serde_json::json!({
            "ok": true,
            "action": action,
            "model": model,
            "execution_mode": "assisted",
        }))
        .to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e).to_json(),
    }
}
