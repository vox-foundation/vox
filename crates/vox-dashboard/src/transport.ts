export class VoxTransport {
  private ws: WebSocket | null = null;
  private listeners: Record<string, ((data: any) => void)[]> = {};
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 10;
  private isConnecting = false;

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
    
    // Fallback
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    return `${protocol}//${host}/v1/ws`;
  }

  connect() {
    if (this.ws || this.isConnecting || this.reconnectAttempts >= this.maxReconnectAttempts) return;
    this.isConnecting = true;

    let wsUrl = this.getWsUrl();
    const token = this.getToken();
    if (token) {
        const joiner = wsUrl.includes('?') ? '&' : '?';
        wsUrl += `${joiner}token=${encodeURIComponent(token)}`;
    }
    
    this.ws = new WebSocket(wsUrl);
    
    this.ws.onopen = () => {
        console.log('WS connected');
        this.reconnectAttempts = 0;
        this.isConnecting = false;
        
        if (token && this.ws) {
            this.ws.send(JSON.stringify({ type: 'auth', args: { token } }));
        }

        this.emit('connection_status', { status: 'connected' });
    };

    this.ws.onerror = (err) => {
        console.error('WS error:', err);
    };
    
    this.ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        if (msg.type === 'agent_event' && msg.data) {
          const evtType = msg.data.type || msg.msg_type;
          if (evtType && this.listeners[evtType] && this.listeners[evtType].length > 0) {
            this.emit(evtType, msg.data);
          } else if (evtType) {
            this.emit('unhandled_typed_event', msg.data);
          } else {
            this.emit('unknown', msg.data);
          }
        } else {
          this.emit(msg.type, msg);
        }
      } catch (err) {
        console.error('Failed to parse WS message:', err);
      }
    };

    this.ws.onclose = (event) => {
      console.log(`WS disconnected: code=${event.code}, reason=${event.reason}`);
      this.ws = null;
      this.isConnecting = false;
      this.emit('connection_status', { status: 'disconnected', code: event.code });
      
      // Stop reconnecting on auth failure (1008 Policy Violation or 4000+ custom auth codes)
      if (event.code === 1008 || event.code === 4001 || event.code === 4003) {
          console.error("WS authentication failed. Stopping reconnects.");
          return;
      }
      
      if (this.reconnectAttempts < this.maxReconnectAttempts) {
          const backoff = Math.min(2000 * Math.pow(1.5, this.reconnectAttempts), 15000);
          console.log(`Attempting reconnect ${this.reconnectAttempts + 1}/${this.maxReconnectAttempts} in ${backoff}ms...`);
          this.reconnectAttempts++;
          setTimeout(() => this.connect(), backoff);
      } else {
          console.error("WS max reconnect attempts reached.");
      }
    };
  }

  async callTool(toolName: string, args: any): Promise<any> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json'
    };
    const token = this.getToken();
    if (token) {
        headers['Authorization'] = `Bearer ${token}`;
    }

    const res = await fetch('/v1/tools/call', {
      method: 'POST',
      headers,
      body: JSON.stringify({
        name: toolName,
        args
      })
    });
    if (!res.ok) {
        throw new Error(`Tool call failed: ${res.status} ${res.statusText}`);
    }
    return res.json();
  }

  on(event: string, cb: (data: any) => void) {
    if (!this.listeners[event]) {
      this.listeners[event] = [];
    }
    this.listeners[event].push(cb);
    return () => {
      this.listeners[event] = this.listeners[event].filter(l => l !== cb);
    };
  }

  emit(event: string, data: any) {
    if (this.listeners[event]) {
      this.listeners[event].forEach(cb => cb(data));
    }
  }
}

export const voxTransport = new VoxTransport();

export function useVoxTransport() {
  return voxTransport;
}
