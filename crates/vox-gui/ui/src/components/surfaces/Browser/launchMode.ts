export type LaunchMode = "ephemeral" | "named" | "connect-chrome";

export interface McpOpenInput {
  url: string;
  headless: boolean;
  mode: LaunchMode;
  profileId?: string;
  saveProfile?: boolean;
  cdpUrl?: string;
}

export interface McpOpenArgs {
  tool: "vox_browser_open" | "vox_browser_open_ex";
  args: {
    url: string;
    headless: boolean;
    mode?: "named" | "attach";
    profile_id?: string;
    save_profile?: boolean;
    cdp_url?: string;
  };
}

/** Maps GUI launch-mode state to the MCP tool + args the daemon should call. */
export function mcpOpenArgs(input: McpOpenInput): McpOpenArgs {
  if (input.mode === "ephemeral") {
    return {
      tool: "vox_browser_open",
      args: { url: input.url, headless: input.headless },
    };
  }

  if (input.mode === "named") {
    return {
      tool: "vox_browser_open_ex",
      args: {
        url: input.url,
        headless: input.headless,
        mode: "named",
        profile_id: input.profileId,
        save_profile: input.saveProfile ?? false,
      },
    };
  }

  return {
    tool: "vox_browser_open_ex",
    args: {
      url: input.url,
      headless: input.headless,
      mode: "attach",
      cdp_url: input.cdpUrl,
      save_profile: input.saveProfile ?? false,
    },
  };
}
