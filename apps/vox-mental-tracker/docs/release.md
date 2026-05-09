# Release: signing & store submission

## One-time: generate Android upload keystore

```bash
keytool -genkey -v -keystore ./vox-mental-tracker-upload.jks \
    -keyalg RSA -keysize 2048 -validity 10000 -alias vox-mental
```

Store the keystore file outside the repo. Set env vars before each release build:

- `VOX_ANDROID_KEYSTORE` — path to the .jks
- `VOX_ANDROID_KEYSTORE_PASSWORD`
- `VOX_ANDROID_KEY_ALIAS=vox-mental`
- `VOX_ANDROID_KEY_PASSWORD`

## Release build

```bash
vox run scripts/sign-android.vox
```

Outputs `apps/vox-mental-tracker/android/app/build/outputs/apk/release/app-release.apk`.

## iOS

Open `ios/App/App.xcworkspace` in Xcode, select team in Signing & Capabilities, click Archive. Upload via Organizer to App Store Connect.

(iOS automation is intentionally manual until we have a paid Apple Developer account in CI.)
