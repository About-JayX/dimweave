import { useEffect, useMemo, useState } from "react";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import { useClaudeAccountStore } from "@/stores/claude-account-store";
import {
  useProviderAuthStore,
  type ProviderAuthConfig,
} from "@/stores/provider-auth-store";
import { ClaudeIcon, CodexIcon } from "@/components/AgentStatus/BrandIcons";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface ProviderAuthDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

type Kind = "claude" | "codex";

interface FormState {
  apiKey: string;
  baseUrl: string;
  wireApi: string; // codex only
  authMode: "bearer" | "api_key"; // claude only
  providerName: string; // codex only
  showAdvanced: boolean;
}

const EMPTY_FORM: FormState = {
  apiKey: "",
  baseUrl: "",
  wireApi: "chat",
  authMode: "bearer",
  providerName: "",
  showAdvanced: false,
};

function fromConfig(cfg: ProviderAuthConfig | undefined): FormState {
  if (!cfg) return { ...EMPTY_FORM };
  return {
    apiKey: cfg.apiKey ?? "",
    baseUrl: cfg.baseUrl ?? "",
    wireApi: cfg.wireApi ?? "chat",
    authMode: (cfg.authMode as "bearer" | "api_key") ?? "bearer",
    providerName: cfg.providerName ?? "",
    showAdvanced: Boolean(
      cfg.baseUrl || cfg.wireApi || cfg.providerName || cfg.authMode,
    ),
  };
}

function toConfig(kind: Kind, f: FormState): ProviderAuthConfig {
  const base: ProviderAuthConfig = {
    provider: kind,
    apiKey: f.apiKey.trim() || null,
    baseUrl: f.baseUrl.trim() || null,
    wireApi: kind === "codex" && f.baseUrl.trim() ? f.wireApi : null,
    authMode: kind === "claude" && f.apiKey.trim() ? f.authMode : null,
    providerName:
      kind === "codex" && f.providerName.trim() ? f.providerName.trim() : null,
    updatedAt: 0,
  };
  return base;
}

