use crate::spec::ids::SecretId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeTierOffer {
    pub provider_id: String,
    pub name: String,
    pub signup_url: String,
    pub free_tier_description: String,
    pub quota_summary: String,
    pub requires_credit_card: bool,
    pub secret_id: SecretId,
}

#[must_use]
pub fn list_free_tier_offers() -> Vec<FreeTierOffer> {
    vec![
        FreeTierOffer {
            provider_id: "tavily".to_string(),
            name: "Tavily Search".to_string(),
            signup_url: "https://app.tavily.com/sign-up".to_string(),
            free_tier_description: "1,000 queries/month free web search for AI agents and LLMs."
                .to_string(),
            quota_summary: "1,000 searches/mo".to_string(),
            requires_credit_card: false,
            secret_id: SecretId::TavilyApiKey,
        },
        FreeTierOffer {
            provider_id: "gemini".to_string(),
            name: "Google Gemini".to_string(),
            signup_url: "https://aistudio.google.com/app/apikey".to_string(),
            free_tier_description: "Free access to Gemini models with rate-limited quota."
                .to_string(),
            quota_summary: "Free rate-limited tier".to_string(),
            requires_credit_card: false,
            secret_id: SecretId::GeminiApiKey,
        },
        FreeTierOffer {
            provider_id: "openrouter".to_string(),
            name: "OpenRouter".to_string(),
            signup_url: "https://openrouter.ai/keys".to_string(),
            free_tier_description: "Unified API access to free tier open models and routers."
                .to_string(),
            quota_summary: "Free model access".to_string(),
            requires_credit_card: false,
            secret_id: SecretId::OpenRouterApiKey,
        },
        FreeTierOffer {
            provider_id: "semantic_scholar".to_string(),
            name: "Semantic Scholar".to_string(),
            signup_url: "https://www.semanticscholar.org/product/api".to_string(),
            free_tier_description: "Free academic graph and paper search API.".to_string(),
            quota_summary: "Free academic tier".to_string(),
            requires_credit_card: false,
            secret_id: SecretId::VoxSemanticScholarApiKey,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_free_tier_offers_not_empty() {
        let offers = list_free_tier_offers();
        assert!(!offers.is_empty());
        assert!(offers.iter().any(|o| o.provider_id == "tavily"));
        assert!(offers.iter().any(|o| o.provider_id == "gemini"));
        assert!(offers.iter().any(|o| o.provider_id == "openrouter"));
        assert!(offers.iter().any(|o| o.provider_id == "semantic_scholar"));
    }
}
