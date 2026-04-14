import { GripVertical, Pencil, Plus, Trash2 } from "lucide-react";
import { useCallback, useRef, useState } from "react";
import { useTaskStore } from "@/stores/task-store";
import {
  selectActiveTask,
  selectActiveTaskAgents,
} from "@/stores/task-store/selectors";
import type { TaskAgentInfo } from "@/stores/task-store/types";
import { TaskAgentEditor, type AgentEditorPayload } from "./TaskAgentEditor";

const PROVIDER_STYLES: Record<string, string> = {
  claude: "border-claude/30 bg-claude/8 text-claude/80",
  codex: "border-codex/30 bg-codex/8 text-codex/80",
};

function AgentRow({
  agent,
  onEdit,
  onRemove,
  onDragStart,
  onDragOver,
  onDrop,
  onDragEnd,
  isDragOver,
}: {
  agent: TaskAgentInfo;
  onEdit: () => void;
  onRemove: () => void;
  onDragStart: (e: React.DragEvent) => void;
  onDragOver: (e: React.DragEvent) => void;
  onDrop: (e: React.DragEvent) => void;
  onDragEnd: () => void;
  isDragOver: boolean;
}) {
  const providerStyle = PROVIDER_STYLES[agent.provider] ?? "border-border/40 bg-muted/20 text-muted-foreground";
  return (
    <div
      draggable
      onDragStart={onDragStart}
      onDragOver={onDragOver}
      onDrop={onDrop}
      onDragEnd={onDragEnd}
      className={`group flex cursor-grab items-center gap-2 rounded-lg px-2 py-1.5 transition-colors active:cursor-grabbing hover:bg-muted/30 ${isDragOver ? "border-t-2 border-primary/50" : ""}`}
      data-testid="agent-row"
    >
      <GripVertical className="size-3 shrink-0 text-muted-foreground/40" />
      <span className={`inline-flex shrink-0 rounded-full border px-2 py-0.5 text-[10px] font-medium ${providerStyle}`}>
        {agent.provider}
      </span>
      <span className="min-w-0 flex-1 truncate text-xs text-foreground">
        {agent.displayName ?? agent.role}
      </span>
      <span className="shrink-0 text-[10px] text-muted-foreground/60">
        {agent.displayName ? agent.role : ""}
      </span>
      <div className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
        <button type="button" onClick={onEdit} className="rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground" title="Edit">
          <Pencil className="size-3" />
        </button>
        <button type="button" onClick={onRemove} className="rounded p-0.5 text-muted-foreground hover:bg-rose-500/20 hover:text-rose-400" title="Remove">
          <Trash2 className="size-3" />
        </button>
      </div>
    </div>
  );
}

export function TaskAgentList() {
  const task = useTaskStore(selectActiveTask);
  const agents = useTaskStore(selectActiveTaskAgents);
  const addTaskAgent = useTaskStore((s) => s.addTaskAgent);
  const removeTaskAgent = useTaskStore((s) => s.removeTaskAgent);
  const updateTaskAgent = useTaskStore((s) => s.updateTaskAgent);
  const reorderTaskAgents = useTaskStore((s) => s.reorderTaskAgents);

  const [editorOpen, setEditorOpen] = useState(false);
  const [editingAgent, setEditingAgent] = useState<TaskAgentInfo | null>(null);
  const dragIndexRef = useRef<number | null>(null);
  const [dragOverIndex, setDragOverIndex] = useState<number | null>(null);

  const handleAdd = useCallback(() => {
    setEditingAgent(null);
    setEditorOpen(true);
  }, []);

  const handleEdit = useCallback((agent: TaskAgentInfo) => {
    setEditingAgent(agent);
    setEditorOpen(true);
  }, []);

  const handleRemove = useCallback(
    (agentId: string) => void removeTaskAgent(agentId),
    [removeTaskAgent],
  );

  const handleDragStart = useCallback(
    (index: number, e: React.DragEvent) => {
      dragIndexRef.current = index;
      e.dataTransfer.effectAllowed = "move";
    },
    [],
  );

  const handleDragOver = useCallback(
    (index: number, e: React.DragEvent) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";
      setDragOverIndex(index);
    },
    [],
  );

  const handleDrop = useCallback(
    (targetIndex: number) => {
      const sourceIndex = dragIndexRef.current;
      if (sourceIndex === null || sourceIndex === targetIndex || !task) return;
      const ids = agents.map((a) => a.agentId);
      const [moved] = ids.splice(sourceIndex, 1);
      ids.splice(targetIndex, 0, moved);
      void reorderTaskAgents(task.taskId, ids);
      dragIndexRef.current = null;
      setDragOverIndex(null);
    },
    [agents, reorderTaskAgents, task],
  );

  const handleDragEnd = useCallback(() => {
    dragIndexRef.current = null;
    setDragOverIndex(null);
  }, []);

  const handleEditorSubmit = useCallback(
    (payload: AgentEditorPayload) => {
      if (editingAgent) {
        void updateTaskAgent(
          editingAgent.agentId,
          payload.provider,
          payload.role,
          payload.displayName,
        );
      } else if (task) {
        void addTaskAgent(task.taskId, payload.provider, payload.role, payload.displayName);
      }
      setEditorOpen(false);
    },
    [addTaskAgent, editingAgent, task, updateTaskAgent],
  );

  if (!task) return null;

  return (
    <div className="rounded-xl border border-border/50 bg-card/50 px-3 py-2">
      <div className="mb-1.5 flex items-center justify-between">
        <span className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground/60">
          Agents
        </span>
        <button
          type="button"
          onClick={handleAdd}
          className="inline-flex items-center gap-0.5 rounded-md px-1.5 py-0.5 text-[10px] text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          data-testid="add-agent-btn"
        >
          <Plus className="size-3" />
          Add
        </button>
      </div>
      {agents.length === 0 ? (
        <div className="py-2 text-center text-xs text-muted-foreground/50">
          No agents configured
        </div>
      ) : (
        <div className="space-y-0.5">
          {agents.map((agent, i) => (
            <AgentRow
              key={agent.agentId}
              agent={agent}
              onEdit={() => handleEdit(agent)}
              onRemove={() => handleRemove(agent.agentId)}
              onDragStart={(e) => handleDragStart(i, e)}
              onDragOver={(e) => handleDragOver(i, e)}
              onDrop={() => handleDrop(i)}
              onDragEnd={handleDragEnd}
              isDragOver={dragOverIndex === i}
            />
          ))}
        </div>
      )}
      {editorOpen && (
        <TaskAgentEditor
          agent={editingAgent}
          onSubmit={handleEditorSubmit}
          onCancel={() => setEditorOpen(false)}
        />
      )}
    </div>
  );
}
