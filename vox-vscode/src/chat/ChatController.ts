// Thin chat controller — all history, context injection and LLM generation live in Rust (vox-mcp).
// TypeScript only: renders messages, captures user input, forwards to MCP.

import type { ChatMessage } from '../types';
import { VoxMcpClient } from '../core/VoxMcpClient';

export type { ChatMessage };

export class ChatController {
    private _onUpdate: (messages: ChatMessage[]) => void;

    constructor(
        private readonly _mcp: VoxMcpClient,
        onUpdate: (messages: ChatMessage[]) => void,
    ) {
        this._onUpdate = onUpdate;
    }

    /** Fetch the authoritative history from the Rust session store. */
    async loadHistory(): Promise<void> {
        const result = await this._mcp.chatHistory();
        this._onUpdate(Array.isArray(result) ? result : []);
    }

    /** Submit a message. Rust handles @mention expansion, context injection, model selection, and streaming. */
    async submitMessage(prompt: string, contextFiles: string[] = []): Promise<void> {
        // Push optimistic user bubble immediately so the UI feels instant
        const optimistic: ChatMessage = {
            id: `opt-${Date.now()}`,
            role: 'user',
            content: prompt,
            timestamp: Date.now(),
            context_files: contextFiles,
        };
        // Also show streaming assistant placeholder
        const streamId = `stream-${Date.now()}`;
        const streamingMsg: ChatMessage = {
            id: streamId,
            role: 'assistant',
            content: '',
            is_streaming: true,
            timestamp: Date.now() + 1,
        };

        // Immediately notify webview with these two pending messages
        const currentHistory = await this._mcp.chatHistory();
        const base = Array.isArray(currentHistory) ? currentHistory : [];
        this._onUpdate([...base, optimistic, streamingMsg]);

        // Call native MCP tool — Rust resolves @mentions, injects context, queries LLM
        const result = await this._mcp.chatMessage(prompt, contextFiles);

        if (result?.history) {
            this._onUpdate(result.history);
        } else {
            // Fallback: reload history
            await this.loadHistory();
        }
    }

    async clearHistory(): Promise<void> {
        await this._mcp.sessionReset();
        this._onUpdate([]);
    }
}
