import { useCallback, useEffect, useState } from "react";
import {
  useTelegramStore,
  type TelegramStateInfo,
} from "@/stores/telegram-store";
import { StatusDot } from "./StatusDot";

function statusLabel(state: TelegramStateInfo | null): string {
  if (!state) return "Loading…";
  if (!state.enabled) return "Disabled";
  if (state.lastError) return `Error: ${state.lastError}`;
  if (state.connected) return "Connected";
  return "Disconnected";
}

function statusDot(
  state: TelegramStateInfo | null,
): "connected" | "disconnected" | "error" {
  if (!state || !state.enabled) return "disconnected";
  if (state.lastError) return "error";
  return state.connected ? "connected" : "disconnected";
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
    if (!token) return; // require explicit token input; masked label cannot reconstruct original
    void saveConfig(token, true, true);
    setShowTokenInput(false);
    setTokenInput("");
  }, [tokenInput, saveConfig]);

  const handleDisable = useCallback(() => {
    void saveConfig("", false, false);
  }, [saveConfig]);

  return (
    <div className="rounded-2xl border border-border/40 bg-card/45 px-4 py-3">
      <div className="flex items-center gap-2">
        <StatusDot status={statusDot(tgState)} />
        <span className="flex-1 text-[13px] font-medium text-card-foreground">
          Telegram
        </span>
        <span className="text-[11px] text-muted-foreground">
          {statusLabel(tgState)}
        </span>
      </div>

      <div className="mt-2 space-y-2 text-[12px]">
        {/* Token config */}
        {tgState?.tokenLabel ? (
          <div className="flex items-center justify-between gap-2">
            <span className="text-muted-foreground">
              Bot: <span className="font-mono">{tgState.tokenLabel}</span>
            </span>
            <div className="flex gap-1">
              <button
                className="rounded-md border border-border/50 px-2 py-0.5 text-[10px] text-foreground hover:border-primary/50"
                onClick={() => setShowTokenInput(!showTokenInput)}
              >
                Change
              </button>
              {tgState.enabled && (
                <button
                  className="rounded-md border border-border/50 px-2 py-0.5 text-[10px] text-rose-400 hover:border-rose-400/50"
                  onClick={handleDisable}
                >
                  Disable
                </button>
              )}
            </div>
          </div>
        ) : (
          <button
            className="rounded-md border border-border/50 px-2 py-1 text-[11px] text-foreground hover:border-primary/50"
            onClick={() => setShowTokenInput(true)}
          >
            Set bot token
          </button>
        )}

        {showTokenInput && (
          <div className="flex gap-1">
            <input
              type="password"
              className="min-w-0 flex-1 rounded-md border border-border/50 bg-background/40 px-2 py-1 text-[11px] text-foreground placeholder:text-muted-foreground/40"
              placeholder="123456:ABC-DEF..."
              value={tokenInput}
              onChange={(e) => setTokenInput(e.target.value)}
            />
            <button
              className="rounded-md border border-primary/50 px-2 py-1 text-[11px] text-primary hover:bg-primary/10"
              onClick={handleSave}
              disabled={loading}
            >
              Save
            </button>
          </div>
        )}

        {/* Pairing */}
        {tgState?.enabled && (
          <div className="space-y-1">
            {tgState.pairedChatLabel ? (
              <div className="flex items-center justify-between gap-2">
                <span className="text-muted-foreground">
                  Paired: {tgState.pairedChatLabel}
                </span>
                <button
                  className="rounded-md border border-border/50 px-2 py-0.5 text-[10px] text-rose-400 hover:border-rose-400/50"
                  onClick={() => void clearPairing()}
                >
                  Unpair
                </button>
              </div>
            ) : tgState.pendingPairCode ? (
              <div className="text-muted-foreground">
                Send to your bot:{" "}
                <code className="font-mono text-foreground">
                  /pair {tgState.pendingPairCode}
                </code>
              </div>
            ) : (
              <button
                className="rounded-md border border-border/50 px-2 py-0.5 text-[10px] text-foreground hover:border-primary/50"
                onClick={() => void generatePairCode()}
              >
                Generate pairing code
              </button>
            )}
          </div>
        )}

        {error && <div className="text-[10px] text-rose-400">{error}</div>}
      </div>
    </div>
  );
}