function SubscriptionRow({ kind }: { kind: Kind }) {
  const codexProfile = useCodexAccountStore((s) => s.profile);
  const codexLogin = useCodexAccountStore((s) => s.login);
  const codexLogout = useCodexAccountStore((s) => s.logout);
  const codexLoginPending = useCodexAccountStore((s) => s.loginPending);
  const codexLoginUri = useCodexAccountStore((s) => s.loginUri);
  const codexCancelLogin = useCodexAccountStore((s) => s.cancelLogin);
  const claudeProfile = useClaudeAccountStore((s) => s.profile);
  const claudeAuthStatus = useClaudeAccountStore((s) => s.authStatus);
  const claudeLogin = useClaudeAccountStore((s) => s.login);
  const claudeLogout = useClaudeAccountStore((s) => s.logout);
  const claudeLoginPending = useClaudeAccountStore((s) => s.loginPending);
  const claudeLoginUri = useClaudeAccountStore((s) => s.loginUri);
  const claudeCancelLogin = useClaudeAccountStore((s) => s.cancelLogin);
  const claudeLoginError = useClaudeAccountStore((s) => s.loginError);

  if (kind === "codex") {
    if (codexLoginPending) {
      return (
        <div className="rounded-md border border-codex/20 bg-codex/5 px-3 py-2 space-y-1.5">
          <div className="flex items-center gap-2 text-[11px] text-codex">
            <span className="size-3 border-2 border-codex/30 border-t-codex rounded-full radius-keep animate-spin shrink-0" />
            Waiting for browser login...
          </div>
          {codexLoginUri && (
            <a
              href={codexLoginUri}
              target="_blank"
              rel="noreferrer"
              className="block truncate text-[10px] text-codex/80 hover:text-codex hover:underline"
              title={codexLoginUri}
            >
              Open login page →
            </a>
          )}
          <Button
            size="sm"
            variant="ghost"
            className="h-6 text-[10px] text-muted-foreground hover:text-destructive"
            onClick={() => void codexCancelLogin()}
          >
            Cancel
          </Button>
        </div>
      );
    }
    if (codexProfile?.email) {
      return (
        <div className="flex items-center justify-between rounded-md bg-muted/30 px-2.5 py-1.5 text-[11px]">
          <div className="flex items-center gap-1.5 min-w-0">
            <span className="size-1.5 shrink-0 rounded-full radius-keep bg-codex" />
            <span
              className="truncate text-foreground/80"
              title={codexProfile.email}
            >
              {codexProfile.email}
            </span>
            {codexProfile.planType && (
              <span className="shrink-0 rounded bg-codex/10 px-1 py-px text-[9px] font-semibold uppercase text-codex">
                {codexProfile.planType}
              </span>
            )}
          </div>
          <button
            type="button"
            onClick={() => void codexLogout()}
            className="ml-2 text-[10px] text-muted-foreground hover:text-destructive"
          >
            Logout
          </button>
        </div>
      );
    }
    return (
      <Button
        size="sm"
        variant="outline"
        className="w-full text-[11px] border-codex/30 text-codex hover:bg-codex/10"
        onClick={() => void codexLogin()}
      >
        Login with ChatGPT
      </Button>
    );
  }

  // Claude
  if (claudeLoginPending) {
    return (
      <div className="rounded-md border border-primary/20 bg-primary/5 px-3 py-2 space-y-1.5">
        <div className="flex items-center gap-2 text-[11px] text-primary">
          <span className="size-3 border-2 border-primary/30 border-t-primary rounded-full radius-keep animate-spin shrink-0" />
          Waiting for browser login...
        </div>
        {claudeLoginUri && (
          <a
            href={claudeLoginUri}
            target="_blank"
            rel="noreferrer"
            className="block truncate text-[10px] text-primary/80 hover:text-primary hover:underline"
            title={claudeLoginUri}
          >
            Open login page →
          </a>
        )}
        <Button
          size="sm"
          variant="ghost"
          className="h-6 text-[10px] text-muted-foreground hover:text-destructive"
          onClick={() => void claudeCancelLogin()}
        >
          Cancel
        </Button>
      </div>
    );
  }
  const claudeEmail = claudeProfile?.email ?? claudeAuthStatus?.email ?? null;
  const claudeTier =
    claudeProfile?.subscriptionTier ??
    claudeAuthStatus?.subscriptionType ??
    null;
  if (claudeEmail) {
    return (
      <div className="flex items-center justify-between rounded-md bg-muted/30 px-2.5 py-1.5 text-[11px]">
        <div className="flex items-center gap-1.5 min-w-0">
          <span className="size-1.5 shrink-0 rounded-full radius-keep bg-primary" />
          <span className="truncate text-foreground/80" title={claudeEmail}>
            {claudeEmail}
          </span>
          {claudeTier && (
            <span className="shrink-0 rounded bg-primary/10 px-1 py-px text-[9px] font-semibold uppercase text-primary">
              {claudeTier}
            </span>
          )}
        </div>
        <button
          type="button"
          onClick={() => void claudeLogout()}
          className="ml-2 text-[10px] text-muted-foreground hover:text-destructive"
        >
          Logout
        </button>
      </div>
    );
  }
  return (
    <div className="space-y-1">
      <Button
        size="sm"
        variant="outline"
        className="w-full text-[11px] border-primary/30 text-primary hover:bg-primary/10"
        onClick={() => void claudeLogin()}
      >
        Login with Claude Max
      </Button>
      {claudeLoginError && (
        <p
          className="truncate text-[10px] text-destructive"
          title={claudeLoginError}
        >
          {claudeLoginError}
        </p>
      )}
    </div>
  );
}

