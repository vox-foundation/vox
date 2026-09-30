import { useLocalStorage } from './useLocalStorage';

export type ChatVerbosity = 'quiet' | 'normal' | 'verbose';

export const CHAT_VERBOSITY_KEY = 'gui.chat.verbosity.v1';

/**
 * Global chat-feed verbosity, set by `ChatVerbosityControl`. It decides how much of each turn's
 * trace opens by default (`buildTurnTrace`): quiet keeps traces collapsed, normal opens a trace
 * when a receipt failed or something needs you, verbose opens every trace. Interrupts show at
 * every level. Quiet also hides the per-task "Done · $x" row (`buildChatOnlyTimeline`).
 */
export function useChatVerbosity() {
  return useLocalStorage<ChatVerbosity>(CHAT_VERBOSITY_KEY, 'normal');
}
