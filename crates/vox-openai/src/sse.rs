//! UTF-8 SSE `data:` line assembly across arbitrary chunk boundaries and
//! delta extraction from OpenAI-shaped streaming JSON.
//!
//! **`eventsource-stream` / `reqwest-eventsource`:** deferred — we need lossy UTF-8 assembly and
//! OpenAI-shaped `delta.content` without assuming one provider’s event framing; evaluate again if
//! a crate exposes pluggable line assembly compatible with OpenRouter/HF chunking.

use serde_json::Value;

/// Accumulates UTF-8 chunks and invokes `on_line` for each newline-delimited line (CR stripped).
#[derive(Debug, Default)]
pub struct Utf8LineBuffer {
    buf: String,
}

impl Utf8LineBuffer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push lossy-decoded `bytes` and emit complete lines.
    pub fn push_lossy_bytes(&mut self, bytes: &[u8], mut on_line: impl FnMut(&str)) {
        self.buf.push_str(&String::from_utf8_lossy(bytes));
        while let Some(pos) = self.buf.find('\n') {
            let line = self.buf[..pos].trim_end_matches('\r').to_string();
            self.buf.drain(..=pos);
            on_line(line.as_str());
        }
    }

    /// Invoke `on_line` on any trailing fragment that did not end with `\n`, then clear.
    pub fn flush_trailing(&mut self, mut on_line: impl FnMut(&str)) {
        let tail = self.buf.trim();
        if !tail.is_empty() {
            on_line(tail);
        }
        self.buf.clear();
    }
}

/// Parse `data` JSON from one SSE `data: …` line (after the `data: ` prefix) and return delta content, if any.
#[must_use]
pub fn chat_completion_delta_content(data: &str) -> Option<String> {
    let v: Value = serde_json::from_str(data).ok()?;
    v["choices"].get(0)?["delta"]["content"]
        .as_str()
        .map(std::string::ToString::to_string)
}

/// Incremental tool-call delta fragment from an OpenAI-compatible SSE `delta.tool_calls[i]` object.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToolCallDelta {
    pub index: usize,
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments_chunk: Option<String>,
}

/// Parsed content, tool-call fragments, and/or final-chunk `usage` telemetry from a single SSE `data: {json}` frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SseDeltaEvent {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCallDelta>,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub cache_read_tokens: Option<u32>,
}

impl SseDeltaEvent {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.content.as_deref().unwrap_or("").is_empty()
            && self.tool_calls.is_empty()
            && self.prompt_tokens.is_none()
            && self.completion_tokens.is_none()
            && self.cache_read_tokens.is_none()
    }
}

/// Parse one SSE `data: {json}` line into content, `delta.tool_calls`, and/or `usage` telemetry.
#[must_use]
pub fn sse_data_line_delta_event(line: &str) -> Option<SseDeltaEvent> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let payload = line.strip_prefix("data: ")?.trim();
    if payload == "[DONE]" {
        return None;
    }
    let v: Value = serde_json::from_str(payload).ok()?;
    let delta = v
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("delta"));
    let content = delta
        .and_then(|d| d.get("content"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string);
    let mut tool_calls = Vec::new();
    if let Some(arr) = delta
        .and_then(|d| d.get("tool_calls"))
        .and_then(Value::as_array)
    {
        for (fallback_idx, entry) in arr.iter().enumerate() {
            let index = entry
                .get("index")
                .and_then(Value::as_u64)
                .map(|i| i as usize)
                .unwrap_or(fallback_idx);
            let id = entry
                .get("id")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(std::string::ToString::to_string);
            let func = entry.get("function");
            let name = func
                .and_then(|f| f.get("name"))
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(std::string::ToString::to_string);
            let arguments_chunk = func
                .and_then(|f| f.get("arguments"))
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(std::string::ToString::to_string);
            if id.is_some() || name.is_some() || arguments_chunk.is_some() {
                tool_calls.push(ToolCallDelta {
                    index,
                    id,
                    name,
                    arguments_chunk,
                });
            }
        }
    }
    let usage = v.get("usage").filter(|u| u.is_object());
    let prompt_tokens = usage
        .and_then(|u| u.get("prompt_tokens"))
        .and_then(Value::as_u64)
        .map(|n| n as u32);
    let completion_tokens = usage
        .and_then(|u| u.get("completion_tokens"))
        .and_then(Value::as_u64)
        .map(|n| n as u32);
    let cache_read_tokens = usage
        .and_then(|u| {
            u.get("cache_read_input_tokens")
                .and_then(Value::as_u64)
                .or_else(|| {
                    u.get("prompt_tokens_details")
                        .and_then(|d| d.get("cached_tokens"))
                        .and_then(Value::as_u64)
                })
        })
        .map(|n| n as u32);
    let ev = SseDeltaEvent {
        content,
        tool_calls,
        prompt_tokens,
        completion_tokens,
        cache_read_tokens,
    };
    if ev.is_empty() { None } else { Some(ev) }
}

