import { useCallback, useState } from "react";
import type {
  FeishuProjectRuntimeState,
  FeishuProjectConfigInput,
  McpConnectionStatus,
} from "@/stores/feishu-project-store";
import { ConfigInput } from "./ConfigInput";
import { formatSyncTime } from "./view-model";

interface ConfigCardProps {
  runtimeState: FeishuProjectRuntimeState | null;
  loading: boolean;
  onSave: (config: FeishuProjectConfigInput) => void;
  onSync: () => void;
}

function statusLabel(s: McpConnectionStatus | undefined): string {
  if (!s || s === "disconnected") return "Disconnected";
  if (s === "connecting") return "Connecting...";
  if (s === "connected") return "Connected";
  if (s === "unauthorized") return "Unauthorized";
  return "Error";
}

function statusColor(s: McpConnectionStatus | undefined): string {
  if (s === "connected") return "text-emerald-400";
  if (s === "unauthorized" || s === "error") return "text-rose-400";
  if (s === "connecting") return "text-amber-400";
  return "text-muted-foreground";
}

export function ConfigCard({
  runtimeState,
  loading,
  onSave,
  onSync,
}: ConfigCardProps) {
  const [editing, setEditing] = useState(false);
  const [domain, setDomain] = useState("https://project.feishu.cn");
  const [mcpToken, setMcpToken] = useState("");
  const [workspaceHint, setWorkspaceHint] = useState("");
  const [refreshInterval, setRefreshInterval] = useState("10");

  const enterEdit = useCallback(() => {
    setDomain(runtimeState?.domain ?? "https://project.feishu.cn");
    setWorkspaceHint(runtimeState?.workspaceHint ?? "");
    setRefreshInterval(String(runtimeState?.refreshIntervalMinutes || 10));
    setMcpToken("");
    setEditing(true);
  }, [runtimeState]);

  const handleSave = useCallback(() => {
    onSave({
      enabled: true,
      domain: domain.trim() || "https://project.feishu.cn",
      mcp_user_token: mcpToken.trim(),
      workspace_hint: workspaceHint.trim(),
      refresh_interval_minutes: Math.max(1, Number(refreshInterval) || 10),
    });
    setEditing(false);
    setMcpToken("");
  }, [domain, mcpToken, workspaceHint, refreshInterval, onSave]);

  const handleDisable = useCallback(
    () =>
      onSave({
        enabled: false,
        domain: "https://project.feishu.cn",
        mcp_user_token: "",
        workspace_hint: "",
        refresh_interval_minutes: 10,
      }),
    [onSave],
  );

  const isConfigured = runtimeState?.tokenLabel || runtimeState?.enabled;

  if (!isConfigured && !editing) {
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
        <ConfigInput label="Domain" value={domain} onChange={setDomain} />
        <ConfigInput
          label="MCP User Token"
          value={mcpToken}
          onChange={setMcpToken}
          type="password"
          placeholder={runtimeState?.tokenLabel ?? ""}
        />
        <ConfigInput
          label="Workspace hint"
          value={workspaceHint}
          onChange={setWorkspaceHint}
          placeholder="optional"
        />
        <ConfigInput
          label="Refresh interval (minutes)"
          value={refreshInterval}
          onChange={setRefreshInterval}
        />
        <div className="flex gap-1">
          <button
            className="rounded-md border border-primary/50 px-2 py-0.5 text-[10px] text-primary hover:bg-primary/10 active:bg-primary/20 focus-visible:ring-1 focus-visible:ring-primary/40"
            onClick={handleSave}
            disabled={loading}
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
        <span
          className={`text-[11px] font-medium ${statusColor(runtimeState?.mcpStatus)}`}
        >
          {statusLabel(runtimeState?.mcpStatus)}
        </span>
        <span className="font-mono text-[10px] text-muted-foreground">
          {runtimeState?.tokenLabel ?? "no token"}
        </span>
      </div>
      {runtimeState?.mcpStatus === "connected" && (
        <div className="text-[10px] text-muted-foreground">
          {runtimeState.discoveredToolCount} tools discovered
        </div>
      )}
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
