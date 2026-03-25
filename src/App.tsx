import { useBridgeStore } from "./stores/bridge-store";
import { AgentStatusPanel } from "./components/AgentStatus";
import { MessagePanel } from "./components/MessagePanel";
import { ReplyInput } from "./components/ReplyInput";

export default function App() {
  const connected = useBridgeStore((s) => s.connected);
  const messages = useBridgeStore((s) => s.messages);
  const agents = useBridgeStore((s) => s.agents);
  const daemonStatus = useBridgeStore((s) => s.daemonStatus);

  return (
    <div
      className="flex h-screen text-foreground font-sans"
      style={{
        background:
          "linear-gradient(180deg, #0a0a0a 0%, #0d0d12 50%, #0a0a0a 100%)",
      }}
    >
      <div className="w-70 shrink-0 border-r border-border/50 flex flex-col relative noise-bg bg-linear-to-b from-[#0e0e14] to-[#0a0a0a]">
        <div className="flex items-baseline gap-2 p-4 border-b border-border/50 relative">
          <h2 className="m-0 text-base font-bold text-gradient-cyber relative z-10">
            AgentBridge
          </h2>
          <span className="text-xs text-muted-foreground/70 relative z-10">
            v0.1.0
          </span>
          <div className="absolute bottom-0 left-4 right-4 h-px bg-linear-to-r from-transparent via-claude/30 to-transparent" />
        </div>
        <AgentStatusPanel
          agents={agents}
          daemonStatus={daemonStatus}
          connected={connected}
        />
      </div>

      <div className="flex-1 flex flex-col min-w-0 animate-in fade-in duration-500">
        <MessagePanel messages={messages} />
        <ReplyInput connected={connected} />
      </div>
    </div>
  );
}
