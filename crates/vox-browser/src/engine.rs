//! Process-wide browser host: one Chromium instance, many tabs (`page_id`).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chromiumoxide::Browser;
use chromiumoxide::browser::BrowserConfig;
use chromiumoxide::page::ScreenshotParams;
use chromiumoxide_cdp::cdp::browser_protocol::page::CaptureScreenshotFormat;
use futures::StreamExt;
use tokio::sync::Mutex;
use tracing::debug;

struct HostInner {
    _handler_task: tokio::task::JoinHandle<()>,
    browser: Browser,
    pages: HashMap<String, chromiumoxide::Page>,
}

/// Shared browser engine (singleton per process).
pub struct BrowserEngine {
    host: Mutex<Option<HostInner>>,
}

impl Default for BrowserEngine {
    fn default() -> Self {
        Self {
            host: Mutex::new(None),
        }
    }
}

impl BrowserEngine {
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    async fn ensure_host(&self, headless: bool) -> Result<(), String> {
        let mut guard = self.host.lock().await;
        if guard.is_some() {
            return Ok(());
        }

        let mut builder = BrowserConfig::builder()
            .request_timeout(Duration::from_secs(90))
            .launch_timeout(Duration::from_secs(60));
        builder = if headless {
            builder.new_headless_mode()
        } else {
            builder.with_head()
        };
        if let Ok(exe) = std::env::var("VOX_CHROME_EXECUTABLE") {
            let exe = exe.trim();
            if !exe.is_empty() {
                builder = builder.chrome_executable(exe);
            }
        }
        if std::env::var("VOX_BROWSER_NO_SANDBOX")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            builder = builder.no_sandbox();
        }

