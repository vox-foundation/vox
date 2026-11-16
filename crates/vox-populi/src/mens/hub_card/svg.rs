//! Raster JPEG hero banners for Hugging Face Hub model cards and the MENS ladder index.
//!
//! Embeds the 7 pre-generated 16:9 JPEG banners from `crates/vox-populi/assets/hub/`
//! and emits `assets/banner.jpg` (per-model card) and `assets/overview.jpg` (index hub).

use super::CardContext;
use super::facts::RunRecords;
use crate::mens::hub_repo::{family_of_model, size_of_pin};
use crate::mens::hub_sync::SizeSyncRow;

const OVERVIEW_JPG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/hub/overview.jpg"
));
const BANNER_0_8B_JPG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/hub/banner-0.8b.jpg"
));
const BANNER_2B_JPG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/hub/banner-2b.jpg"
));
const BANNER_4B_JPG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/hub/banner-4b.jpg"
));
const BANNER_9B_JPG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/hub/banner-9b.jpg"
));
const BANNER_27B_JPG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/hub/banner-27b.jpg"
));
const BANNER_GENERIC_JPG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/hub/banner-generic.jpg"
));

/// Binary asset file written beside a rendered Hub markdown card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedAsset {
    pub rel_path: String,
    pub bytes: Vec<u8>,
}

fn resolved_family(cx: &CardContext<'_>) -> &'static str {
    let pin = cx.base_pin.trim();
    if !pin.is_empty()
        && !pin.eq_ignore_ascii_case("test-pin")
        && (pin.to_ascii_lowercase().contains("qwen3") || size_of_pin(pin).is_some())
    {
        family_of_model(pin)
    } else {
        family_of_model(cx.size)
    }
}

/// Select the per-model JPEG hero banner (`assets/banner.jpg`).
pub fn render_model_assets(cx: &CardContext<'_>, _rec: &RunRecords) -> Vec<RenderedAsset> {
    let size_upper = cx.size.trim().to_ascii_uppercase();
    let family = resolved_family(cx);
    let selected = match (family, size_upper.as_str()) {
        ("qwen3.5", "0.8B") => BANNER_0_8B_JPG,
        ("qwen3.5", "2B") => BANNER_2B_JPG,
        ("qwen3.5", "4B") => BANNER_4B_JPG,
        ("qwen3.5", "9B") => BANNER_9B_JPG,
        ("qwen3.8", "27B") => BANNER_27B_JPG,
        _ => BANNER_GENERIC_JPG,
    };
    vec![RenderedAsset {
        rel_path: "assets/banner.jpg".to_string(),
        bytes: selected.to_vec(),
    }]
}

/// Select the index overview JPEG hero banner (`assets/overview.jpg`).
pub fn render_index_assets(
    _namespace: &str,
    _entries: &[(SizeSyncRow, RunRecords)],
) -> Vec<RenderedAsset> {
    vec![RenderedAsset {
        rel_path: "assets/overview.jpg".to_string(),
        bytes: OVERVIEW_JPG.to_vec(),
    }]
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn assert_valid_jpeg(bytes: &[u8], label: &str) {
        assert!(
            bytes.len() > 1024,
            "{label} JPEG unexpectedly small: {} bytes",
            bytes.len()
        );
        assert_eq!(
            &bytes[..3],
            &[0xFF, 0xD8, 0xFF],
            "{label} missing JPEG SOI header"
        );
        assert!(
            !bytes.windows(4).any(|w| w.eq_ignore_ascii_case(b"<svg")),
            "{label} must not contain inline SVG markup"
        );
    }

    #[test]
    fn nov_2026_rungs_emit_distinct_valid_jpeg_banners() {
        let cases = [
            (
                "0.8B",
                "Qwen/Qwen3.5-0.8B@2fc06364715b967f1860aea9cf38778875588b17", // gitleaks:allow
            ),
            (
                "2B",
                "Qwen/Qwen3.5-2B@15852e8c16360a2fea060d615a32b45270f8a8fc", // gitleaks:allow
            ),
            (
                "4B",
                "Qwen/Qwen3.5-4B@851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a", // gitleaks:allow
            ),
            (
                "9B",
                "Qwen/Qwen3.5-9B@c202236235762e1c871ad0ccb60c8ee5ba337b9a", // gitleaks:allow
            ),
            (
                "27B",
                "Qwen/Qwen3.8-27B@37561816e66b5b9153b1b64c10968c18c1e6f4a0", // gitleaks:allow
            ),
        ];
        let rec = RunRecords::default();
        let mut seen = BTreeSet::new();
        for (size, pin) in cases {
            let cx = CardContext {
                size,
                base_pin: pin,
                namespace: "brbrainerd",
            };
            let assets = render_model_assets(&cx, &rec);
            assert_eq!(assets.len(), 1);
            assert_eq!(assets[0].rel_path, "assets/banner.jpg");
            assert_valid_jpeg(&assets[0].bytes, size);
            assert!(
                seen.insert(assets[0].bytes.clone()),
                "banner for {size} should be distinct"
            );
        }
        assert_eq!(seen.len(), 5);
    }

    #[test]
    fn legacy_and_unknown_rungs_fall_back_to_generic_banner() {
        let rec = RunRecords::default();
        let legacy_4b = render_model_assets(
            &CardContext {
                size: "4B",
                base_pin: "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c", // gitleaks:allow
                namespace: "brbrainerd",
            },
            &rec,
        );
        let legacy_32b = render_model_assets(
            &CardContext {
                size: "32B",
                base_pin: "test-pin",
                namespace: "brbrainerd",
            },
            &rec,
        );
        assert_eq!(legacy_4b.len(), 1);
        assert_eq!(legacy_32b.len(), 1);
        assert_eq!(legacy_4b[0].rel_path, "assets/banner.jpg");
        assert_eq!(legacy_32b[0].rel_path, "assets/banner.jpg");
        assert_valid_jpeg(&legacy_4b[0].bytes, "legacy Qwen3-4B");
        assert_eq!(legacy_4b[0].bytes, BANNER_GENERIC_JPG);
        assert_eq!(legacy_32b[0].bytes, BANNER_GENERIC_JPG);
    }

    #[test]
    fn index_assets_emit_overview_jpeg() {
        let assets = render_index_assets("brbrainerd", &[]);
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].rel_path, "assets/overview.jpg");
        assert_valid_jpeg(&assets[0].bytes, "overview");
        assert_eq!(assets[0].bytes, OVERVIEW_JPG);
    }
}
