//! `mens/config/hub-cards.yaml`: per-size wording, spokes and re-gate estimates (spec D4).
//!
//! The file is embedded at compile time and validated on load. This module holds no size name,
//! sentence or number: sizes come from [`crate::mens::hub_repo::SIZES`] and the yaml.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::mens::hub_repo::SIZES;

const SCHEMA: &str = "vox_mens_hub_cards_v1";

const EMBEDDED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../mens/config/hub-cards.yaml"
));

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HubCardsConfig {
    pub schema: String,
    pub default_spoke: String,
    pub default_spoke_source: String,
    pub spokes: Vec<SpokeEntry>,
    pub sizes: BTreeMap<String, SizeEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SpokeEntry {
    pub id: String,
    pub label: String,
    pub status: SpokeStatus,
    pub purpose: String,
    pub link: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SpokeStatus {
    Trained,
    Planned,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SizeEntry {
    /// Two or three sentences: the hardware tier, the work, when to step up or down.
    pub why: Vec<String>,
    pub regate_eta: Option<RegateEta>,
}

/// A projection, shown only as an estimate.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegateEta {
    pub hours_expected: f64,
    pub hours_conservative: f64,
    pub basis: String,
    pub confidence: String,
}

fn sentence_count(s: &str) -> usize {
    s.split(['.', '!', '?'])
        .filter(|p| !p.trim().is_empty())
        .count()
}

/// True when `s` has an ASCII digit outside a ladder size name such as `4B`.
fn has_stray_digit(s: &str) -> bool {
    let mut names = SIZES.to_vec();
    names.sort_by_key(|n| std::cmp::Reverse(n.len())); // `14B` before `4B`
    let mut rest = s.to_string();
    for n in names {
        rest = rest.replace(n, "");
    }
    rest.chars().any(|c| c.is_ascii_digit())
}

/// Parse `text` and check every rule; the first violation is returned as text naming the key.
pub(crate) fn parse_and_validate(text: &str) -> Result<HubCardsConfig, String> {
    let cfg: HubCardsConfig =
        serde_yaml::from_str(text).map_err(|e| format!("hub-cards.yaml: {e}"))?;
    if cfg.schema != SCHEMA {
        return Err(format!(
            "hub-cards.yaml: schema `{}` is not `{SCHEMA}`",
            cfg.schema
        ));
    }
    for size in cfg.sizes.keys() {
        if !SIZES.contains(&size.as_str()) {
            return Err(format!("hub-cards.yaml: unknown size `{size}` in sizes"));
        }
    }
    for size in SIZES {
        let Some(entry) = cfg.sizes.get(size) else {
            return Err(format!("hub-cards.yaml: missing size `{size}` in sizes"));
        };
        let n: usize = entry.why.iter().map(|s| sentence_count(s)).sum();
        if entry.why.iter().any(|s| s.trim().is_empty()) || !(2..=3).contains(&n) {
            return Err(format!(
                "hub-cards.yaml: sizes.{size}.why needs two or three sentences, found {n}"
            ));
        }
        if let Some(s) = entry.why.iter().find(|s| has_stray_digit(s)) {
            return Err(format!(
                "hub-cards.yaml: sizes.{size}.why holds a digit (numbers come from records): {s}"
            ));
        }
        if let Some(e) = &entry.regate_eta {
            let ok = e.hours_expected > 0.0
                && e.hours_expected <= e.hours_conservative
                && !e.basis.trim().is_empty()
                && e.confidence == "low";
            if !ok {
                return Err(format!(
                    "hub-cards.yaml: sizes.{size}.regate_eta needs 0 < hours_expected <= hours_conservative, a basis and confidence: low"
                ));
            }
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for sp in &cfg.spokes {
        if !seen.insert(sp.id.as_str()) {
            return Err(format!("hub-cards.yaml: duplicate spoke id `{}`", sp.id));
        }
        if sp.status == SpokeStatus::Planned && sp.link.as_deref().is_none_or(str::is_empty) {
            return Err(format!(
                "hub-cards.yaml: planned spoke `{}` needs a link",
                sp.id
            ));
        }
        if has_stray_digit(&sp.purpose) {
            return Err(format!(
                "hub-cards.yaml: spokes `{}` purpose holds a digit",
                sp.id
            ));
        }
    }
    let default_ok = cfg
        .spokes
        .iter()
        .any(|s| s.id == cfg.default_spoke && s.status == SpokeStatus::Trained);
    if !default_ok {
        return Err(format!(
            "hub-cards.yaml: default_spoke `{}` is not a trained spoke",
            cfg.default_spoke
        ));
    }
    Ok(cfg)
}

/// The embedded file, parsed once. Never panics: a broken file is an `Err` every call.
pub(crate) fn hub_cards() -> Result<&'static HubCardsConfig, String> {
    static CELL: OnceLock<Result<HubCardsConfig, String>> = OnceLock::new();
    CELL.get_or_init(|| parse_and_validate(EMBEDDED))
        .as_ref()
        .map_err(Clone::clone)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Value;

    /// The embedded yaml, edited, run through the validator.
    fn edited(f: impl FnOnce(&mut Value)) -> Result<HubCardsConfig, String> {
        let mut v: Value = serde_yaml::from_str(EMBEDDED).unwrap();
        f(&mut v);
        parse_and_validate(&serde_yaml::to_string(&v).unwrap())
    }

    fn err_of(f: impl FnOnce(&mut Value)) -> String {
        edited(f).expect_err("edit must be rejected")
    }

    #[test]
    fn real_file_validates() {
        let cfg = hub_cards().expect("embedded hub-cards.yaml validates");
        let mut keys: Vec<&str> = cfg.sizes.keys().map(String::as_str).collect();
        let mut want = SIZES.to_vec();
        keys.sort_unstable();
        want.sort_unstable();
        assert_eq!(keys, want);
        let d = cfg
            .spokes
            .iter()
            .find(|s| s.id == cfg.default_spoke)
            .unwrap();
        assert_eq!(
            (cfg.default_spoke.as_str(), d.status),
            ("vox-lang", SpokeStatus::Trained)
        );
    }

    #[test]
    fn unknown_size_is_an_error() {
        let e = err_of(|v| {
            let four = v["sizes"][SIZES[2]].clone();
            v["sizes"]["3B"] = four;
        });
        assert!(e.contains("3B"), "{e}");
    }

    #[test]
    fn missing_size_is_an_error() {
        let e = err_of(|v| {
            v["sizes"].as_mapping_mut().unwrap().remove(SIZES[4]);
        });
        assert!(e.contains(SIZES[4]), "{e}");
    }

    #[test]
    fn missing_blurb_is_an_error() {
        let sentences = |n: usize| {
            Value::Sequence(
                (0..n)
                    .map(|i| Value::String(format!("Sentence number {} here.", "x".repeat(i + 1))))
                    .collect(),
            )
        };
        for why in [Value::Sequence(vec![]), sentences(1), sentences(4)] {
            let e = err_of(|v| v["sizes"][SIZES[3]]["why"] = why);
            assert!(e.contains(SIZES[3]), "{e}");
        }
        let e = err_of(|v| {
            v["sizes"][SIZES[3]].as_mapping_mut().unwrap().remove("why");
        });
        assert!(e.contains("why"), "{e}");
    }

    #[test]
    fn blurb_prose_has_no_digits() {
        let cfg = hub_cards().unwrap();
        for (size, e) in &cfg.sizes {
            for s in &e.why {
                assert!(!has_stray_digit(s), "{size}: {s}");
            }
        }
        for sp in &cfg.spokes {
            assert!(!has_stray_digit(&sp.purpose), "{}", sp.id);
        }
        // A size name is allowed, a bare number is not.
        assert!(!has_stray_digit("Compare with 14B and 4B."));
        assert!(has_stray_digit("It needs 12 GB."));
        let e = err_of(|v| {
            v["sizes"][SIZES[2]]["why"][0] = Value::String("About 12 GB of memory.".into());
        });
        assert!(e.contains(SIZES[2]) && e.contains("digit"), "{e}");
    }

    #[test]
    fn spokes_are_well_formed() {
        let cfg = hub_cards().unwrap();
        let mut ids: Vec<&str> = cfg.spokes.iter().map(|s| s.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), cfg.spokes.len());
        assert!(
            cfg.spokes
                .iter()
                .filter(|s| s.status == SpokeStatus::Planned)
                .all(|s| s.link.is_some())
        );
        assert!(edited(|v| v["spokes"][1]["status"] = "shipped".into()).is_err());
        assert!(
            edited(|v| {
                v["spokes"][1].as_mapping_mut().unwrap().remove("link");
            })
            .is_err()
        );
        assert!(edited(|v| v["spokes"][1]["id"] = "vox-lang".into()).is_err());
        assert!(edited(|v| v["default_spoke"] = "rust".into()).is_err());
    }

    #[test]
    fn regate_eta_is_an_estimate() {
        let cfg = hub_cards().unwrap();
        for (size, e) in &cfg.sizes {
            if let Some(r) = &e.regate_eta {
                assert!(
                    0.0 < r.hours_expected && r.hours_expected <= r.hours_conservative,
                    "{size}"
                );
                assert!(
                    !r.basis.trim().is_empty() && r.confidence == "low",
                    "{size}"
                );
            }
        }
        assert!(
            edited(|v| v["sizes"][SIZES[2]]["regate_eta"]["confidence"] = "high".into()).is_err()
        );
        assert!(
            edited(|v| v["sizes"][SIZES[2]]["regate_eta"]["hours_expected"] = 99.0.into()).is_err()
        );
        assert!(edited(|v| v["sizes"][SIZES[2]]["regate_eta"]["basis"] = "".into()).is_err());
    }

    #[test]
    fn unknown_keys_are_an_error() {
        assert!(edited(|v| v["surprise"] = true.into()).is_err());
        assert!(edited(|v| v["sizes"][SIZES[2]]["surprise"] = true.into()).is_err());
        assert!(edited(|v| v["spokes"][0]["surprise"] = true.into()).is_err());
        assert!(edited(|v| v["sizes"][SIZES[2]]["regate_eta"]["surprise"] = true.into()).is_err());
    }
}
