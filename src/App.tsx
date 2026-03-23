import { useState } from "react";
import { useBridgeStore } from "./stores/bridge-store";
import { AgentStatusPanel } from "./components/AgentStatus";
import { MessagePanel } from "./components/MessagePanel";
import { ReplyInput } from "./components/ReplyInput";

export default function App() {
  const connected = useBridgeStore((s) => s.connected);
  const messages = useBridgeStore((s) => s.messages);
  const agents = useBridgeStore((s) => s.agents);
  const daemonStatus = useBridgeStore((s) => s.daemonStatus);
  const [activeTab, setActiveTab] = useState<"messages" | "terminal" | "logs">(
    "messages",
  );

  return (
    <div className="flex h-screen bg-background text-foreground font-sans">
      <div className="w-70 shrink-0 border-r border-border flex flex-col">
        <div className="flex items-baseline gap-2 p-4 border-b border-border">
          <h2 className="m-0 text-base font-bold text-foreground">
            AgentBridge
          </h2>
          <span className="text-xs text-muted-foreground">v0.1.0</span>
        </div>
        <AgentStatusPanel
          agents={agents}
          daemonStatus={daemonStatus}
          connected={connected}
        />
      </div>

      <div className="flex-1 flex flex-col min-w-0">
        <MessagePanel messages={messages} onTabChange={setActiveTab} />
        {activeTab === "messages" && <ReplyInput connected={connected} />}
      </div>
    </div>
  );
}
