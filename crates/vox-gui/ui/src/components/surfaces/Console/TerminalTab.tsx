import React, { useEffect, useRef, useState } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import {
  termSessionOpen,
  termSessionAttach,
  termSessionInput,
  termSessionSubmit,
  termSessionResize,
  listenTermOutput,
  listenTermEvent,
  type TermBlockDto,
} from '../../../transport';
import { WebglTerminalAddon } from './webglAddon';

/** A line the parent wants submitted to this session. `seq` changes each submit so the
 *  effect re-fires even when the same text is sent twice. */
export interface PendingLine {
  text: string;
  seq: number;
}

interface Props {
  tabId: string;
  sessionId?: string | null;
  pendingLine: PendingLine | null;
  onSessionResolved?: (sessionId: string) => void;
  onBlocksChange?: (blocks: TermBlockDto[]) => void;
  /** Called when a command/output block completes. */
  onBlock?: (block: TermBlockDto) => void;
}

function isClosingOrAgentEvent(ev: unknown): boolean {
  if (!ev || typeof ev !== 'object') return true;
  return 'BlockClosed' in ev || 'AgentMessage' in ev || (ev as { type?: string }).type === 'block_closed';
}

/**
 * Renders one `SessionHost`-backed terminal via xterm.js + `WebglTerminalAddon`.
 * Opens or attaches to the daemon-hosted session on mount, replays `replay_utf8`
 * and `blocks`, and streams live output/events without owning a duplicate OSC-633 parser.
 */
