import { useEffect, useMemo, useState } from "react";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import { useClaudeAccountStore } from "@/stores/claude-account-store";
import { CyberSelect } from "@/components/ui/cyber-select";
import {
  useProviderAuthStore,
  type ProviderAuthConfig,
} from "@/stores/provider-auth-store";
import { ClaudeIcon, CodexIcon } from "@/components/AgentStatus/BrandIcons";
import { Button } from "@/components/ui/button";
import { DialogLayout } from "@/components/ui/dialog-layout";
import { cn } from "@/lib/utils";

interface ProviderAuthDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

type Kind = "claude" | "codex";

type ActiveMode = "subscription" | "api_key";

interface FormState {
  activeMode: ActiveMode;
  apiKey: string;
  baseUrl: string;
  wireApi: string; // codex only
  authMode: "bearer" | "api_key"; // claude only
  providerName: string; // codex only
  showAdvanced: boolean;
}

const EMPTY_FORM: FormState = {
  activeMode: "subscription",
  apiKey: "",
  baseUrl: "",
  wireApi: "responses",
  authMode: "api_key",
  providerName: "",
  showAdvanced: false,
};

function fromConfig(cfg: ProviderAuthConfig | undefined): FormState {
  if (!cfg) return { ...EMPTY_FORM };
  const derived: ActiveMode = cfg.activeMode
    ? cfg.activeMode
    : cfg.apiKey
      ? "api_key"
      : "subscription";
  return {
    activeMode: derived,
    apiKey: cfg.apiKey ?? "",
    baseUrl: cfg.baseUrl ?? "",
    // Codex 已弃用 "chat" wire_api；把持久化的旧值映射到 "responses" 让
    // 下拉显示的默认就是新值，用户保存时一并把旧的 "chat" 覆盖掉。
    wireApi: cfg.wireApi && cfg.wireApi !== "chat" ? cfg.wireApi : "responses",
    authMode: (cfg.authMode as "bearer" | "api_key") ?? "api_key",
    providerName: cfg.providerName ?? "",
    showAdvanced: Boolean(cfg.baseUrl || cfg.providerName),
  };
}

/// Build a config from form state. When the active mode is subscription
/// we persist only the mode and drop the API-key side entirely so a
/// previous stale key can't get injected at launch.
function toConfig(kind: Kind, f: FormState): ProviderAuthConfig {
  if (f.activeMode === "subscription") {
    return {
      provider: kind,
      activeMode: "subscription",
      apiKey: null,
      baseUrl: null,
      wireApi: null,
      authMode: null,
      providerName: null,
      updatedAt: 0,
    };
  }
  return {
    provider: kind,
    activeMode: "api_key",
    apiKey: f.apiKey.trim() || null,
    baseUrl: f.baseUrl.trim() || null,
    wireApi: kind === "codex" && f.baseUrl.trim() ? f.wireApi : null,
    authMode: kind === "claude" && f.apiKey.trim() ? f.authMode : null,
    providerName:
      kind === "codex" && f.providerName.trim() ? f.providerName.trim() : null,
    updatedAt: 0,
  };
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

function ModeRadio({
  value,
  label,
  active,
  onSelect,
}: {
  value: ActiveMode;
  label: string;
  active: boolean;
  onSelect: (v: ActiveMode) => void;
}) {
  return (
    <button
      type="button"
      onClick={() => onSelect(value)}
      className={cn(
        "flex-1 rounded-md border px-3 py-1.5 text-[11px] font-medium transition-colors",
        active
          ? "border-primary/60 bg-primary/10 text-primary"
          : "border-border/40 bg-background/50 text-muted-foreground hover:bg-muted",
      )}
    >
      {label}
    </button>
  );
}

function ProviderSection({
  kind,
  form,
  setForm,
}: {
  kind: Kind;
  form: FormState;
  setForm: (update: Partial<FormState>) => void;
}) {
  const Icon = kind === "claude" ? ClaudeIcon : CodexIcon;
  const isApiKey = form.activeMode === "api_key";
  return (
    <section className="space-y-2 rounded-lg border border-border/30 bg-card/30 px-3 py-3">
      <div className="flex items-center gap-1.5">
        <Icon className="size-3.5 shrink-0" />
        <h3 className="text-[12px] font-semibold capitalize">{kind}</h3>
      </div>

      <div className="flex gap-1.5">
        <ModeRadio
          value="subscription"
          label="Subscription"
          active={!isApiKey}
          onSelect={(v) => setForm({ activeMode: v })}
        />
        <ModeRadio
          value="api_key"
          label="API Key"
          active={isApiKey}
          onSelect={(v) => setForm({ activeMode: v })}
        />
      </div>

      {!isApiKey && (
        <div>
          <SubscriptionRow kind={kind} />
        </div>
      )}

      {isApiKey && (
        <div className="space-y-1">
          <span className="text-[10px] uppercase tracking-wide text-muted-foreground/70">
            API Key
          </span>
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
      )}

      {isApiKey && (
        <button
          type="button"
          onClick={() => setForm({ showAdvanced: !form.showAdvanced })}
          className="text-[10px] text-muted-foreground/70 hover:text-foreground"
        >
          {form.showAdvanced ? "▼" : "▶"} Advanced (third-party endpoint)
        </button>
      )}

      {isApiKey && form.showAdvanced && (
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
                <CyberSelect
                  variant="form"
                  value={form.wireApi}
                  options={[{ value: "responses", label: "responses" }]}
                  onChange={(v) => setForm({ wireApi: v })}
                />
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
              <CyberSelect
                variant="form"
                value={form.authMode}
                options={[
                  { value: "bearer", label: "ANTHROPIC_AUTH_TOKEN (Bearer)" },
                  { value: "api_key", label: "ANTHROPIC_API_KEY (x-api-key)" },
                ]}
                onChange={(v) =>
                  setForm({ authMode: v as "bearer" | "api_key" })
                }
              />
            </div>
          )}
        </div>
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

  const claudeMissingKey =
    claudeForm.activeMode === "api_key" && !claudeForm.apiKey.trim();
  const codexMissingKey =
    codexForm.activeMode === "api_key" && !codexForm.apiKey.trim();
  const blocked = claudeMissingKey || codexMissingKey;
  const blockReason = [
    claudeMissingKey ? "Claude" : null,
    codexMissingKey ? "Codex" : null,
  ]
    .filter(Boolean)
    .join(", ");
  return (
    <DialogLayout
      open={open}
      onClose={close}
      width="md"
      header={
        <>
          <h2 className="text-sm font-semibold">Provider Authentication</h2>
          <p className="mt-0.5 text-[10px] text-muted-foreground/70">
            Pick one mode per provider. Save applies on the next launch;
            already-running agents keep their current credentials.
          </p>
        </>
      }
      body={
        <div className="space-y-3 px-4 py-3">
          <ProviderSection
            kind="claude"
            form={claudeForm}
            setForm={updateClaude}
          />
          <ProviderSection
            kind="codex"
            form={codexForm}
            setForm={updateCodex}
          />
          {saveError && (
            <p className="text-[11px] text-destructive">{saveError}</p>
          )}
        </div>
      }
      footer={
        <div className="flex items-center justify-end gap-2">
          {blocked && (
            <span className="mr-auto text-[10px] text-destructive/80">
              {blockReason} API key is empty — fill it in or switch to
              Subscription.
            </span>
          )}
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
            className={cn("text-[11px]", (saving || blocked) && "opacity-60")}
            onClick={() => void handleSave()}
            disabled={saving || blocked}
          >
            {saving ? "Saving…" : "Save & Apply"}
          </Button>
        </div>
      }
    />
  );
}
