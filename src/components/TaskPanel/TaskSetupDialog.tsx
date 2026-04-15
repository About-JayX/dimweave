import { useState, useEffect, useCallback } from "react";
import { GripVertical, Plus, Trash2 } from "lucide-react";
import { DndContext, closestCenter, PointerSensor, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy, useSortable, arrayMove } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import {
  PROVIDER_CAPS, buildProviderHistoryOptions, findProviderHistoryEntry, resolveProviderHistoryAction,
  historyActionToSelectValue, buildDraftConfigFromDef, type AgentDraftConfig, type ProviderHistoryAction,
} from "@/components/AgentStatus/provider-session-view-model";
import type { ProviderHistoryInfo } from "@/stores/task-store/types";
import { ClaudeIcon, CodexIcon } from "@/components/AgentStatus/BrandIcons";
import { CyberSelect } from "@/components/ui/cyber-select";
import { AGENT_ROLE_OPTIONS } from "@/components/AgentStatus/RoleSelect";
import type { Provider } from "@/stores/task-store/types";

export interface AgentDef {
  provider: Provider; role: string; model?: string; effort?: string;
  historyAction?: ProviderHistoryAction; agentId?: string; displayName?: string | null;
}
export interface TaskSetupSubmitPayload {
  agents: AgentDef[]; claudeConfig: AgentDraftConfig | null; codexConfig: AgentDraftConfig | null; requestLaunch: boolean;
}
export type TaskSetupMode = "create" | "edit";
export interface CodexModelInfo { slug: string; displayName: string; reasoningLevels?: { effort: string }[] }

interface TaskSetupDialogProps {
  mode?: TaskSetupMode; workspace: string; open: boolean;
  onOpenChange: (open: boolean) => void; onSubmit: (payload: TaskSetupSubmitPayload) => void;
  initialAgents?: AgentDef[]; providerHistory?: ProviderHistoryInfo[];
  codexModels?: CodexModelInfo[];
}

const PROVIDERS = [{ value: "claude", label: "Claude" }, { value: "codex", label: "Codex" }];
const DEFAULT_FIRST: AgentDef = { provider: "claude", role: "" };

function ProviderIcon({ provider }: { provider: Provider }) {
  return (<span data-provider-icon="true" className="shrink-0">
    {provider === "claude" ? <ClaudeIcon className="size-3.5" /> : <CodexIcon className="size-3.5" />}
  </span>);
}

function AgentConfigForm({ def, onChange, onRemove, locked, providerHistory = [], codexModels }: { def: AgentDef; onChange: (u: AgentDef) => void; onRemove: () => void; locked?: boolean; providerHistory?: ProviderHistoryInfo[]; codexModels?: CodexModelInfo[] }) {
  const caps = PROVIDER_CAPS[def.provider], eDis = caps.effortRequiresModel && !(def.model ?? "").trim();
  const isCodex = def.provider === "codex";
  const mOpts = isCodex && codexModels ? codexModels.map(m => ({ value: m.slug, label: m.displayName })) : caps.modelOptions;
  const selCodexModel = isCodex && codexModels ? codexModels.find(m => m.slug === def.model) : undefined;
  const eOpts = isCodex && selCodexModel?.reasoningLevels ? selCodexModel.reasoningLevels.map(r => ({ value: r.effort, label: r.effort })) : (isCodex ? [] : caps.effortOptions);
  const modelPlaceholder = isCodex && codexModels?.length === 0 ? "Loading models…" : "Select model";
  const setP = (p: string) => onChange({ ...def, provider: p as Provider, model: "", effort: "", historyAction: { kind: "new" } });
  const histOpts = buildProviderHistoryOptions(def.provider, providerHistory);
  const histVal = historyActionToSelectValue(def.historyAction, providerHistory);
  const onHist = (v: string) => onChange({ ...def, historyAction: resolveProviderHistoryAction(findProviderHistoryEntry(def.provider, providerHistory, v)) });
  const effortOpts = eOpts.some(o => o.value === "") ? eOpts : [{ value: "", label: "Default" }, ...eOpts];
  return (
    <div data-provider-card="true" className="m-3 rounded-xl border border-border/40 bg-card/60 shadow-sm">
      <div className="flex items-center gap-2 border-b border-border/30 px-4 py-2.5">
        <ProviderIcon provider={def.provider} />
        <span className="text-xs font-semibold text-foreground capitalize">{def.provider}</span>
        <div className="flex-1" />{!locked && <button type="button" onClick={onRemove} className="rounded p-1 text-muted-foreground hover:bg-rose-500/20 hover:text-rose-400"><Trash2 className="size-3" /></button>}
      </div>
      <div className="space-y-2.5 px-4 py-3">
        <div className="flex items-center justify-between" data-provider-select="true">
          <span className="text-[10px] text-muted-foreground">Provider</span>
          <CyberSelect value={def.provider} options={PROVIDERS} onChange={setP} />
        </div>
        <div className="flex items-center justify-between" data-role-select="true">
          <span className="text-[10px] text-muted-foreground">Role</span>
          <CyberSelect value={def.role} options={AGENT_ROLE_OPTIONS} onChange={(v) => onChange({ ...def, role: v })} placeholder="Select role" />
        </div>
        {caps.supportsModel && <div className="flex items-center justify-between" data-model-select="true">
          <span className="text-[10px] text-muted-foreground">Model</span>
          <CyberSelect value={def.model ?? "\x00"} options={mOpts} onChange={(v) => onChange({ ...def, model: v })} placeholder={modelPlaceholder} />
        </div>}
        {caps.supportsEffort && <div className="flex items-center justify-between" data-effort-select="true">
          <span className="text-[10px] text-muted-foreground">{caps.effortLabel}</span>
          <CyberSelect value={def.effort ?? ""} options={effortOpts} onChange={(v) => onChange({ ...def, effort: v })} disabled={eDis} />
        </div>}
        {caps.supportsSessionResume && <div className="flex items-center justify-between" data-history-select="true">
          <span className="text-[10px] text-muted-foreground">Session</span>
          <CyberSelect compact variant="history" value={histVal} options={histOpts} onChange={onHist} placeholder="New session" />
        </div>}
      </div>
    </div>);
}

