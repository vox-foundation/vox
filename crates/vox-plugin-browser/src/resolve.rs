use std::path::Path;
use std::time::Duration;

use chromiumoxide::page::ScreenshotParams;
use chromiumoxide_cdp::cdp::browser_protocol::page::{
    CaptureScreenshotFormat, EventScreencastFrame, ScreencastFrameAckParams, StartScreencastFormat,
    StartScreencastParams, StopScreencastParams, Viewport,
};
use futures::StreamExt;

use crate::engine::BrowserEngine;

/// Lower clamp on `visible_text_summary`'s `max_chars` argument: requests below
/// this are raised to it so the returned summary is never trivially short.
pub(crate) const MIN_TEXT_SUMMARY_CHARS: usize = 256;

impl BrowserEngine {
    pub async fn wait_for(
        &self,
        page_id: &str,
        target: &str,
        timeout_secs: u64,
    ) -> Result<(), String> {
        let page = self.page_ref(page_id).await?;
        let deadline = Duration::from_secs(timeout_secs.max(1));
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > deadline {
                return Err(format!(
                    "wait_for timeout after {timeout_secs}s for selector {target:?}"
                ));
            }
            match resolve_element(&page, target).await {
                Ok(_) => return Ok(()),
                Err(_) => tokio::time::sleep(vox_config::timeouts::D_200MS).await,
            }
        }
    }

    pub async fn text(&self, page_id: &str, target: &str) -> Result<String, String> {
        let page = self.page_ref(page_id).await?;
        let el = resolve_element(&page, target).await?;
        el.inner_text()
            .await
            .map_err(Self::map_page_err)?
            .ok_or_else(|| "element has no inner_text".to_string())
    }

    pub async fn html(&self, page_id: &str, target: &str) -> Result<String, String> {
        let page = self.page_ref(page_id).await?;
        if target.trim().is_empty() {
            return page.content().await.map_err(Self::map_page_err);
        }
        let el = resolve_element(&page, target).await?;
        el.outer_html()
            .await
            .map_err(Self::map_page_err)?
            .ok_or_else(|| "element has no outer_html".to_string())
    }

    pub async fn screenshot_bytes(&self, page_id: &str) -> Result<Vec<u8>, String> {
        let page = self.page_ref(page_id).await?;
        page.screenshot(
            ScreenshotParams::builder()
                .format(CaptureScreenshotFormat::Png)
                .full_page(true)
                .build(),
        )
        .await
        .map_err(Self::map_page_err)
    }

    pub async fn screenshot_viewport_bytes(&self, page_id: &str) -> Result<Vec<u8>, String> {
        let page = self.page_ref(page_id).await?;
        let vp = self.viewport_for(page_id).await;
        let clip = Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(vp.width as f64)
            .height(vp.height as f64)
            .scale(1.0)
            .build()
            .map_err(|e| e.to_string())?;
        page.screenshot(
            ScreenshotParams::builder()
                .format(CaptureScreenshotFormat::Png)
                .full_page(false)
                .clip(clip)
                .build(),
        )
        .await
        .map_err(Self::map_page_err)
    }

    pub async fn screencast_frame(&self, page_id: &str) -> Result<serde_json::Value, String> {
        let page = self.page_ref(page_id).await?;
        let vp = self.viewport_for(page_id).await;
        page.execute(
            StartScreencastParams::builder()
                .format(StartScreencastFormat::Jpeg)
                .quality(80)
                .max_width(vp.width as i64)
                .max_height(vp.height as i64)
                .every_nth_frame(1)
                .build(),
        )
        .await
        .map_err(Self::map_page_err)?;
        let mut events = page
            .event_listener::<EventScreencastFrame>()
            .await
            .map_err(|e| e.to_string())?;
        let next = tokio::time::timeout(vox_config::timeouts::D_2S, events.next())
            .await
            .ok()
            .flatten();
        let _ = page.execute(StopScreencastParams::default()).await;
        if let Some(frame) = next {
            let _ = page
                .execute(ScreencastFrameAckParams::new(frame.session_id))
                .await;
            return Ok(serde_json::json!({
                "image_base64": AsRef::<str>::as_ref(&frame.data),
                "viewport_width": frame.metadata.device_width as u32,
                "viewport_height": frame.metadata.device_height as u32
            }));
        }
        Err("no screencast frame received before timeout".to_string())
    }

    pub async fn screenshot(&self, page_id: &str, path: &str) -> Result<String, String> {
        let page = self.page_ref(page_id).await?;
        let p = Path::new(path);
        if let Some(parent) = p.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| e.to_string())?;
        }
        page.save_screenshot(
            ScreenshotParams::builder()
                .format(CaptureScreenshotFormat::Png)
                .full_page(true)
                .build(),
            path,
        )
        .await
        .map_err(Self::map_page_err)?;
        Ok(path.to_string())
    }

    pub async fn visible_text_summary(
        &self,
        page_id: &str,
        max_chars: usize,
    ) -> Result<String, String> {
        let page = self.page_ref(page_id).await?;
        let html = page.content().await.map_err(Self::map_page_err)?;
        Ok(truncate_summary(strip_html_tags(&html), max_chars))
    }

    pub async fn ax_tree(&self, page_id: &str) -> Result<serde_json::Value, String> {
        let page = self.page_ref(page_id).await?;
        let res = page
            .execute(
                chromiumoxide_cdp::cdp::browser_protocol::accessibility::GetFullAxTreeParams::default(),
            )
            .await
            .map_err(|e| format!("AXTree CDP failed: {e}"))?;
        serde_json::to_value(res.nodes.clone()).map_err(|e: serde_json::Error| e.to_string())
    }
}

pub(crate) async fn resolve_element(
    page: &chromiumoxide::Page,
    target: &str,
) -> Result<chromiumoxide::Element, String> {
    let t = target.trim();
    if t.is_empty() {
        return Err("target selector must not be empty".to_string());
    }
    if let Some(rest) = t.strip_prefix("xpath:").map(str::trim) {
        return page.find_xpath(rest).await.map_err(|e| e.to_string());
    }
    page.find_element(t).await.map_err(|e| e.to_string())
}

/// Pure truncation behind `visible_text_summary` (extracted so the clamp +
/// ellipsis budgeting are unit-testable without a live page): the requested cap
/// is floored at [`MIN_TEXT_SUMMARY_CHARS`]; over-long text is truncated by CHAR
/// count, not byte index — readability text from arbitrary pages routinely
/// contains multibyte UTF-8 (curly quotes, accents, CJK) and a byte slice could
/// panic on a non-char-boundary. The 1-char ellipsis is budgeted into the cap so
/// the result never exceeds it.
pub(crate) fn truncate_summary(stripped: String, max_chars: usize) -> String {
    let max_chars = max_chars.max(MIN_TEXT_SUMMARY_CHARS);
    if stripped.chars().count() <= max_chars {
        stripped
    } else {
        format!(
            "{}…",
            stripped
                .chars()
                .take(max_chars.saturating_sub(1))
                .collect::<String>()
        )
    }
}

pub(crate) fn strip_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len().min(262_144));
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn history_capabilities(current_index: usize, total_entries: usize) -> (bool, bool) {
    if total_entries == 0 {
        return (false, false);
    }
    let can_go_back = current_index > 0;
    let can_go_forward = (current_index + 1) < total_entries;
    (can_go_back, can_go_forward)
}
