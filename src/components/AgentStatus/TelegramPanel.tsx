import { useCallback, useEffect, useState } from "react";
import { useTelegramStore } from "@/stores/telegram-store";
import type { TelegramStateInfo } from "@/stores/telegram-store";
import { StatusDot } from "./StatusDot";
import { ActionMenu, type ActionMenuItem } from "./ActionMenu";

function statusDot(
  state: TelegramStateInfo | null,
): "connected" | "disconnected" | "error" {
  if (!state || !state.enabled) return "disconnected";
  if (state.lastError) return "error";
  return state.connected ? "connected" : "disconnected";
}

function Skeleton({ className = "" }: { className?: string }) {
  return <div className={`animate-pulse rounded-md bg-muted/40 ${className}`} />;
}

function PairCodeRow({ code, onRefresh }: { code?: string | null; onRefresh: () => void }) {
  useEffect(() => {
    if (!code) onRefresh();
  }, []); // auto-generate on mount if no code

  if (!code) return null;

  return (
    <div className="flex items-center gap-2">
      <span className="text-[11px] text-muted-foreground">
        Send to bot:{" "}
        <code className="font-mono text-foreground">/pair {code}</code>
      </span>
      <button
        className="shrink-0 rounded-md p-0.5 text-muted-foreground transition-colors hover:bg-muted/40 hover:text-foreground"
        onClick={onRefresh}
        title="Refresh code"
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
          <path d="M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8" />
          <path d="M21 3v5h-5" />
        </svg>
      </button>
    </div>
  );
}

export function TelegramPanel() {
  const tgState = useTelegramStore((s) => s.state);
  const loading = useTelegramStore((s) => s.loading);
  const error = useTelegramStore((s) => s.error);
  const fetchState = useTelegramStore((s) => s.fetchState);
  const saveConfig = useTelegramStore((s) => s.saveConfig);
  const generatePairCode = useTelegramStore((s) => s.generatePairCode);
  const clearPairing = useTelegramStore((s) => s.clearPairing);

  const [tokenInput, setTokenInput] = useState("");
  const [showTokenInput, setShowTokenInput] = useState(false);

  useEffect(() => {
    void fetchState();
  }, [fetchState]);

  const handleSave = useCallback(() => {
    const token = tokenInput.trim();
    if (!token) return;
    void saveConfig(token, true, true);
    setShowTokenInput(false);
    setTokenInput("");
  }, [tokenInput, saveConfig]);

  if (!tgState || loading) {
    return (
      <div className="rounded-2xl border border-border/50 bg-card/50 px-4 py-3">
        <div className="flex items-center gap-2">
          <Skeleton className="h-2.5 w-2.5 rounded-full" />
          <Skeleton className="h-4 w-16" />
        </div>
        <div className="mt-2 space-y-2">
          <Skeleton className="h-7 w-full" />
          <Skeleton className="h-4 w-48" />
        </div>
      </div>
    );
  }

  const menuItems: ActionMenuItem[] = [];
  if (tgState.tokenLabel) {
    menuItems.push({ label: "Change token", onClick: () => setShowTokenInput(!showTokenInput) });
  }
  if (tgState.enabled) {
    menuItems.push({ label: "Disable", danger: true, onClick: () => void saveConfig("", false, false) });
  }

  const botDisplay = tgState.botUsername ? `@${tgState.botUsername}` : tgState.tokenLabel;

  return (
    <div className="rounded-2xl border border-border/50 bg-card/50 px-4 py-3">
      <div className="flex items-center gap-2">
        <StatusDot status={statusDot(tgState)} />
        <span className="flex-1 text-[13px] font-medium text-card-foreground">Telegram</span>
        <ActionMenu items={menuItems} />
      </div>

      <div className="mt-2 space-y-2 text-[12px]">
        {tgState.lastError && (
          <div className="rounded-md border border-rose-500/30 bg-rose-500/8 px-2.5 py-1.5 text-[11px] text-rose-300">
            {tgState.lastError}
          </div>
        )}

        {botDisplay && (
          <div className="text-muted-foreground">
            Bot: <span className="font-mono">{botDisplay}</span>
          </div>
        )}

        {(showTokenInput || !tgState.tokenLabel) && (
          <div className="flex gap-1">
            <input
              type="password"
              className="min-w-0 flex-1 rounded-md border border-border/50 bg-background/40 px-2 py-1 text-[11px] text-foreground placeholder:text-muted-foreground/40 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary/50"
              placeholder="123456:ABC-DEF..."
              value={tokenInput}
              onChange={(e) => setTokenInput(e.target.value)}
            />
            <button
              className="rounded-md border border-primary/50 px-2 py-1 text-[11px] text-primary transition-colors hover:bg-primary/10 active:bg-primary/15"
              onClick={handleSave}
              disabled={loading}
            >
              Save
            </button>
          </div>
        )}

        {tgState.enabled && tgState.pairedChatLabel && (
          <div className="flex items-center justify-between gap-2">
            <span className="text-muted-foreground">Paired: {tgState.pairedChatLabel}</span>
            <button
              className="rounded-md border border-border/60 px-2 py-0.5 text-[11px] text-rose-400 transition-colors hover:border-rose-400/50 hover:bg-rose-400/10 active:bg-rose-400/15"
              onClick={() => void clearPairing()}
            >
              Unpair
            </button>
          </div>
        )}

        {tgState.enabled && !tgState.pairedChatLabel && (
          <PairCodeRow code={tgState.pendingPairCode} onRefresh={() => void generatePairCode()} />
        )}

        {error && <div className="text-[11px] text-rose-400">{error}</div>}
      </div>
    </div>
  );
}
