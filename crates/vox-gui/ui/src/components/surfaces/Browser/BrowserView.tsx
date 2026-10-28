import React, { useCallback, useEffect, useRef, useState } from 'react';
import { backendAvailable, sanitizeErrorForToast } from '../../../lib/backendGuard';
import { invoke } from '@tauri-apps/api/core';
import { useLabel } from '../../../hooks/useLanguage';
import {
  listenAgentEvents,
  listenBrowserFrames,
  listenPreviewAvailable,
  type BrowserPageInfo,
  type BrowserPageSummary,
  type BrowserFramePayload,
  type PreviewAvailablePayload,
} from '../../../transport';
import { recordGamifyGuiEvent } from '../../../lib/gamifyGuiEvents';
import type { Toast } from '../../../types/tauri';
import { useIsEmbeddedSurface } from '../../dashboard/EmbeddedSurfaceContext';
import { BrowserToolbar, type ControlMode } from './BrowserToolbar';
import { mapClickToViewport } from './clickMap';
import type { LaunchMode } from './launchMode';
import { isLoopbackPreviewUrl } from './previewUrl';
import { overlayItems, type OverlayRef } from './refOverlay';

export { mapClickToViewport } from './clickMap';

interface BrowserViewProps {
  pushToast: (item: Toast) => void;
  gamifyEnabled?: boolean;
}

interface PreviewStatus {
  active: boolean;
  url: string | null;
  app_dir: string | null;
  source: string;
}

interface PlaywrightValidateResult {
  exit_code: number;
  stdout: string;
  stderr: string;
  preview_url: string | null;
}

type BrowserTab = 'preview' | 'agent';
export type SonghandOperation = 'act' | 'extract' | 'extract_json' | 'wait_for' | 'read_dom';
const DEFAULT_VIEWPORT_WIDTH = 1280;
const DEFAULT_VIEWPORT_HEIGHT = 800;

const MAX_ACTION_LOG = 50;

interface SnapshotPayload {
  refs?: Record<string, OverlayRef>;
  tree?: string;
}

