import { useState } from "react";
import { MessagePanel } from "./components/MessagePanel";
import { ReplyInput } from "./components/ReplyInput";
import { ShellContextBar } from "./components/ShellContextBar";
import { TaskContextPopover } from "./components/TaskContextPopover";
import {
  closeShellSidebar,
  createShellLayoutState,
  toggleShellNavItem,
} from "./components/shell-layout-state";
import { useTaskStore } from "./stores/task-store";
import { selectActiveTask } from "./stores/task-store/selectors";

export default function App() {
  const [shellLayout, setShellLayout] = useState(createShellLayoutState);
  const activeTask = useTaskStore(selectActiveTask);

  return (
    <div
      className="flex h-screen flex-col overflow-hidden font-sans text-foreground"
      style={{
        background:
          "radial-gradient(circle at top, rgba(34,197,94,0.08), transparent 28%), linear-gradient(180deg, #090a0d 0%, #0c0d12 48%, #08090c 100%)",
      }}
    >
      <div className="flex flex-1 min-h-0">
        <ShellContextBar
          activeItem={shellLayout.activeItem}
          onToggle={(item) =>
            setShellLayout((current) => toggleShellNavItem(current, item))
          }
        />
        <TaskContextPopover
          activePane={shellLayout.sidebarPane}
          onClose={() =>
            setShellLayout((current) => closeShellSidebar(current))
          }
          task={activeTask}
        />
        <main className="flex min-w-0 flex-1 flex-col animate-in fade-in duration-500">
          <MessagePanel surfaceMode={shellLayout.mainSurface} />
          <ReplyInput />
        </main>
      </div>
    </div>
  );
}
