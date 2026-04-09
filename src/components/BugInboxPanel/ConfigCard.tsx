import { useCallback, useState } from "react";
import type {
  FeishuProjectRuntimeState,
  FeishuProjectConfigInput,
} from "@/stores/feishu-project-store";
import { ConfigInput } from "./ConfigInput";
import { formatSyncTime } from "./view-model";

interface ConfigCardProps {
  runtimeState: FeishuProjectRuntimeState | null;
  loading: boolean;
  onSave: (config: FeishuProjectConfigInput) => void;
  onSync: () => void;
}

export function ConfigCard({
  runtimeState,
  loading,
  onSave,
  onSync,
}: ConfigCardProps) {
  const [editing, setEditing] = useState(false);
  const [projectKey, setProjectKey] = useState("");
  const [pluginToken, setPluginToken] = useState("");
  const [userKey, setUserKey] = useState("");
  const [webhookToken, setWebhookToken] = useState("");
  const [pollInterval, setPollInterval] = useState("10");
  const [webhookBaseUrl, setWebhookBaseUrl] = useState("");

  const enterEdit = useCallback(() => {
    setProjectKey(runtimeState?.projectKey ?? "");
    setUserKey(runtimeState?.userKey ?? "");
    setPollInterval(String(runtimeState?.pollIntervalMinutes || 10));
    setWebhookBaseUrl(runtimeState?.publicWebhookBaseUrl ?? "");
    setPluginToken("");
    setWebhookToken("");
    setEditing(true);
  }, [runtimeState]);

  const handleSave = useCallback(() => {
    const interval = Math.max(1, Number(pollInterval) || 10);
    const baseUrl = webhookBaseUrl.trim() || null;
    onSave({
      enabled: true,
      project_key: projectKey.trim(),
      plugin_token: pluginToken.trim(),
      user_key: userKey.trim(),
      webhook_token: webhookToken.trim(),
      poll_interval_minutes: interval,
      public_webhook_base_url: baseUrl,
    });
    setEditing(false);
    setPluginToken("");
    setWebhookToken("");
  }, [
    projectKey,
    pluginToken,
    userKey,
    webhookToken,
    pollInterval,
    webhookBaseUrl,
    onSave,
  ]);

  const handleDisable = useCallback(
    () =>
      onSave({
        enabled: false,
        project_key: "",
        plugin_token: "",
        user_key: "",
        webhook_token: "",
        poll_interval_minutes: 10,
      }),
    [onSave],
  );

  if (!runtimeState?.projectKey && !editing) {
    return (
      <div className="rounded-xl border border-border/40 bg-card/45 px-3 py-2.5">
        <div className="flex items-center justify-between">
          <span className="text-[12px] text-muted-foreground">
            Not configured
          </span>
          <button
            className="rounded-md border border-border/50 px-2 py-0.5 text-[10px] text-foreground hover:border-primary/50 active:bg-primary/10 focus-visible:ring-1 focus-visible:ring-primary/40"
            onClick={enterEdit}
          >
            Configure
          </button>
        </div>
      </div>
    );
  }

  if (editing) {
    return (
      <div className="space-y-2 rounded-xl border border-border/40 bg-card/45 px-3 py-2.5">
        <ConfigInput
          label="Project key"
          value={projectKey}
          onChange={setProjectKey}
        />
        <ConfigInput
          label="Plugin token"
          value={pluginToken}
          onChange={setPluginToken}
          type="password"
        />
        <ConfigInput label="User key" value={userKey} onChange={setUserKey} />
        <ConfigInput
          label="Webhook token"
          value={webhookToken}
          onChange={setWebhookToken}
          type="password"
        />
        <ConfigInput
          label="Poll interval (minutes)"
          value={pollInterval}
          onChange={setPollInterval}
        />
        <ConfigInput
          label="Public webhook base URL"
          value={webhookBaseUrl}
          onChange={setWebhookBaseUrl}
          placeholder="https://abc.ngrok.app"
        />
        <div className="flex gap-1">
          <button
            className="rounded-md border border-primary/50 px-2 py-0.5 text-[10px] text-primary hover:bg-primary/10 active:bg-primary/20 focus-visible:ring-1 focus-visible:ring-primary/40"
            onClick={handleSave}
            disabled={loading || !projectKey.trim() || !pluginToken.trim()}
          >
            Save
          </button>
          <button
            className="rounded-md border border-border/50 px-2 py-0.5 text-[10px] text-foreground hover:border-border active:bg-muted/50 focus-visible:ring-1 focus-visible:ring-primary/40"
            onClick={() => setEditing(false)}
          >
            Cancel
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-1 rounded-xl border border-border/40 bg-card/45 px-3 py-2.5">
      <div className="flex items-center justify-between">
        <span className="text-[12px] font-medium text-card-foreground">
          {runtimeState?.projectKey}
        </span>
        <span className="font-mono text-[10px] text-muted-foreground">
          {runtimeState?.tokenLabel ?? "no token"}
        </span>
      </div>
      <div className="flex items-center gap-2 text-[10px] text-muted-foreground">
        <span>Synced: {formatSyncTime(runtimeState?.lastSyncAt)}</span>
        {runtimeState?.lastError && (
          <span className="text-rose-400 truncate max-w-[160px]">
            {runtimeState.lastError}
          </span>
        )}
      </div>
      <div className="flex gap-1 pt-0.5">
        <button
          className="rounded-md border border-border/50 px-2 py-0.5 text-[10px] text-foreground hover:border-primary/50 active:bg-primary/10 focus-visible:ring-1 focus-visible:ring-primary/40"
          onClick={onSync}
          disabled={loading}
        >
          {loading ? "Syncing..." : "Sync now"}
        </button>
        <button
          className="rounded-md border border-border/50 px-2 py-0.5 text-[10px] text-foreground hover:border-border active:bg-muted/50 focus-visible:ring-1 focus-visible:ring-primary/40"
          onClick={enterEdit}
        >
          Edit
        </button>
        <button
          className="rounded-md border border-border/50 px-2 py-0.5 text-[10px] text-rose-400 hover:border-rose-400/50 active:bg-rose-400/10 focus-visible:ring-1 focus-visible:ring-rose-400/40"
          onClick={handleDisable}
        >
          Disable
        </button>
      </div>
    </div>
  );
}
