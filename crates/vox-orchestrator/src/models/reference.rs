//! Routing scales derived from the live catalog, so a benchmark rescale or a market-wide price move
//! changes routing without a code edit. Each value falls back to the fixed constant it replaces when
//! the catalog is too small to derive from (offline bootstrap, small test registries). The registry
//! holds the current reference and stamps each spec's `QualityPrior` from it.

use serde::{Deserialize, Serialize};

use super::spec::{QualityPrior, QualitySource};
use super::{ModelSpec, ProviderType};

/// Fewest benchmarked (or priced) cloud models a value is derived from.
pub const MIN_DERIVE_SAMPLE: usize = 20;
/// Paid output-price quantiles for the Elite and Pro bands. On the 2026-09-28 OpenRouter catalog
/// (343 paid models) they reproduce the former fixed bands exactly: $20/M at 0.93, $4/M at 0.70.
pub const ELITE_PRICE_QUANTILE: f64 = 0.93;
pub const PRO_PRICE_QUANTILE: f64 = 0.70;
/// Weight on an index inherited from an older family member.
// ponytail: flat discount; learn it from outcomes once the family-keyed scoreboard lands.
pub const INHERITED_INDEX_DISCOUNT: f32 = 0.95;
const UNBENCHMARKED_SCALE_MIN: f64 = 0.3;
const UNBENCHMARKED_SCALE_MAX: f64 = 1.0;

/// Whether a value was derived from the catalog or is the built-in fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceSource {
    Derived,
    Fallback,
}

/// The catalog-derived scales routing scores against.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RoutingReference {
    /// Intelligence index treated as quality 1.0.
    pub quality_reference: f64,
    /// Scale on the proxy for unbenchmarked OpenRouter models.
    pub unbenchmarked_scale: f64,
    pub quality_source: ReferenceSource,
    /// USD per 1k output tokens at or above which a paid model is Elite.
    pub elite_min_out: f64,
    /// USD per 1k output tokens at or above which a paid model is Pro.
    pub pro_min_out: f64,
    pub bands_source: ReferenceSource,
    /// Benchmarked OpenRouter models the quality values came from (0 on fallback).
    pub benchmarked: usize,
    /// Paid, priced cloud models the bands came from (0 on fallback).
    pub priced: usize,
}

impl Default for RoutingReference {
    fn default() -> Self {
        Self::FALLBACK
    }
}

impl RoutingReference {
    /// The fixed constants this module replaces.
    pub const FALLBACK: Self = Self {
        quality_reference: super::scoring::QUALITY_INDEX_REFERENCE,
        unbenchmarked_scale: super::scoring::UNBENCHMARKED_QUALITY_SCALE,
        quality_source: ReferenceSource::Fallback,
        elite_min_out: super::tiering::ELITE_MIN_OUTPUT_USD_PER_1K,
        pro_min_out: super::tiering::PRO_MIN_OUTPUT_USD_PER_1K,
        bands_source: ReferenceSource::Fallback,
        benchmarked: 0,
        priced: 0,
    };

    /// Derive from registered specs. Local models never shape the reference; free models
    /// shape the quality scale but not the price bands.
    #[must_use]
    pub fn derive<'a>(specs: impl IntoIterator<Item = &'a ModelSpec>) -> Self {
        let mut out = Self::FALLBACK;
        let mut indexed: Vec<(f64, f64)> = Vec::new();
        let mut paid: Vec<f64> = Vec::new();
        for m in specs {
            if is_local(m) {
                continue;
            }
            if let Some(i) = m
                .capabilities
                .intelligence_index
                .filter(|i| i.is_finite() && *i > 0.0)
            {
                if m.provider_type == ProviderType::OpenRouter {
                    indexed.push((f64::from(i), super::scoring::proxy_quality(m)));
                }
            }
            if !m.is_free && m.cost_per_1k_output.is_finite() && m.cost_per_1k_output > 0.0 {
                paid.push(m.cost_per_1k_output);
            }
        }
        if indexed.len() >= MIN_DERIVE_SAMPLE {
            let top = indexed.iter().map(|(i, _)| *i).fold(f64::MIN, f64::max);
            let norm = median(indexed.iter().map(|(i, _)| i / top).collect());
            let proxy = median(indexed.iter().map(|(_, p)| *p).collect());
            out.quality_reference = top;
            if proxy > 0.0 {
                out.unbenchmarked_scale =
                    (norm / proxy).clamp(UNBENCHMARKED_SCALE_MIN, UNBENCHMARKED_SCALE_MAX);
            }
            out.quality_source = ReferenceSource::Derived;
            out.benchmarked = indexed.len();
        }
        if paid.len() >= MIN_DERIVE_SAMPLE {
            paid.sort_by(f64::total_cmp);
            out.elite_min_out = quantile(&paid, ELITE_PRICE_QUANTILE);
            out.pro_min_out = quantile(&paid, PRO_PRICE_QUANTILE);
            out.bands_source = ReferenceSource::Derived;
            out.priced = paid.len();
        }
        out
    }

    /// Quality prior for `m`. `inherited` is its family's benchmark `(index, from_id)`, used only
    /// when `m` has no index of its own.
    #[must_use]
    pub fn quality_prior(&self, m: &ModelSpec, inherited: Option<(f32, &str)>) -> QualityPrior {
        if let Some(index) = m.capabilities.intelligence_index {
            return QualityPrior {
                value: self.normalise(f64::from(index)),
                source: QualitySource::Benchmark { index },
            };
        }
        if let Some((index, from)) = inherited {
            return QualityPrior {
                value: self.normalise(f64::from(index * INHERITED_INDEX_DISCOUNT)),
                source: QualitySource::Inherited {
                    index,
                    from: from.to_string(),
                },
            };
        }
        let scale = if m.provider_type == ProviderType::OpenRouter {
            self.unbenchmarked_scale
        } else {
            1.0
        };
        QualityPrior {
            value: (super::scoring::proxy_quality(m) * scale).clamp(0.0, 1.0),
            source: QualitySource::Estimate,
        }
    }

    fn normalise(&self, index: f64) -> f64 {
        (index / self.quality_reference).clamp(0.0, 1.0)
    }
}

