/**
 * Guest (WebView) facade for `vox-tauri-sherpa`.
 * Contract: `transcribe(): Promise<{ text: string; confidence?: number }>`
 */
import { invoke } from "@tauri-apps/api/core";

export interface TranscribeResult {
  text: string;
  confidence?: number;
}

const DEBUG =
  typeof localStorage !== "undefined" &&
  localStorage.getItem("VOX_DEBUG_SHERPA") === "1";

export async function transcribe(): Promise<TranscribeResult> {
  if (DEBUG) {
    console.debug("[vox-tauri-sherpa] invoke transcribe with empty payload");
  }
  return invoke<TranscribeResult>("plugin:vox-sherpa|transcribe", {});
}
