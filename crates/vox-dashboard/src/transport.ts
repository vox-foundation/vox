export class VoxTransport {
  private ws: WebSocket | null = null;
  private listeners: Record<string, ((data: any) => void)[]> = {};

  connect() {
    // Determine the host dynamically or use Vite dev server proxy
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const wsUrl = `${protocol}//${host}/v1/ws`;
    
    this.ws = new WebSocket(wsUrl);
    
    this.ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        if (msg.type === 'agent_event' && msg.data) {
          this.emit(msg.data.type || msg.msg_type || 'unknown', msg.data);
        } else {
          this.emit(msg.type, msg);
        }
      } catch (err) {
        console.error('Failed to parse WS message:', err);
      }
    };

    this.ws.onclose = () => {
      console.log('WS disconnected, attempting reconnect...');
      setTimeout(() => this.connect(), 2000);
    };
  }

  async callTool(toolName: string, args: any): Promise<any> {
    const res = await fetch('/v1/tools/call', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        name: toolName,
        args
      })
    });
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
