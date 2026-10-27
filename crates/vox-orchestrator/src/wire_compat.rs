//! Forward-compatible discriminated union envelope (`ForwardCompatibleEnvelope<T>`).
//!
//! Adapted from `packages/shared/src/ForwardCompatibleUnion.ts`:
//! When a newer daemon or peer emits an unrecognized union variant over RPC/WS/SSE,
//! `ForwardCompatibleEnvelope<T>` deserializes it into `ForwardCompatibleEnvelope::Unknown { raw }`
//! instead of failing the entire snapshot or stream payload, and serializes it back losslessly.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

/// Forward-compatible wrapper around a known discriminated union `T`.
#[derive(Debug, Clone, PartialEq)]
pub enum ForwardCompatibleEnvelope<T> {
    /// Recognized union member decoded into `T`.
    Known(T),
    /// Unrecognized future union member preserved with its original tag (if any) and raw JSON value.
    Unknown {
        /// Discriminator tag (`"tag"`, `"type"`, or `"kind"`) extracted from `raw`, or `"unknown"`.
        tag: String,
        /// Full raw JSON payload for lossless round-tripping.
        raw: Value,
    },
}

impl<T> ForwardCompatibleEnvelope<T> {
    /// Returns `Some(&T)` if the variant is known, or `None` if it is an unknown future variant.
    pub fn as_known(&self) -> Option<&T> {
        match self {
            Self::Known(inner) => Some(inner),
            Self::Unknown { .. } => None,
        }
    }

    /// Returns `true` when the payload fell back to `Unknown`.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown { .. })
    }
}

impl<T: Serialize> Serialize for ForwardCompatibleEnvelope<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Known(inner) => inner.serialize(serializer),
            Self::Unknown { raw, .. } => raw.serialize(serializer),
        }
    }
}

impl<'de, T> Deserialize<'de> for ForwardCompatibleEnvelope<T>
where
    T: for<'a> Deserialize<'a>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = Value::deserialize(deserializer)?;
        match T::deserialize(raw.clone()) {
            Ok(known) => Ok(Self::Known(known)),
            Err(_) => {
                let tag = raw
                    .as_object()
                    .and_then(|obj| {
                        obj.get("tag")
                            .or_else(|| obj.get("type"))
                            .or_else(|| obj.get("kind"))
                    })
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                Ok(Self::Unknown { tag, raw })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ForwardCompatibleEnvelope;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "tag", rename_all = "snake_case")]
    enum SampleWireEvent {
        TurnStarted { turn_id: String },
        TurnCompleted { turn_id: String },
    }

    #[test]
    fn known_variant_deserializes_and_roundtrips() {
        let payload = json!({
            "tag": "turn_started",
            "turn_id": "turn-1"
        });
        let parsed: ForwardCompatibleEnvelope<SampleWireEvent> =
            serde_json::from_value(payload.clone()).expect("deserialize known");
        assert_eq!(
            parsed,
            ForwardCompatibleEnvelope::Known(SampleWireEvent::TurnStarted {
                turn_id: "turn-1".to_string()
            })
        );
        let reserialized = serde_json::to_value(&parsed).expect("serialize known");
        assert_eq!(reserialized, payload);
    }

    #[test]
    fn unknown_future_variant_preserves_tag_and_raw_value() {
        let future_payload = json!({
            "tag": "future.event.v3",
            "turn_id": "turn-99",
            "new_field": { "nested": true }
        });
        let parsed: ForwardCompatibleEnvelope<SampleWireEvent> =
            serde_json::from_value(future_payload.clone()).expect("deserialize unknown");
        assert!(parsed.is_unknown());
        assert_eq!(parsed.as_known(), None);
        match &parsed {
            ForwardCompatibleEnvelope::Unknown { tag, raw } => {
                assert_eq!(tag, "future.event.v3");
                assert_eq!(raw, &future_payload);
            }
            ForwardCompatibleEnvelope::Known(_) => panic!("expected Unknown"),
        }
        let reserialized = serde_json::to_value(&parsed).expect("serialize unknown");
        assert_eq!(reserialized, future_payload);
    }
}
