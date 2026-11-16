import { useCallback, useEffect, useRef, useState } from 'react';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { consumePendingOpenRunbook, onOpenRunbook, type RunbookLaunchRequest } from '../../../lib/terminalBridge';
import * as transport from '../../../transport';
import type { Toast } from '../../../types/tauri';

export type VoxifyShellHint = 'bash' | 'zsh' | 'fish' | 'pwsh' | 'nu';

const SSH_VALUE_FLAGS = new Set(['-p', '-i', '-l', '-F', '-J', '-L', '-R', '-D', '-o', '-b', '-c', '-E', '-I', '-m', '-S', '-W', '-w']);
const SUDO_VALUE_FLAGS = new Set(['-u', '-g', '-C', '-D', '-R', '-T', '-p']);
const EXEC_VALUE_FLAGS = new Set(['-u', '--user', '-w', '--workdir', '-e', '--env', '--env-file', '-c', '--container', '-n', '--namespace']);
const ONESHOT_SHELL_FLAGS = new Set(['-c', '/c', '/C', '-Command', '-command', '-File', '-file', '--commands']);
const SHELL_MAP: Record<string, VoxifyShellHint> = { bash: 'bash', sh: 'bash', zsh: 'zsh', fish: 'fish', pwsh: 'pwsh', powershell: 'pwsh', nu: 'nu', nushell: 'nu' };

function skipValuedFlags(args: string[], valuedFlags: Set<string>): string[] {
  const pos: string[] = [];
  for (let i = 0; i < args.length; i += 1) {
    const tok = args[i]!;
    if (valuedFlags.has(tok)) { i += 1; continue; }
    if (!tok.startsWith('-')) pos.push(tok);
  }
  return pos;
}

export function detectSubshellShellHint(command: string | null | undefined): VoxifyShellHint | null {
  const stripped = (command ?? '').trim().replace(/^!/, '').trim();
  if (!stripped) return null;
  const tokens = stripped.split(/\s+/);
  const head = (tokens[0]?.split(/[\\/]/).pop() ?? '').toLowerCase().replace(/\.exe$/, '');
  const args = tokens.slice(1);
  if (head === 'env') {
    const cmdIdx = args.findIndex((a) => !a.startsWith('-') && !a.includes('='));
    return cmdIdx >= 0 ? detectSubshellShellHint(args.slice(cmdIdx).join(' ')) : null;
  }
  const direct = SHELL_MAP[head] ?? null;
  if (direct) return args.some((a) => ONESHOT_SHELL_FLAGS.has(a) || !a.startsWith('-')) ? null : direct;
  if (head === 'sudo') {
    if (args.some((a) => a === '-i' || a === '--login' || a === '-s' || a === '--shell') && !args.includes('-c')) return 'bash';
    const pos = skipValuedFlags(args, SUDO_VALUE_FLAGS);
    return pos.length > 0 ? detectSubshellShellHint(pos.join(' ')) : null;
  }
  if (head === 'ssh') {
    if (args.some((a) => a === '-N' || a === '-G' || a === '-V' || a === '-O' || a === '-Q')) return null;
    const pos = skipValuedFlags(args, SSH_VALUE_FLAGS);
    return pos.length === 1 ? 'bash' : pos.length > 1 ? detectSubshellShellHint(pos.slice(1).join(' ')) : null;
  }
  if ((head === 'docker' || head === 'podman' || head === 'kubectl') && args[0] === 'exec') {
    const rest = args.slice(1);
    const hasTty = rest.some((a) => a === '-it' || a === '-ti')
      || (rest.some((a) => a === '-i' || a === '--interactive' || a === '--stdin') && rest.some((a) => a === '-t' || a === '--tty'));
    if (!hasTty) return null;
    const dashDash = rest.indexOf('--');
    if (dashDash >= 0) return detectSubshellShellHint(rest.slice(dashDash + 1).join(' '));
    const pos = skipValuedFlags(rest, EXEC_VALUE_FLAGS);
    return pos.length >= 2 ? detectSubshellShellHint(pos.slice(1).join(' ')) : null;
  }
  return null;
}

interface UseConsoleWarpActionsOptions {
  runningBySession: Record<string, string | null | undefined>; submit: (line: string) => void;
  setApplyLine: (line: string | null) => void; pushToast: (toast: Toast) => void;
}

export function useConsoleWarpActions({ runningBySession, submit, setApplyLine, pushToast }: UseConsoleWarpActionsOptions) {
  const [historyOpenRequestSeq, setHistoryOpenRequestSeq] = useState(0);
  const [railTab, setRailTab] = useState<'help' | 'runbooks'>('help');
  const [selectedRunbookId, setSelectedRunbookId] = useState<string | null>(null);
  const [voxifiedSessionIds, setVoxifiedSessionIds] = useState<Set<string>>(() => new Set());
  const runningRef = useRef(runningBySession); runningRef.current = runningBySession;
  const subshellHintRef = useRef<Record<string, VoxifyShellHint>>({});
  const openHistorySearch = useCallback(() => setHistoryOpenRequestSeq((n) => n + 1), []);

  useEffect(() => {
    const handleReq = (req: RunbookLaunchRequest) => {
      if (req.runbookId) setSelectedRunbookId(req.runbookId);
      if (req.mode === 'run' && req.command) submit(req.command);
      else if (req.mode === 'insert' && req.command) {
        setApplyLine(req.command);
        requestAnimationFrame(() => document.querySelector<HTMLElement>('[aria-label="console input"]')?.focus());
      } else if (req.mode === 'open_rail') setRailTab('runbooks');
    };
    const pendingReq = consumePendingOpenRunbook();
    if (pendingReq) handleReq(pendingReq);
    return onOpenRunbook(handleReq);
  }, [submit, setApplyLine]);

  const resetSessionVoxifiedIfSubshell = useCallback((sessionId: string | null, line: string) => {
    const hint = sessionId ? detectSubshellShellHint(line) : null;
    if (!sessionId || !hint) return;
    subshellHintRef.current[sessionId] = hint;
    setVoxifiedSessionIds((prev) => {
      if (!prev.has(sessionId)) return prev;
      const next = new Set(prev); next.delete(sessionId); return next;
    });
  }, []);

  const handleVoxifySubshell = useCallback(async (sessionId: string) => {
    const fn = (() => { try { return typeof transport.termSessionVoxify === 'function' ? transport.termSessionVoxify : undefined; } catch { return undefined; } })();
    if (!fn) return;
    const shellHint = detectSubshellShellHint(runningRef.current[sessionId]) ?? subshellHintRef.current[sessionId];
    try {
      const ok = await Promise.resolve(fn(sessionId, shellHint));
      if (ok === false) throw new Error('Could not inject OSC 633 prompt hooks into subshell.');
      setVoxifiedSessionIds((prev) => new Set(prev).add(sessionId));
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Voxify Subshell failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  }, [pushToast]);

  return { historyOpenRequestSeq, openHistorySearch, railTab, setRailTab, selectedRunbookId, voxifiedSessionIds, handleVoxifySubshell, resetSessionVoxifiedIfSubshell };
}
