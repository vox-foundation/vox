import { describe, expect, it } from "vitest";
import fixtures from "./fixtures/parser_cases.json";
import { parseIntent } from "../src/ts/intent_parser";

type Fixture = {
  transcript: string;
  expect_kind: string;
  expect_payload?: Record<string, unknown>;
  note?: string;
};

describe("intent_parser — quick checks", () => {
  it("parses mood phrase", () => {
    const r = parseIntent("My mood is like a 4");
    expect(r.kind).toBe("mood_recorded");
    if (r.kind === "mood_recorded") expect(r.payload.mood_score).toBe(4);
  });

  it("parses sleep phrase", () => {
    const r = parseIntent("I'm going to bed now");
    expect(r.kind).toBe("sleep_started");
  });
});

describe("intent_parser — fixtures (drives both runtimes)", () => {
  for (const fx of fixtures as Fixture[]) {
    const label = fx.transcript;
    it(`classifies "${label}" as ${fx.expect_kind}`, () => {
      const r = parseIntent(fx.transcript);
      expect(r.kind).toBe(fx.expect_kind);

      if (fx.expect_payload) {
        for (const [k, v] of Object.entries(fx.expect_payload)) {
          expect((r.payload as Record<string, unknown>)[k]).toEqual(v);
        }
      }
    });
  }
});
