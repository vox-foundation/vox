import { describe, expect, it } from "vitest";
import { mcpOpenArgs } from "./launchMode";

describe("mcpOpenArgs", () => {
  it("ephemeral uses vox_browser_open", () => {
    const a = mcpOpenArgs({
      url: "https://example.com",
      headless: true,
      mode: "ephemeral",
    });
    expect(a.tool).toBe("vox_browser_open");
    expect(a.args).toEqual({ url: "https://example.com", headless: true });
  });

  it("named without save is still open_ex with save_profile false", () => {
    const a = mcpOpenArgs({
      url: "https://example.com",
      headless: false,
      mode: "named",
      profileId: "staging-1",
      saveProfile: false,
    });
    expect(a.tool).toBe("vox_browser_open_ex");
    expect(a.args.save_profile).toBe(false);
    expect(a.args.profile_id).toBe("staging-1");
  });
});
