import { useCallback, useState } from "react";
import type {
  FeishuProjectRuntimeState,
  FeishuProjectConfigInput,
  McpConnectionStatus,
  FeishuSyncMode,
} from "@/stores/feishu-project-store";
import { StatusDot } from "@/components/AgentStatus/StatusDot";
import {
  ActionMenu,
  type ActionMenuItem,
} from "@/components/AgentStatus/ActionMenu";
import { ConfigInput } from "./ConfigInput";
import { formatSyncTime } from "./view-model";

interface ConfigCardProps {
  runtimeState: FeishuProjectRuntimeState | null;
  loading: boolean;
  onSave: (config: FeishuProjectConfigInput) => void;
  onSync: () => void;
}

function dotStatus(
  s: McpConnectionStatus | undefined,
): "connected" | "disconnected" | "error" {
  if (s === "connected") return "connected";
  if (s === "unauthorized" || s === "error") return "error";
  return "disconnected";
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
  const [syncMode, setSyncMode] = useState<FeishuSyncMode>("todo");

  const enterEdit = useCallback(() => {
    setDomain(runtimeState?.domain ?? "https://project.feishu.cn");
    setWorkspaceHint(runtimeState?.workspaceHint ?? "");
    setRefreshInterval(String(runtimeState?.refreshIntervalMinutes || 10));
    setSyncMode(runtimeState?.syncMode ?? "todo");
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
      sync_mode: syncMode,
    });
    setEditing(false);
    setMcpToken("");
  }, [domain, mcpToken, workspaceHint, refreshInterval, syncMode, onSave]);

  const handleDisable = useCallback(
    () =>
      onSave({
        enabled: false,
        domain: "https://project.feishu.cn",
        mcp_user_token: "",
        workspace_hint: "",
        refresh_interval_minutes: 10,
        sync_mode: "todo",
      }),
    [onSave],
  );

  const isConfigured = runtimeState?.tokenLabel || runtimeState?.enabled;

  const menuItems: ActionMenuItem[] = [];
  if (isConfigured && !editing) {
    menuItems.push({
      label: loading ? "Syncing..." : "Sync now",
      onClick: () => onSync(),
    });
    menuItems.push({ label: "Edit", onClick: enterEdit });
    menuItems.push({ label: "Disable", danger: true, onClick: handleDisable });
  }
  if (!isConfigured && !editing) {
    menuItems.push({ label: "Configure", onClick: enterEdit });
  }

  if (!isConfigured && !editing) {
    return (
      <div className="flex items-center gap-2">
        <StatusDot status="disconnected" />
        <span className="flex-1 text-[13px] font-medium text-card-foreground">
          Feishu Project
        </span>
        <ActionMenu items={menuItems} />
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
        <div className="flex items-center gap-2">
          <span className="w-24 text-[10px] text-muted-foreground">
            Sync mode
          </span>
          <select
            className="flex-1 rounded border border-border/50 bg-background px-1.5 py-0.5 text-[11px] text-foreground"
            value={syncMode}
            onChange={(e) => setSyncMode(e.target.value as FeishuSyncMode)}
          >
            <option value="todo">我的待办</option>
            <option value="issues">缺陷管理</option>
          </select>
        </div>
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
    <div className="flex items-center gap-2">
      <StatusDot status={dotStatus(runtimeState?.mcpStatus)} />
      <span className="flex-1 text-[13px] font-medium text-card-foreground">
        {runtimeState?.projectName ||
          runtimeState?.workspaceHint ||
          "Feishu Project"}
      </span>
      <span className="text-[10px] text-muted-foreground">
        {formatSyncTime(runtimeState?.lastSyncAt)}
      </span>
      <ActionMenu items={menuItems} />
    </div>
  );
}