function ProviderSection({
  kind,
  form,
  setForm,
  onClear,
}: {
  kind: Kind;
  form: FormState;
  setForm: (update: Partial<FormState>) => void;
  onClear: () => void;
}) {
  const Icon = kind === "claude" ? ClaudeIcon : CodexIcon;
  const hasConfig = form.apiKey.trim().length > 0;
  return (
    <section className="space-y-2 rounded-lg border border-border/30 bg-card/30 px-3 py-3">
      <div className="flex items-center gap-1.5">
        <Icon className="size-3.5 shrink-0" />
        <h3 className="text-[12px] font-semibold capitalize">{kind}</h3>
      </div>

      <div className="space-y-1">
        <span className="text-[10px] uppercase tracking-wide text-muted-foreground/70">
          Subscription
        </span>
        <SubscriptionRow kind={kind} />
      </div>

      <div className="space-y-1">
        <div className="flex items-center justify-between">
          <span className="text-[10px] uppercase tracking-wide text-muted-foreground/70">
            API Key
          </span>
          {hasConfig && (
            <button
              type="button"
              onClick={onClear}
              className="text-[10px] text-muted-foreground/80 hover:text-destructive"
            >
              Clear
            </button>
          )}
        </div>
        <input
          type="password"
          spellCheck={false}
          autoComplete="off"
          placeholder={kind === "claude" ? "sk-ant-..." : "sk-..."}
          value={form.apiKey}
          onChange={(e) => setForm({ apiKey: e.target.value })}
          className="w-full rounded-md border border-border/40 bg-background px-2 py-1.5 font-mono text-[11px] outline-none focus:border-primary/50"
        />
      </div>

      <button
        type="button"
        onClick={() => setForm({ showAdvanced: !form.showAdvanced })}
        className="text-[10px] text-muted-foreground/70 hover:text-foreground"
      >
        {form.showAdvanced ? "▼" : "▶"} Advanced (third-party endpoint)
      </button>

      {form.showAdvanced && (
        <div className="space-y-2 rounded-md border border-dashed border-border/40 px-3 py-2">
          <div>
            <label className="mb-0.5 block text-[10px] text-muted-foreground/70">
              Base URL
            </label>
            <input
              type="text"
              spellCheck={false}
              placeholder={
                kind === "claude"
                  ? "https://api.anthropic.com"
                  : "https://api.openai.com/v1"
              }
              value={form.baseUrl}
              onChange={(e) => setForm({ baseUrl: e.target.value })}
              className="w-full rounded-md border border-border/40 bg-background px-2 py-1.5 font-mono text-[11px] outline-none focus:border-primary/50"
            />
          </div>

          {kind === "codex" && (
            <>
              <div>
                <label className="mb-0.5 block text-[10px] text-muted-foreground/70">
                  Wire API
                </label>
                <select
                  value={form.wireApi}
                  onChange={(e) => setForm({ wireApi: e.target.value })}
                  className="w-full rounded-md border border-border/40 bg-background px-2 py-1.5 text-[11px] outline-none focus:border-primary/50"
                >
                  <option value="chat">chat</option>
                  <option value="responses">responses</option>
                </select>
              </div>
              <div>
                <label className="mb-0.5 block text-[10px] text-muted-foreground/70">
                  Provider name (TOML key)
                </label>
                <input
                  type="text"
                  spellCheck={false}
                  placeholder="dimweave-openrouter"
                  value={form.providerName}
                  onChange={(e) => setForm({ providerName: e.target.value })}
                  className="w-full rounded-md border border-border/40 bg-background px-2 py-1.5 font-mono text-[11px] outline-none focus:border-primary/50"
                />
                <p className="mt-0.5 text-[9px] text-muted-foreground/60">
                  Reserved names (openai/chatgpt/codex/atlas) are rejected by
                  Codex — use a custom prefix.
                </p>
              </div>
            </>
          )}

          {kind === "claude" && (
            <div>
              <label className="mb-0.5 block text-[10px] text-muted-foreground/70">
                Auth header
              </label>
              <select
                value={form.authMode}
                onChange={(e) =>
                  setForm({
                    authMode: e.target.value as "bearer" | "api_key",
                  })
                }
                className="w-full rounded-md border border-border/40 bg-background px-2 py-1.5 text-[11px] outline-none focus:border-primary/50"
              >
                <option value="bearer">ANTHROPIC_AUTH_TOKEN (Bearer)</option>
                <option value="api_key">ANTHROPIC_API_KEY (x-api-key)</option>
              </select>
            </div>
          )}
        </div>
      )}

      {!hasConfig && (
        <p className="text-[10px] text-muted-foreground/60">
          Leave API key empty to keep the subscription path unchanged.
        </p>
      )}
    </section>
  );
}

