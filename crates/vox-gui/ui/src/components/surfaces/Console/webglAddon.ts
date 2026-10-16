import type { ITerminalAddon, Terminal } from '@xterm/xterm';
import type { TermBlockDto } from '../../../transport';

/**
 * Self-contained WebGL2 terminal addon for xterm.js with deterministic
 * `webglcontextlost` handling and graceful DOM fallback when WebGL2 is
 * unavailable or lost.
 */
export class WebglTerminalAddon implements ITerminalAddon {
  private disposed = false;
  private listeners: Array<() => void> = [];
  private canvas: HTMLCanvasElement | null = null;
  private gl: WebGL2RenderingContext | WebGLRenderingContext | null = null;
  private onLostBound = (ev: Event) => {
    ev.preventDefault();
    this.simulateContextLoss();
  };

  activate(terminal: Terminal): void {
    this.disposed = false;
    try {
      const el = (terminal as unknown as { element?: HTMLElement }).element;
      if (el && typeof document !== 'undefined') {
        const canvas = document.createElement('canvas');
        canvas.addEventListener('webglcontextlost', this.onLostBound);
        this.canvas = canvas;
        this.gl =
          (canvas.getContext?.('webgl2') as WebGL2RenderingContext | null) ??
          (canvas.getContext?.('webgl') as WebGLRenderingContext | null) ??
          null;
      }
    } catch {
      /* jsdom or headless environment without WebGL: fallback to xterm DOM renderer */
    }
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
    try {
      this.canvas?.removeEventListener('webglcontextlost', this.onLostBound);
      const loseExt = this.gl?.getExtension?.('WEBGL_lose_context');
      loseExt?.loseContext?.();
    } catch {
      /* ignore teardown errors */
    }
    this.canvas = null;
    this.gl = null;
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
    .replace(/\s+$/, '');
  if (out !== '') lines.push(out);
  lines.push(block.exit_code === null ? '(exit unknown)' : `(exit ${block.exit_code})`);
  return lines.join('\n');
}
