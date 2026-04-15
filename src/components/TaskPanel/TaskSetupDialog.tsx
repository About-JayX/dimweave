import { useState, useEffect, useCallback } from "react";
import { GripVertical, Plus, Trash2 } from "lucide-react";
import { DndContext, closestCenter, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy, useSortable, arrayMove } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { AgentDraftConfig } from "@/components/AgentStatus/provider-session-view-model";
import type { Provider } from "@/stores/task-store/types";

export interface AgentDef {
  provider: Provider;
  role: string;
  agentId?: string;
  displayName?: string | null;
}

export interface TaskSetupSubmitPayload {
  agents: AgentDef[];
  claudeConfig: AgentDraftConfig | null;
  codexConfig: AgentDraftConfig | null;
  requestLaunch: boolean;
}

export type TaskSetupMode = "create" | "edit";

interface TaskSetupDialogProps {
  mode?: TaskSetupMode;
  workspace: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSubmit: (payload: TaskSetupSubmitPayload) => void;
  initialAgents?: AgentDef[];
}

const PROVIDERS: Provider[] = ["claude", "codex"];

function AgentConfigForm({ def, onChange, onRemove }: {
  def: AgentDef; onChange: (u: AgentDef) => void; onRemove: () => void;
}) {
  return (
    <div className="space-y-2 p-4">
      <div className="flex items-center gap-2">
        <select value={def.provider} onChange={(e) => onChange({ ...def, provider: e.target.value as Provider })}
          className="rounded-lg border border-border/50 bg-background px-2 py-1 text-xs text-foreground outline-none focus:border-primary/40">
          {PROVIDERS.map((p) => <option key={p} value={p}>{p}</option>)}
        </select>
        <button type="button" onClick={onRemove} className="rounded p-1 text-muted-foreground hover:bg-rose-500/20 hover:text-rose-400">
          <Trash2 className="size-3" />
        </button>
      </div>
      <input type="text" value={def.role} onChange={(e) => onChange({ ...def, role: e.target.value })}
        placeholder="role" className="w-full rounded-lg border border-border/50 bg-background px-2 py-1 text-xs text-foreground outline-none placeholder:text-muted-foreground/40 focus:border-primary/40" />
    </div>
  );
}

