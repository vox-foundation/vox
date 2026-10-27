//! Live desktop checks against Calculator, launched in the background.
//! Needs Accessibility + Screen Recording for the host process:
//! `cargo test -p vox-plugin-computer --lib live -- --ignored --nocapture`

use std::process::Command;

use std::thread::sleep;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::computer::Computer;
use crate::macos::Mac;

fn parse(v: Value) -> Value {
    println!(
        "{}",
        serde_json::to_string(&v)
            .unwrap_or_default()
            .chars()
            .take(400)
            .collect::<String>()
    );
    v
}

fn ref_named(tree: &str, role: &str, name: &str) -> Option<String> {
    let needle = format!("- {role} \"{name}\" [ref=");
    let line = tree.lines().find(|l| l.trim_start().starts_with(&needle))?;
    let start = line.find("[ref=")? + 5;
    Some(line[start..].split(']').next()?.to_string())
}

fn display(c: &Computer<Mac>) -> String {
    let s = c.snapshot(r#"{"target":{"app":"Calculator"}}"#);
    let tree = s["tree"].as_str().unwrap_or_default();
    // The result is the last static text in the display group.
    tree.lines()
        .filter(|l| l.trim_start().starts_with("- text "))
        .filter_map(|l| l.split('"').nth(1))
        .rfind(|t| t.chars().any(|c| c.is_ascii_digit()))
        .unwrap_or_default()
        .replace(['\u{200e}', ','], "")
}

#[test]
#[ignore = "drives the real desktop (Calculator); needs Accessibility and Screen Recording"]
fn calculator_ax_press_background_keys_snapshot_and_capture() {
    let c = Computer::new(Mac);
    let perms = parse(c.permissions());
    assert_eq!(
        perms["accessibility"], "granted",
        "grant Accessibility to the host app first"
    );

    Command::new("open")
        .args(["-g", "-a", "Calculator"])
        .status()
        .expect("open Calculator");
    let deadline = Instant::now() + Duration::from_secs(10);
    let snap = loop {
        let s = c.snapshot(r#"{"target":{"app":"Calculator"},"interactive_only":true}"#);
        if s["ok"] == true && s["tree"].as_str().is_some_and(|t| t.contains("button")) {
            break s;
        }
        assert!(
            Instant::now() < deadline,
            "Calculator never exposed buttons: {s}"
        );
        sleep(Duration::from_millis(250));
    };
    let tree = snap["tree"].as_str().unwrap();
    println!("{tree}");

    let clear =
        ref_named(tree, "button", "All Clear").or_else(|| ref_named(tree, "button", "Clear"));
    if let Some(r) = clear {
        parse(c.act(&format!(
            r#"{{"target":{{"app":"Calculator"}},"kind":"press","ref":"{r}"}}"#
        )));
    }
    let seven = ref_named(tree, "button", "7").expect("button 7 in tree");
    let r = parse(c.act(&format!(
        r#"{{"target":{{"app":"Calculator"}},"kind":"click","snapshot_id":"{}","ref":"{seven}"}}"#,
        snap["snapshot_id"].as_str().unwrap()
    )));
    assert_eq!(
        (r["ok"].as_bool(), r["tier"].as_str()),
        (Some(true), Some("ax")),
        "{r}"
    );
    sleep(Duration::from_millis(200));
    assert_eq!(display(&c), "7", "AX press of 7 should show 7");

    // Background keyboard input: Calculator stays behind the user's app.
    let front_before = super::frontmost_pid();
    let r = parse(c.act(r#"{"target":{"app":"Calculator"},"kind":"type","text":"2"}"#));
    assert_eq!(r["tier"], "event", "{r}");
    sleep(Duration::from_millis(300));
    assert_eq!(
        display(&c),
        "72",
        "posted keystroke should reach Calculator in the background"
    );
    assert_eq!(
        super::frontmost_pid(),
        front_before,
        "event tier must not steal focus"
    );

    let windows = parse(c.list_windows(r#"{"app":"Calculator"}"#));
    assert!(!windows["windows"].as_array().unwrap().is_empty());
    let denied = c.snapshot(r#"{"target":{"app":"Keychain Access"}}"#);
    assert!(denied["ok"] == false, "{denied}");

    if perms["screen_recording"] == "granted" {
        let shot = parse(c.screenshot(r#"{"max_long_edge":800}"#));
        assert_eq!(
            (shot["ok"].as_bool(), shot["width"].as_u64()),
            (Some(true), Some(800)),
            "{shot}"
        );
        let zoom = parse(c.screenshot(r#"{"region":[0,0,200,100]}"#));
        assert_eq!(zoom["zoomed"], true, "{zoom}");
        // Largest visible window of any app: per-window capture works even
        // when the window is behind others.
        let all = c.list_windows("");
        let biggest = all["windows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|w| w["title"].is_string())
            .max_by(|a, b| {
                let area =
                    |w: &Value| w["bounds"][2].as_f64().unwrap() * w["bounds"][3].as_f64().unwrap();
                area(a).total_cmp(&area(b))
            })
            .expect("a titled window");
        let shot = parse(c.screenshot(&format!(
            r#"{{"target":{{"pid":{}}},"max_long_edge":640}}"#,
            biggest["pid"]
        )));
        assert_eq!(shot["ok"], true, "{shot}");
        assert!(shot["image_base64"].as_str().unwrap().len() > 1000);
    } else {
        let shot = c.screenshot("");
        assert_eq!(shot["reason"], "screen_recording_denied", "{shot}");
    }

    if let Some(app) = super::running(calc_pid(&c)) {
        app.terminate();
    }
}

fn calc_pid(c: &Computer<Mac>) -> i64 {
    let apps = c.list_apps();
    apps["apps"]
        .as_array()
        .and_then(|a| a.iter().find(|x| x["name"] == "Calculator"))
        .and_then(|x| x["pid"].as_i64())
        .unwrap_or(-1)
}
