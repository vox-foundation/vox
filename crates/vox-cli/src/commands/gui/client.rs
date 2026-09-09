use super::session::{load_session, read_token};
use anyhow::{Context, Result, bail};
use std::io::{Read, Write};
use std::net::TcpStream;

pub fn post(verb: &str, body: &str) -> Result<(u16, String)> {
    let session = load_session().map_err(|e| anyhow::anyhow!(e.message))?;
    let token = read_token().map_err(|e| anyhow::anyhow!(e.message))?;
    let addr = format!("127.0.0.1:{}", session.port);
    let mut stream = TcpStream::connect(&addr).context("connect drive listener")?;
    let req = format!(
        "POST /v1/{verb} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes())?;
    let mut out = String::new();
    stream.read_to_string(&mut out)?;
    let status = out
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let resp_body = out.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    Ok((status, resp_body))
}

pub fn wait_until(until: &str, timeout: &str) -> Result<()> {
    let budget = parse_timeout(timeout)?;
    let start = std::time::Instant::now();
    loop {
        let (status, body) = post("state", "{}")?;
        if status == 200 && matches_until(until, &body) {
            println!("{body}");
            return Ok(());
        }
        if start.elapsed() > budget {
            bail!("wait timed out ({timeout})");
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

fn parse_timeout(s: &str) -> Result<std::time::Duration> {
    let s = s.trim();
    if let Some(num) = s.strip_suffix('s') {
        let n: u64 = num.parse().context("timeout seconds")?;
        return Ok(std::time::Duration::from_secs(n));
    }
    if let Some(num) = s.strip_suffix("ms") {
        let n: u64 = num.parse().context("timeout ms")?;
        return Ok(std::time::Duration::from_millis(n));
    }
    let n: u64 = s.parse().context("timeout")?;
    Ok(std::time::Duration::from_secs(n))
}

fn drive_view(v: &serde_json::Value) -> &serde_json::Value {
    v.get("state").unwrap_or(v)
}

fn matches_until(until: &str, body: &str) -> bool {
    let v: serde_json::Value = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
    let view = drive_view(&v);
    if until == "error" {
        return view
            .get("last_error")
            .map(|x| !x.is_null())
            .unwrap_or(false);
    }
    if until == "reply" {
        if view
            .get("last_error")
            .map(|x| !x.is_null())
            .unwrap_or(false)
        {
            return true;
        }
        return view
            .get("bubbles")
            .and_then(|b| b.as_array())
            .and_then(|a| a.last())
            .and_then(|b| b.get("role"))
            .and_then(|r| r.as_str())
            == Some("assistant");
    }
    if let Some(id) = until.strip_prefix("selectable=") {
        return view
            .get("catalog")
            .and_then(|c| c.as_array())
            .into_iter()
            .flatten()
            .any(|row| {
                row.get("id").and_then(|i| i.as_str()) == Some(id)
                    && row.get("selectable") == Some(&serde_json::Value::Bool(true))
            });
    }
    if until == "reply_ok" {
        if view
            .get("last_error")
            .map(|x| !x.is_null())
            .unwrap_or(false)
        {
            return false;
        }
        let last = view
            .get("bubbles")
            .and_then(|b| b.as_array())
            .and_then(|a| a.last());
        let Some(bubble) = last else {
            return false;
        };
        if bubble.get("role").and_then(|r| r.as_str()) != Some("assistant") {
            return false;
        }
        if bubble.get("error") == Some(&serde_json::Value::Bool(true)) {
            return false;
        }
        return bubble
            .get("content")
            .and_then(|c| c.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
    }
    if let Some(kind) = until.strip_prefix("event=") {
        if kind.is_empty() {
            return false;
        }
        return view
            .get("events")
            .and_then(|e| e.as_array())
            .into_iter()
            .flatten()
            .any(|row| row.get("kind").and_then(|k| k.as_str()) == Some(kind));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_timeout_seconds() {
        assert_eq!(
            parse_timeout("90s").unwrap(),
            std::time::Duration::from_secs(90)
        );
    }

    #[test]
    fn matches_selectable_row() {
        let body = r#"{"catalog":[{"id":"mens/e2e-smoke","selectable":true}]}"#;
        assert!(matches_until("selectable=mens/e2e-smoke", body));
        assert!(!matches_until("selectable=other", body));
    }

    #[test]
    fn matches_error_and_reply_nested_under_state() {
        let body = r#"{"status":200,"plane":"live","state":{"last_error":"load tokenizer","bubbles":[{"role":"user"},{"role":"assistant","error":true}],"catalog":[{"id":"mens/e2e-smoke","selectable":true}]}}"#;
        assert!(matches_until("error", body));
        assert!(matches_until("reply", body));
        assert!(matches_until("selectable=mens/e2e-smoke", body));
        let quiet = r#"{"status":200,"state":{"last_error":null,"bubbles":[{"role":"user"}]}}"#;
        assert!(!matches_until("error", quiet));
        assert!(!matches_until("reply", quiet));
    }

    #[test]
    fn matches_event_kind_nested_under_state() {
        let body = r#"{"status":200,"state":{"events":[{"kind":"token_streamed"},{"kind":"submit_ok"}],"last_error":null}}"#;
        assert!(matches_until("event=submit_ok", body));
        assert!(matches_until("event=token_streamed", body));
        assert!(!matches_until("event=missing", body));
        let empty = r#"{"status":200,"state":{"events":[],"last_error":null}}"#;
        assert!(!matches_until("event=submit_ok", empty));
        let flat = r#"{"catalog":[],"last_error":null}"#;
        assert!(!matches_until("event=submit_ok", flat));
    }

    #[test]
    fn matches_reply_ok_rejects_error_settlement() {
        let err = r#"{"status":200,"state":{"last_error":"load tokenizer","bubbles":[{"role":"assistant","error":true,"content":"load tokenizer"}]}}"#;
        assert!(matches_until("reply", err)); // legacy: settled
        assert!(!matches_until("reply_ok", err));
        let ok = r#"{"status":200,"state":{"last_error":null,"bubbles":[{"role":"user","content":"hi"},{"role":"assistant","content":"hello"}]}}"#;
        assert!(matches_until("reply_ok", ok));
        let empty_asst = r#"{"status":200,"state":{"last_error":null,"bubbles":[{"role":"assistant","content":""}]}}"#;
        assert!(!matches_until("reply_ok", empty_asst));
    }
}
