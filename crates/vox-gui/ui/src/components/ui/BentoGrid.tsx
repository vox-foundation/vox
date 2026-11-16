import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useGroupRef, type LayoutChangedMeta } from 'react-resizable-panels';
import { PanelRightOpen } from 'lucide-react';
import { cn } from '../../lib/cn';
import { voxTransport } from '../../transport';
import { useIsEmbeddedSurface } from '../dashboard/EmbeddedSurfaceContext';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from './Resizable';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from './Sheet';

export type BentoGridPreset = 'balanced' | 'dense' | 'hero' | 'triad';

const PRESET_CLASSES: Record<BentoGridPreset, string> = {
  balanced: 'grid grid-cols-1 gap-3 @2xl/surface:grid-cols-2 @5xl/surface:grid-cols-3',
  dense: 'grid grid-cols-1 gap-2.5 @xl/surface:grid-cols-2 @4xl/surface:grid-cols-3 @6xl/surface:grid-cols-4',
  triad: 'grid grid-cols-1 gap-3 @3xl/surface:grid-cols-3',
  hero: 'grid grid-cols-1 gap-3 @3xl/surface:grid-cols-12',
};

export interface BentoGridProps extends React.HTMLAttributes<HTMLDivElement> {
  preset?: BentoGridPreset;
}

export const BentoGrid = React.forwardRef<HTMLDivElement, BentoGridProps>(
  ({ preset = 'balanced', className, ...props }, ref) => (
    <div
      ref={ref}
      data-slot="bento-grid"
      data-bento-preset={preset}
      className={cn('min-h-0 min-w-0', PRESET_CLASSES[preset], className)}
      {...props}
    />
  ),
);
BentoGrid.displayName = 'BentoGrid';

export type BentoCardSpan = 'default' | 'wide' | 'full' | 'hero-main' | 'hero-rail';

const SPAN_CLASSES: Record<BentoCardSpan, string> = {
  default: '',
  wide: '@2xl/surface:col-span-2',
  full: 'col-span-full',
  'hero-main': '@3xl/surface:col-span-7 @5xl/surface:col-span-8',
  'hero-rail': '@3xl/surface:col-span-5 @5xl/surface:col-span-4',
};

export interface BentoCardProps extends React.HTMLAttributes<HTMLElement> {
  span?: BentoCardSpan;
  as?: 'section' | 'div' | 'article';
}

export const BentoCard = React.forwardRef<HTMLElement, BentoCardProps>(
  ({ span = 'default', as: Comp = 'div', className, ...props }, ref) => (
    <Comp
      ref={ref as never}
      data-slot="bento-card"
      className={cn(
        'flex min-h-0 min-w-0 flex-col rounded-xl border border-border-subtle bg-bg-surface/90 p-3.5 text-text-primary shadow-xs',
        SPAN_CLASSES[span],
        className,
      )}
      {...props}
    />
  ),
);
BentoCard.displayName = 'BentoCard';

export interface BentoScrollRegionProps extends React.HTMLAttributes<HTMLDivElement> {
  /** Mandatory accessible region label so Axe `scrollable-region-focusable` passes cleanly. */
  ariaLabel: string;
}

