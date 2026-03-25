use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderLimitOwned {
    pub provider: String,
    pub model: String,
    pub daily_limit: u32,
}

fn parse_limit_json(raw: &str) -> Vec<ProviderLimitOwned> {
    let parsed: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let Some(obj) = parsed.as_object() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (k, v) in obj {
        let Some(limit) = v.as_u64() else {
            continue;
        };
        let mut parts = k.splitn(2, '/');
        let provider = parts.next().unwrap_or_default().trim();
        let model = parts.next().unwrap_or_default().trim();
        if provider.is_empty() || model.is_empty() {
            continue;
        }
        out.push(ProviderLimitOwned {
            provider: provider.to_string(),
            model: model.to_string(),
            daily_limit: limit.min(u32::MAX as u64) as u32,
        });
    }
    out
}

fn default_limits() -> Vec<ProviderLimitOwned> {
    let default_cloud_limit = std::env::var("VOX_PROVIDER_DAILY_LIMIT_DEFAULT")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(100);
    let providers = std::env::var("VOX_PROVIDER_LIMIT_PROVIDERS")
        .ok()
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            vec![
                "google".to_string(),
                "openrouter".to_string(),
                "ollama".to_string(),
                "groq".to_string(),
                "cerebras".to_string(),
                "mistral".to_string(),
                "deepseek".to_string(),
                "sambanova".to_string(),
                "custom".to_string(),
            ]
        });
    let mut out = Vec::new();
    for provider in providers {
        let (model, daily_limit) = match provider.as_str() {
            "openrouter" => (":free".to_string(), default_cloud_limit / 2),
            "ollama" => ("*".to_string(), u32::MAX),
            _ => ("*".to_string(), default_cloud_limit),
        };
        out.push(ProviderLimitOwned {
            provider,
            model,
            daily_limit: daily_limit.max(1),
        });
    }
    out
}

pub fn resolve_provider_limits() -> Vec<ProviderLimitOwned> {
    let mut merged: BTreeMap<(String, String), u32> = BTreeMap::new();
    for d in default_limits() {
        merged.insert((d.provider, d.model), d.daily_limit);
    }

    if let Ok(path) = std::env::var("VOX_PROVIDER_DAILY_LIMITS_FILE") {
        let p = std::path::PathBuf::from(path);
        if let Ok(raw) = std::fs::read_to_string(&p) {
            for d in parse_limit_json(&raw) {
                merged.insert((d.provider, d.model), d.daily_limit);
            }
        }
    }
    if let Ok(raw) = std::env::var("VOX_PROVIDER_DAILY_LIMITS_JSON") {
        for d in parse_limit_json(&raw) {
            merged.insert((d.provider, d.model), d.daily_limit);
        }
    }

    merged
        .into_iter()
        .map(|((provider, model), daily_limit)| ProviderLimitOwned {
            provider,
            model,
            daily_limit,
        })
        .collect()
}
