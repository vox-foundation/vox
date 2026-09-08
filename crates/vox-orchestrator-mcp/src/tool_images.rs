//! Strip `image_base64` from MCP tool JSON and optionally keep the decoded PNG.

use base64::Engine;
use serde_json::Value;

pub const BROWSER_FRAME_IMAGE_PART_MAX_BYTES: usize = 400_000;

/// Only these MCP tool names may produce an image part (stdio `call_tool` + Axis).
pub const FRAME_IMAGE_TOOL_NAMES: &[&str] = &[
    "vox_browser_screenshot_viewport",
    "vox_browser_screencast_frame",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBlob {
    pub mime: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PromotedToolImage {
    pub json: Value,
    pub image: Option<ImageBlob>,
}

pub fn json_text(promoted: &PromotedToolImage) -> String {
    serde_json::to_string(&promoted.json).unwrap_or_else(|_| "{}".into())
}

pub fn promote_tool_image(result_json: &str) -> PromotedToolImage {
    let mut json: Value = serde_json::from_str(result_json)
        .unwrap_or_else(|_| Value::String(result_json.to_string()));
    let image = strip_image_base64(&mut json);
    PromotedToolImage { json, image }
}

fn strip_image_base64(value: &mut Value) -> Option<ImageBlob> {
    let mut found = None;
    match value {
        Value::Object(map) => {
            if let Some(Value::String(b64)) = map.remove("image_base64") {
                found = decode_png_b64(&b64);
            }
            for v in map.values_mut() {
                if found.is_none() {
                    found = strip_image_base64(v);
                } else {
                    let _ = strip_image_base64(v);
                }
            }
        }
        Value::Array(items) => {
            for v in items {
                if found.is_none() {
                    found = strip_image_base64(v);
                } else {
                    let _ = strip_image_base64(v);
                }
            }
        }
        _ => {}
    }
    found
}

fn decode_png_b64(b64: &str) -> Option<ImageBlob> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .ok()?;
    if bytes.len() > BROWSER_FRAME_IMAGE_PART_MAX_BYTES {
        return None;
    }
    let mime = sniff_image_mime(&bytes)?;
    Some(ImageBlob {
        mime: mime.into(),
        bytes,
    })
}

pub fn sniff_image_mime(bytes: &[u8]) -> Option<&'static str> {
    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if bytes.starts_with(PNG) {
        return Some("image/png");
    }
    if bytes.len() >= 3 && bytes[0] == 0xff && bytes[1] == 0xd8 && bytes[2] == 0xff {
        return Some("image/jpeg");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_1X1_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

    #[test]
    fn promote_strips_image_base64_and_keeps_decoded_bytes() {
        let raw = serde_json::json!({
            "success": true,
            "data": {
                "image_base64": PNG_1X1_B64,
                "viewport_width": 1280,
                "page_id": "p1"
            }
        })
        .to_string();
        let got = promote_tool_image(&raw);
        let data = got.json["data"].as_object().expect("data");
        assert!(!data.contains_key("image_base64"), "{data:?}");
        assert_eq!(data["viewport_width"], 1280);
        let img = got.image.clone().expect("image");
        assert_eq!(img.mime, "image/png");
        assert_eq!(img.bytes.len(), 70);
        let text = json_text(&got);
        assert!(!text.contains("image_base64"));
    }

    #[test]
    fn promote_over_cap_strips_but_drops_part() {
        let big = vec![0u8; BROWSER_FRAME_IMAGE_PART_MAX_BYTES + 1];
        let b64 = base64::engine::general_purpose::STANDARD.encode(&big);
        let raw = serde_json::json!({
            "success": true,
            "data": { "image_base64": b64 }
        })
        .to_string();
        let got = promote_tool_image(&raw);
        assert!(got.image.is_none());
        assert!(
            !got.json["data"]
                .as_object()
                .unwrap()
                .contains_key("image_base64")
        );
    }

    #[test]
    fn promote_invalid_base64_strips_without_echo() {
        let raw = r#"{"success":true,"data":{"image_base64":"%%%not-valid%%%"}}"#;
        let got = promote_tool_image(raw);
        assert!(got.image.is_none());
        assert!(!json_text(&got).contains("image_base64"));
        assert!(!json_text(&got).contains("%%%not-valid%%%"));
    }

    #[test]
    fn promote_leaves_cookie_shaped_json_alone() {
        let raw = serde_json::json!({
            "success": true,
            "data": { "count": 3, "path": "consents.json" }
        })
        .to_string();
        let got = promote_tool_image(&raw);
        assert!(got.image.is_none());
        assert_eq!(got.json["data"]["count"], 3);
        assert_eq!(got.json["data"]["path"], "consents.json");
    }
}