function SortableListRow({ id, def, selected, onSelect, onRemove, locked }: { id: string; def: AgentDef; selected: boolean; onSelect: () => void; onRemove: () => void; locked?: boolean }) {
  const s = useSortable({ id });
  return (
    <div ref={s.setNodeRef} data-draggable-row="true" {...(locked ? { "data-locked-row": "true" } : {})} onClick={onSelect}
      style={{ transform: CSS.Transform.toString(s.transform), transition: s.transition }}
      className={`flex items-center gap-1.5 px-3 py-2 cursor-pointer hover:bg-muted/30 ${selected ? "bg-muted/50" : ""}`}>
      <button type="button" data-drag-handle="true" {...s.attributes} {...s.listeners}
        className="cursor-grab rounded p-0.5 text-muted-foreground/40 hover:text-muted-foreground"
        onClick={(e) => e.stopPropagation()}><GripVertical className="size-3 shrink-0" /></button>
      <ProviderIcon provider={def.provider} />
      <div className="flex-1 min-w-0 text-xs truncate">
        <span className="font-medium text-foreground">{def.provider}</span>
        {def.role && <span className="ml-1 text-muted-foreground">{def.role}</span>}
        {def.model && <span className="ml-1 text-muted-foreground/60">{def.model}</span>}</div>
      {!locked && <button type="button" data-delete-btn="true" onClick={(e) => { e.stopPropagation(); onRemove(); }}
        className="rounded p-0.5 text-muted-foreground/40 hover:bg-rose-500/20 hover:text-rose-400">
        <Trash2 className="size-3 shrink-0" /></button>}
    </div>);
}

