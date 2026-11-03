import { describe, expect, it } from "vitest";
import { overlayItems } from "./refOverlay";

describe("overlayItems", () => {
  it("skips refs without boxes", () => {
    const items = overlayItems(
      { e1: { box_css: { x: 10, y: 20, width: 80, height: 24 } }, e2: {} },
      { frameW: 640, frameH: 400, viewW: 1280, viewH: 800 },
    );
    expect(items).toHaveLength(1);
    expect(items[0].refId).toBe("e1");
    expect(items[0].left).toBe(5);
    expect(items[0].top).toBe(10);
  });

  it("uses the same letterbox scale and offset as mapClickToViewport", () => {
    // Container 1280x900 displaying a 1280x800 viewport → 50px top/bottom pads.
    const items = overlayItems(
      { e1: { box_css: { x: 10, y: 20, width: 80, height: 24 } } },
      { frameW: 1280, frameH: 900, viewW: 1280, viewH: 800 },
    );
    expect(items).toHaveLength(1);
    expect(items[0].left).toBe(10);
    expect(items[0].top).toBe(70);
    expect(items[0].width).toBe(80);
    expect(items[0].height).toBe(24);
  });
});
