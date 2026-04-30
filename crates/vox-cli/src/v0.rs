//! Integration with **v0.dev** for AI-generated React components referenced from Vox `@v0` declarations.
//!
//! Requires `V0_API_KEY`. Called from `commands::build` when a generated `.tsx` path is still missing.
//!
//! When built with **`feature = "island"`**, also exposes [`IslandCache`], [`generate_island_tsx`], and
//! [`emit_island_stub`] for the `vox island` command.

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

use base64::Engine;

#[cfg(feature = "island")]
use crate::island_paths::{island_component_dir, island_component_tsx_path};

const V0_API_URL: &str = "https://api.v0.dev/v1/chats";

/// Full URL for the v0 chats endpoint (including path). Override with **`VOX_V0_API_URL`** for tests or proxies.
fn v0_chats_url() -> String {
    std::env::var("VOX_V0_API_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| V0_API_URL.to_string())
}

fn extract_tsx_from_chat_response(chat_res: ChatResponse) -> Result<String> {
    if let Some(files) = chat_res.files {
        for file in files {
            if file.name.ends_with(".tsx") || file.name.ends_with(".jsx") {
                return Ok(file.content);
            }
        }
        Err(anyhow!("v0 response did not contain any .tsx/.jsx files"))
    } else {
        Err(anyhow!("v0 response did not contain any files"))
    }
}

#[derive(Serialize)]
struct ChatRequest {
    message: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ChatResponse {
    id: String,
    files: Option<Vec<V0File>>,
    demo: Option<String>,
}

#[derive(Deserialize)]
struct V0File {
    name: String,
    content: String,
}

fn v0_refined_prompt(component_name: &str, user_instruction: &str) -> String {
    format!(
        "Create a React component named {component_name}. {user_instruction}. \
        Return ONLY the code for this component in a file named {component_name}.tsx. \
        Use Tailwind CSS for styling. Use a **named** export: `export function {component_name}` \
        (not default export) so Vox `routes:` can import `{{ {component_name} }}`."
    )
}

async fn fetch_v0_tsx(
    component_name: &str,
    user_instruction: &str,
    image_path: Option<&Path>,
) -> Result<String> {
    let api_key = vox_clavis::resolve_secret(vox_clavis::SecretId::V0ApiKey)
        .expose()
        .map(std::string::ToString::to_string)
        .ok_or_else(|| {
            anyhow!(
                "V0_API_KEY environment variable not found. Please set it to use @v0 components."
            )
        })?;

    if let Some(path) = image_path {
        info!(
            "Generating v0 component '{}' with image: {:?}",
            component_name, path
        );
    } else {
        info!(
            "Generating v0 component '{}' with prompt: \"{}\"",
            component_name, user_instruction
        );
    }

    let client = reqwest::Client::new();
    let message = v0_refined_prompt(component_name, user_instruction);

    let image_data = if let Some(path) = image_path {
        let bytes = fs::read(path).context(format!("Failed to read image file: {:?}", path))?;
        Some(base64::engine::general_purpose::STANDARD.encode(&bytes))
    } else {
        None
    };

    let req_body = ChatRequest {
        message,
        stream: false,
        image: image_data,
    };

    let url = v0_chats_url();
    let res = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&req_body)
        .send()
        .await
        .with_context(|| format!("Failed to send request to v0 API ({url})"))?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(anyhow!("v0 API error ({}): {}", status, text));
    }

    let chat_res: ChatResponse = res
        .json()
        .await
        .context("Failed to parse v0 API response")?;

    extract_tsx_from_chat_response(chat_res)
}