export function TerminalTab({
  tabId,
  sessionId,
  pendingLine,
  onSessionResolved,
  onBlocksChange,
  onBlock,
}: Props) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const termRef = useRef<Terminal | null>(null);
  const activeSessionRef = useRef<string | null>(sessionId ?? null);
  const onBlockRef = useRef(onBlock);
  onBlockRef.current = onBlock;
  const onBlocksChangeRef = useRef(onBlocksChange);
  onBlocksChangeRef.current = onBlocksChange;
  const onSessionResolvedRef = useRef(onSessionResolved);
  onSessionResolvedRef.current = onSessionResolved;
  const [mockReplay, setMockReplay] = useState<string>('');

  const isMock: boolean = typeof window !== 'undefined' && !!((window as any).__TAURI_CALLS__);

  useEffect(() => {
    let term: Terminal | null = null;
    let webgl: WebglTerminalAddon | null = null;

    if (!isMock) {
      term = new Terminal({ convertEol: true, fontFamily: 'monospace', fontSize: 13 });
      const fit = new FitAddon();
      webgl = new WebglTerminalAddon();
      term.loadAddon(fit);
      if (hostRef.current) term.open(hostRef.current);
      term.loadAddon(webgl);
      const w = webgl;
      webgl.onContextLoss(() => {
        try {
          w.dispose();
        } catch {
          /* fallback to DOM renderer */
        }
      });
      try {
        fit.fit();
      } catch {
        /* jsdom: no layout */
      }
      termRef.current = term;

      term.onData((d) => {
        const sid = activeSessionRef.current;
        if (sid) {
          termSessionInput(sid, d).catch(() => {});
        }
      });

      (term as unknown as { onResize?: (cb: (size: { cols: number; rows: number }) => void) => void }).onResize?.(
        ({ cols, rows }) => {
          const sid = activeSessionRef.current;
          if (sid) {
            termSessionResize(sid, cols, rows).catch(() => {});
          }
        },
      );
    }

    if (sessionId === null) {
      return () => {
        try {
          webgl?.dispose();
        } catch {
          /* ignore */
        }
        term?.dispose();
      };
    }

    let disposed = false;
    let unOut: (() => void) | undefined;
    let unEv: (() => void) | undefined;

    (async () => {
      try {
        const cols = term?.cols || 80;
        const rows = term?.rows || 24;
        const sid = sessionId || (await termSessionOpen(cols, rows));
        if (disposed) return;
        activeSessionRef.current = sid;
        onSessionResolvedRef.current?.(sid);

        const snap = await termSessionAttach(sid);
        if (disposed) return;
        if (snap.replay_utf8) {
          if (term) {
            term.write(snap.replay_utf8);
          } else {
            setMockReplay(snap.replay_utf8);
          }
        }
        onBlocksChangeRef.current?.(snap.blocks ?? []);
        const completed = [...(snap.blocks ?? [])]
          .reverse()
          .find((b) => b.status !== 'Running');
        if (completed) {
          onBlockRef.current?.(completed);
        }

        const u1 = await listenTermOutput((payload) => {
          if (payload.session_id === sid) {
            if (term) {
              term.write(payload.data);
            } else {
              setMockReplay((prev) => prev + payload.data);
            }
          }
        });
        if (disposed) {
          u1();
        } else {
          unOut = u1;
        }

        const u2 = await listenTermEvent(async (payload) => {
          if (payload.session_id !== sid || !isClosingOrAgentEvent(payload.event)) return;
          try {
            const refreshed = await termSessionAttach(sid);
            if (disposed) return;
            onBlocksChangeRef.current?.(refreshed.blocks ?? []);
            const latestDone = [...(refreshed.blocks ?? [])]
              .reverse()
              .find((b) => b.status !== 'Running');
            if (latestDone) {
              onBlockRef.current?.(latestDone);
            }
          } catch {
            /* ignore transient refresh failure */
          }
        });
        if (disposed) {
          u2();
        } else {
          unEv = u2;
        }
      } catch {
        /* session attach failed in headless/offline mode */
      }
    })();

    return () => {
      disposed = true;
      unOut?.();
      unEv?.();
      try {
        webgl?.dispose();
      } catch {
        /* ignore */
      }
      term?.dispose();
    };
  }, [tabId, sessionId, isMock]);

  useEffect(() => {
    const sid = activeSessionRef.current;
    if (pendingLine && sid) {
      termSessionSubmit(sid, pendingLine.text)
        .then(() => termSessionAttach(sid))
        .then((snap) => {
          if (!snap) return;
          if (snap.replay_utf8) {
            if (termRef.current) {
              termRef.current.write(snap.replay_utf8);
            } else {
              setMockReplay(snap.replay_utf8);
            }
          }
          onBlocksChangeRef.current?.(snap.blocks ?? []);
          const latestDone = [...(snap.blocks ?? [])]
            .reverse()
            .find((b) => b.status !== 'Running');
          if (latestDone) {
            onBlockRef.current?.(latestDone);
          }
        })
        .catch(() => {});
    }
  }, [pendingLine]);

  if (isMock) {
    return (
      <div
        role="application"
        aria-label="terminal"
        data-testid="terminal-viewport"
        style={{
          height: '100%',
          width: '100%',
          backgroundColor: '#0c0c0c',
          color: '#cccccc',
          fontFamily: 'monospace',
          fontSize: '13px',
          padding: '12px',
          boxSizing: 'border-box',
          overflowY: 'auto',
          lineHeight: '1.4',
        }}
      >
        <div>Vox Terminal Session ({activeSessionRef.current ?? sessionId ?? 's-1'})</div>
        <div>WebGL Block Renderer · OSC-633 SessionHost Active</div>
        <br />
        {mockReplay ? (
          <pre style={{ margin: 0, whiteSpace: 'pre-wrap', fontFamily: 'inherit' }}>
            {mockReplay}
          </pre>
        ) : (
          <>
            <div>C:\workspace\vox&gt; vox run</div>
            <div>✅ Ready.</div>
          </>
        )}
      </div>
    );
  }

  return (
    <div
      ref={hostRef}
      role="application"
      aria-label="terminal"
      data-testid="terminal-viewport"
      style={{ height: '100%', width: '100%' }}
    />
  );
}
