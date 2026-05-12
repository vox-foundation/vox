# Build Android (Tauri 2)

1. **Web bundle**

   ```bash
   cd apps/vox-mental-tracker
   pnpm install
   pnpm build:web
   ```

2. **Tauri mobile workspace** (from the app directory, with `vox` on `PATH`)

   ```bash
   vox compile --target mobile-android -o dist src/main.vox
   ```

   Generated Rust + `src-tauri/` live under the **repository** `target/generated/` (shared workspace target dir). Continue with `cargo tauri android init` / `cargo tauri android build` per [application packaging SSOT](../../../../docs/src/architecture/vox-application-packaging-ssot-2026.md) once the mobile project is initialized.

3. **Signing / Play**: generate keystore locally; never commit secrets.

Mic permission strings live in the Android manifest for the Tauri mobile project — merge edits carefully on upgrades.
