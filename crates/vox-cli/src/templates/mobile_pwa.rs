//! Mobile PWA template — generates a Vite + Capacitor scaffold
//! that works on desktop browsers and iOS/Android WebViews.

use anyhow::{Context, Result};
use std::path::Path;

pub fn scaffold(name: &str, out_dir: &Path) -> Result<()> {
    // Write out the additional files needed for mobile PWA/Capacitor.
    // Ensure standard Vox manifest setup is already executed or handles the base.

    let capacitor_config = format!(
        r#"import {{ CapacitorConfig }} from '@capacitor/cli';

const config: CapacitorConfig = {{
  appId: 'com.vox.{n}',
  appName: '{name}',
  webDir: 'dist',
  server: {{
    androidScheme: 'https'
  }}
}};

export default config;
"#,
        n = name.to_lowercase().replace('-', ""),
        name = name
    );
    std::fs::write(out_dir.join("capacitor.config.ts"), capacitor_config)
        .context("Write capacitor config")?;

    let package_json = format!(
        r#"{{
  "name": "{}",
  "version": "0.1.0",
  "private": true,
  "dependencies": {{
    "@capacitor/core": "^6.0.0",
    "@capacitor/camera": "^6.0.0",
    "@capacitor/haptics": "^6.0.0",
    "@capacitor/geolocation": "^6.0.0",
    "@capacitor/clipboard": "^6.0.0",
    "@capacitor/push-notifications": "^6.0.0",
    "workbox-window": "^7.0.0"
  }},
  "devDependencies": {{
    "@capacitor/cli": "^6.0.0",
    "@capacitor/ios": "^6.0.0",
    "@capacitor/android": "^6.0.0"
  }}
}}
"#,
        name
    );
    std::fs::write(out_dir.join("package.json"), package_json).context("Write package.json")?;

    let public_dir = out_dir.join("public");
    std::fs::create_dir_all(&public_dir).context("Create public dir")?;

    let manifest = r##"{
  "name": "Vox Mobile App",
  "short_name": "VoxApp",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#ffffff",
  "theme_color": "#000000",
  "icons": [
    {
      "src": "/icon-192.png",
      "type": "image/png",
      "sizes": "192x192"
    },
    {
      "src": "/icon-512.png",
      "type": "image/png",
      "sizes": "512x512"
    }
  ]
}"##;
    std::fs::write(public_dir.join("manifest.webmanifest"), manifest)
        .context("Write webmanifest")?;

    let sw = r#"importScripts('https://storage.googleapis.com/workbox-cdn/releases/7.0.0/workbox-sw.js');

const { routing, strategies, backgroundSync } = workbox;

const bgSyncPlugin = new backgroundSync.BackgroundSyncPlugin('vox-offline-queue', {
  maxRetentionTime: 24 * 60 // Retry for max 24 Hours
});

routing.registerRoute(
  ({request}) => request.method === 'POST',
  new strategies.NetworkOnly({
    plugins: [bgSyncPlugin]
  })
);

routing.registerRoute(
  ({request}) => request.destination === 'document' || request.destination === 'script' || request.destination === 'style',
  new strategies.StaleWhileRevalidate()
);
"#;
    std::fs::write(public_dir.join("sw.js"), sw).context("Write sw.js")?;

    Ok(())
}