fn is_local(m: &ModelSpec) -> bool {
    crate::route_policy::is_local_http_provider(&m.provider_type)
}

/// Value at `floor(q * (n - 1))` of an ascending, non-empty slice.
fn quantile(sorted: &[f64], q: f64) -> f64 {
    sorted[((q * (sorted.len() - 1) as f64).floor() as usize).min(sorted.len() - 1)]
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::StrengthTag;
    use crate::models::spec::{ModelCapabilities, PricingSource, QualityPrior, QualitySource};

    fn spec(
        id: &str,
        provider_type: ProviderType,
        out_per_1k: f64,
        index: Option<f32>,
    ) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type,
            max_tokens: 128_000,
            cost_per_1k: out_per_1k,
            cost_per_1k_input: out_per_1k / 4.0,
            cost_per_1k_output: out_per_1k,
            is_free: out_per_1k == 0.0,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Generalist],
            capabilities: ModelCapabilities {
                intelligence_index: index,
                ..Default::default()
            },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::OpenRouter,
            supported_parameters: vec![],
        }
    }

    /// 40 paid OpenRouter models: output prices 0.0005 … 0.0395 per 1k, indexes 10 … 49.
    fn catalog(price_scale: f64, index_scale: f32) -> Vec<ModelSpec> {
        (0..40)
            .map(|i| {
                spec(
                    &format!("acme/m-{i}"),
                    ProviderType::OpenRouter,
                    (0.0005 + 0.001 * f64::from(i)) * price_scale,
                    Some((10 + i) as f32 * index_scale),
                )
            })
            .collect()
    }

    #[test]
    fn a_small_catalog_uses_the_fallback_constants() {
        let small: Vec<ModelSpec> = catalog(1.0, 1.0)
            .into_iter()
            .take(MIN_DERIVE_SAMPLE - 1)
            .collect();
        assert_eq!(RoutingReference::derive(&small), RoutingReference::FALLBACK);
        assert_eq!(
            RoutingReference::derive(std::iter::empty::<&ModelSpec>()),
            RoutingReference::FALLBACK
        );
    }

    #[test]
    fn quality_reference_is_the_top_benchmark_and_follows_a_rescale() {
        let r = RoutingReference::derive(&catalog(1.0, 1.0));
        assert_eq!(r.quality_source, ReferenceSource::Derived);
        assert_eq!(r.quality_reference, 49.0);
        assert_eq!(r.benchmarked, 40);
        assert_eq!(
            RoutingReference::derive(&catalog(1.0, 2.0)).quality_reference,
            98.0
        );
    }

    #[test]
    fn bands_are_the_configured_quantiles_of_paid_prices() {
        // n = 40: Elite at floor(0.93 * 39) = 36, Pro at floor(0.70 * 39) = 27.
        let r = RoutingReference::derive(&catalog(1.0, 1.0));
        assert_eq!(r.bands_source, ReferenceSource::Derived);
        assert!(
            (r.elite_min_out - 0.0365).abs() < 1e-12,
            "{}",
            r.elite_min_out
        );
        assert!((r.pro_min_out - 0.0275).abs() < 1e-12, "{}", r.pro_min_out);
        assert_eq!(r.priced, 40);
    }

    #[test]
    fn tiers_survive_a_market_wide_price_move() {
        let base = RoutingReference::derive(&catalog(1.0, 1.0));
        let halved = RoutingReference::derive(&catalog(0.5, 1.0));
        assert!((halved.elite_min_out - base.elite_min_out * 0.5).abs() < 1e-12);
        assert!((halved.pro_min_out - base.pro_min_out * 0.5).abs() < 1e-12);
        for (a, b) in catalog(1.0, 1.0).iter().zip(catalog(0.5, 1.0).iter()) {
            assert_eq!(
                crate::models::tiering::derive_tier_with(
                    false,
                    a.cost_per_1k_output,
                    base.elite_min_out,
                    base.pro_min_out
                ),
                crate::models::tiering::derive_tier_with(
                    false,
                    b.cost_per_1k_output,
                    halved.elite_min_out,
                    halved.pro_min_out
                ),
                "{}",
                a.id
            );
        }
    }

    #[test]
    fn free_and_local_models_do_not_shape_the_reference() {
        let mut specs = catalog(1.0, 1.0);
        let base = RoutingReference::derive(&specs);
        for i in 0..50 {
            specs.push(spec(
                &format!("acme/free-{i}"),
                ProviderType::OpenRouter,
                0.0,
                None,
            ));
            specs.push(spec(
                &format!("local/big-{i}"),
                ProviderType::Ollama,
                1.0,
                Some(99.0),
            ));
        }
        assert_eq!(RoutingReference::derive(&specs), base);
    }

    #[test]
    fn unbenchmarked_scale_is_fitted_to_benchmarked_models() {
        let specs = catalog(1.0, 1.0);
        let r = RoutingReference::derive(&specs);
        // Median of index/top over 10..=49 is 29.5/49; every proxy is equal (same context, all paid).
        let proxy = crate::models::scoring::proxy_quality(&specs[0]);
        let expected = ((29.5 / 49.0) / proxy).clamp(0.3, 1.0);
        assert!(
            (r.unbenchmarked_scale - expected).abs() < 1e-12,
            "{} vs {expected}",
            r.unbenchmarked_scale
        );
    }

    #[test]
    fn quality_prior_says_where_it_came_from() {
        let r = RoutingReference::derive(&catalog(1.0, 1.0));
        let benched = r.quality_prior(
            &spec("acme/b", ProviderType::OpenRouter, 0.01, Some(24.5)),
            None,
        );
        assert!((benched.value - 0.5).abs() < 1e-12);
        assert_eq!(benched.source, QualitySource::Benchmark { index: 24.5 });
        let unbenched = spec("acme/u", ProviderType::OpenRouter, 0.01, None);
        let inherited = r.quality_prior(&unbenched, Some((24.5, "acme/b")));
        assert!(
            (inherited.value - f64::from(24.5_f32 * INHERITED_INDEX_DISCOUNT) / 49.0).abs() < 1e-6
        );
        assert_eq!(
            inherited.source,
            QualitySource::Inherited {
                index: 24.5,
                from: "acme/b".into()
            }
        );
        let estimate_or = r.quality_prior(&unbenched, None);
        let estimate_direct =
            r.quality_prior(&spec("acme/d", ProviderType::Anthropic, 0.01, None), None);
        assert_eq!(estimate_or.source, QualitySource::Estimate);
        assert!(
            estimate_or.value < estimate_direct.value,
            "only OpenRouter's long tail is scaled down"
        );
    }

    #[test]
    fn the_fallback_prior_equals_the_unstamped_formula() {
        for m in [
            spec("acme/b", ProviderType::OpenRouter, 0.01, Some(47.5)),
            spec("acme/u", ProviderType::OpenRouter, 0.01, None),
            spec("acme/d", ProviderType::Anthropic, 0.01, None),
            spec("local/q", ProviderType::Ollama, 0.0, None),
        ] {
            assert_eq!(
                RoutingReference::FALLBACK.quality_prior(&m, None).value,
                crate::models::scoring::quality_score(&m),
                "{}",
                m.id
            );
        }
    }

    #[test]
    fn quality_score_reads_a_stamped_prior() {
        let mut m = spec("acme/s", ProviderType::OpenRouter, 0.01, Some(47.5));
        m.capabilities.quality_prior = Some(QualityPrior {
            value: 0.123,
            source: QualitySource::Estimate,
        });
        assert_eq!(crate::models::scoring::quality_score(&m), 0.123);
    }
}
