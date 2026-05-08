# `vox-sherpa-transcribe` (Capacitor)

Capacitor-native bridge implementing `transcribe()` → `{ text, confidence? }`, consumed from Vox-generated `mobile.transcribe_microphone()` (`Speech.transcribe_microphone()`).

## Android

- **Implementation:** `SpeechRecognizer` with `EXTRA_PREFER_OFFLINE` for on-device packs when installed.
- **Permission:** `RECORD_AUDIO` is declared in this package’s `AndroidManifest.xml` (merged into the app).

## iOS

- **Stub** returns a clear `reject` until Sherpa-ONNX / ONNXRuntime or Apple Speech is wired (keep plugin name + method stable).

## Sherpa-ONNX follow-up

Bundle streaming ASR models under `android/src/main/assets/` and JNI from `k2-fsa/sherpa-onnx` per upstream docs; swap the Kotlin body without changing the JS/Capacitor contract.
