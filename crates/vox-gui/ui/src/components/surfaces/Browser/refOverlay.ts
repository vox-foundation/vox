import { containLayout } from "./clickMap";

export interface OverlayBox {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface OverlayRef {
  box_css?: OverlayBox | null;
}

export interface OverlayFrame {
  frameW: number;
  frameH: number;
  viewW: number;
  viewH: number;
}

export interface OverlayItem {
  refId: string;
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Inverse of `mapClickToViewport`: CSS viewport boxes → letterboxed frame pixels. */
export function overlayItems(
  refs: Record<string, OverlayRef>,
  frame: OverlayFrame,
): OverlayItem[] {
  const layout = containLayout(frame.frameW, frame.frameH, frame.viewW, frame.viewH);
  if (!layout) return [];
  const items: OverlayItem[] = [];
  for (const [refId, ref] of Object.entries(refs)) {
    const box = ref.box_css;
    if (!box) continue;
    items.push({
      refId,
      left: layout.padX + (box.x / frame.viewW) * layout.shownW,
      top: layout.padY + (box.y / frame.viewH) * layout.shownH,
      width: (box.width / frame.viewW) * layout.shownW,
      height: (box.height / frame.viewH) * layout.shownH,
    });
  }
  return items;
}