/// Generate a UI component using v0.dev based on a prompt.
///
/// This function calls the v0 Platform API to generate React code.
/// It expects the `V0_API_KEY` environment variable to be set.
pub async fn generate_component(
    prompt: &str,
    component_name: &str,
    out_dir: &Path,
    image_path: Option<&Path>,
) -> Result<PathBuf> {
    let raw = fetch_v0_tsx(component_name, prompt, image_path).await?;
    let content = crate::v0_tsx_normalize::normalize_v0_tsx_named_export(raw, component_name);
    let file_path = out_dir.join(format!("{component_name}.tsx"));
    fs::write(&file_path, &content).context(format!(
        "Failed to write generated component to {:?}",
        file_path
    ))?;
    info!("Successfully generated v0 component at {:?}", file_path);
    Ok(file_path)
}

// ── Island support (`--features island`) ─────────────────────────────────────

#[cfg(feature = "island")]
fn island_cache_safe_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_string()
            } else {
                "_".to_string()
            }
        })
        .collect()
}

/// One cached v0 island generation under `~/.vox/island-cache/`.
#[cfg(feature = "island")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IslandCacheEntry {
    /// PascalCase island name (matches `vox island generate <NAME>`).
    pub name: String,
    /// Prompt used for this generation.
    pub prompt: String,
    /// Unix seconds when cached.
    pub generated_at: u64,
    /// Full TSX source.
    pub tsx: String,
}

/// JSON cache in `~/.vox/island-cache/` for `vox island generate` (skip API when prompt matches).
#[cfg(feature = "island")]
use crate::commands::ci::bounded_read::read_utf8_path_capped;

#[cfg(feature = "island")]
pub struct IslandCache {
    /// Absolute path to the cache directory (typically `~/.vox/island-cache`).
    root: PathBuf,
}

