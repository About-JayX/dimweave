import { describe, expect, test, mock } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

let claudeState = {
  profile: null,
  profileError:
    'profile api status 401 Unauthorized: {"type":"error","error":{"type":"authentication_error","message":"Invalid authentication credentials","details":{"error_visibility":"user_facing"}},"request_id":"req_011CaExtremelyLongIdentifier"}',
  usage: null,
  usageError: null,
  usageRefreshing: false,
  fetchProfile: async () => {},
  refreshUsage: async () => {},
};

let codexState = {
  profile: {
    email: "jason2@mega-server.xyz",
    name: "Mdakuyo",
    planType: "prolite",
  },
  usage: null,
  refreshing: false,
  fetchProfile: async () => {},
  fetchUsage: async () => {},
  refreshUsage: async () => {},
};

let providerAuthState = {
  configs: {
    claude: {
      provider: "claude" as const,
      activeMode: "subscription" as const,
      updatedAt: 1,
    },
    codex: {
      provider: "codex" as const,
      activeMode: "subscription" as const,
      updatedAt: 1,
    },
  },
  fetchAll: async () => {},
};

const claudeStore = Object.assign(
  (selector: (state: typeof claudeState) => unknown) => selector(claudeState),
  {
    getState: () => claudeState,
    setState: (next: Partial<typeof claudeState>) => {
      claudeState = { ...claudeState, ...next };
    },
  },
);

const codexStore = Object.assign(
  (selector: (state: typeof codexState) => unknown) => selector(codexState),
  {
    getState: () => codexState,
    setState: (next: Partial<typeof codexState>) => {
      codexState = { ...codexState, ...next };
    },
  },
);

const providerAuthStore = Object.assign(
  (selector: (state: typeof providerAuthState) => unknown) =>
    selector(providerAuthState),
  {
    getState: () => providerAuthState,
    setState: (next: Partial<typeof providerAuthState>) => {
      providerAuthState = { ...providerAuthState, ...next };
    },
  },
);

mock.module("@/stores/claude-account-store", () => ({
  useClaudeAccountStore: claudeStore,
}));

mock.module("@/stores/codex-account-store", () => ({
  useCodexAccountStore: codexStore,
}));

mock.module("@/stores/provider-auth-store", () => ({
  useProviderAuthStore: providerAuthStore,
}));

mock.module("./ProviderAuthDialog", () => ({
  ProviderAuthDialog: () => null,
}));

describe("AccountsInfoPanel", () => {
  test("long Claude profile errors render with wrapping safeguards", async () => {
    const { AccountsInfoPanel } = await import("./AccountsInfoPanel");
    const html = renderToStaticMarkup(<AccountsInfoPanel />);

    expect(html).toContain(
      "max-w-full overflow-hidden whitespace-pre-wrap break-all [overflow-wrap:anywhere]",
    );
    expect(html).toContain("Invalid authentication credentials");
  });

  test("account rows allow value text to shrink and truncate inside the card", async () => {
    const { AccountsInfoPanel } = await import("./AccountsInfoPanel");
    const html = renderToStaticMarkup(<AccountsInfoPanel />);

    expect(html).toContain("min-w-0 flex-1 truncate text-right");
    expect(html).toContain('title="jason2@mega-server.xyz"');
  });
});
