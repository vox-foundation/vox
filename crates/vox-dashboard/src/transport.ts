import type {
  AuthStatusEvent,
  ConnectionStatusPayload,
  VoxTransportEventMap,
} from './types';

export class VoxTransport {
  private ws: WebSocket | null = null;
  private listeners: Record<string, ((data: unknown) => void)[]> = {};
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 10;
  private isConnecting = false;
  /** Last emitted authStatus — replayed to late subscribers. */
  private lastAuthStatus: AuthStatusEvent | null = null;

  /** Maximum reconnect delay in ms. */
  private static readonly MAX_BACKOFF_MS = 30_000;

  constructor() {
    // Defer so the transport instance is fully constructed before emitting.
    // We store the value in lastAuthStatus so late subscribers don't miss it.
    queueMicrotask(() => {
      if (!this.getToken()) {
        this._emitAuthStatus('no_token');
      }
    });
  }

  private _emitAuthStatus(status: AuthStatusEvent): void {
    this.lastAuthStatus = status;
    this.emit('authStatus', status);
  }

  private getMetaContent(name: string): string | null {
    const el = document.querySelector(`meta[name="${name}"]`);
    return el ? el.getAttribute('content') : null;
  }

  private getToken(): string | null {
    return this.getMetaContent('vox-bearer');
  }

  private getWsUrl(): string {
    const metaUrl = this.getMetaContent('vox-ws-url');
    if (metaUrl) return metaUrl;

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    return `${protocol}//${host}/v1/ws`;
  }

  connect() {
    if (this.ws || this.isConnecting || this.reconnectAttempts > this.maxReconnectAttempts) return;
    this.isConnecting = true;

    // Token is NOT sent in the URL (avoids server-log / referrer leakage).
    // It is sent exclusively as the first WebSocket message after connection.
    const wsUrl = this.getWsUrl();
    const token = this.getToken();

    this.ws = new WebSocket(wsUrl);

    this.ws.onopen = () => {
      console.log('WS connected');
      this.reconnectAttempts = 0;
      this.isConnecting = false;

      if (token && this.ws) {
        this.ws.send(JSON.stringify({ type: 'auth', args: { token } }));
      }

      this.emit('connection_status', { status: 'connected' } satisfies ConnectionStatusPayload);
    };

    this.ws.onerror = (err) => {
      console.error('WS error:', err);
      this.emit('connection_status', { status: 'error', error: 'WebSocket error' } satisfies ConnectionStatusPayload);
    };

    this.ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data as string) as Record<string, unknown>;
        if (msg['type'] === 'agent_event' && msg['data']) {
          const data = msg['data'] as Record<string, unknown>;
          const evtType = (data['type'] ?? msg['msg_type']) as string | undefined;
          if (evtType && this.listeners[evtType] && this.listeners[evtType].length > 0) {
            this.emit(evtType, data);
          } else if (evtType) {
            this.emit('unhandled_typed_event', data);
          } else {
            this.emit('unknown', data);
          }
        } else {
          this.emit(msg['type'] as string, msg);
        }
      } catch (err) {
        console.error('Failed to parse WS message:', err);
      }
    };

    this.ws.onclose = (event) => {
      console.log(`WS disconnected: code=${event.code}, reason=${event.reason}`);
      this.ws = null;
      this.isConnecting = false;
      this.emit('connection_status', {
        status: 'disconnected',
        code: event.code,
        attempt: this.reconnectAttempts,
      } satisfies ConnectionStatusPayload);

      // Stop reconnecting on auth failure (1008 Policy Violation or 4xxx custom auth codes).
      if (event.code === 1008 || event.code === 4001 || event.code === 4003 || event.code === 4401) {
        console.error('WS authentication failed. Stopping reconnects.');
        this._emitAuthStatus('unauthorized');
        return;
      }

      if (this.reconnectAttempts < this.maxReconnectAttempts) {
        // Exponential backoff capped at MAX_BACKOFF_MS.
        const backoff = Math.min(
          250 * Math.pow(2, this.reconnectAttempts),
          VoxTransport.MAX_BACKOFF_MS,
        );
        console.log(
          `Attempting reconnect ${this.reconnectAttempts + 1}/${this.maxReconnectAttempts} in ${backoff}ms…`,
        );
        this.reconnectAttempts++;
        setTimeout(() => this.connect(), backoff);
      } else {
        console.error('WS max reconnect attempts reached.');
        this.emit('connection_status', { status: 'failed_permanently' } satisfies ConnectionStatusPayload);
      }
    };
  }

  async callTool(toolName: string, args: unknown): Promise<unknown> {
    const headers: Record<string, string> = { 'Content-Type': 'application/json' };
    const token = this.getToken();
    if (token) headers['Authorization'] = `Bearer ${token}`;

    const res = await fetch('/v1/tools/call', {
      method: 'POST',
      headers,
      body: JSON.stringify({ name: toolName, args }),
    });
    if (res.status === 401 || res.status === 403) {
      this._emitAuthStatus('unauthorized');
    }
    if (!res.ok) {
      throw new Error(`Tool call failed: ${res.status} ${res.statusText}`);
    }
    return res.json();
  }

  // Typed overloads for known event names — callers get the correct data type.
  on<K extends keyof VoxTransportEventMap>(
    event: K,
    cb: (data: VoxTransportEventMap[K]) => void,
  ): () => void;
  on(event: string, cb: (data: unknown) => void): () => void;
  on(event: string, cb: (data: unknown) => void): () => void {
    if (!this.listeners[event]) this.listeners[event] = [];
    this.listeners[event].push(cb);
    // Replay the last authStatus to late subscribers so they don't miss the
    // one-shot emission from the constructor microtask.
    if (event === 'authStatus' && this.lastAuthStatus !== null) {
      cb(this.lastAuthStatus);
    }
    return () => {
      this.listeners[event] = this.listeners[event].filter((l) => l !== cb);
    };
  }

  // Typed overloads for known event names — callers must pass the correct data type.
  emit<K extends keyof VoxTransportEventMap>(event: K, data: VoxTransportEventMap[K]): void;
  emit(event: string, data: unknown): void;
  emit(event: string, data: unknown): void {
    this.listeners[event]?.forEach((cb) => cb(data));
  }
}

export const voxTransport = new VoxTransport();

export function useVoxTransport(): VoxTransport {
  return voxTransport;
}