#[cfg(feature = "island")]
impl IslandCache {
    /// Opens the cache directory, creating it if needed.
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow!("Cannot resolve home directory"))?;
        let root = home.join(".vox").join("island-cache");
        fs::create_dir_all(&root).context("create island cache dir")?;
        Ok(Self { root })
    }

    fn path_for(&self, name: &str) -> PathBuf {
        self.root
            .join(format!("{}.json", island_cache_safe_name(name)))
    }

    /// Load a cache entry by island name, if present.
    pub fn get(&self, name: &str) -> Result<Option<IslandCacheEntry>> {
        let p = self.path_for(name);
        if !p.exists() {
            return Ok(None);
        }
        let s = read_utf8_path_capped(&p).context("read island cache")?;
        let e: IslandCacheEntry = serde_json::from_str(&s).context("parse island cache JSON")?;
        Ok(Some(e))
    }

    /// Write or overwrite cache for this name.
    pub fn put(&self, name: &str, prompt: &str, tsx: &str) -> Result<()> {
        let generated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let e = IslandCacheEntry {
            name: name.to_string(),
            prompt: prompt.to_string(),
            generated_at,
            tsx: tsx.to_string(),
        };
        let p = self.path_for(name);
        let s = serde_json::to_string_pretty(&e).context("serialize island cache")?;
        fs::write(&p, s).context("write island cache")?;
        Ok(())
    }

    /// Sorted list of entries (reads every `*.json` in the cache dir).
    pub fn list(&self) -> Result<Vec<IslandCacheEntry>> {
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.root).context("read island cache dir")? {
            let entry = entry?;
            let p = entry.path();
            if p.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let s = match read_utf8_path_capped(&p) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if let Ok(e) = serde_json::from_str::<IslandCacheEntry>(&s) {
                out.push(e);
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Remove all `*.json` cache files.
    pub fn clear(&self) -> Result<usize> {
        let mut n = 0usize;
        for entry in fs::read_dir(&self.root).context("read island cache dir")? {
            let entry = entry?;
            let p = entry.path();
            if p.extension().and_then(|x| x.to_str()) == Some("json") {
                fs::remove_file(&p).ok();
                n += 1;
            }
        }
        Ok(n)
    }

    /// Remove one entry by island name.
    pub fn remove(&self, name: &str) -> Result<usize> {
        let p = self.path_for(name);
        if p.exists() {
            fs::remove_file(&p).context("remove island cache file")?;
            Ok(1)
        } else {
            Ok(0)
        }
    }
}

/// Generate or restore TSX at `islands/src/<Name>/<Name>.component.tsx` (API or cache).
#[cfg(feature = "island")]
pub async fn generate_island_tsx(
    prompt: &str,
    component_name: &str,
    project_root: &Path,
    image_path: Option<&Path>,
    force_refresh: bool,
) -> Result<PathBuf> {
    let island_dir = island_component_dir(project_root, component_name);
    fs::create_dir_all(&island_dir).context("create islands/src/<Name>/")?;
    let out = island_component_tsx_path(project_root, component_name);

    if !force_refresh {
        if let Ok(cache) = IslandCache::new() {
            if let Some(entry) = cache.get(component_name)? {
                if entry.prompt == prompt {
                    fs::write(&out, &entry.tsx).context("write cached island TSX")?;
                    return Ok(out);
                }
            }
        }
    }

    let raw = fetch_v0_tsx(component_name, prompt, image_path).await?;
    let content = crate::v0_tsx_normalize::normalize_v0_tsx_named_export(raw, component_name);

    // TASK-5.4: Run a11y validator on the generated TSX before writing.
    let a11y_diags = crate::v0_tsx_validate::validate_tsx_a11y(&content);
    if let Some(report) = crate::v0_tsx_validate::format_diagnostics(&a11y_diags, component_name) {
        eprintln!("\n{report}\n");
        if crate::v0_tsx_validate::has_errors(&a11y_diags) {
            eprintln!(
                "⚠️  Error-level a11y violations detected. The island will still be written,\n\
                 but you should fix these before shipping. Re-run with a more specific prompt\n\
                 or patch the generated TSX manually.\n"
            );
        }
    }

    fs::write(&out, &content).context("write island TSX")?;
    if let Ok(cache) = IslandCache::new() {
        let _ = cache.put(component_name, prompt, &content);
    }
    Ok(out)
}

/// Build an `@island` Vox stub from TSX source (heuristic prop inference from `*Props` interfaces).
#[cfg(feature = "island")]
pub fn emit_island_stub(
    tsx_source: &str,
    component_name: &str,
    _target_vox: Option<&Path>,
) -> String {
    let props = infer_island_props(tsx_source, component_name);
    let mut lines = vec![format!("@island {component_name}:")];
    if props.is_empty() {
        lines.push(
            "  # No props inferred — add fields manually if this island takes props.".to_string(),
        );
    } else {
        lines.push("  # Inferred from TypeScript; adjust Vox types as needed.".to_string());
        for (k, v) in props {
            lines.push(format!("  {k}: {v}"));
        }
    }
    lines.join("\n")
}

#[cfg(feature = "island")]
fn infer_island_props(tsx: &str, component_name: &str) -> Vec<(String, String)> {
    use regex::Regex;

    let preferred = format!("{component_name}Props");
    let re_iface = Regex::new(r"(?s)interface\s+(\w+)\s*\{([^}]*)\}").expect("valid regex");
    let re_type = Regex::new(r"(?s)type\s+(\w+)\s*=\s*\{([^}]*)\}").expect("valid regex");

    let mut block: Option<(String, &str)> = None;
    for cap in re_iface.captures_iter(tsx) {
        let iface_name = cap
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let body = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        if iface_name == preferred || block.is_none() {
            block = Some((iface_name.clone(), body));
            if iface_name == preferred {
                break;
            }
        }
    }
    if block.as_ref().map(|(n, _)| n.as_str()) != Some(preferred.as_str()) {
        for cap in re_type.captures_iter(tsx) {
            let type_name = cap
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let body = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            if type_name == preferred || block.is_none() {
                block = Some((type_name.clone(), body));
                if type_name == preferred {
                    break;
                }
            }
        }
    }

    let Some((_, body)) = block else {
        return Vec::new();
    };

    let re_prop = Regex::new(r#"(?m)^\s*(\w+)\s*\??\s*:\s*([^;\n]+);?"#).expect("valid regex");
    let mut out = Vec::new();
    for cap in re_prop.captures_iter(body) {
        let key = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        if key.is_empty() || key.starts_with('/') {
            continue;
        }
        let ts_ty = cap.get(2).map(|m| m.as_str().trim()).unwrap_or("str");
        let vox_ty = ts_type_to_vox(ts_ty);
        out.push((key.to_string(), vox_ty));
    }
    out
}

#[cfg(feature = "island")]
fn ts_type_to_vox(ts: &str) -> String {
    let t = ts.split('|').next().unwrap_or(ts).trim().to_lowercase();
    let t = t.trim_end_matches(';').trim();
    if t.contains("boolean") || t == "bool" {
        "bool".to_string()
    } else if t.contains("number") || t == "int" || t == "integer" {
        "int".to_string()
    } else if t.contains("string") || t.contains("react.reactnode") || t.contains("reactnode") {
        "str".to_string()
    } else if t == "any" || t.is_empty() {
        "str".to_string()
    } else {
        "str".to_string()
    }
}

#[cfg(test)]
mod v0_response_tests {
    use super::*;

    #[test]
    fn extract_tsx_prefers_tsx_file() {
        let chat = ChatResponse {
            id: "1".into(),
            files: Some(vec![
                V0File {
                    name: "readme.md".into(),
                    content: "# x".into(),
                },
                V0File {
                    name: "Widget.tsx".into(),
                    content: "export function Widget() {}".into(),
                },
            ]),
            demo: None,
        };
        let s = extract_tsx_from_chat_response(chat).unwrap();
        assert!(s.contains("Widget"));
    }

    #[test]
    fn extract_tsx_errors_when_no_files() {
        let chat = ChatResponse {
            id: "1".into(),
            files: None,
            demo: None,
        };
        assert!(extract_tsx_from_chat_response(chat).is_err());
    }
}

#[cfg(test)]
#[allow(unsafe_code)]
mod v0_wiremock_tests {
    use super::*;
    use serial_test::serial;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn restore_env(key: &str, prev: Option<String>) {
        // SAFETY: `serial_test` runs this module's tests sequentially; we restore before returning.
        unsafe {
            match prev {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[serial]
    async fn fetch_hits_vox_v0_api_url() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chats"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"id":"mock","files":[{"name":"Demo.tsx","content":"export function Demo() { return null; }"}]}"#,
            ))
            .mount(&server)
            .await;

        let url = format!("{}/v1/chats", server.uri());
        let prev_url = std::env::var("VOX_V0_API_URL").ok();
        let prev_key = std::env::var(vox_clavis::SecretId::V0ApiKey.spec().canonical_env).ok();
        // SAFETY: paired with `restore_env` below; serialized by `#[serial]`.
        unsafe {
            std::env::set_var("VOX_V0_API_URL", url.as_str());
            std::env::set_var(
                vox_clavis::SecretId::V0ApiKey.spec().canonical_env,
                "test-key",
            );
        }

        let got = fetch_v0_tsx("Demo", "make a card", None)
            .await
            .expect("fetch");
        assert!(got.contains("export function Demo"));

        restore_env("VOX_V0_API_URL", prev_url);
        restore_env(
            vox_clavis::SecretId::V0ApiKey.spec().canonical_env,
            prev_key,
        );
    }
}

#[cfg(all(test, feature = "island"))]
mod island_stub_tests {
    use super::*;

    #[test]
    fn emit_stub_includes_inferred_props() {
        let tsx = r#"
export interface AgentCardProps {
  title: string;
  count?: number;
  active: boolean;
}
export default function AgentCard(props: AgentCardProps) { return null; }
"#;
        let s = emit_island_stub(tsx, "AgentCard", None);
        assert!(s.contains("@island AgentCard:"));
        assert!(s.contains("title"));
        assert!(s.contains("count"));
        assert!(s.contains("active"));
    }
}
