#![cfg(test)]

use std::sync::Mutex;

use crate::backend::{NoopBackend, UnavailableBackend};
use crate::resolver::{ResolveOptions, SecretResolver};
use crate::spec::{
    Profile, RequirementMode, RequirementSet, SecretBundle, SecretId, Workflow,
    required_for_profile, requirements_for_bundle, requirements_for_profile_mode,
};
use crate::{ResolutionStatus, resolve_env_only};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
#[allow(unsafe_code)]
fn canonical_env_wins_over_alias() {
    let _g = ENV_LOCK.lock().expect("env lock");
    unsafe {
        unsafe { std::env::set_var("GEMINI_API_KEY", "canonical") };
        unsafe { std::env::set_var("GOOGLE_AI_STUDIO_KEY", "alias") };
    }
    let resolved = resolve_env_only(SecretId::GeminiApiKey);
    assert_eq!(resolved.expose(), Some("canonical"));
    assert!(matches!(resolved.status, ResolutionStatus::Present));
    unsafe {
        std::env::remove_var("GEMINI_API_KEY");
        std::env::remove_var("GOOGLE_AI_STUDIO_KEY");
    }
}

#[test]
fn backend_unavailable_status_is_explicit() {
    let resolver = SecretResolver::new(UnavailableBackend {
        reason: "feature disabled".to_string(),
    });
    let resolved = resolver.resolve(
        SecretId::OpenRouterApiKey,
        &ResolveOptions {
            include_auth_json: false,
            include_populi_env: false,
        },
    );
    assert!(matches!(
        resolved.status,
        ResolutionStatus::BackendUnavailable
    ));
    assert!(
        resolved
            .detail
            .unwrap_or_default()
            .contains("feature disabled")
    );
}

#[test]
#[allow(unsafe_code)]
fn env_only_ignores_backend() {
    let _g = ENV_LOCK.lock().expect("env lock");
    unsafe {
        std::env::remove_var("OPENROUTER_API_KEY");
    }
    let resolved = SecretResolver::new(NoopBackend).resolve(
        SecretId::OpenRouterApiKey,
        &ResolveOptions {
            include_auth_json: false,
            include_populi_env: false,
        },
    );
    assert!(matches!(resolved.status, ResolutionStatus::MissingRequired));
}

#[test]
fn profile_requirements_are_dynamic() {
    let dev = required_for_profile(Workflow::Chat, Profile::Dev);
    let ci = required_for_profile(Workflow::Chat, Profile::Ci);
    assert!(!dev.contains(&SecretId::OpenRouterApiKey));
    assert!(ci.contains(&SecretId::ForgeToken));
}

#[test]
fn workflow_requirements_have_any_of_for_chat() {
    let req = requirements_for_profile_mode(Workflow::Chat, Profile::Dev, RequirementMode::Cloud);
    assert!(
        req.blocking
            .iter()
            .any(|group| matches!(group, RequirementSet::AllOf(_)))
    );
}

#[test]
fn bundle_requirements_are_defined() {
    let local = requirements_for_bundle(SecretBundle::MinimalLocalDev);
    let cloud = requirements_for_bundle(SecretBundle::MinimalCloudDev);
    assert!(local.blocking.is_empty());
    assert!(!cloud.blocking.is_empty());
}

#[test]
#[allow(unsafe_code)]
fn deprecated_alias_marks_status() {
    let _g = ENV_LOCK.lock().expect("env lock");
    unsafe {
        unsafe { std::env::set_var("GOOGLE_AI_STUDIO_KEY", "legacy") };
        std::env::remove_var("GEMINI_API_KEY");
    }
    let resolved = resolve_env_only(SecretId::GeminiApiKey);
    assert!(matches!(
        resolved.status,
        ResolutionStatus::DeprecatedAliasUsed
    ));
    unsafe {
        std::env::remove_var("GOOGLE_AI_STUDIO_KEY");
    }
}
