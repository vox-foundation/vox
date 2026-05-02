import { useState, useCallback } from 'react';
import { voxTransport } from '../transport';

export interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
}

interface ToolCallResult {
  success: boolean;
  is_error: boolean;
  result?: {
    success: boolean;
    data?: {
      message?: { content?: string } | string;
      session_id?: string;
      error?: string;
    };
    error?: string;
  };
}

function extractReplyText(data: ToolCallResult['result']): string {
  if (!data?.success) {
    return data?.error ?? 'Request failed.';
  }
  const msg = data.data?.message;
  if (typeof msg === 'string') return msg;
  if (msg?.content) return msg.content;
  return JSON.stringify(data.data ?? {});
}

export function useVoxChat() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [sessionId, setSessionId] = useState<string | undefined>(undefined);
  const [error, setError] = useState<string | null>(null);

  const send = useCallback(async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed || isLoading) return;

    setMessages(prev => [...prev, { role: 'user', content: trimmed }]);
    setIsLoading(true);
    setError(null);

    try {
      const raw = await voxTransport.callTool('vox_chat_message', {
        prompt: trimmed,
        ...(sessionId ? { session_id: sessionId } : {}),
      }) as ToolCallResult;

      if (raw?.is_error) {
        const errMsg = raw.result?.error ?? 'Tool call returned an error.';
        setError(errMsg);
        setMessages(prev => [...prev, { role: 'assistant', content: `Error: ${errMsg}` }]);
        return;
      }

      const result = raw?.result;
      const replyText = extractReplyText(result);
      const nextSession = result?.data?.session_id;

      if (nextSession) setSessionId(nextSession);
      setMessages(prev => [...prev, { role: 'assistant', content: replyText }]);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      setMessages(prev => [...prev, { role: 'assistant', content: `Error: ${msg}` }]);
    } finally {
      setIsLoading(false);
    }
  }, [isLoading, sessionId]);

  const reset = useCallback(() => {
    setMessages([]);
    setSessionId(undefined);
    setError(null);
  }, []);

  return { messages, isLoading, sessionId, error, send, reset };
}
