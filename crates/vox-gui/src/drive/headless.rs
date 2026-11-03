use crate::drive::protocol::{DriveSet, DriveState, apply_set};
use serde_json::{Value, json};
use std::io::{Read, Write};

pub fn run_stdio() -> Result<(), String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| e.to_string())?;
    let req: Value = if buf.trim().is_empty() {
        json!({ "verb": "state" })
    } else {
        serde_json::from_str(&buf).map_err(|e| e.to_string())?
    };
    let out = handle_headless(&req);
    let mut stdout = std::io::stdout();
    stdout
        .write_all(out.to_string().as_bytes())
        .map_err(|e| e.to_string())?;
    stdout.write_all(b"\n").map_err(|e| e.to_string())?;
    if out.get("error").and_then(|e| e.as_str()) == Some("empty_text") {
        return Err("empty_text".into());
    }
    Ok(())
}

pub fn handle_headless(req: &Value) -> Value {
    let verb = req.get("verb").and_then(|v| v.as_str()).unwrap_or("state");
    let mut state = DriveState::empty_headless();
    match verb {
        "set" => {
            let set: DriveSet =
                serde_json::from_value(req.get("set").cloned().unwrap_or(json!({})))
                    .unwrap_or_default();
            if let Err(err) = apply_set(&mut state, set) {
                return headless_error(&err.message, 400);
            }
            headless_ok(json!({ "state": state }))
        }
        "send" => {
            let text = req
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if text.is_empty() {
                return headless_error("empty_text", 400);
            }
            headless_ok(json!({
                "accepted": true,
                "text": text,
            }))
        }
        "state" => headless_ok(json!({ "state": state })),
        other => headless_error(&format!("unknown_verb:{other}"), 404),
    }
}

fn headless_claims() -> Value {
    json!({
        "picker_ui": false,
        "composer_knobs": false,
        "bubbles": false,
        "events": false,
    })
}

fn headless_ok(mut extra: Value) -> Value {
    let obj = extra.as_object_mut().cloned().unwrap_or_default();
    let mut out = json!({
        "plane": "headless",
        "claims": headless_claims(),
    });
    if let Some(map) = out.as_object_mut() {
        for (k, v) in obj {
            map.insert(k, v);
        }
    }
    out
}

fn headless_error(code: &str, status: u16) -> Value {
    json!({
        "plane": "headless",
        "claims": headless_claims(),
        "error": code,
        "status": status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_send_empty_text() {
        let out = handle_headless(&json!({ "verb": "send", "text": "" }));
        assert_eq!(out["plane"], "headless");
        assert_eq!(out["claims"]["picker_ui"], false);
        assert_eq!(out["claims"]["composer_knobs"], false);
        assert_eq!(out["claims"]["bubbles"], false);
        assert_eq!(out["claims"]["events"], false);
        assert_eq!(out["error"], "empty_text");
    }

    #[test]
    fn headless_state_declares_non_claims() {
        let out = handle_headless(&json!({ "verb": "state" }));
        assert_eq!(out["plane"], "headless");
        assert_eq!(out["claims"]["picker_ui"], false);
        assert_eq!(out["claims"]["events"], false);
        assert_eq!(out["state"]["plane"], "headless");
    }
}
