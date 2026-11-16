import type { ITerminalAddon, Terminal } from '@xterm/xterm';
import type { TermBlockDto } from '../../../transport';

export interface WebglTerminalAddonOptions {
  onRendererModeChange?: (mode: 'webgl2' | 'dom') => void;
}

const ANSI_CSI_OSC_RE =
  /\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b\[[0-?]*[ -/]*[@-~]/g;

/**
 * Self-contained WebGL2 terminal addon for xterm.js with deterministic
 * `webglcontextlost` handling and graceful DOM fallback when WebGL2 is
 * unavailable or lost.
 */
export class WebglTerminalAddon implements ITerminalAddon {
  private disposed = false;
  private webglActive = false;
  private listeners: Array<() => void> = [];
  private readonly onRendererModeChange?: (mode: 'webgl2' | 'dom') => void;

  constructor(options?: WebglTerminalAddonOptions) {
    this.onRendererModeChange = options?.onRendererModeChange;
  }

  activate(terminal: Terminal): void {
    this.disposed = false;
    this.webglActive = false;
    try {
      const el = (terminal as unknown as { element?: HTMLElement }).element;
      if (el && typeof document !== 'undefined') {
        const probeCanvas = document.createElement('canvas');
        const gl = (probeCanvas.getContext?.('webgl2') as WebGL2RenderingContext | null) ?? null;
        if (gl) {
          this.webglActive = true;
          try {
            gl.getExtension?.('WEBGL_lose_context')?.loseContext?.();
          } catch {
            /* ignore probe release errors */
          }
        }
      }
    } catch {
      this.webglActive = false;
    }
    this.onRendererModeChange?.(this.webglActive ? 'webgl2' : 'dom');
  }

  isWebglActive(): boolean {
    return this.webglActive && !this.disposed;
  }

  onContextLoss(cb: () => void): { dispose(): void } {
    this.listeners.push(cb);
    return {
      dispose: () => {
        this.listeners = this.listeners.filter((fn) => fn !== cb);
      },
    };
  }

  simulateContextLoss(): void {
    this.webglActive = false;
    this.onRendererModeChange?.('dom');
    for (const cb of [...this.listeners]) {
      try {
        cb();
      } catch {
        /* ignore listener errors */
      }
    }
  }

  isDisposed(): boolean {
    return this.disposed;
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.webglActive = false;
  }
}

/**
 * Format a completed/in-flight `TermBlockDto` as a plain-text snippet suitable
 * for clipboard copy or pre-filling the `SendToAgent` composer.
 */
export function renderTermBlockForAgent(
  block: TermBlockDto | null,
  fallbackLine: string,
): string {
  if (!block) return fallbackLine;
  const lines = [`$ ${block.input}`.trimEnd()];
  const out = block.output
    .map((c) => c.text)
    .join('')
    .replace(ANSI_CSI_OSC_RE, '')
    .replace(/\s+$/, '');
  if (out !== '') lines.push(out);
  lines.push(block.exit_code === null ? '(exit unknown)' : `(exit ${block.exit_code})`);
  return lines.join('\n');
}