function SortableListRow({ id, def, selected, onSelect, onRemove }: {
  id: string; def: AgentDef; selected: boolean; onSelect: () => void; onRemove: () => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({ id });
  return (
    <div ref={setNodeRef} data-draggable-row="true"
      style={{ transform: CSS.Transform.toString(transform), transition }}
      className={`flex items-center gap-1.5 px-3 py-2 cursor-pointer hover:bg-muted/30 ${selected ? "bg-muted/50" : ""}`}
      onClick={onSelect}>
      <button type="button" data-drag-handle="true" {...attributes} {...listeners}
        className="cursor-grab rounded p-0.5 text-muted-foreground/40 hover:text-muted-foreground"
        onClick={(e) => e.stopPropagation()}>
        <GripVertical className="size-3 shrink-0" />
      </button>
      <div className="flex-1 min-w-0 text-xs truncate">
        <span className="font-medium text-foreground">{def.provider}</span>
        {def.role && <span className="ml-1 text-muted-foreground">{def.role}</span>}
      </div>
      <button type="button" onClick={(e) => { e.stopPropagation(); onRemove(); }}
        className="rounded p-0.5 text-muted-foreground/40 hover:bg-rose-500/20 hover:text-rose-400">
        <Trash2 className="size-3 shrink-0" />
      </button>
    </div>
  );
}

export function TaskSetupDialog({
  mode = "create", workspace: _workspace, open, onOpenChange, onSubmit, initialAgents = [],
}: TaskSetupDialogProps) {
  const [agentDefs, setAgentDefs] = useState<AgentDef[]>(initialAgents);
  const [sortIds, setSortIds] = useState<string[]>(() =>
    initialAgents.map((d, i) => d.agentId ?? `new-${i}`));
  const [selectedId, setSelectedId] = useState<string | null>(
    initialAgents.length > 0 ? (initialAgents[0].agentId ?? "new-0") : null);
  const sensors = useSensors(useSensor(PointerSensor));
  const handleClose = useCallback(() => onOpenChange(false), [onOpenChange]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") handleClose(); };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, handleClose]);

  if (!open) return null;

  const selectedIdx = sortIds.indexOf(selectedId ?? "");
  const updateDef = (i: number, u: AgentDef) => setAgentDefs(p => p.map((d, j) => j === i ? u : d));
  const removeDef = (i: number) => {
    const rid = sortIds[i];
    setAgentDefs(p => p.filter((_, j) => j !== i));
    setSortIds(p => p.filter((_, j) => j !== i));
    if (selectedId === rid) setSelectedId(null);
  };
  const addDef = () => {
    const newId = `new-${Date.now()}`;
    setAgentDefs(p => [...p, { provider: "claude", role: "" }]);
    setSortIds(p => [...p, newId]);
    setSelectedId(newId);
  };
  const handleDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return;
    const oi = sortIds.indexOf(active.id as string);
    const ni = sortIds.indexOf(over.id as string);
    setAgentDefs(p => arrayMove(p, oi, ni));
    setSortIds(p => arrayMove(p, oi, ni));
  };
  const validAgents = agentDefs.filter(d => d.role.trim().length > 0);
  const submit = (launch: boolean) => {
    onSubmit({ agents: validAgents, claudeConfig: null, codexConfig: null, requestLaunch: launch });
    onOpenChange(false);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={handleClose} />
      <div role="dialog" aria-modal="true"
        className="relative z-10 flex flex-col w-full max-w-2xl max-h-[90vh] rounded-xl border border-border/50 bg-card shadow-xl">
        <div className="shrink-0 px-4 pt-4 pb-2">
          <h3 className="text-sm font-semibold text-foreground">
            {mode === "edit" ? "Edit Task" : "New Task"}
          </h3>
        </div>
        <div className="min-h-0 flex-1 flex overflow-hidden border-t border-border/30">
          <div data-left-pane="true" className="flex w-52 shrink-0 flex-col border-r border-border/30">
            <div className="flex items-center justify-between px-3 py-2">
              <span className="text-xs font-medium text-muted-foreground">Agents</span>
              <button type="button" onClick={addDef}
                className="inline-flex items-center gap-0.5 rounded-md px-1.5 py-0.5 text-[10px] text-muted-foreground hover:bg-muted hover:text-foreground">
                <Plus className="size-3" />Add
              </button>
            </div>
            <div data-scroll-region="true" className="min-h-0 flex-1 overflow-y-auto">
              <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
                <SortableContext items={sortIds} strategy={verticalListSortingStrategy}>
                  {agentDefs.map((def, i) => (
                    <SortableListRow key={sortIds[i]} id={sortIds[i]} def={def}
                      selected={selectedId === sortIds[i]}
                      onSelect={() => setSelectedId(sortIds[i])}
                      onRemove={() => removeDef(i)} />
                  ))}
                </SortableContext>
              </DndContext>
            </div>
          </div>
          <div data-right-pane="true" className="flex flex-1 flex-col overflow-y-auto">
            {selectedIdx === -1 ? (
              <div data-right-pane-placeholder="true"
                className="flex flex-1 items-center justify-center text-xs text-muted-foreground/60">
                Select an agent to configure
              </div>
            ) : (
              <AgentConfigForm def={agentDefs[selectedIdx]}
                onChange={(u) => updateDef(selectedIdx, u)}
                onRemove={() => removeDef(selectedIdx)} />
            )}
          </div>
        </div>
        <div data-dialog-footer="true" className="shrink-0 flex items-center justify-end gap-2 border-t border-border/30 px-4 py-3">
          <button type="button" onClick={handleClose}
            className="rounded-lg px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground">
            Cancel
          </button>
          {mode === "edit" ? (
            <button type="button" onClick={() => submit(false)}
              className="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90">
              Save
            </button>
          ) : (
            <>
              <button type="button" onClick={() => submit(true)} disabled={validAgents.length === 0}
                className="rounded-lg border border-primary/30 bg-primary/10 px-3 py-1.5 text-xs font-medium text-primary transition-colors hover:bg-primary/20 disabled:opacity-40">
                Create &amp; Connect
              </button>
              <button type="button" onClick={() => submit(false)}
                className="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90">
                Create
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
