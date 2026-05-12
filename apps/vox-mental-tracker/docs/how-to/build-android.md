# Build Android (Capacitor)

1. **Web bundle**

   ```bash
   vox build src/main.vox -o dist
   ```

2. **Capacitor**

   ```bash
   pnpm install
   npx cap add android   # first time only
   vox build src/main.vox -o dist --target android   # runs cap sync when wired to CLI
   ```

3. **Signing / Play**: generate keystore locally; never commit secrets.

Mic permission strings live in `AndroidManifest.xml` after `cap add android` — merge edits carefully on upgrades.
