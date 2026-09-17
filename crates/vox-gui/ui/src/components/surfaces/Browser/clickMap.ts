export interface ContainLayout {
  scale: number;
  shownW: number;
  shownH: number;
  padX: number;
  padY: number;
}

/** Letterboxed `object-contain` layout shared by click mapping and the ref overlay. */
export function containLayout(
  frameW: number,
  frameH: number,
  viewW: number,
  viewH: number,
): ContainLayout | null {
  if (frameW <= 0 || frameH <= 0 || viewW <= 0 || viewH <= 0) return null;
  const scale = Math.min(frameW / viewW, frameH / viewH);
  const shownW = viewW * scale;
  const shownH = viewH * scale;
  return {
    scale,
    shownW,
    shownH,
    padX: (frameW - shownW) / 2,
    padY: (frameH - shownH) / 2,
  };
}

export function mapClickToViewport(
  clientX: number,
  clientY: number,
  rect: { left: number; top: number; width: number; height: number },
  viewWidth: number,
  viewHeight: number,
): { x: number; y: number } | null {
  const layout = containLayout(rect.width, rect.height, viewWidth, viewHeight);
  if (!layout) return null;
  const localX = clientX - rect.left - layout.padX;
  const localY = clientY - rect.top - layout.padY;
  if (localX < 0 || localY < 0 || localX > layout.shownW || localY > layout.shownH) {
    return null;
  }
  return {
    x: (localX / layout.shownW) * viewWidth,
    y: (localY / layout.shownH) * viewHeight,
  };
}