        let config = builder
            .build()
            .map_err(|e| format!("browser config: {e}"))?;
        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| format!("Browser::launch failed: {e}"))?;

        let handler_task = tokio::spawn(async move {
            while handler.next().await.is_some() {
                // keep handler alive; ignore individual events
            }
        });

        *guard = Some(HostInner {
            _handler_task: handler_task,
            browser,
            pages: HashMap::new(),
        });
        debug!(target: "vox_browser", "chromium host launched");
        Ok(())
    }

    /// Open a new tab at `url`. Returns an opaque `page_id`.
    pub async fn open(&self, url: &str, headless: bool) -> Result<String, String> {
        self.ensure_host(headless).await?;
        let mut guard = self.host.lock().await;
        let host = guard
            .as_mut()
            .ok_or_else(|| "browser host missing".to_string())?;
        let page = host
            .browser
            .new_page("about:blank")
            .await
            .map_err(|e| format!("new_page: {e}"))?;
        page.goto(url)
            .await
            .map_err(|e| format!("goto {url}: {e}"))?;
        let id = format!("page-{}", uuid::Uuid::new_v4());
        host.pages.insert(id.clone(), page);
        Ok(id)
    }

    fn map_page_err(e: chromiumoxide::error::CdpError) -> String {
        e.to_string()
    }

    async fn page_ref(&self, page_id: &str) -> Result<chromiumoxide::Page, String> {
        let guard = self.host.lock().await;
        let host = guard
            .as_ref()
            .ok_or_else(|| "no browser host; call open first".to_string())?;
        host.pages
            .get(page_id)
            .cloned()
            .ok_or_else(|| format!("unknown page_id {page_id:?}"))
    }

    pub async fn close(&self, page_id: &str) -> Result<(), String> {
        let mut guard = self.host.lock().await;
        let shutdown = {
            let Some(host) = guard.as_mut() else {
                return Ok(());
            };
            if let Some(page) = host.pages.remove(page_id) {
                let _ = page.close().await;
            }
            host.pages.is_empty()
        };
        if shutdown {
            if let Some(inner) = guard.take() {
                inner._handler_task.abort();
                drop(inner.browser);
            }
            debug!(target: "vox_browser", "browser host shut down (no sessions)");
        }
        Ok(())
    }

    pub async fn goto(&self, page_id: &str, url: &str) -> Result<(), String> {
        let page = self.page_ref(page_id).await?;
        page.goto(url).await.map_err(Self::map_page_err)?;
        Ok(())
    }

    pub async fn click(&self, page_id: &str, target: &str) -> Result<(), String> {
        let page = self.page_ref(page_id).await?;
        let el = resolve_element(&page, target).await?;
        el.click().await.map_err(Self::map_page_err)?;
        Ok(())
    }

    pub async fn fill(&self, page_id: &str, target: &str, value: &str) -> Result<(), String> {
        let page = self.page_ref(page_id).await?;
        let el = resolve_element(&page, target).await?;
        el.click().await.map_err(Self::map_page_err)?;
        el.type_str(value).await.map_err(Self::map_page_err)?;
        Ok(())
    }

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
                Err(_) => tokio::time::sleep(Duration::from_millis(200)).await,
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

    /// `target` empty → full document HTML via [`chromiumoxide::Page::content`].
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

    /// Trimmed page text for LLM prompts (lossy but bounded via env).
    pub async fn visible_text_summary(
        &self,
        page_id: &str,
        max_chars: usize,
    ) -> Result<String, String> {
        let page = self.page_ref(page_id).await?;
        let html = page.content().await.map_err(Self::map_page_err)?;
        let stripped = strip_html_tags(&html);
        let max_chars = max_chars.max(256);
        if stripped.len() <= max_chars {
            Ok(stripped)
        } else {
            Ok(format!("{}…", &stripped[..max_chars]))
        }
    }

    /// Extract the full Accessibility Tree (AXTree) for hybrid VLM analysis.
    pub async fn ax_tree(&self, page_id: &str) -> Result<serde_json::Value, String> {
        let page = self.page_ref(page_id).await?;
        let res = page
            .execute(chromiumoxide_cdp::cdp::browser_protocol::accessibility::GetFullAxTreeParams::default())
            .await
            .map_err(|e| format!("AXTree CDP failed: {e}"))?;

        serde_json::to_value(res.nodes.clone()).map_err(|e: serde_json::Error| e.to_string())
    }

    /// Layer 1: Deterministic Overlap Detector.
    /// Checks if any interactive elements (buttons, links, inputs) overlap visually.
    pub async fn check_overlaps(&self, page_id: &str) -> Result<Vec<OverlapFinding>, String> {
        let page = self.page_ref(page_id).await?;
        let interactive_selectors = "button, a, input, [role='button'], [role='link']";
        let elements = page
            .find_elements(interactive_selectors)
            .await
            .map_err(Self::map_page_err)?;

        let mut rects = Vec::with_capacity(elements.len());
        for el in elements {
            let box_model_res = page
                .execute(
                    chromiumoxide_cdp::cdp::browser_protocol::dom::GetBoxModelParams::builder()
                        .node_id(el.node_id)
                        .build(),
                )
                .await;

            if let Ok(res) = box_model_res {
                let box_model = &res.model;
                let points_json =
                    serde_json::to_value(&box_model.content).map_err(|e| e.to_string())?;
                let pts = points_json.as_array().ok_or("Quad is not an array")?;

                if pts.len() >= 8 {
                    let get_val = |idx: usize| pts[idx].as_f64().unwrap_or(0.0);
                    // chromiumoxide points are [x1, y1, x2, y2, x3, y3, x4, y4]
                    let x = get_val(0).min(get_val(2)).min(get_val(4)).min(get_val(6));
                    let y = get_val(1).min(get_val(3)).min(get_val(5)).min(get_val(7));
                    let x2 = get_val(0).max(get_val(2)).max(get_val(4)).max(get_val(6));
                    let y2 = get_val(1).max(get_val(3)).max(get_val(5)).max(get_val(7));

                    let node = el.description().await.map_err(Self::map_page_err)?;
                    let selector = node.node_name.clone();
                    rects.push(ElementRect {
                        selector,
                        x,
                        y,
                        w: x2 - x,
                        h: y2 - y,
                    });
                }
            }
        }

        let mut findings = Vec::new();
        for i in 0..rects.len() {
            for j in (i + 1)..rects.len() {
                let r1 = &rects[i];
                let r2 = &rects[j];

                if r1.overlaps(r2) {
                    findings.push(OverlapFinding {
                        element_1: r1.selector.clone(),
                        element_2: r2.selector.clone(),
                        overlap_area: r1.intersection_area(r2),
                    });
                }
            }
        }

        Ok(findings)
    }
}

pub struct ElementRect {
    pub selector: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl ElementRect {
    pub fn overlaps(&self, other: &Self) -> bool {
        self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }

    pub fn intersection_area(&self, other: &Self) -> f64 {
        let x_overlap = (self.x + self.w).min(other.x + other.w) - self.x.max(other.x);
        let y_overlap = (self.y + self.h).min(other.y + other.h) - self.y.max(other.y);
        if x_overlap > 0.0 && y_overlap > 0.0 {
            x_overlap * y_overlap
        } else {
            0.0
        }
    }
}

#[derive(serde::Serialize)]
pub struct OverlapFinding {
    pub element_1: String,
    pub element_2: String,
    pub overlap_area: f64,
}

async fn resolve_element(
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

fn strip_html_tags(html: &str) -> String {
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
    let collapsed: String = out.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
}

/// Global engine for MCP / CLI reuse within one process.
static GLOBAL_ENGINE: std::sync::OnceLock<Arc<BrowserEngine>> = std::sync::OnceLock::new();

#[must_use]
pub fn global_engine() -> Arc<BrowserEngine> {
    GLOBAL_ENGINE.get_or_init(BrowserEngine::new).clone()
}
