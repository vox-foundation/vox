//! End-to-end checks against a real Chrome. Run with
//! `cargo test -p vox-plugin-browser --lib live_tests -- --ignored`.

use crate::actionability::DEFAULT_ACTION_TIMEOUT_MS;
use crate::engine::BrowserEngine;
use crate::events::{DialogAction, EventQuery};
use crate::snapshot::SnapshotOptions;

const FIXTURE: &str = r#"<!doctype html><html><body>
<div id="overlay" style="position:fixed;inset:0;background:rgba(0,0,0,.3)"></div>
<button id="late" disabled onclick="document.title='late-clicked'">Late</button>
<form onsubmit="event.preventDefault();document.title='submitted:'+this.q.value">
  <input name="q" aria-label="Query" value="old text">
  <select aria-label="Color"><option value="r">Red</option><option value="g">Green</option></select>
</form>
<button id="ask" onclick="document.title=confirm('Proceed?')?'yes':'no'">Ask</button>
<script>
  console.error("fixture-error");
  setTimeout(() => document.getElementById("overlay").remove(), 600);
  setTimeout(() => document.getElementById("late").disabled = false, 900);
  fetch("http://127.0.0.1:9/missing-endpoint").catch(() => {});
</script>
</body></html>"#;

fn data_url(html: &str) -> String {
    let mut out = String::from("data:text/html,");
    for b in html.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

async fn ref_named(engine: &BrowserEngine, page_id: &str, name: &str) -> String {
    let snap = engine
        .snapshot(page_id, SnapshotOptions::default())
        .await
        .expect("snapshot");
    snap.refs
        .values()
        .find(|r| r.name == name)
        .unwrap_or_else(|| panic!("no ref named {name:?} in {}", snap.tree))
        .ref_id
        .clone()
}

async fn title(engine: &BrowserEngine, page_id: &str) -> String {
    engine.page_info(page_id).await.expect("page_info")["title"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[tokio::test]
#[ignore = "owner:browser sunset:never slow; requires local Chrome/Chromium binary"]
async fn actionability_fill_keys_dialogs_and_events_end_to_end() {
    let engine = BrowserEngine::new();
    let page_id = engine.open(&data_url(FIXTURE), true).await.expect("open");

    // Covered for 600 ms and disabled for 900 ms: auto-wait must ride both out.
    let late = ref_named(&engine, &page_id, "Late").await;
    let out = engine
        .click_ref(&page_id, &late, true, DEFAULT_ACTION_TIMEOUT_MS)
        .await
        .expect("click late");
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(title(&engine, &page_id).await, "late-clicked");

    // A too-short timeout reports why instead of clicking blind.
    engine
        .goto(&page_id, &data_url(FIXTURE))
        .await
        .expect("reload fixture");
    let late = ref_named(&engine, &page_id, "Late").await;
    let out = engine
        .click_ref(&page_id, &late, true, 100)
        .await
        .expect("click late fast");
    assert_eq!(out["reason"], "not_actionable", "{out}");

    // Fill replaces the existing value; Enter submits the form.
    let query = ref_named(&engine, &page_id, "Query").await;
    let out = engine
        .fill_ref(&page_id, &query, "new", true, DEFAULT_ACTION_TIMEOUT_MS)
        .await
        .expect("fill");
    assert_eq!(out["ok"], true, "{out}");
    engine.press(&page_id, "Enter").await.expect("enter");
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(title(&engine, &page_id).await, "submitted:new");

    // Selects pick an option by label; unknown labels list the choices.
    let color = ref_named(&engine, &page_id, "Color").await;
    let out = engine
        .fill_ref(&page_id, &color, "Green", true, DEFAULT_ACTION_TIMEOUT_MS)
        .await
        .expect("select");
    assert_eq!(out["ok"], true, "{out}");
    let out = engine
        .fill_ref(&page_id, &color, "Blue", true, DEFAULT_ACTION_TIMEOUT_MS)
        .await
        .expect("select missing");
    assert_eq!(out["reason"], "no_matching_option", "{out}");

    // Console errors and the failed fetch are captured.
    let events = engine
        .page_events(&page_id, &EventQuery::default())
        .await
        .expect("events");
    let console = events["console"].as_array().expect("console array");
    assert!(
        console.iter().any(|e| e["text"] == "fixture-error"),
        "{events}"
    );
    assert!(
        events["network"]
            .as_array()
            .expect("network array")
            .iter()
            .any(|e| e["url"]
                .as_str()
                .unwrap_or_default()
                .contains("missing-endpoint")),
        "{events}"
    );

    // A confirm() blocks the page: actions report it, handle_dialog resolves it.
    let ask = ref_named(&engine, &page_id, "Ask").await;
    let clicker = {
        let engine = engine.clone();
        let page_id = page_id.clone();
        tokio::spawn(async move {
            engine
                .click_ref(&page_id, &ask, true, DEFAULT_ACTION_TIMEOUT_MS)
                .await
        })
    };
    let mut dialog = None;
    for _ in 0..50 {
        dialog = engine.pending_dialog(&page_id).await;
        if dialog.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let dialog = dialog.expect("confirm dialog captured");
    assert_eq!(dialog.message, "Proceed?");
    let out = engine
        .fill_ref(&page_id, &query, "x", true, DEFAULT_ACTION_TIMEOUT_MS)
        .await
        .expect("blocked fill");
    assert_eq!(out["reason"], "dialog_open", "{out}");
    let out = engine
        .handle_dialog(
            &page_id,
            &DialogAction {
                accept: true,
                prompt_text: None,
            },
        )
        .await
        .expect("handle dialog");
    assert_eq!(out["ok"], true, "{out}");
    let _ = clicker.await;
    assert_eq!(title(&engine, &page_id).await, "yes");

    engine.close(&page_id).await.expect("close");
}