export const BentoScrollRegion = React.forwardRef<HTMLDivElement, BentoScrollRegionProps>(
  ({ ariaLabel, className, tabIndex, children, ...props }, ref) => {
    const innerRef = useRef<HTMLDivElement | null>(null);
    const [isOverflowing, setIsOverflowing] = useState<boolean>(true);

    const setMergedRef = useCallback(
      (node: HTMLDivElement | null) => {
        innerRef.current = node;
        if (typeof ref === 'function') {
          ref(node);
        } else if (ref) {
          ref.current = node;
        }
      },
      [ref],
    );

    const syncOverflow = useCallback(() => {
      const el = innerRef.current;
      if (!el) return;
      if (el.scrollHeight === 0 && el.clientHeight === 0 && el.scrollWidth === 0 && el.clientWidth === 0) {
        setIsOverflowing(true);
        return;
      }
      setIsOverflowing(el.scrollHeight > el.clientHeight || el.scrollWidth > el.clientWidth);
    }, []);

    useEffect(() => {
      syncOverflow();
    }, [syncOverflow, children]);

    useEffect(() => {
      const el = innerRef.current;
      if (!el) return;
      syncOverflow();
      const resizeObs =
        typeof ResizeObserver !== 'undefined' ? new ResizeObserver(() => syncOverflow()) : null;
      resizeObs?.observe(el);
      const mutObs =
        typeof MutationObserver !== 'undefined' ? new MutationObserver(() => syncOverflow()) : null;
      mutObs?.observe(el, { childList: true, subtree: true, characterData: true });
      return () => {
        resizeObs?.disconnect();
        mutObs?.disconnect();
      };
    }, [syncOverflow]);

    return (
      <div
        ref={setMergedRef}
        data-slot="bento-scroll-region"
        role="region"
        aria-label={ariaLabel}
        tabIndex={tabIndex ?? (isOverflowing ? 0 : undefined)}
        className={cn(
          'min-h-0 min-w-0 overflow-auto custom-scrollbar focus-visible:outline-solid focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring',
          className,
        )}
        {...props}
      >
        {children}
      </div>
    );
  },
);
BentoScrollRegion.displayName = 'BentoScrollRegion';

export interface WorkbenchLayoutV4 {
  version: 4;
  masterPct: number;
}

export function clampMasterPct(value: number, min = 25, max = 75): number {
  const lo = Math.min(min, max);
  const hi = Math.max(min, max);
  if (!Number.isFinite(value)) return Math.max(lo, Math.min(hi, 58));
  return Math.max(lo, Math.min(hi, Math.round(value)));
}

export function parseWorkbenchLayoutV4(
  raw: string | null | undefined,
  fallbackPct = 58,
  minPct = 25,
  maxPct = 75,
): number {
  const lo = Math.min(minPct, maxPct);
  const hi = Math.max(minPct, maxPct);
  const safeFallback = clampMasterPct(fallbackPct, lo, hi);
  if (!raw || typeof raw !== 'string') return safeFallback;
  try {
    const parsed = JSON.parse(raw) as Partial<WorkbenchLayoutV4>;
    if (
      parsed &&
      typeof parsed === 'object' &&
      parsed.version === 4 &&
      typeof parsed.masterPct === 'number' &&
      Number.isFinite(parsed.masterPct) &&
      parsed.masterPct >= lo &&
      parsed.masterPct <= hi
    ) {
      return Math.round(parsed.masterPct);
    }
  } catch {
    // Reject corrupt or pre-v4 payloads cleanly
  }
  return safeFallback;
}

export function serializeWorkbenchLayoutV4(
  masterPct: number,
  minPct = 25,
  maxPct = 75,
): string {
  const payload: WorkbenchLayoutV4 = {
    version: 4,
    masterPct: clampMasterPct(masterPct, minPct, maxPct),
  };
  return JSON.stringify(payload);
}

export interface MasterDetailWorkbenchProps {
  /** Preference storage key (must end with `.v4` for schema-validated split persistence). */
  storageKey: string;
  master: React.ReactNode;
  detail: React.ReactNode;
  detailTitle: string;
  detailDescription?: string;
  /** When true in compact (<960px / embedded dock) mode, exposes or opens the Sheet inspector drawer. */
  hasActiveDetail?: boolean;
  /** Controlled open override for compact Sheet drawer; if omitted, tracks `hasActiveDetail` + toggle button. */
  compactOpen?: boolean;
  onCompactOpenChange?: (open: boolean) => void;
  defaultMasterPct?: number;
  minMasterPct?: number;
  minDetailPct?: number;
  /** Force wide (>=960px) or compact (<960px) mode in deterministic unit tests. */
  forceMode?: 'wide' | 'compact';
  className?: string;
}

const WIDE_WORKBENCH_PX = 960;

