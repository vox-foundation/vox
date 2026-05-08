# Privacy

- **Local-first:** your journal stays on device in the embedded database created by the Vox stack for this app.
- **No accounts** and **no cloud sync** in v1 — nothing is uploaded to our servers (there are no servers in this product path).
- **Voice:** on-device transcription requires the **`VoxSherpaTranscribe`** Capacitor plugin; audio does not leave the device once implemented.
- **Exports:** when you export CSV/JSON/PDF, you explicitly share via the OS share sheet — that action is under your control.

Rebuild exports before clinical visits — metadata includes generation time and checksum (planned in export pipeline).