export function BrowserView({ pushToast, gamifyEnabled }: BrowserViewProps) {
  const embedded = useIsEmbeddedSurface();
  const [tab, setTab] = useState<BrowserTab>('preview');
  const [previewUrl, setPreviewUrl] = useState('http://127.0.0.1:3000');
  const [appDir, setAppDir] = useState('');
  const [preview, setPreview] = useState<PreviewStatus | null>(null);
  const [previewReloadNonce, setPreviewReloadNonce] = useState(0);
  const [busy, setBusy] = useState(false);

  const [agentUrl, setAgentUrl] = useState('https://example.com');
  const [headless, setHeadless] = useState(true);
  const [launchMode, setLaunchMode] = useState<LaunchMode>('ephemeral');
  const [profileId, setProfileId] = useState('');
  const [saveProfile, setSaveProfile] = useState(false);
  const [cdpUrl, setCdpUrl] = useState('http://127.0.0.1:9222');
  const [pageId, setPageId] = useState<string | null>(null);
  const lastNeedsHumanLine = useRef<string | null>(null);
  const [pages, setPages] = useState<BrowserPageSummary[]>([]);
  const [pageInfo, setPageInfo] = useState<BrowserPageInfo | null>(null);
  const [agentNavUrl, setAgentNavUrl] = useState('');
  const [controlMode, setControlMode] = useState<ControlMode>('you');
  const [previewFrameBlocked, setPreviewFrameBlocked] = useState(false);
  const [frame, setFrame] = useState<BrowserFramePayload | null>(null);
  const [actionLog, setActionLog] = useState<string[]>([]);
  const [validateOut, setValidateOut] = useState('');
  const [showRefs, setShowRefs] = useState(false);
  const [snapshotRefs, setSnapshotRefs] = useState<Record<string, OverlayRef>>({});
  const [songhandOp, setSonghandOp] = useState<SonghandOperation>('act');
  const [songhandInstruction, setSonghandInstruction] = useState('');
  const [songhandSchema, setSonghandSchema] = useState(
    '{"type":"object","properties":{"title":{"type":"string"},"items":{"type":"array"}}}',
  );
  const [songhandOutput, setSonghandOutput] = useState<string>('');
  const [refFillValues, setRefFillValues] = useState<Record<string, string>>({});
  const frameBoxRef = useRef<HTMLDivElement>(null);
  const frameImgRef = useRef<HTMLImageElement>(null);
  const [frameSize, setFrameSize] = useState({ frameW: 0, frameH: 0 });

  const refreshPreviewStatus = useCallback(async () => {
    try {
      const status = await invoke<PreviewStatus>('preview_status');
      setPreview(status);
      if (status.url) setPreviewUrl(status.url);
      if (status.app_dir) setAppDir(status.app_dir);
    } catch {
      setPreview(null);
    }
  }, []);

  const refreshSessionStatus = useCallback(async () => {
    try {
      const status = await invoke<{
        page_id: string | null;
        headless: boolean;
        control_mode?: string;
        action_log: string[];
      } | null>(
        'browser_session_status',
      );
      if (!status) return;
      setPageId(status.page_id ?? null);
      if (typeof status.headless === 'boolean') setHeadless(status.headless);
      if (status.control_mode === 'agent' || status.control_mode === 'you') {
        setControlMode(status.control_mode);
      }
      setActionLog(Array.isArray(status.action_log) ? status.action_log : []);
      if (!status.page_id) setPageInfo(null);
    } catch {
      setPageId(null);
    }
  }, []);

  const refreshPages = useCallback(async () => {
    try {
      const list = await invoke<BrowserPageSummary[] | null>('browser_list_pages');
      const safeList = Array.isArray(list) ? list : [];
      setPages(safeList);
      if (safeList.length === 0) {
        setPageInfo(null);
        setAgentNavUrl('');
      }
    } catch {}
  }, []);

  const refreshPageInfo = useCallback(async (selectedPageId?: string | null) => {
    const currentPage = selectedPageId ?? pageId;
    if (!currentPage) {
      setPageInfo(null);
      return;
    }
    try {
      const info = await invoke<BrowserPageInfo>('browser_page_info', {
        input: { page_id: currentPage },
      });
      setPageInfo(info);
      if (info.url) setAgentNavUrl(info.url);
    } catch {
      setPageInfo(null);
    }
  }, [pageId]);

  useEffect(() => {
    refreshPreviewStatus();
    refreshSessionStatus();
    refreshPages();
  }, [refreshPreviewStatus, refreshSessionStatus, refreshPages]);

  useEffect(() => {
    // Embedded mini-render: the initial fetch (effect above) already populated
    // the thumbnail; skip the 2s poll.
    if (embedded) return;
    const id = window.setInterval(() => {
      refreshPages();
      refreshSessionStatus();
    }, 2000);
    return () => window.clearInterval(id);
  }, [refreshPages, refreshSessionStatus, embedded]);

  useEffect(() => {
    refreshPageInfo(pageId);
  }, [pageId, refreshPageInfo]);

  const toastNeedsHuman = useCallback(
    (reason?: string | null) => {
      pushToast({
        tone: 'warn',
        title: 'Human action required',
        body: reason || undefined,
        cause: 'backend-ok',
      });
    },
    [pushToast],
  );

  useEffect(() => {
    const hit = [...actionLog].reverse().find((line) => line.startsWith('needs_human:'));
    if (hit && hit !== lastNeedsHumanLine.current) {
      lastNeedsHumanLine.current = hit;
      toastNeedsHuman(hit.slice('needs_human:'.length).trim());
    }
  }, [actionLog, toastNeedsHuman]);

  // Surface genuine agent-driven browser activity from the orchestrator event
  // stream and auto-sync open browser pages whenever an agent invokes a
  // `vox_browser_*` tool from Chat (`Loquela`) or the orchestrator.
  useEffect(() => {
    let unlistenAgent: (() => void) | undefined;
    listenAgentEvents((frameEvt) => {
      const kind = (frameEvt.kind ?? { type: '' }) as Record<string, unknown>;
      const kindType = typeof kind.type === 'string' ? kind.type : '';
      const toolKey: string | undefined =
        typeof kind.tool_key === 'string'
          ? kind.tool_key
          : typeof kind.tool === 'string'
            ? kind.tool
            : undefined;
      if (toolKey?.startsWith('vox_browser') || kindType.startsWith('browser_')) {
        const when = new Date(frameEvt.timestamp_ms ?? Date.now()).toLocaleTimeString();
        const statusSuffix =
          kindType === 'tool_timed_out' ? 'timed out' : kindType || 'invoked';
        const line = `${when} agent: ${toolKey ?? kindType} (${statusSuffix})`;
        setActionLog((prev) => [...prev.slice(-(MAX_ACTION_LOG - 1)), line]);
        void refreshPages();
        void refreshSessionStatus();
      }
    })
      .then((fn) => {
        unlistenAgent = fn;
      })
      .catch(() => {});
    return () => {
      if (unlistenAgent) unlistenAgent();
    };
  }, [refreshPages, refreshSessionStatus]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listenBrowserFrames((payload) => {
      setFrame(payload);
      if (payload.action_log?.length) setActionLog(payload.action_log);
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {});
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listenPreviewAvailable((payload: PreviewAvailablePayload) => {
      if (!isLoopbackPreviewUrl(payload.url)) {
        return;
      }
      setPreviewFrameBlocked(false);
      setTab('preview');
      setPreview({
        active: true,
        url: payload.url,
        app_dir: payload.app_dir,
        source: payload.source,
      });
      setPreviewUrl(payload.url);
      void recordGamifyGuiEvent(
        'browser_preview_loaded',
        { url: payload.url, source: payload.source },
        { enabled: gamifyEnabled },
      );
      // Treat subsequent preview-available events as rebuild/restart signals.
      setPreviewReloadNonce((n) => n + 1);
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {});
    return () => {
      if (unlisten) unlisten();
    };
  }, [gamifyEnabled]);

  const startPreview = async () => {
    const url = previewUrl.trim() || null;
    // Preview without Tauri: iframe the URL; never leave the toolbar stuck busy.
    if (!backendAvailable()) {
      setPreview({
        active: Boolean(url),
        url,
        app_dir: appDir.trim() || null,
        source: 'iframe',
      });
      return;
    }
    setBusy(true);
    try {
      const status = await Promise.race([
        invoke<PreviewStatus>('preview_start', {
          input: {
            url,
            app_dir: appDir.trim() || null,
          },
        }),
        new Promise<never>((_, reject) => {
          window.setTimeout(() => reject(new Error('preview_start_timeout')), 4_000);
        }),
      ]);
      setPreview(status);
      pushToast({ tone: 'ok', title: 'Preview started', body: status.url ?? undefined, cause: 'backend-ok' });
      void recordGamifyGuiEvent(
        'browser_preview_loaded',
        { url: status.url, source: status.source },
        { enabled: gamifyEnabled },
      );
    } catch (err) {
      // Hung or missing IPC: still show the URL in an iframe so the toolbar recovers.
      if (url) {
        setPreview({
          active: true,
          url,
          app_dir: appDir.trim() || null,
          source: 'iframe',
        });
      }
      pushToast({ tone: 'warn', title: 'Preview failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  };

  const stopPreview = async () => {
    if (!backendAvailable()) {
      setPreview(null);
      return;
    }
    setBusy(true);
    try {
      const status = await invoke<PreviewStatus>('preview_stop');
      setPreview(status);
      pushToast({ tone: 'info', title: 'Preview stopped', cause: 'backend-ok' });
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Stop failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  };

  const openAgentSession = async () => {
    setBusy(true);
    try {
      const result = await invoke<{
        page_id: string | null;
        needs_human?: boolean;
        reason?: string | null;
      }>('browser_open_session', {
        input: {
          url: agentUrl.trim(),
          headless,
          mode: launchMode,
          profile_id: launchMode === 'named' ? profileId.trim() || null : null,
          save_profile: launchMode === 'ephemeral' ? null : saveProfile,
          cdp_url: launchMode === 'connect-chrome' ? cdpUrl.trim() || null : null,
        },
      });
      setPageId(result.page_id ?? null);
      setTab('agent');
      if (result.needs_human) {
        lastNeedsHumanLine.current = `needs_human: ${result.reason ?? 'needs_human'}`;
        toastNeedsHuman(result.reason);
      } else {
        pushToast({ tone: 'ok', title: 'Browser session opened', body: result.page_id ?? undefined, cause: 'backend-ok' });
      }
      await refreshSessionStatus();
      await refreshPages();
      await refreshPageInfo(result.page_id ?? null);
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Open failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  };

  const closeAgentSession = async () => {
    setBusy(true);
    try {
      await invoke('browser_close_session');
      setPageId(null);
      setFrame(null);
      setPageInfo(null);
      setShowRefs(false);
      setSnapshotRefs({});
      pushToast({ tone: 'info', title: 'Browser session closed', cause: 'backend-ok' });
      await refreshPages();
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Close failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    const el = frameImgRef.current ?? frameBoxRef.current;
    if (!el) return;
    const syncSize = () => {
      const rect = el.getBoundingClientRect();
      setFrameSize({ frameW: rect.width, frameH: rect.height });
    };
    syncSize();
    const observer = new ResizeObserver(syncSize);
    observer.observe(el);
    return () => observer.disconnect();
  }, [tab, frame]);

  const captureSnapshot = async () => {
    try {
      const snap = await invoke<SnapshotPayload>('browser_snapshot');
      setSnapshotRefs(snap.refs ?? {});
    } catch (err) {
      setSnapshotRefs({});
      pushToast({ tone: 'warn', title: 'Snapshot failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  };

  const onShowRefsChange = async (checked: boolean) => {
    setShowRefs(checked);
    if (checked) {
      await captureSnapshot();
    } else {
      setSnapshotRefs({});
    }
  };

  const captureFrame = async () => {
    try {
      const payload = await invoke<BrowserFramePayload>('browser_screenshot_frame');
      setFrame(payload);
      if (payload.action_log?.length) setActionLog(payload.action_log);
      if (payload.page_id) {
        setPageId(payload.page_id);
        await refreshPageInfo(payload.page_id);
      }
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Screenshot failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  };

  const attachPage = async (selectedPageId: string) => {
    setBusy(true);
    try {
      await invoke('browser_attach_session', { input: { page_id: selectedPageId } });
      setPageId(selectedPageId);
      await refreshSessionStatus();
      await refreshPageInfo(selectedPageId);
      if (showRefs) await captureSnapshot();
      pushToast({ tone: 'ok', title: 'Attached session', body: selectedPageId, cause: 'backend-ok' });
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Attach failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  };

  const closePage = async (selectedPageId: string) => {
    setBusy(true);
    try {
      await invoke('browser_close_page', { input: { page_id: selectedPageId } });
      if (pageId === selectedPageId) {
        setPageId(null);
        setPageInfo(null);
        setShowRefs(false);
        setSnapshotRefs({});
      }
      await refreshPages();
      pushToast({ tone: 'info', title: 'Page closed', body: selectedPageId, cause: 'backend-ok' });
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Close page failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  };

  const setControlModeRemote = async (nextMode: ControlMode) => {
    setControlMode(nextMode);
    try {
      await invoke('browser_set_control_mode', { input: { mode: nextMode } });
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Failed to set control mode', body: sanitizeErrorForToast(err), cause: 'backend-error' });
      await refreshSessionStatus();
    }
  };

  const navigate = async (action: 'back' | 'forward' | 'reload' | 'stop') => {
    if (!pageId) return;
    setBusy(true);
    try {
      await invoke('browser_navigate', { input: { action } });
      await refreshPageInfo(pageId);
      if (showRefs) await captureSnapshot();
    } catch (err) {
      pushToast({ tone: 'warn', title: `Navigation ${action} failed`, body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  };

  const gotoUrl = async () => {
    if (!pageId || !agentNavUrl.trim()) return;
    setBusy(true);
    try {
      await invoke('browser_goto_url', { input: { url: agentNavUrl.trim() } });
      await refreshPageInfo(pageId);
      if (showRefs) await captureSnapshot();
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Goto failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  };

  const onFrameClick = async (event: React.MouseEvent<HTMLDivElement>) => {
    if (!pageId || controlMode !== 'you') return;
    const viewWidth = frame?.viewport_width ?? DEFAULT_VIEWPORT_WIDTH;
    const viewHeight = frame?.viewport_height ?? DEFAULT_VIEWPORT_HEIGHT;
    const rect = (frameImgRef.current ?? event.currentTarget).getBoundingClientRect();
    const mapped = mapClickToViewport(
      event.clientX,
      event.clientY,
      { left: rect.left, top: rect.top, width: rect.width, height: rect.height },
      viewWidth,
      viewHeight,
    );
    if (!mapped) return;
    try {
      await invoke('browser_click_xy', { input: mapped });
      await captureFrame();
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Click failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  };

  const onFrameWheel = async (event: React.WheelEvent<HTMLDivElement>) => {
    if (!pageId || controlMode !== 'you') return;
    event.preventDefault();
    try {
      await invoke('browser_scroll', {
        input: { dx: Math.round(event.deltaX), dy: Math.round(event.deltaY) },
      });
      await captureFrame();
    } catch (err) {
      pushToast({ tone: 'warn', title: 'Scroll failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    }
  };

  const onFrameKeyDown = async (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (!pageId || controlMode !== 'you') return;
    if (event.key.length === 1 && !event.ctrlKey && !event.altKey && !event.metaKey) {
      event.preventDefault();
      await invoke('browser_type_text', { input: { text: event.key } });
      await captureFrame();
      return;
    }
    const handledKeys = new Set([
      'Enter',
      'Backspace',
      'Tab',
      'Escape',
      'ArrowUp',
      'ArrowDown',
      'ArrowLeft',
      'ArrowRight',
      'Home',
      'End',
      'PageUp',
      'PageDown',
    ]);
    if (!handledKeys.has(event.key)) return;
    event.preventDefault();
    await invoke('browser_input_key', { input: { key: event.key } });
    await captureFrame();
  };

  const runPlaywrightValidate = async () => {
    setBusy(true);
    setValidateOut('');
    try {
      const result = await invoke<PlaywrightValidateResult>('browser_validate_playwright', {
        input: { preview_url: (preview?.url ?? previewUrl.trim()) || null },
      });
      const text = [result.stdout, result.stderr].filter(Boolean).join('\n').trim();
      setValidateOut(text || `(exit ${result.exit_code})`);
      pushToast({
        tone: result.exit_code === 0 ? 'ok' : 'warn',
        title: result.exit_code === 0 ? 'Playwright passed' : 'Playwright failed',
        body: result.preview_url ?? undefined,
        cause: result.exit_code === 0 ? 'backend-ok' : 'backend-error',
      });
    } catch (err) {
      setValidateOut(sanitizeErrorForToast(err));
      pushToast({ tone: 'warn', title: 'Validate failed', body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  };

  const runSonghandCommand = async () => {
    const instruction = songhandInstruction.trim();
    if (!instruction && songhandOp !== 'read_dom') {
      pushToast({
        tone: 'warn',
        title: 'Instruction required',
        body: 'Enter a natural-language Songhand instruction or CSS/XPath selector.',
        cause: 'validation',
      });
      return;
    }
    setBusy(true);
    try {
      let res: unknown = null;
      if (songhandOp === 'act') {
        res = await invoke('browser_act', {
          input: { instruction, page_id: pageId },
        });
      } else if (songhandOp === 'extract') {
        res = await invoke('browser_extract', {
          input: { instruction, page_id: pageId },
        });
      } else if (songhandOp === 'extract_json') {
        res = await invoke('browser_extract_json', {
          input: {
            instruction,
            schema_json: songhandSchema.trim() || '{"type":"object"}',
            page_id: pageId,
          },
        });
      } else if (songhandOp === 'wait_for') {
        res = await invoke('browser_wait_for', {
          input: { target: instruction, timeout_secs: 15, page_id: pageId },
        });
      } else {
        res = await invoke('browser_read_dom', {
          input: { target: instruction || 'body', html: false, page_id: pageId },
        });
      }
      const formatted =
        typeof res === 'string' ? res : JSON.stringify(res ?? { ok: true, op: songhandOp }, null, 2);
      setSonghandOutput(formatted);
      setActionLog((prev) => [
        ...prev.slice(-(MAX_ACTION_LOG - 1)),
        `songhand.${songhandOp}: ${instruction || 'body'}`,
      ]);
      await refreshSessionStatus();
      if (pageId) await captureFrame();
      pushToast({
        tone: 'ok',
        title: `Songhand ${songhandOp} complete`,
        body: instruction.slice(0, 64) || 'DOM read complete',
        cause: 'backend-ok',
      });
    } catch (err) {
      const msg = sanitizeErrorForToast(err);
      setSonghandOutput(`Error: ${msg}`);
      pushToast({ tone: 'warn', title: `Songhand ${songhandOp} failed`, body: msg, cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  };

  const clickSnapshotRef = async (refId: string) => {
    setBusy(true);
    try {
      const res = await invoke<{ needs_human?: boolean; reason?: string }>('browser_click_ref', {
        input: { ref_id: refId, page_id: pageId },
      });
      setActionLog((prev) => [...prev.slice(-(MAX_ACTION_LOG - 1)), `click_ref [${refId}]`]);
      if (res?.needs_human) {
        toastNeedsHuman(res.reason ?? `Ref [${refId}] requires human confirmation`);
      } else {
        pushToast({ tone: 'ok', title: `Clicked ref [${refId}]`, cause: 'backend-ok' });
      }
      if (pageId) await captureFrame();
    } catch (err) {
      pushToast({ tone: 'warn', title: `Click ref [${refId}] failed`, body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  };

  const fillSnapshotRef = async (refId: string) => {
    const val = (refFillValues[refId] ?? '').trim();
    if (!val) return;
    setBusy(true);
    try {
      const res = await invoke<{ needs_human?: boolean; reason?: string }>('browser_fill_ref', {
        input: { ref_id: refId, value: val, page_id: pageId },
      });
      setActionLog((prev) => [...prev.slice(-(MAX_ACTION_LOG - 1)), `fill_ref [${refId}]`]);
      if (res?.needs_human) {
        toastNeedsHuman(res.reason ?? `Ref [${refId}] requires human confirmation`);
      } else {
        pushToast({ tone: 'ok', title: `Filled ref [${refId}]`, cause: 'backend-ok' });
      }
      if (pageId) await captureFrame();
    } catch (err) {
      pushToast({ tone: 'warn', title: `Fill ref [${refId}] failed`, body: sanitizeErrorForToast(err), cause: 'backend-error' });
    } finally {
      setBusy(false);
    }
  };

  const sendBrowserContextToChat = () => {
    const activeTargetUrl = pageInfo?.url || (preview?.active ? preview.url : null) || agentUrl || previewUrl;
    const refCount = Object.keys(snapshotRefs).length;
    const summarySnippet = songhandOutput
      ? songhandOutput.slice(0, 280)
      : validateOut
        ? validateOut.slice(0, 280)
        : `page_id=${pageId ?? 'preview'} refs=${refCount} mode=${controlMode}`;
    const token = `@browser(${activeTargetUrl})`;
    window.dispatchEvent(
      new CustomEvent('vox:browser-context', {
        detail: {
          url: activeTargetUrl,
          pageId: pageId ?? 'preview',
          refCount,
          summary: summarySnippet,
          token,
        },
      }),
    );
    window.dispatchEvent(
      new CustomEvent('vox:atlas-mention', {
        detail: { token },
      }),
    );
    pushToast({
      tone: 'ok',
      title: 'Sent Browser context to Chat',
      body: `${token} attached to Loquela composer`,
      cause: 'backend-ok',
    });
  };

  // CRITICAL FIX: Never mount an unstarted `http://127.0.0.1:3000` iframe on initial load!
  // Previously `preview?.url ?? previewUrl` evaluated to `http://127.0.0.1:3000` even when
  // `preview?.active` was false, causing BrowserView to either iframe whatever unrelated local
  // app was listening on port 3000 (e.g., "Open Idle / Melvor Engine") or render a giant white box.
  const rawPreviewUrl = preview?.active ? preview.url : null;
  const activePreviewUrl =
    rawPreviewUrl && isLoopbackPreviewUrl(rawPreviewUrl) ? rawPreviewUrl : null;
  const candidatePlaywrightUrl =
    activePreviewUrl ?? (isLoopbackPreviewUrl(previewUrl.trim()) ? previewUrl.trim() : null);

  return (
    <section className="space-y-4">
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="font-display text-lg tracking-[0.14em] uppercase text-text-primary">{useLabel('browser')}</h2>
          <p className="text-[12px] text-text-muted mt-1">
            Preview Vox web apps and mirror agent-driven CDP browser sessions.
          </p>
        </div>
        <div className="flex gap-2" role="tablist" aria-label="Browser view">
          <button
            type="button"
            role="tab"
            aria-selected={tab === 'preview'}
            onClick={() => setTab('preview')}
            className={`px-3 py-1.5 rounded-lg text-[11px] uppercase tracking-wider ${tab === 'preview' ? 'bg-brass/15 text-brass ring-1 ring-brass/30' : 'bg-overlay-subtle text-text-muted'}`}
          >
            Preview
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={tab === 'agent'}
            onClick={() => setTab('agent')}
            className={`px-3 py-1.5 rounded-lg text-[11px] uppercase tracking-wider ${tab === 'agent' ? 'bg-brass/15 text-brass ring-1 ring-brass/30' : 'bg-overlay-subtle text-text-muted'}`}
          >
            Agent live view
          </button>
        </div>
      </header>

      {tab === 'preview' && (
        <div className="space-y-3">
          <div className="grid gap-3 md:grid-cols-2">
            <label className="block space-y-1">
              <span className="text-[10px] uppercase tracking-wider text-text-muted">Preview URL</span>
              <input
                value={previewUrl}
                onChange={(e) => setPreviewUrl(e.target.value)}
                className="w-full rounded-lg bg-overlay-subtle border border-border-subtle px-3 py-2 text-sm text-text-secondary"
                placeholder="http://127.0.0.1:3000"
              />
            </label>
            <label className="block space-y-1">
              <span className="text-[10px] uppercase tracking-wider text-text-muted">
                App dir (Preview spawn — needs dev:ssr-upstream or dev script)
              </span>
              <input
                value={appDir}
                onChange={(e) => setAppDir(e.target.value)}
                className="w-full rounded-lg bg-overlay-subtle border border-border-subtle px-3 py-2 text-sm text-text-secondary"
                placeholder="path to a vox web app (leave blank to use the URL above)"
              />
            </label>
          </div>
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              disabled={busy}
              onClick={startPreview}
              className="rounded-lg bg-brass/20 text-brass px-4 py-2 text-[11px] uppercase tracking-wider disabled:opacity-50"
            >
              Start preview
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={stopPreview}
              className="rounded-lg bg-overlay-subtle text-text-secondary px-4 py-2 text-[11px] uppercase tracking-wider disabled:opacity-50"
            >
              Stop
            </button>
            <button
              type="button"
              disabled={busy || !candidatePlaywrightUrl}
              onClick={runPlaywrightValidate}
              className="rounded-lg bg-overlay-subtle text-text-secondary px-4 py-2 text-[11px] uppercase tracking-wider disabled:opacity-50"
            >
              Validate (Playwright)
            </button>
            <button
              type="button"
              data-testid="browser-preview-send-to-chat"
              onClick={sendBrowserContextToChat}
              className="rounded-lg border border-brass/30 bg-brass/10 text-brass px-4 py-2 text-[11px] uppercase tracking-wider hover:bg-brass/20"
            >
              Send to Chat
            </button>
          </div>
          {preview && (
            <p className="text-[11px] text-text-muted font-mono">
              source={preview.source} active={String(preview.active)} url={preview.url ?? '—'}
            </p>
          )}
          {validateOut && (
            <pre className="max-h-40 overflow-auto rounded-lg bg-black/40 p-3 text-[11px] text-text-muted font-mono whitespace-pre-wrap">
              {validateOut}
            </pre>
          )}
          {activePreviewUrl && !previewFrameBlocked ? (
            <iframe
              key={`${activePreviewUrl}-${previewReloadNonce}`}
              title="Vox app preview"
              src={activePreviewUrl}
              className="w-full min-h-[480px] rounded-xl border border-border-subtle bg-bg-base"
              sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
              onError={() => setPreviewFrameBlocked(true)}
            />
          ) : (
            <div
              data-testid="browser-harness-quick-deck"
              className="rounded-xl border border-border-subtle bg-bg-surface/60 p-5 space-y-4"
            >
              <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border-subtle pb-3">
                <div>
                  <h3 className="font-mono text-xs font-semibold uppercase tracking-wider text-brass">
                    Playwright &amp; Songhand Browser Harness Control Deck
                  </h3>
                  <p className="text-xs text-text-muted mt-1">
                    {activePreviewUrl
                      ? 'Preview frame blocked by page framing policy (X-Frame-Options/CSP). Switch to Agent Live View for CDP streaming.'
                      : 'No loopback iframe preview is active. Start a Vox web app preview above, run Playwright E2E validation, or launch the Songhand AI Agent Browser (`vox_browser_act` / `extract` / `[eN]` accessibility refs).'}
                  </p>
                </div>
                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    data-testid="browser-launch-songhand-cta"
                    onClick={() => setTab('agent')}
                    className="rounded-lg bg-brass/20 text-brass border border-brass/30 px-3.5 py-2 text-[11px] font-mono uppercase tracking-wider hover:bg-brass/30"
                  >
                    Open Songhand &amp; Agent View
                  </button>
                  <button
                    type="button"
                    data-testid="browser-run-playwright-cta"
                    disabled={busy || !candidatePlaywrightUrl}
                    onClick={runPlaywrightValidate}
                    className="rounded-lg bg-overlay-subtle border border-border-subtle text-text-secondary px-3.5 py-2 text-[11px] font-mono uppercase tracking-wider hover:border-white/20 disabled:opacity-50"
                  >
                    Run Playwright Suite
                  </button>
                </div>
              </div>

              <div className="grid gap-3 md:grid-cols-3">
                <div className="rounded-lg border border-border-subtle bg-black/25 p-3.5 space-y-1.5">
                  <div className="flex items-center justify-between">
                    <span className="font-mono text-[10px] uppercase tracking-wider text-brass">
                      Songhand AI Primitives
                    </span>
                    <span className="rounded bg-brass/15 px-1.5 py-0.5 font-mono text-[10px] text-brass">
                      MCP Ready
                    </span>
                  </div>
                  <p className="text-xs text-text-secondary">
                    Natural-language <code className="font-mono text-[11px]">vox_browser_act</code>,{' '}
                    <code className="font-mono text-[11px]">vox_browser_extract</code>, and schema-constrained{' '}
                    <code className="font-mono text-[11px]">vox_browser_extract_json</code> over live CDP tabs.
                  </p>
                </div>

                <div className="rounded-lg border border-border-subtle bg-black/25 p-3.5 space-y-1.5">
                  <div className="flex items-center justify-between">
                    <span className="font-mono text-[10px] uppercase tracking-wider text-emerald-400">
                      Accessibility [eN] Refs
                    </span>
                    <span className="rounded bg-emerald-500/15 px-1.5 py-0.5 font-mono text-[10px] text-emerald-300">
                      Deterministic
                    </span>
                  </div>
                  <p className="text-xs text-text-secondary">
                    Capture <code className="font-mono text-[11px]">vox_browser_snapshot</code> bounding boxes and drive{' '}
                    <code className="font-mono text-[11px]">click_ref</code> / <code className="font-mono text-[11px]">fill_ref</code> with{' '}
                    <code className="font-mono text-[11px]">needs_human</code> safety locks.
                  </p>
                </div>

                <div className="rounded-lg border border-border-subtle bg-black/25 p-3.5 space-y-1.5">
                  <div className="flex items-center justify-between">
                    <span className="font-mono text-[10px] uppercase tracking-wider text-cyan">
                      Chat &amp; Dock Integration
                    </span>
                    <span className="rounded bg-cyan/15 px-1.5 py-0.5 font-mono text-[10px] text-cyan">
                      Loquela Linked
                    </span>
                  </div>
                  <p className="text-xs text-text-secondary">
                    Dock Browser side-by-side in Chat (<code className="font-mono text-[11px]">Panels ▾ → Browser</code>) and pipe extracted DOM/snapshots into <code className="font-mono text-[11px]">Loquela</code>.
                  </p>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {tab === 'agent' && (
        <div className="space-y-3">
          <BrowserToolbar
            agentUrl={agentUrl}
            onAgentUrlChange={setAgentUrl}
            headless={headless}
            onHeadlessChange={setHeadless}
            launchMode={launchMode}
            onLaunchModeChange={setLaunchMode}
            profileId={profileId}
            onProfileIdChange={setProfileId}
            saveProfile={saveProfile}
            onSaveProfileChange={setSaveProfile}
            cdpUrl={cdpUrl}
            onCdpUrlChange={setCdpUrl}
            busy={busy}
            pageId={pageId}
            pageInfo={pageInfo}
            pages={pages}
            agentNavUrl={agentNavUrl}
            onAgentNavUrlChange={setAgentNavUrl}
            controlMode={controlMode}
            onOpen={openAgentSession}
            onClose={closeAgentSession}
            onCapture={captureFrame}
            onNavigate={navigate}
            onGoto={gotoUrl}
            onAttach={attachPage}
            onClosePage={closePage}
            onControlModeToggle={() => setControlModeRemote(controlMode === 'you' ? 'agent' : 'you')}
          />
          <div className="flex flex-wrap items-center justify-between gap-3">
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={showRefs}
                disabled={!pageId}
                onChange={(e) => {
                  void onShowRefsChange(e.target.checked);
                }}
                className="rounded-sm"
              />
              <span className="text-[11px] text-text-muted">Show [eN] accessibility refs</span>
            </label>
            <div className="flex flex-wrap items-center gap-2">
              <button
                type="button"
                data-testid="browser-capture-snapshot-btn"
                disabled={busy || !pageId}
                onClick={captureSnapshot}
                className="rounded-lg border border-border-subtle bg-overlay-subtle px-3 py-1.5 font-mono text-[11px] uppercase tracking-wider text-text-secondary hover:border-white/20 disabled:opacity-50"
              >
                Snapshot [eN] ({Object.keys(snapshotRefs).length})
              </button>
              <button
                type="button"
                data-testid="browser-send-to-chat"
                onClick={sendBrowserContextToChat}
                className="rounded-lg border border-brass/30 bg-brass/15 px-3 py-1.5 font-mono text-[11px] uppercase tracking-wider text-brass hover:bg-brass/25"
              >
                Send Context to Chat
              </button>
            </div>
          </div>

          {/* Songhand (Stagehand-Style AI) Act / Extract / Extract JSON / Wait Command Bar */}
          <div
            data-testid="browser-songhand-bar"
            className="rounded-xl border border-border-subtle bg-bg-surface/70 p-3 space-y-2.5"
          >
            <div className="flex flex-wrap items-center justify-between gap-2">
              <span className="font-mono text-[10px] font-semibold uppercase tracking-wider text-brass">
                Songhand Agentic Browser Bar (Act · Extract · Schema JSON · Wait · DOM)
              </span>
              <span className="font-mono text-[10px] text-text-muted">
                Control lock: <strong className="text-text-secondary">{controlMode.toUpperCase()}</strong>
              </span>
            </div>
            <div className="grid gap-2 md:grid-cols-[200px_1fr_auto]">
              <select
                aria-label="Songhand operation"
                data-testid="browser-songhand-op"
                data-unified-knob="true"
                value={songhandOp}
                onChange={(e) => setSonghandOp(e.target.value as SonghandOperation)}
                className="h-9 rounded-lg border border-border-subtle bg-overlay-subtle px-2.5 font-mono text-xs text-text-primary focus:border-brass focus:outline-none"
              >
                <option value="act">Songhand Act (LLM)</option>
                <option value="extract">Extract (Natural Language)</option>
                <option value="extract_json">Extract JSON (Schema)</option>
                <option value="wait_for">Wait For Selector</option>
                <option value="read_dom">Read DOM Text</option>
              </select>
              <input
                aria-label="Songhand instruction"
                data-testid="browser-songhand-input"
                value={songhandInstruction}
                onChange={(e) => setSonghandInstruction(e.target.value)}
                placeholder={
                  songhandOp === 'wait_for' || songhandOp === 'read_dom'
                    ? 'CSS or xpath:/... selector (e.g. #main-content, button.submit)'
                    : 'Natural-language goal or extraction prompt (e.g. Click Sign In, summarize pricing tiers)'
                }
                className="h-9 w-full rounded-lg border border-border-subtle bg-overlay-subtle px-3 text-xs text-text-secondary"
              />
              <button
                type="button"
                data-testid="browser-songhand-run"
                disabled={busy}
                onClick={runSonghandCommand}
                className="h-9 rounded-lg bg-brass/20 border border-brass/30 px-4 font-mono text-[11px] uppercase tracking-wider text-brass hover:bg-brass/30 disabled:opacity-50"
              >
                Run Songhand
              </button>
            </div>
            {songhandOp === 'extract_json' && (
              <label className="block space-y-1">
                <span className="font-mono text-[10px] uppercase tracking-wider text-text-muted">
                  JSON Schema (draft-07)
                </span>
                <input
                  aria-label="Extraction JSON schema"
                  value={songhandSchema}
                  onChange={(e) => setSonghandSchema(e.target.value)}
                  className="w-full rounded-lg border border-border-subtle bg-overlay-subtle px-3 py-1.5 font-mono text-xs text-text-secondary"
                />
              </label>
            )}
            {songhandOutput && (
              <pre
                data-testid="browser-songhand-output"
                className="max-h-36 overflow-auto rounded-lg border border-white/5 bg-black/50 p-2.5 font-mono text-[11px] text-text-secondary whitespace-pre-wrap"
              >
                {songhandOutput}
              </pre>
            )}
          </div>

          <div className="grid gap-3 lg:grid-cols-[2fr_1fr]">
            <div
              ref={frameBoxRef}
              tabIndex={0}
              onClick={onFrameClick}
              onWheel={onFrameWheel}
              onKeyDown={onFrameKeyDown}
              className="relative rounded-xl border border-border-subtle bg-black/30 min-h-[360px] flex items-center justify-center overflow-hidden focus:outline-hidden focus:ring-2 focus:ring-brass/30"
            >
              {frame?.image_base64 ? (
                <img
                  ref={frameImgRef}
                  src={`data:${frame.mime ?? "image/png"};base64,${frame.image_base64}`}
                  alt="Agent browser frame"
                  className="w-full h-full max-h-[480px] object-contain pointer-events-none"
                />
              ) : (
                <span className="text-text-muted text-sm px-4 text-center">
                  {frame?.error ?? 'No frame yet — open a session or wait for the live CDP screencast stream (~3s).'}
                </span>
              )}
              {showRefs &&
                overlayItems(snapshotRefs, {
                  frameW: frameSize.frameW,
                  frameH: frameSize.frameH,
                  viewW: frame?.viewport_width ?? DEFAULT_VIEWPORT_WIDTH,
                  viewH: frame?.viewport_height ?? DEFAULT_VIEWPORT_HEIGHT,
                }).map((item) => (
                  <button
                    type="button"
                    key={item.refId}
                    onClick={(e) => {
                      e.stopPropagation();
                      void clickSnapshotRef(item.refId);
                    }}
                    title={`Click accessibility ref [${item.refId}]`}
                    className="absolute border border-brass/70 bg-brass/15 text-[10px] leading-none text-brass font-mono px-0.5 hover:bg-brass/35"
                    style={{
                      left: item.left,
                      top: item.top,
                      width: item.width,
                      height: item.height,
                    }}
                  >
                    [{item.refId}]
                  </button>
                ))}
            </div>
            <div className="space-y-3">
              <div
                data-testid="browser-ref-inspector"
                className="rounded-xl border border-border-subtle bg-black/20 p-3 max-h-[200px] overflow-auto"
              >
                <h3 className="text-[10px] uppercase tracking-wider text-text-muted mb-2">
                  Accessibility Snapshot Refs ({Object.keys(snapshotRefs).length})
                </h3>
                {Object.keys(snapshotRefs).length === 0 ? (
                  <p className="text-[11px] font-mono text-text-muted">
                    No [eN] refs captured yet. Click &ldquo;Snapshot [eN]&rdquo; or toggle &ldquo;Show [eN] accessibility refs&rdquo;.
                  </p>
                ) : (
                  <ul className="space-y-1.5 text-[11px] font-mono">
                    {Object.entries(snapshotRefs).map(([refId, refMeta]) => (
                      <li
                        key={refId}
                        className="flex flex-wrap items-center justify-between gap-1.5 rounded border border-white/5 bg-black/30 px-2 py-1"
                      >
                        <span className="text-brass font-semibold">
                          [{refId}]{' '}
                          <span className="text-text-secondary font-normal">
                            {refMeta.role ?? refMeta.tag ?? 'element'}
                            {refMeta.name ? ` "${refMeta.name}"` : ''}
                          </span>
                        </span>
                        <div className="flex items-center gap-1">
                          <button
                            type="button"
                            disabled={busy}
                            onClick={() => void clickSnapshotRef(refId)}
                            className="rounded border border-border-subtle bg-overlay-subtle px-1.5 py-0.5 text-[10px] text-text-secondary hover:text-brass"
                          >
                            Click
                          </button>
                          <input
                            aria-label={`Fill value for ${refId}`}
                            value={refFillValues[refId] ?? ''}
                            onChange={(e) =>
                              setRefFillValues((prev) => ({ ...prev, [refId]: e.target.value }))
                            }
                            placeholder="fill..."
                            className="w-20 rounded border border-border-subtle bg-overlay-subtle px-1.5 py-0.5 text-[10px] text-text-secondary"
                          />
                          <button
                            type="button"
                            disabled={busy || !(refFillValues[refId] ?? '').trim()}
                            onClick={() => void fillSnapshotRef(refId)}
                            className="rounded border border-border-subtle bg-overlay-subtle px-1.5 py-0.5 text-[10px] text-text-secondary hover:text-brass disabled:opacity-50"
                          >
                            Fill
                          </button>
                        </div>
                      </li>
                    ))}
                  </ul>
                )}
              </div>

              <div className="rounded-xl border border-border-subtle bg-black/20 p-3 max-h-[200px] overflow-auto">
                <h3 className="text-[10px] uppercase tracking-wider text-text-muted mb-2">Action log</h3>
                <ul role="log" aria-live="polite" aria-label="Browser action log" className="space-y-1 text-[11px] font-mono text-text-muted">
                  {(actionLog.length ? actionLog : ['(empty)']).map((line, i) => (
                    <li key={`${line}-${i}`}>{line}</li>
                  ))}
                </ul>
              </div>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
