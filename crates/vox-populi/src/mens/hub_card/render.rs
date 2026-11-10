//! Fills the askama templates under `templates/hub/`.

use askama::Template;

use super::card_facts::{CardFacts, card_facts};
use super::facts::{AdapterEvidence, Blocker};
use super::index_facts::{GridCell, HubCell, IndexFacts, index_facts};
use super::{CardContext, RunRecords};
use crate::mens::hub_sync::SizeSyncRow;

/// The model card (`model.md` and its partials). Markdown, so nothing is HTML-escaped.
#[derive(Template)]
#[template(path = "hub/model.md", escape = "none")]
struct ModelCardTemplate<'a> {
    f: &'a CardFacts,
}

/// The ladder index (`index.md`). Markdown, so nothing is HTML-escaped.
#[derive(Template)]
#[template(path = "hub/index.md", escape = "none")]
struct IndexTemplate<'a> {
    f: &'a IndexFacts,
}

/// The one place a template error becomes a loud marker line instead of a panic. A card
/// carrying the marker fails every `manifests_match` and golden comparison, so it is
/// never published silently.
fn render_or_marker(t: &impl Template) -> String {
    t.render()
        .unwrap_or_else(|e| format!("<!-- hub card render error: {e} -->\n"))
}

/// Render the `README.md` / `MODEL_CARD.md` for one ladder size from its records.
#[inline(never)]
pub fn render_model_card(cx: &CardContext, rec: &RunRecords) -> String {
    render_or_marker(&ModelCardTemplate {
        f: &card_facts(cx, rec),
    })
}

/// `README.md` for `<namespace>/vox-mens-index`, one grid row per ladder size.
#[inline(never)]
pub fn render_index_readme(namespace: &str, entries: &[(SizeSyncRow, RunRecords)]) -> String {
    render_or_marker(&IndexTemplate {
        f: &index_facts(namespace, entries),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Broken;
    impl std::fmt::Display for Broken {
        fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            Err(std::fmt::Error)
        }
    }
    impl Template for Broken {
        fn render_into(&self, _: &mut (impl std::fmt::Write + ?Sized)) -> askama::Result<()> {
            Err(askama::Error::Fmt(std::fmt::Error))
        }
        const EXTENSION: Option<&'static str> = None;
        const SIZE_HINT: usize = 0;
        const MIME_TYPE: &'static str = "text/plain";
    }

    #[test]
    fn render_error_becomes_a_marker_not_a_panic() {
        assert!(render_or_marker(&Broken).starts_with("<!-- hub card render error:"));
    }

    #[test]
    fn model_card_passes_markup_through_unescaped() {
        // A record value with markup reaches the card as written (escape = "none").
        let rec = RunRecords {
            train_record: Some(serde_json::json!({
                "status": "not_trained", "reason": "a < b && \"c\" `d`"
            })),
            ..Default::default()
        };
        let cx = CardContext {
            size: "32B",
            base_pin: "Qwen/Qwen3-32B@9216db5781bf21249d130ec9da846c4624c16137",
            namespace: "ns",
        };
        let card = render_model_card(&cx, &rec);
        assert!(card.contains("a < b && \"c\" `d`"), "{card}");
    }
}
