use super::*;

impl ModelRegistry {
    #[cfg_attr(test, allow(dead_code))]
    pub(super) fn maybe_refresh_catalogs(&mut self) {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let min_refresh = Self::min_refresh_interval().as_secs();
        let jitter = Self::jitter_ms();

        enum RefreshFail {
            Runtime(String),
            Fetch(String),
        }

        let joined = std::thread::spawn(move || -> Result<Option<Vec<ModelSpec>>, RefreshFail> {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| RefreshFail::Runtime(e.to_string()))?;
            rt.block_on(async {
                let db = vox_db::VoxDb::open_default()
                    .await
                    .map_err(|_| anyhow::anyhow!("db error"))?;
                if let Ok(Some(last_str)) = db
                    .get_user_preference("global", MODEL_CATALOG_LAST_REFRESH_KEY)
                    .await
                {
                    if let Ok(last_secs) = last_str.parse::<u64>() {
                        if now_secs.saturating_sub(last_secs) < min_refresh {
                            return Ok::<_, anyhow::Error>(None);
                        }
                    }
                }

                if jitter > 0 {
                    let offset = now_secs % (jitter + 1);
                    tokio::time::sleep(Duration::from_millis(offset)).await;
                }

                let mut models = OpenRouterCatalog::new().refresh().await.unwrap_or_default();
                // Mark all OpenRouter-sourced models with the correct pricing source.
                for m in &mut models {
                    if m.pricing_source == crate::models::spec::PricingSource::Bootstrap {
                        m.pricing_source = crate::models::spec::PricingSource::OpenRouter;
                    }
                }
                let repo_root = vox_repository::find_project_manifest_root(&std::env::current_dir().unwrap_or_default()).unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                if let Ok(mens_models) = crate::catalog::MensCatalog::new(&repo_root).refresh().await {
                    models.extend(mens_models);
                }
                crate::catalog_classifier::classify_models(&mut models).await;

                // Fetch LiteLLM pricing oracle to supplement cache costs and Anthropic pricing.
                let litellm_entries = crate::catalog::LiteLLMCatalog::new().fetch().await
                    .unwrap_or_default();

                // Fetch AnthropicDirect catalog (key-gated; silently skipped when no key is
                // present) so background refresh stays in parity with run_foreground_refresh().
                // no Anthropic key is expected in many environments
                if let Ok(mut anthropic_models) =
                    crate::catalog::AnthropicDirectCatalog::new().refresh().await
                {
                    for m in &mut anthropic_models {
                        if m.pricing_source == crate::models::spec::PricingSource::Bootstrap {
                            m.pricing_source =
                                crate::models::spec::PricingSource::AnthropicDirect;
                        }
                    }
                    for m in anthropic_models {
                        if !models.iter().any(|existing| existing.id == m.id) {
                            models.push(m);
                        }
                    }
                }

                #[cfg(feature = "populi-transport")]
                {
                    // Task 3.2: trusted, probed peers replace the asserted HTTP
                    // directory. A peer that is off is absent, so it cannot be
                    // routed work.
                    for peer in crate::models::mesh_directory::trusted_peers().await {

                                // Sybil/Reliability check: query peer reputation from db
                                let peer_key = peer.endpoint_id.to_string();
                                if let Ok(Some((success, fail, timeout, invalid))) = db.get_peer_reputation(&peer_key).await {
                                    let total_bad = fail + timeout + invalid;
                                    // Blacklist condition: more than 3 failures, and bad events exceed successful tasks.
                                    if total_bad > 3 && total_bad > success {
                                        tracing::warn!(target: "vox.orchestrator.models", peer=%peer_key, success, total_bad, "mesh peer blacklisted due to poor reputation");
                                        continue;
                                    }
                                }

                                for kind in peer.task_kinds {
                                let kind_str = serde_json::to_value(&kind).unwrap().as_str().unwrap().to_string();
                                    models.push(ModelSpec {
                                        id: format!("mesh/{}/{}", peer_key, kind_str),
                                        canonical_slug: format!("mesh/{}/{}", peer_key, kind_str),
                                        provider: "mens".to_string(),
                                        provider_type: ProviderType::PopuliMesh,
                                        max_tokens: 128_000,
                                        cost_per_1k: 0.0,
                                        cost_per_1k_input: 0.0,
                                        cost_per_1k_output: 0.0,
                                        is_free: true,
                                        strengths: vec![
                                            kind_str.parse::<crate::models::StrengthTag>().unwrap_or(crate::models::StrengthTag::Unknown),
                                            crate::models::StrengthTag::Generalist,
                                        ],
                                        capabilities: crate::models::spec::ModelCapabilities {
                                            tier: crate::models::ModelTier::Local,
                                            writes_vox: true,
                                            ..Default::default()
                                        },
                                        supported_parameters: vec![],
                                        observed_cost_per_1k: None,
                                        cache_creation_cost_per_1k: 0.0,
                                        cache_read_cost_per_1k: 0.0,
                                        supports_prompt_caching: false,
                                        pricing_source: super::spec::PricingSource::Bootstrap,
                                    });
                                }
                    }
                }

                let _ = db
                    .set_user_preference(
                        "global",
                        MODEL_CATALOG_LAST_REFRESH_KEY,
                        &now_secs.to_string(),
                    )
                    .await;

                // Apply LiteLLM pricing patches via the canonical apply_litellm_pricing()
                // method so both foreground and background refresh paths share the same
                // matching logic (exact → litellm_provider+suffix → suffix fallback).
                if !litellm_entries.is_empty() {
                    let mut tmp = ModelRegistry {
                        models: models.into_iter().map(|m| (m.id.clone(), m)).collect(),
                        agent_overrides: HashMap::new(),
                        premium_alias: HashMap::new(),
                        scoreboard: HashMap::new(),
                        bench: HashMap::new(),
                        arm_stats: HashMap::new(),
                        penalty_map: HashMap::new(),
                        operational_status: HashMap::new(),
                        reference: super::reference::RoutingReference::FALLBACK,
                    };
                    tmp.apply_litellm_pricing(&litellm_entries);
                    models = tmp.list_models();
                    tracing::debug!(
                        target: "vox.orchestrator.models",
                        litellm_entries = litellm_entries.len(),
                        "applied litellm pricing patches"
                    );
                }

                if let Ok(json) = serde_json::to_string(&models) {
                    let cache_file = vox_config::paths::dot_vox_user_dir().join("cache").join("model-catalog.v1.json");
                    if let Some(parent) = cache_file.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&cache_file, json);
                }

                Ok(Some(models))
            })
            .map_err(|e| RefreshFail::Fetch(e.to_string()))
        })
        .join();

        let models_opt = match joined {
            Ok(Ok(m)) => m,
            Ok(Err(RefreshFail::Runtime(msg))) => {
                tracing::warn!(target: "vox.orchestrator.models", error = %msg, "openrouter catalog runtime init failed");
                return;
            }
            Ok(Err(RefreshFail::Fetch(msg))) => {
                tracing::warn!(target: "vox.orchestrator.models", error = %msg, "openrouter model catalog refresh failed");
                return;
            }
            Err(_) => {
                tracing::warn!(target: "vox.orchestrator.models", "openrouter catalog refresh panicked");
                return;
            }
        };

        if let Some(models) = models_opt {
            let count = models.len();
            for m in models {
                self.register(m);
            }
            tracing::info!(target: "vox.orchestrator.models", count, "catalog refresh merged into model registry");
        } else {
            tracing::debug!(target: "vox.orchestrator.models", "catalog refresh skipped (within min refresh interval)");
        }
    }

    /// Create a registry loaded only from the on-disk cache and local config — **no network call**.
    ///
    /// Suitable for fast CLI commands (e.g. `vox model pricing show`) that need current pricing
    /// without triggering an OpenRouter/LiteLLM refresh.
    pub fn from_cache() -> Self {
        let mut registry = Self {
            models: HashMap::new(),
            agent_overrides: HashMap::new(),
            premium_alias: HashMap::new(),
            scoreboard: HashMap::new(),
            bench: HashMap::new(),
            arm_stats: HashMap::new(),
            penalty_map: HashMap::new(),
            operational_status: HashMap::new(),
            reference: super::reference::RoutingReference::FALLBACK,
        };

        if let Some(mut config_path) = vox_db::paths::config_dir() {
            config_path.push("models.toml");
            if config_path.exists() {
                if let Ok(contents) = vox_bounded_fs::read_utf8_path_capped(&config_path) {
                    let cfg: super::spec::ModelConfig = toml::from_str(&contents)
                        .unwrap_or_else(|_| super::spec::ModelConfig::default());
                    registry.premium_alias = if cfg.premium_alias.is_empty() {
                        vox_config::load_model_routing_config().premium_alias
                    } else {
                        cfg.premium_alias.clone()
                    };
                    for model in cfg.models {
                        registry.register(model);
                    }
                }
            }
        }

        if registry.models.is_empty() {
            // Seed from bootstrap so the registry is never empty.
            for model in super::spec::ModelConfig::default().models {
                registry.register(model);
            }
            registry.premium_alias = vox_config::load_model_routing_config().premium_alias;
        }

        let cache_file = vox_config::paths::dot_vox_user_dir()
            .join("cache")
            .join("model-catalog.v1.json");
        if let Ok(contents) = std::fs::read_to_string(&cache_file) {
            if let Ok(cached_models) =
                serde_json::from_str::<Vec<super::spec::ModelSpec>>(&contents)
            {
                for m in cached_models {
                    registry.register(m);
                }
            }
        }

        registry.register_mens_local_candidates();
        registry.apply_routing_reference();
        registry
    }

    /// Create a new model registry, loading from the configuration file or falling back to defaults.
    pub fn new() -> Self {
        let mut registry = Self {
            models: HashMap::new(),
            agent_overrides: HashMap::new(),
            premium_alias: HashMap::new(),
            scoreboard: HashMap::new(),
            bench: HashMap::new(),
            arm_stats: HashMap::new(),
            penalty_map: HashMap::new(),
            operational_status: HashMap::new(),
            reference: super::reference::RoutingReference::FALLBACK,
        };

        // In test mode, keep tests hermetic on ModelConfig::default().
        // In production, try to load from models.toml in the config directory.
        #[cfg(not(test))]
        let model_config = if let Some(mut config_path) = vox_db::paths::config_dir() {
            config_path.push("models.toml");
            if config_path.exists() {
                if let Ok(contents) = vox_bounded_fs::read_utf8_path_capped(&config_path) {
                    toml::from_str(&contents).unwrap_or_else(|_| ModelConfig::default())
                } else {
                    ModelConfig::default()
                }
            } else {
                let default_config = ModelConfig::default();
                // Create config dir if needed and write default file
                if let Some(parent) = config_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                    if let Ok(toml_str) = toml::to_string_pretty(&default_config) {
                        let _ = std::fs::write(&config_path, toml_str);
                    }
                }
                default_config
            }
        } else {
            ModelConfig::default()
        };

        #[cfg(test)]
        let model_config = ModelConfig::default();

        registry.premium_alias = if model_config.premium_alias.is_empty() {
            vox_config::load_model_routing_config().premium_alias
        } else {
            model_config.premium_alias.clone()
        };

        for model in model_config.models {
            registry.register(model);
        }

        #[cfg(not(test))]
        {
            let cache_file = vox_config::paths::dot_vox_user_dir()
                .join("cache")
                .join("model-catalog.v1.json");
            if let Ok(contents) = std::fs::read_to_string(&cache_file) {
                if let Ok(cached_models) = serde_json::from_str::<Vec<ModelSpec>>(&contents) {
                    for m in cached_models {
                        registry.register(m);
                    }
                }
            }
        }

        // Live catalog merge hits the network and shifts `best_for` rankings; keep unit tests on the
        // static TOML/default model list unless integration coverage opts in elsewhere.
        #[cfg(not(test))]
        registry.maybe_refresh_catalogs();

        registry.register_mens_local_candidates();
        // Test builds stay on the fixed constants (hermetic, like `maybe_refresh_catalogs`).
        #[cfg(not(test))]
        registry.apply_routing_reference();
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_seeds_a_well_formed_non_empty_registry() {
        let reg = ModelRegistry::new();
        let models = reg.list_models();
        assert!(!models.is_empty());
        let mut ids = std::collections::HashSet::new();
        for m in &models {
            assert!(!m.id.is_empty(), "model id must not be empty");
            assert!(ids.insert(m.id.clone()), "duplicate model id {}", m.id);
            assert!(m.max_tokens > 0, "{} has zero max_tokens", m.id);
            assert!(m.cost_per_1k >= 0.0, "{} has negative cost", m.id);
        }
    }

    #[test]
    fn new_registers_mens_local_candidates() {
        let reg = ModelRegistry::new();
        let m = reg
            .get("vox-mens-8b-v0.6")
            .expect("mens 8b candidate seeded");
        assert_eq!(m.provider_type, ProviderType::VoxLocal);
        assert!(m.is_free);
    }
}