/// One SSE line: `data: {json}` → non-empty delta content; `[DONE]`, blanks, and non-`data:` lines → `None`.
#[must_use]
pub fn sse_data_line_delta(line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let payload = line.strip_prefix("data: ")?;
    let payload = payload.trim();
    if payload == "[DONE]" {
        return None;
    }
    chat_completion_delta_content(payload).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_split_across_chunks() {
        let mut b = Utf8LineBuffer::new();
        let mut lines = Vec::new();
        b.push_lossy_bytes(b"data: {\"x\":1}\r\nda", |l| lines.push(l.to_string()));
        assert_eq!(lines.len(), 1);
        lines.clear();
        b.push_lossy_bytes(b"ta: ok\n", |l| lines.push(l.to_string()));
        assert_eq!(lines, vec!["data: ok"]);
    }

    #[test]
    fn delta_extracts_content() {
        let j = r#"{"choices":[{"delta":{"content":"hi"}}]}"#;
        assert_eq!(chat_completion_delta_content(j).as_deref(), Some("hi"));
    }

    #[test]
    fn sse_line_done_and_data_prefix() {
        assert_eq!(sse_data_line_delta(""), None);
        assert_eq!(sse_data_line_delta("event: ping"), None);
        assert_eq!(sse_data_line_delta("data: [DONE]"), None);
        let j = r#"{"choices":[{"delta":{"content":"x"}}]}"#;
        assert_eq!(
            sse_data_line_delta(&format!("data: {j}")).as_deref(),
            Some("x")
        );
    }

    #[test]
    fn sse_data_line_delta_event_parses_streamed_tool_calls() {
        let frame_1 = r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"vox_git_status","arguments":"{\"pa"}}]}}]}"#;
        let frame_2 = r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"th\":\".\"}"}}]}}]}"#;
        let ev1 = sse_data_line_delta_event(frame_1).expect("ev1");
        assert_eq!(ev1.tool_calls.len(), 1);
        assert_eq!(ev1.tool_calls[0].id.as_deref(), Some("call_1"));
        assert_eq!(ev1.tool_calls[0].name.as_deref(), Some("vox_git_status"));
        assert_eq!(ev1.tool_calls[0].arguments_chunk.as_deref(), Some("{\"pa"));

        let ev2 = sse_data_line_delta_event(frame_2).expect("ev2");
        assert_eq!(ev2.tool_calls.len(), 1);
        assert_eq!(
            ev2.tool_calls[0].arguments_chunk.as_deref(),
            Some("th\":\".\"}")
        );
    }

    #[test]
    fn sse_data_line_delta_event_parses_final_usage_and_cached_tokens() {
        let usage_frame = r#"data: {"choices":[],"usage":{"prompt_tokens":180,"completion_tokens":42,"prompt_tokens_details":{"cached_tokens":144}}}"#;
        let ev = sse_data_line_delta_event(usage_frame).expect("usage event");
        assert_eq!(ev.prompt_tokens, Some(180));
        assert_eq!(ev.completion_tokens, Some(42));
        assert_eq!(ev.cache_read_tokens, Some(144));
    }
}
