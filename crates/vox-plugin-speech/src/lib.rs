//! Vox plugin: oratio
//!
//! Provides the AudioCapture and SpeechToText extension points for the Oratio
//! speech-to-code pipeline. The SpeechToText impl uses the Candle Whisper backend
//! extracted from vox-speech (Unit 4 of the vox-populi extraction follow-up plan).

mod audio;
mod backends;
mod oratio_internals;

// Dylib export glue, stamped with the current ABI version. `init` delegates to the
// host-aware constructor `audio::make_plugin`. Byte-identical to the previous hand-written
// block; the `oratio` id is retained (the crate is vox-plugin-speech).
vox_plugin_sdk::declare_plugin! {
    init: |host| audio::make_plugin(host),
}

#[cfg(test)]
mod tests {
    /// Plugin.toml is hand-maintained, and vox-plugin-host refuses to load a code plugin
    /// whose declared version differs from the host's own `CARGO_PKG_VERSION`. A stale
    /// Plugin.toml version therefore makes an installed `oratio` plugin unloadable.
    #[test]
    fn plugin_toml_version_matches_crate_version() {
        let manifest: toml::Value = include_str!("../Plugin.toml")
            .parse()
            .expect("Plugin.toml must be valid TOML");
        assert_eq!(
            manifest["plugin"]["version"].as_str(),
            Some(env!("CARGO_PKG_VERSION")),
            "Plugin.toml's [plugin] version must match this crate's version"
        );
    }
}
