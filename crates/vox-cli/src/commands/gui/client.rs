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

fn matches_until(until: &str, body: &str) -> bool {
    let v: serde_json::Value = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
    if until == "error" {
        return v.get("last_error").map(|x| !x.is_null()).unwrap_or(false);
    }
    if until == "reply" {
        if v.get("last_error").map(|x| !x.is_null()).unwrap_or(false) {
            return true;
        }
        return v
            .get("bubbles")
            .and_then(|b| b.as_array())
            .and_then(|a| a.last())
            .and_then(|b| b.get("role"))
            .and_then(|r| r.as_str())
            == Some("assistant");
    }
    if let Some(id) = until.strip_prefix("selectable=") {
        return v
            .get("catalog")
            .and_then(|c| c.as_array())
            .into_iter()
            .flatten()
            .any(|row| {
                row.get("id").and_then(|i| i.as_str()) == Some(id)
                    && row.get("selectable") == Some(&serde_json::Value::Bool(true))
            });
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
}
