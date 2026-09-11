//! Suitability filter for cloud GPU offers.
//!
//! `CloudResolver` ranks offers by cost. Cost alone cannot express "this offer is
//! structurally wrong for the job": an 8-GPU node reports 640 GB of VRAM and bills
//! for eight GPUs to run a single-device QLoRA, and a marketplace host with a 12%
//! reliability score will evict you.

use super::{CloudProviderConfig, GpuOffer};

/// Why an offer cannot run this job. Carries the measured values so the CLI can
/// print a reason the operator can act on rather than "no offers found".
#[derive(Debug, Clone, PartialEq)]
pub enum UnsuitableReason {
    /// A paid multi-GPU node for a single-device job: you are billed for silicon
    /// a single-device QLoRA will never touch.
    MultiGpuNode {
        /// GPUs in the offer.
        gpu_count: u32,
        /// Hourly price for the whole node.
        price_per_hour_usd: f64,
    },
    /// No single device in the offer holds the working set.
    InsufficientVram {
        /// `vram_mb / gpu_count` — what one device actually has.
        per_gpu_mb: u64,
        /// What `plan_for` says the job needs.
        required_mb: u64,
    },
    /// Host reliability below the configured floor.
    Unreliable {
        /// The offer's score, [0, 100].
        reliability_pct: f32,
        /// `config.min_reliability * 100.0`.
        floor_pct: f32,
    },
}

impl std::fmt::Display for UnsuitableReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MultiGpuNode {
                gpu_count,
                price_per_hour_usd,
            } => write!(
                f,
                "{gpu_count}-GPU node at ${price_per_hour_usd:.2}/hr; this job uses one GPU"
            ),
            Self::InsufficientVram {
                per_gpu_mb,
                required_mb,
            } => {
                write!(f, "{per_gpu_mb} MB per GPU < {required_mb} MB required")
            }
            Self::Unreliable {
                reliability_pct,
                floor_pct,
            } => {
                write!(
                    f,
                    "reliability {reliability_pct:.1}% < floor {floor_pct:.1}%"
                )
            }
        }
    }
}