export function MasterDetailWorkbench({
  storageKey,
  master,
  detail,
  detailTitle,
  detailDescription,
  hasActiveDetail = false,
  compactOpen,
  onCompactOpenChange,
  defaultMasterPct = 58,
  minMasterPct = 30,
  minDetailPct = 25,
  forceMode,
  className,
}: MasterDetailWorkbenchProps) {
  const embedded = useIsEmbeddedSurface();
  const hostRef = useRef<HTMLDivElement | null>(null);
  const groupRef = useGroupRef();
  const maxMasterPct = 100 - minDetailPct;
  const [masterPct, setMasterPct] = useState<number>(() => {
    const localRaw =
      typeof window !== 'undefined' && window.localStorage
        ? window.localStorage.getItem(storageKey)
        : null;
    return localRaw
      ? parseWorkbenchLayoutV4(localRaw, defaultMasterPct, minMasterPct, maxMasterPct)
      : clampMasterPct(defaultMasterPct, minMasterPct, maxMasterPct);
  });
  const hydratedKeyRef = useRef<string | null>(null);
  const userInteractedKeyRef = useRef<string | null>(null);
  const [measuredWide, setMeasuredWide] = useState<boolean>(() =>
    typeof window !== 'undefined' ? window.innerWidth >= WIDE_WORKBENCH_PX : true,
  );
  const [internalDrawerOpen, setInternalDrawerOpen] = useState(false);

  useEffect(() => {
    let cancelled = false;
    hydratedKeyRef.current = null;
    userInteractedKeyRef.current = null;
    const localRaw =
      typeof window !== 'undefined' && window.localStorage
        ? window.localStorage.getItem(storageKey)
        : null;
    const fallbackPct = clampMasterPct(defaultMasterPct, minMasterPct, maxMasterPct);
    const initialPct = localRaw
      ? parseWorkbenchLayoutV4(localRaw, defaultMasterPct, minMasterPct, maxMasterPct)
      : fallbackPct;
    setMasterPct(initialPct);
    try {
      groupRef.current?.setLayout({ master: initialPct, detail: 100 - initialPct });
    } catch {
      // Group may not be mounted yet in compact mode
    }
    if (typeof voxTransport.getGuiPreference === 'function') {
      void voxTransport
        .getGuiPreference(storageKey)
        .then((remoteRaw) => {
          if (cancelled) return;
          hydratedKeyRef.current = storageKey;
          if (remoteRaw && userInteractedKeyRef.current !== storageKey) {
            const remotePct = parseWorkbenchLayoutV4(
              remoteRaw,
              defaultMasterPct,
              minMasterPct,
              maxMasterPct,
            );
            setMasterPct(remotePct);
            try {
              groupRef.current?.setLayout({ master: remotePct, detail: 100 - remotePct });
            } catch {
              // Group may be in compact mode
            }
          }
        })
        .catch(() => {
          if (!cancelled) hydratedKeyRef.current = storageKey;
        });
    } else {
      hydratedKeyRef.current = storageKey;
    }
    return () => {
      cancelled = true;
    };
  }, [storageKey, defaultMasterPct, minMasterPct, maxMasterPct, groupRef]);

  useEffect(() => {
    if (forceMode) return;
    const node = hostRef.current;
    if (!node || typeof ResizeObserver === 'undefined') return;
    let rafId = 0;
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width ?? 0;
      if (width > 0) {
        if (rafId) cancelAnimationFrame(rafId);
        rafId = requestAnimationFrame(() => setMeasuredWide(width >= WIDE_WORKBENCH_PX));
      }
    });
    observer.observe(node);
    return () => {
      if (rafId) cancelAnimationFrame(rafId);
      observer.disconnect();
    };
  }, [forceMode]);

  const isWide = forceMode ? forceMode === 'wide' : !embedded && measuredWide;
  const drawerOpen = compactOpen !== undefined ? compactOpen : internalDrawerOpen;
  const setDrawerOpen = useCallback(
    (next: boolean) => {
      if (compactOpen === undefined) {
        setInternalDrawerOpen(next);
      }
      onCompactOpenChange?.(next);
    },
    [compactOpen, onCompactOpenChange],
  );

  const prevActiveRef = useRef(hasActiveDetail);
  useEffect(() => {
    if (!isWide && hasActiveDetail && !prevActiveRef.current) {
      setDrawerOpen(true);
    }
    prevActiveRef.current = hasActiveDetail;
  }, [hasActiveDetail, isWide, setDrawerOpen]);

  const persistMasterPct = useCallback(
    (nextPct: number) => {
      const clamped = clampMasterPct(nextPct, minMasterPct, maxMasterPct);
      setMasterPct(clamped);
      const serialized = serializeWorkbenchLayoutV4(clamped, minMasterPct, maxMasterPct);
      try {
        if (typeof window !== 'undefined' && window.localStorage) {
          window.localStorage.setItem(storageKey, serialized);
        }
      } catch {
        // Ignore storage quota errors
      }
      if (typeof voxTransport.setGuiPreference === 'function') {
        void voxTransport.setGuiPreference(storageKey, serialized).catch(() => {});
      }
    },
    [storageKey, minMasterPct, maxMasterPct],
  );

  const handleLayoutChanged = useCallback(
    (layout: Record<string, number>, meta?: LayoutChangedMeta) => {
      if (meta && !meta.isUserInteraction) return;
      if (!meta && hydratedKeyRef.current !== storageKey) return;
      const next = layout.master;
      if (typeof next === 'number' && Number.isFinite(next)) {
        userInteractedKeyRef.current = storageKey;
        persistMasterPct(next);
      }
    },
    [persistMasterPct, storageKey],
  );

  if (isWide) {
    return (
      <div
        ref={hostRef}
        data-slot="master-detail-workbench"
        data-workbench-mode="wide"
        className={cn('flex min-h-0 min-w-0 flex-1 flex-col', className)}
      >
        <ResizablePanelGroup
          key={storageKey}
          groupRef={groupRef}
          orientation="horizontal"
          onLayoutChanged={handleLayoutChanged}
          className="min-h-[320px] flex-1 gap-0 rounded-xl border border-border-subtle bg-bg-surface/40"
        >
          <ResizablePanel
            id="master"
            defaultSize={`${masterPct}%`}
            minSize={`${minMasterPct}%`}
            className="flex min-h-0 min-w-0 flex-col p-3"
          >
            {master}
          </ResizablePanel>
          <ResizableHandle withHandle aria-label={`${detailTitle} resize handle`} />
          <ResizablePanel
            id="detail"
            defaultSize={`${100 - masterPct}%`}
            minSize={`${minDetailPct}%`}
            className="flex min-h-0 min-w-0 flex-col bg-bg-elevated/45 p-3"
          >
            {detail}
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>
    );
  }

  return (
    <div
      ref={hostRef}
      data-slot="master-detail-workbench"
      data-workbench-mode="compact"
      className={cn('flex min-h-0 min-w-0 flex-1 flex-col gap-2.5', className)}
    >
      <div className="flex items-center justify-end">
        <button
          type="button"
          data-testid="workbench-open-detail-btn"
          onClick={() => setDrawerOpen(true)}
          className="inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap rounded-md border border-border-subtle bg-bg-elevated px-2.5 py-1 font-mono text-[11px] text-text-secondary transition-colors hover:border-brass/50 hover:text-text-primary"
        >
          <PanelRightOpen className="size-3.5 shrink-0 text-brass" strokeWidth={1.5} aria-hidden="true" />
          <span>Inspect {detailTitle}</span>
        </button>
      </div>
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">{master}</div>
      <Sheet open={drawerOpen} onOpenChange={setDrawerOpen}>
        <SheetContent side="right" className="flex flex-col gap-3 overflow-hidden p-4">
          <SheetHeader>
            <SheetTitle>{detailTitle}</SheetTitle>
            <SheetDescription>
              {detailDescription ?? `Inspector details for ${detailTitle}`}
            </SheetDescription>
          </SheetHeader>
          <BentoScrollRegion ariaLabel={`${detailTitle} inspector content`} className="flex-1 pr-1">
            {detail}
          </BentoScrollRegion>
        </SheetContent>
      </Sheet>
    </div>
  );
}
