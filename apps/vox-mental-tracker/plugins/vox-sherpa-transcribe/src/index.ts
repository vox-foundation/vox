import { registerPlugin } from "@capacitor/core";

export interface VoxSherpaTranscribePlugin {
  transcribe(): Promise<{ text: string; confidence?: number }>;
}

/** Native bridge — Android implements on-device STT; iOS stub until assets are linked. */
export const VoxSherpaTranscribe = registerPlugin<VoxSherpaTranscribePlugin>("VoxSherpaTranscribe");
