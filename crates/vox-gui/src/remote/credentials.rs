//! Persistence for the one remembered Remote Axis server: bearer token in the OS keyring (Clavis
//! registry token keyed by origin), URL + pinned certificate fingerprint in `~/.vox/config.toml`.

use reqwest::Url;

use super::error::AxisError;

pub const URL_KEY: &str = "axis_remote_url";
pub const PIN_KEY: &str = "axis_remote_tls_sha3_256";

pub fn token_registry_name(url: &Url) -> String {
    format!("axis-remote:{}", url.origin().ascii_serialization())
}

pub fn pin_for(saved_url: Option<&str>, saved_pin: Option<&str>, url: &Url) -> Option<String> {
    let saved = Url::parse(saved_url?).ok()?;
    (saved.origin() == url.origin())
        .then(|| saved_pin.map(str::to_string))
        .flatten()
}

// toestub-ignore(skeleton/untested-pub-api) — thin wrapper over vox_secrets registry tokens (keyring round-trip covered by vox-secrets tests)
pub fn load_token(url: &Url) -> Option<String> {
    vox_secrets::get_registry_token(&token_registry_name(url))
}

// toestub-ignore(skeleton/untested-pub-api) — thin wrapper over vox_secrets registry tokens (keyring round-trip covered by vox-secrets tests)
pub fn store_token(url: &Url, token: &str) -> Result<(), AxisError> {
    vox_secrets::set_registry_token(&token_registry_name(url), token, None)
        .map(|_| ())
        .map_err(|e| AxisError::Remote {
            code: "secret_store".into(),
            message: e.to_string(),
        })
}

// toestub-ignore(skeleton/untested-pub-api) — reads vox_config user config; decision logic is `pin_for` (tested)
pub fn saved_pin(url: &Url) -> Option<String> {
    let cfg = vox_config::toml_config::load_user_config();
    let get = |k: &str| {
        cfg.values
            .get(k)
            .and_then(|v| v.as_str())
            .map(str::to_string)
    };
    pin_for(get(URL_KEY).as_deref(), get(PIN_KEY).as_deref(), url)
}

// toestub-ignore(skeleton/untested-pub-api) — writes vox_config user config (covered by vox-config tests)
pub fn save_server(url: &Url, pin: Option<&str>) -> Result<(), AxisError> {
    let io = |e: String| AxisError::Remote {
        code: "config_write".into(),
        message: e,
    };
    vox_config::toml_config::set_user_config_value(URL_KEY, url.as_str()).map_err(io)?;
    match pin {
        Some(p) => vox_config::toml_config::set_user_config_value(PIN_KEY, p).map_err(io)?,
        None => {
            vox_config::toml_config::unset_user_config_value(PIN_KEY).map_err(io)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    #[test]
    fn token_is_keyed_by_origin() {
        assert_eq!(
            token_registry_name(&url("https://vox.lan:3921/")),
            "axis-remote:https://vox.lan:3921"
        );
        assert_eq!(
            token_registry_name(&url("https://vox.lan/")),
            "axis-remote:https://vox.lan"
        );
    }

    #[test]
    fn pin_applies_only_to_the_saved_server() {
        let u = url("https://vox.lan:3921/");
        assert_eq!(
            pin_for(Some("https://vox.lan:3921/"), Some("ab"), &u),
            Some("ab".into())
        );
        assert_eq!(
            pin_for(Some("https://other.lan:3921/"), Some("ab"), &u),
            None
        );
        assert_eq!(pin_for(None, Some("ab"), &u), None);
        assert_eq!(pin_for(Some("https://vox.lan:3921/"), None, &u), None);
        assert_eq!(pin_for(Some("not a url"), Some("ab"), &u), None);
    }
}