export function ProviderAuthDialog({
  open,
  onOpenChange,
}: ProviderAuthDialogProps) {
  const configs = useProviderAuthStore((s) => s.configs);
  const fetchAll = useProviderAuthStore((s) => s.fetchAll);
  const save = useProviderAuthStore((s) => s.save);
  const clear = useProviderAuthStore((s) => s.clear);
  const saveError = useProviderAuthStore((s) => s.saveError);

  const [claudeForm, setClaudeForm] = useState<FormState>(() =>
    fromConfig(configs.claude),
  );
  const [codexForm, setCodexForm] = useState<FormState>(() =>
    fromConfig(configs.codex),
  );
  const [saving, setSaving] = useState(false);

  const fetchClaudeAuthStatus = useClaudeAccountStore((s) => s.fetchAuthStatus);
  const fetchClaudeProfile = useClaudeAccountStore((s) => s.fetchProfile);
  const fetchCodexProfile = useCodexAccountStore((s) => s.fetchProfile);

  useEffect(() => {
    if (!open) return;
    void fetchAll();
    void fetchClaudeAuthStatus();
    void fetchClaudeProfile();
    void fetchCodexProfile();
  }, [
    open,
    fetchAll,
    fetchClaudeAuthStatus,
    fetchClaudeProfile,
    fetchCodexProfile,
  ]);

  useEffect(() => {
    if (open) {
      setClaudeForm(fromConfig(configs.claude));
      setCodexForm(fromConfig(configs.codex));
    }
  }, [open, configs.claude, configs.codex]);

  const close = () => onOpenChange(false);

  const handleSave = async () => {
    setSaving(true);
    const results = await Promise.all([
      save(toConfig("claude", claudeForm)),
      save(toConfig("codex", codexForm)),
    ]);
    setSaving(false);
    if (results.every((r) => r)) close();
  };

  const updateClaude = useMemo(
    () => (p: Partial<FormState>) =>
      setClaudeForm((prev) => ({ ...prev, ...p })),
    [],
  );
  const updateCodex = useMemo(
    () => (p: Partial<FormState>) =>
      setCodexForm((prev) => ({ ...prev, ...p })),
    [],
  );

  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div
        className="absolute inset-0 bg-black/40 backdrop-blur-sm"
        onClick={close}
      />
      <div
        role="dialog"
        aria-modal="true"
        className="relative z-10 flex h-[90vh] max-h-160 w-full max-w-lg flex-col overflow-hidden rounded-xl border border-border/50 bg-card shadow-xl"
      >
        <div className="shrink-0 border-b border-border/30 px-4 py-3">
          <h2 className="text-sm font-semibold">Provider Authentication</h2>
          <p className="mt-0.5 text-[10px] text-muted-foreground/70">
            Subscription + optional API key per provider. API key takes priority
            at next launch; leaving key empty falls back to subscription.
          </p>
        </div>
        <div className="min-h-0 flex-1 space-y-3 overflow-y-auto px-4 py-3">
          <ProviderSection
            kind="claude"
            form={claudeForm}
            setForm={updateClaude}
            onClear={() => {
              setClaudeForm({ ...EMPTY_FORM });
              void clear("claude");
            }}
          />
          <ProviderSection
            kind="codex"
            form={codexForm}
            setForm={updateCodex}
            onClear={() => {
              setCodexForm({ ...EMPTY_FORM });
              void clear("codex");
            }}
          />
          {saveError && (
            <p className="text-[11px] text-destructive">{saveError}</p>
          )}
        </div>
        <div className="flex shrink-0 items-center justify-end gap-2 border-t border-border/30 px-4 py-3">
          <Button
            size="sm"
            variant="ghost"
            className="text-[11px] text-muted-foreground"
            onClick={close}
            disabled={saving}
          >
            Cancel
          </Button>
          <Button
            size="sm"
            className={cn("text-[11px]", saving && "opacity-60")}
            onClick={() => void handleSave()}
            disabled={saving}
          >
            {saving ? "Saving…" : "Save & Apply"}
          </Button>
        </div>
      </div>
    </div>
  );
}
