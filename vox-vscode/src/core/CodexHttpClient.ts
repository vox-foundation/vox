/**
 * Typed client for `vox-codex-api` (`run_dashboard`). Base URL from `vox.codex.apiBaseUrl`.
 */

import { ConfigManager } from './ConfigManager';

export interface ResearchSessionUpsertBody {
    session_key: string;
    title?: string;
    status?: string;
    repository_id?: string;
    config_json?: unknown;
    summary_json?: unknown;
}

export interface ConversationVersionBody {
    version_index: number;
    label?: string;
    snapshot_json?: unknown;
}

export interface ConversationEdgeBody {
    from_conversation_id: number;
    to_conversation_id: number;
    edge_kind?: string;
    weight?: number;
    metadata_json?: unknown;
}

export interface TopicEvolutionBody {
    event_kind: string;
    prior_label?: string;
    new_label?: string;
    detail_json?: unknown;
}

const DEFAULT_TIMEOUT_MS = 30_000;

function joinUrl(base: string, path: string): string {
    const b = base.replace(/\/$/, '');
    const p = path.startsWith('/') ? path : `/${path}`;
    return `${b}${p}`;
}

async function fetchWithTimeout(
    input: string,
    init: RequestInit = {},
    timeoutMs: number = DEFAULT_TIMEOUT_MS,
): Promise<Response> {
    const ctl = new AbortController();
    const t = setTimeout(() => ctl.abort(), timeoutMs);
    try {
        return await fetch(input, { ...init, signal: ctl.signal });
    } finally {
        clearTimeout(t);
    }
}

async function parseJson<T>(r: Response): Promise<T> {
    const text = await r.text();
    if (!r.ok) {
        throw new Error(`Codex HTTP ${r.status}: ${text.slice(0, 500)}`);
    }
    return JSON.parse(text) as T;
}

export class CodexHttpClient {
    constructor(private readonly baseUrl: string) {}

    /** Returns client when sync is enabled and base URL is non-empty. */
    static tryFromConfig(): CodexHttpClient | undefined {
        if (!ConfigManager.codexEnableHttpSync) {
            return undefined;
        }
        const u = ConfigManager.codexApiBaseUrl.trim();
        return u ? new CodexHttpClient(u) : undefined;
    }

    async getReady(): Promise<Record<string, unknown>> {
        const r = await fetchWithTimeout(joinUrl(this.baseUrl, '/ready'));
        return parseJson(r);
    }

    async upsertResearchSession(
        body: ResearchSessionUpsertBody,
    ): Promise<{ id: number; session_key: string; repository_id: string }> {
        const r = await fetchWithTimeout(joinUrl(this.baseUrl, '/api/codex/research-session'), {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body),
        });
        return parseJson(r);
    }

    async appendConversationVersion(
        conversationId: number,
        body: ConversationVersionBody,
    ): Promise<{ id: number; conversation_id: number; version_index: number }> {
        const r = await fetchWithTimeout(
            joinUrl(this.baseUrl, `/api/codex/conversations/${conversationId}/versions`),
            {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(body),
            },
        );
        return parseJson(r);
    }

    async insertConversationEdge(body: ConversationEdgeBody): Promise<{ id: number }> {
        const r = await fetchWithTimeout(joinUrl(this.baseUrl, '/api/codex/conversation-edges'), {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body),
        });
        return parseJson(r);
    }

    async appendTopicEvolution(
        topicId: number,
        body: TopicEvolutionBody,
    ): Promise<{ id: number; topic_id: number }> {
        const r = await fetchWithTimeout(
            joinUrl(this.baseUrl, `/api/codex/topics/${topicId}/evolution-events`),
            {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(body),
            },
        );
        return parseJson(r);
    }
}
