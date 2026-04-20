#[cfg(feature = "live-api-canary")]
use crate::PublisherConfig;
#[cfg(feature = "live-api-canary")]
use crate::adapter_health::HeartbeatStatus;
#[cfg(feature = "live-api-canary")]
use reqwest::Client;
#[cfg(feature = "live-api-canary")]
use std::time::Instant;

#[cfg(feature = "live-api-canary")]
pub async fn probe_mastodon(cfg: &PublisherConfig) -> HeartbeatStatus {
    let domain = cfg.mastodon_domain.as_deref().unwrap_or("mastodon.social");
    let instance_url = if domain.starts_with("http") {
        domain.to_string()
    } else {
        format!("https://{}", domain)
    };

    let client = Client::new();
    let start = Instant::now();
    let url = format!("{}/api/v2/instance", instance_url.trim_end_matches('/'));
    let res = client.get(&url).send().await;

    map_res(res, start)
}

#[cfg(feature = "live-api-canary")]
pub async fn probe_bluesky(_cfg: &PublisherConfig) -> HeartbeatStatus {
    // We can't easily probe without a PDS URL, default to bsky.social
    let pds_base = "https://bsky.social";

    let client = Client::new();
    let start = Instant::now();
    let url = format!(
        "{}/xrpc/com.atproto.server.describeServer",
        pds_base.trim_end_matches('/')
    );
    let res = client.get(&url).send().await;

    map_res(res, start)
}

#[cfg(feature = "live-api-canary")]
pub async fn probe_discord(_cfg: &PublisherConfig) -> HeartbeatStatus {
    let webhook_url =
        match vox_clavis::resolve_secret(vox_clavis::SecretId::VoxSocialDiscordWebhook).expose() {
            Some(s) => s.to_string(),
            None => {
                return HeartbeatStatus::AuthFailure {
                    hint: "Missing Discord Webhook Secret".to_string(),
                };
            }
        };

    let client = Client::new();
    let start = Instant::now();
    let res = client.get(&webhook_url).send().await;

    map_res(res, start)
}

#[cfg(feature = "live-api-canary")]
pub async fn probe_twitter(cfg: &PublisherConfig) -> HeartbeatStatus {
    let token = match cfg.twitter_bearer_token.clone().or_else(|| {
        vox_clavis::resolve_secret(vox_clavis::SecretId::VoxNewsTwitterBearer)
            .expose()
            .map(String::from)
    }) {
        Some(t) => t,
        None => {
            return HeartbeatStatus::AuthFailure {
                hint: "Missing Twitter Bearer Token".to_string(),
            };
        }
    };

    let client = Client::new();
    let start = Instant::now();
    let base = cfg
        .twitter_api_base
        .as_deref()
        .unwrap_or("https://api.twitter.com");
    let url = format!("{}/2/users/me", base.trim_end_matches('/'));

    let res = client.get(&url).bearer_auth(token).send().await;
    map_res(res, start)
}

#[cfg(feature = "live-api-canary")]
pub async fn probe_linkedin(cfg: &PublisherConfig) -> HeartbeatStatus {
    let token = match cfg.linkedin_access_token.clone().or_else(|| {
        vox_clavis::resolve_secret(vox_clavis::SecretId::VoxSocialLinkedinAccessToken)
            .expose()
            .map(String::from)
    }) {
        Some(t) => t,
        None => {
            return HeartbeatStatus::AuthFailure {
                hint: "Missing LinkedIn Access Token".to_string(),
            };
        }
    };

    let client = Client::new();
    let start = Instant::now();
    let base = cfg
        .linkedin_api_base
        .as_deref()
        .unwrap_or("https://api.linkedin.com");
    // Safer universal heartbeat using OpenID userinfo
    let url = format!("{}/v2/userinfo", base.trim_end_matches('/'));

    let res = client.get(&url).bearer_auth(token).send().await;
    map_res(res, start)
}

#[cfg(feature = "live-api-canary")]
pub async fn probe_zenodo(_cfg: &PublisherConfig) -> HeartbeatStatus {
    let secret = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxZenodoAccessToken);
    let token = match secret.expose() {
        Some(t) => t,
        None => {
            return HeartbeatStatus::AuthFailure {
                hint: "Missing Zenodo Access Token".to_string(),
            };
        }
    };

    let client = Client::new();
    let start = Instant::now();
    // Zenodo probe
    let url = "https://zenodo.org/api/deposit/depositions";

    let res = client
        .get(url)
        .query(&[("access_token", token)])
        .send()
        .await;
    map_res(res, start)
}

#[cfg(feature = "live-api-canary")]
pub async fn probe_opencollective(cfg: &PublisherConfig) -> HeartbeatStatus {
    let token = match cfg.open_collective_token.clone().or_else(|| {
        vox_clavis::resolve_secret(vox_clavis::SecretId::VoxNewsOpenCollectiveToken)
            .expose()
            .map(String::from)
    }) {
        Some(t) => t,
        None => {
            return HeartbeatStatus::AuthFailure {
                hint: "Missing OpenCollective Token".to_string(),
            };
        }
    };

    let client = Client::new();
    let start = Instant::now();
    let url = cfg
        .opencollective_graphql_url
        .as_deref()
        .unwrap_or("https://api.opencollective.com/graphql/v2");

    // Simple query to check token validity
    let query = serde_json::json!({
        "query": "{ me { id name slug } }"
    });

    let res = client
        .post(url)
        .header("Personal-Token", token)
        .json(&query)
        .send()
        .await;

    map_res(res, start)
}

#[cfg(feature = "live-api-canary")]
fn map_res(res: Result<reqwest::Response, reqwest::Error>, start: Instant) -> HeartbeatStatus {
    match res {
        Ok(resp) if resp.status().is_success() => HeartbeatStatus::Ok {
            latency_ms: start.elapsed().as_millis() as u64,
        },
        Ok(resp) => HeartbeatStatus::ServiceDown {
            http_status: Some(resp.status().as_u16()),
        },
        Err(e) => HeartbeatStatus::NetworkError {
            message: e.to_string(),
        },
    }
}