export function TaskSetupDialog({ mode = "create", workspace: _workspace, open, onOpenChange, onSubmit, initialAgents, providerHistory = [], codexModels }: TaskSetupDialogProps) {
  const init = initialAgents ?? [{ ...DEFAULT_FIRST }];
  const [agentDefs, setAgentDefs] = useState<AgentDef[]>(init);
  const [sortIds, setSortIds] = useState<string[]>(() => init.map((d, i) => d.agentId ?? `new-${i}`));
  const [selectedId, setSelectedId] = useState<string | null>(init.length > 0 ? (init[0].agentId ?? "new-0") : null);
  const sensors = useSensors(useSensor(PointerSensor));
  const handleClose = useCallback(() => onOpenChange(false), [onOpenChange]);
  useEffect(() => {
    if (!open) return;
    const h = (e: KeyboardEvent) => { if (e.key === "Escape") handleClose(); };
    document.addEventListener("keydown", h); return () => document.removeEventListener("keydown", h);
  }, [open, handleClose]);
  if (!open) return null;
  const selectedIdx = sortIds.indexOf(selectedId ?? "");
  const updateDef = (i: number, u: AgentDef) => setAgentDefs(p => p.map((d, j) => j === i ? u : d));
  const removeDef = (i: number) => {
    if (i === 0) return;
    const rid = sortIds[i]; setAgentDefs(p => p.filter((_, j) => j !== i));
    setSortIds(p => p.filter((_, j) => j !== i)); if (selectedId === rid) setSelectedId(null);
  };
  const addDef = () => {
    const nid = `new-${Date.now()}`;
    setAgentDefs(p => [...p, { provider: "claude", role: "" }]);
    setSortIds(p => [...p, nid]); setSelectedId(nid);
  };
  const handleDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return;
    const ai = sortIds.indexOf(active.id as string), oi = sortIds.indexOf(over.id as string);
    setAgentDefs(p => arrayMove(p, ai, oi)); setSortIds(p => arrayMove(p, ai, oi));
  };
  const valid = agentDefs.filter(d => d.role.trim().length > 0);
  const submit = (launch: boolean) => {
    const ca = valid.find(a => a.provider === "claude"), cx = valid.find(a => a.provider === "codex");
    onSubmit({ agents: valid, claudeConfig: ca ? buildDraftConfigFromDef(ca) : null, codexConfig: cx ? buildDraftConfigFromDef(cx) : null, requestLaunch: launch });
    onOpenChange(false);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={handleClose} />
      <div role="dialog" aria-modal="true" className="relative z-10 flex flex-col w-full max-w-2xl max-h-[90vh] rounded-xl border border-border/50 bg-card shadow-xl">
        <div className="shrink-0 px-4 pt-4 pb-2">
          <h3 className="text-sm font-semibold text-foreground">{mode === "edit" ? "Edit Task" : "New Task"}</h3></div>
        <div className="min-h-0 flex-1 flex overflow-hidden border-t border-border/30">
          <div data-left-pane="true" className="flex w-52 shrink-0 flex-col border-r border-border/30">
            <div className="flex items-center justify-between px-3 py-2">
              <span className="text-xs font-medium text-muted-foreground">Agents</span>
              <button type="button" onClick={addDef} className="inline-flex items-center gap-0.5 rounded-md px-1.5 py-0.5 text-[10px] text-muted-foreground hover:bg-muted hover:text-foreground">
                <Plus className="size-3" />Add</button>
            </div>
            <div data-scroll-region="true" className="min-h-0 flex-1 overflow-y-auto">
              <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
                <SortableContext items={sortIds} strategy={verticalListSortingStrategy}>
                  {agentDefs.map((def, i) => (
                    <SortableListRow key={sortIds[i]} id={sortIds[i]} def={def} selected={selectedId === sortIds[i]}
                      onSelect={() => setSelectedId(sortIds[i])} onRemove={() => removeDef(i)} locked={i === 0} />
                  ))}
                </SortableContext>
              </DndContext>
            </div>
          </div>
          <div data-right-pane="true" className="flex flex-1 flex-col overflow-y-auto">
            {selectedIdx === -1 ? (
              <div data-right-pane-placeholder="true" className="flex flex-1 items-center justify-center text-xs text-muted-foreground/60">
                Select an agent to configure</div>
            ) : (
              <AgentConfigForm def={agentDefs[selectedIdx]} onChange={(u) => updateDef(selectedIdx, u)} onRemove={() => removeDef(selectedIdx)} locked={selectedIdx === 0} providerHistory={providerHistory} codexModels={codexModels} />
            )}
          </div>
        </div>
        <div data-dialog-footer="true" className="shrink-0 flex items-center justify-end gap-2 border-t border-border/30 px-4 py-3">
          <button type="button" onClick={handleClose}
            className="rounded-lg px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground">Cancel</button>
          {mode === "edit" ? (
            <button type="button" onClick={() => submit(false)}
              className="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90">Save</button>
          ) : (<>
            <button type="button" onClick={() => submit(true)} disabled={valid.length === 0}
              className="rounded-lg border border-primary/30 bg-primary/10 px-3 py-1.5 text-xs font-medium text-primary transition-colors hover:bg-primary/20 disabled:opacity-40">
              Create &amp; Connect</button>
            <button type="button" onClick={() => submit(false)}
              className="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90">Create</button>
          </>)}
        </div>
      </div>
    </div>
  );
}
