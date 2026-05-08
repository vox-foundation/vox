import { describe, expect, it } from "vitest";
import { parseIntent } from "../src/ts/intent_parser";

describe("intent_parser", () => {
  it("parses mood phrase", () => {
    const r = parseIntent('My mood is like a 4');
    expect(r.kind).toBe("mood_recorded");
    if (r.kind === "mood_recorded") expect(r.payload.mood_score).toBe(4);
  });

  it("parses sleep phrase", () => {
    const r = parseIntent("I'm going to bed now");
    expect(r.kind).toBe("sleep_started");
  });
});