/// Can this offer run a single-device job needing `required_vram_mb` on one GPU?
///
/// `required_vram_mb` comes from `plan_for` (memory-SSOT plan), not from a VRAM
/// ladder: the question is "does my model fit on this box", not "which canned
/// preset is nearest this number".
///
/// Thresholds come from [`CloudProviderConfig`] so this filter and the Vast client
/// read the same knobs.
pub fn offer_is_suitable(
    offer: &GpuOffer,
    config: &CloudProviderConfig,
    required_vram_mb: u64,
) -> Result<(), UnsuitableReason> {
    // Paid multi-GPU nodes only. A free local box with two GPUs is the comparison
    // baseline, not a purchase.
    if offer.gpu_count > 1 && offer.price_per_hour_usd > 0.0 {
        return Err(UnsuitableReason::MultiGpuNode {
            gpu_count: offer.gpu_count,
            price_per_hour_usd: offer.price_per_hour_usd,
        });
    }

    // `vram_mb` is the total across all GPUs (mod.rs:249). A single-device job
    // sees one device's share.
    let per_gpu_mb = offer.vram_mb / u64::from(offer.gpu_count.max(1));
    if per_gpu_mb < required_vram_mb {
        return Err(UnsuitableReason::InsufficientVram {
            per_gpu_mb,
            required_mb: required_vram_mb,
        });
    }

    // config.min_reliability is [0.0, 1.0]; reliability_pct is [0, 100].
    let floor_pct = config.min_reliability * 100.0;
    if offer.reliability_pct < floor_pct {
        return Err(UnsuitableReason::Unreliable {
            reliability_pct: offer.reliability_pct,
            floor_pct,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mens::cloud::{CloudProviderConfig, GpuOffer, ProviderKind, test_offer};

    /// An offer that should pass every arm: 1x H100 SXM 80 GB, 97% reliable.
    fn h100() -> GpuOffer {
        GpuOffer {
            provider: ProviderKind::Vast,
            gpu_name: "h100 sxm".into(),
            vram_mb: 81_920,
            price_per_hour_usd: 2.09,
            reliability_pct: 97.0,
            auto_terminate: true,
            ..test_offer()
        }
    }

    /// MUTATION CAUGHT: deleting the `gpu_count` arm.
    /// An 8x H100 node reports vram_mb = 655_360, so every VRAM-only check waves it
    /// through -- and you then pay 8x $2.09/hr to keep seven GPUs idle. `gpu_count`
    /// is written by every provider today and read by nothing.
    #[test]
    fn rejects_an_eight_gpu_node_for_a_single_gpu_job() {
        let offer = GpuOffer {
            gpu_count: 8,
            vram_mb: 655_360,
            price_per_hour_usd: 16.72,
            ..h100()
        };
        assert!(matches!(
            offer_is_suitable(&offer, &CloudProviderConfig::default(), 81_920).unwrap_err(),
            UnsuitableReason::MultiGpuNode { gpu_count: 8, .. }
        ));
    }

    /// MUTATION CAUGHT: comparing `offer.vram_mb` instead of `vram_mb / gpu_count`.
    /// `GpuOffer.vram_mb` is documented as "Total VRAM in MB across all GPUs"
    /// (mod.rs:249). A 4x A100-40G node totals 163_840 MB and would satisfy a
    /// 96 GB requirement on paper while no single device holds more than 40 GB --
    /// the job OOMs on step 1 after the instance is already billing.
    #[test]
    fn vram_is_checked_per_gpu_not_per_node() {
        let offer = GpuOffer {
            gpu_count: 4,
            vram_mb: 163_840,
            gpu_name: "a100-sxm4-40gb".into(),
            price_per_hour_usd: 0.0, // free: isolates the VRAM arm from the node arm
            ..h100()
        };
        assert!(matches!(
            offer_is_suitable(&offer, &CloudProviderConfig::default(), 98_304).unwrap_err(),
            UnsuitableReason::InsufficientVram {
                per_gpu_mb: 40_960,
                required_mb: 98_304
            }
        ));
    }

    /// MUTATION CAUGHT: comparing `reliability_pct >= config.min_reliability` directly.
    /// `config.min_reliability` is f32 on [0.0, 1.0] (mod.rs:131-132, default 0.90);
    /// `GpuOffer.reliability_pct` is f32 on [0, 100] (mod.rs:255-256). Comparing them
    /// unscaled passes a host with a 1% score. This matters for RunPod and Local
    /// offers specifically: only `vast.rs:195-198` filters reliability provider-side,
    /// so a RunPod offer reaches the resolver ungated.
    #[test]
    fn reliability_floor_is_applied_on_the_offers_own_scale() {
        let offer = GpuOffer {
            provider: ProviderKind::RunPod,
            reliability_pct: 12.0,
            ..h100()
        };
        let err = offer_is_suitable(&offer, &CloudProviderConfig::default(), 81_920).unwrap_err();
        assert!(
            matches!(err, UnsuitableReason::Unreliable { floor_pct, .. } if (floor_pct - 90.0).abs() < 1e-3),
            "floor must be 90.0 pct, not 0.90; got {err:?}"
        );
    }

    /// MUTATION CAUGHT: applying the single-GPU rule unconditionally.
    /// `LocalProvider` emits one offer with `price_per_hour_usd: 0.0` and the host's
    /// real `gpu_count` (local_provider.rs:40-46). That row exists purely so the
    /// operator can compare "rent" against "run it here" -- and under this plan's
    /// entry gate, that comparison is the whole decision. A blanket gpu_count == 1
    /// rule silently deletes the local baseline on any multi-GPU workstation.
    /// The rule is about *paying* for idle silicon, so it is priced, not counted.
    #[test]
    fn a_free_local_multi_gpu_box_is_not_rejected_as_a_node() {
        let offer = GpuOffer {
            provider: ProviderKind::Local,
            gpu_count: 2,
            vram_mb: 163_840,
            price_per_hour_usd: 0.0,
            reliability_pct: 100.0,
            ..h100()
        };
        assert!(offer_is_suitable(&offer, &CloudProviderConfig::default(), 81_920).is_ok());
    }

    /// MUTATION CAUGHT: a filter that fails closed on everything (e.g. an inverted
    /// comparison, or a `required_mb` unit mix-up between MB and GiB). Without this
    /// the three reject tests above all still pass while the resolver returns an
    /// empty ranking and the operator sees "No GPU offers found".
    #[test]
    fn accepts_the_recommended_configuration() {
        assert!(offer_is_suitable(&h100(), &CloudProviderConfig::default(), 81_920).is_ok());
    }
}
