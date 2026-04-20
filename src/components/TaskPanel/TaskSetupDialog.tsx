import { useState, useCallback } from "react";
import { GripVertical, Plus, Trash2 } from "lucide-react";
import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  verticalListSortingStrategy,
  useSortable,
  arrayMove,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import {
  PROVIDER_CAPS,
  buildProviderHistoryOptions,
  findProviderHistoryEntry,
  resolveProviderHistoryAction,
  historyActionToSelectValue,
  buildDraftConfigFromDef,
  type AgentDraftConfig,
  type ProviderHistoryAction,
} from "@/components/AgentStatus/provider-session-view-model";
import type { ProviderHistoryInfo } from "@/stores/task-store/types";
import { ClaudeIcon, CodexIcon } from "@/components/AgentStatus/BrandIcons";
import { CyberSelect } from "@/components/ui/cyber-select";
import { DialogLayout } from "@/components/ui/dialog-layout";
import { AGENT_ROLE_OPTIONS } from "@/components/AgentStatus/RoleSelect";
import type { Provider } from "@/stores/task-store/types";

export interface AgentDef {
  provider: Provider;
  role: string;
  model?: string;
  effort?: string;
  historyAction?: ProviderHistoryAction;
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
export interface CodexModelInfo {
  slug: string;
  displayName: string;
  reasoningLevels?: { effort: string }[];
}
export interface ClaudeModelInfo {
  slug: string;
  displayName: string;
  supportedEfforts: string[];
}

interface TaskSetupDialogProps {
  mode?: TaskSetupMode;
  workspace: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSubmit: (payload: TaskSetupSubmitPayload) => void;
  onDelete?: () => void;
  initialAgents?: AgentDef[];
  providerHistory?: ProviderHistoryInfo[];
  codexModels?: CodexModelInfo[];
  claudeModels?: ClaudeModelInfo[];
}

const PROVIDERS = [
  { value: "claude", label: "Claude" },
  { value: "codex", label: "Codex" },
];
const DEFAULT_FIRST: AgentDef = { provider: "claude", role: "" };

function ProviderIcon({ provider }: { provider: Provider }) {
  return (
    <span data-provider-icon="true" className="shrink-0">
      {provider === "claude" ? (
        <ClaudeIcon className="size-3.5" />
      ) : (
        <CodexIcon className="size-3.5" />
      )}
    </span>
  );
}

function AgentConfigForm({
  def,
  onChange,
  onRemove,
  locked,
  providerHistory = [],
  codexModels,
  claudeModels,
}: {
  def: AgentDef;
  onChange: (u: AgentDef) => void;
  onRemove: () => void;
  locked?: boolean;
  providerHistory?: ProviderHistoryInfo[];
  codexModels?: CodexModelInfo[];
  claudeModels?: ClaudeModelInfo[];
}) {
  const caps = PROVIDER_CAPS[def.provider],
    eDis = caps.effortRequiresModel && !(def.model ?? "").trim();
  const isCodex = def.provider === "codex";
  const isClaude = def.provider === "claude";
  const mOpts =
    isCodex && codexModels?.length
      ? codexModels.map((m) => ({ value: m.slug, label: m.displayName }))
      : isClaude && claudeModels?.length
        ? [
            { value: "", label: "Default" },
            ...claudeModels.map((m) => ({
              value: m.slug,
              label: m.displayName,
            })),
          ]
        : caps.modelOptions;
  const selCodexModel =
    isCodex && codexModels
      ? codexModels.find((m) => m.slug === def.model)
      : undefined;
  const selClaudeModel =
    isClaude && claudeModels
      ? claudeModels.find((m) => m.slug === def.model)
      : undefined;
  const eOpts =
    isCodex && selCodexModel?.reasoningLevels
      ? selCodexModel.reasoningLevels.map((r) => ({
          value: r.effort,
          label: r.effort,
        }))
      : isCodex
        ? []
        : isClaude && selClaudeModel
          ? selClaudeModel.supportedEfforts.map((e) => ({
              value: e,
              label: e,
            }))
          : caps.effortOptions;
  const modelPlaceholder =
    isCodex && codexModels?.length === 0
      ? "Loading models…"
      : isClaude && claudeModels?.length === 0
        ? "Loading models…"
        : "Select model";
  const setP = (p: string) =>
    onChange({
      ...def,
      provider: p as Provider,
      model: "",
      effort: "",
      historyAction: { kind: "new" },
    });
  const histOpts = buildProviderHistoryOptions(def.provider, providerHistory);
  const histVal = historyActionToSelectValue(
    def.historyAction,
    providerHistory,
  );
  const onHist = (v: string) =>
    onChange({
      ...def,
      historyAction: resolveProviderHistoryAction(
        findProviderHistoryEntry(def.provider, providerHistory, v),
      ),
    });
  const effortOpts = eOpts.some((o) => o.value === "")
    ? eOpts
    : [{ value: "", label: "Default" }, ...eOpts];
  return (
    <div
      data-provider-card="true"
      className="m-3 rounded-xl border border-border/40 bg-card/60 shadow-sm"
    >
      <div className="flex items-center gap-2 border-b border-border/30 px-4 py-2.5">
        <ProviderIcon provider={def.provider} />
        <span className="text-xs font-semibold text-foreground capitalize">
          {def.provider}
        </span>
        <div className="flex-1" />
        {!locked && (
          <button
            type="button"
            onClick={onRemove}
            className="rounded p-1 text-muted-foreground hover:bg-rose-500/20 hover:text-rose-400"
          >
            <Trash2 className="size-3" />
          </button>
        )}
      </div>
      <div className="space-y-2 px-4 py-3">
        <div className="space-y-0.5" data-provider-select="true">
          <label className="block text-[10px] text-muted-foreground/70">
            Provider
          </label>
          <CyberSelect
            variant="form"
            value={def.provider}
            options={PROVIDERS}
            onChange={setP}
          />
        </div>
        <div className="space-y-0.5" data-role-select="true">
          <label className="block text-[10px] text-muted-foreground/70">
            Role
          </label>
          <CyberSelect
            variant="form"
            value={def.role}
            options={AGENT_ROLE_OPTIONS}
            onChange={(v) => onChange({ ...def, role: v })}
            placeholder="Select role"
          />
        </div>
        {caps.supportsModel && (
          <div className="space-y-0.5" data-model-select="true">
            <label className="block text-[10px] text-muted-foreground/70">
              Model
            </label>
            <CyberSelect
              variant="form"
              value={def.model ?? "\x00"}
              options={mOpts}
              onChange={(v) => onChange({ ...def, model: v })}
              placeholder={modelPlaceholder}
            />
          </div>
        )}
        {caps.supportsEffort && (
          <div className="space-y-0.5" data-effort-select="true">
            <label className="block text-[10px] text-muted-foreground/70">
              {caps.effortLabel}
            </label>
            <CyberSelect
              variant="form"
              value={def.effort ?? ""}
              options={effortOpts}
              onChange={(v) => onChange({ ...def, effort: v })}
              disabled={eDis}
            />
          </div>
        )}
        {caps.supportsSessionResume && (
          <div className="space-y-0.5" data-history-select="true">
            <label className="block text-[10px] text-muted-foreground/70">
              Session
            </label>
            <CyberSelect
              variant="form"
              value={histVal}
              options={histOpts}
              onChange={onHist}
              placeholder="New session"
            />
          </div>
        )}
      </div>
    </div>
  );
}

function SortableListRow({
  id,
  def,
  selected,
  onSelect,
  onRemove,
  locked,
}: {
  id: string;
  def: AgentDef;
  selected: boolean;
  onSelect: () => void;
  onRemove: () => void;
  locked?: boolean;
}) {
  const s = useSortable({ id });
  return (
    <div
      ref={s.setNodeRef}
      data-draggable-row="true"
      {...(locked ? { "data-locked-row": "true" } : {})}
      onClick={onSelect}
      style={{
        transform: CSS.Transform.toString(s.transform),
        transition: s.transition,
      }}
      className={`flex items-center gap-1.5 px-3 py-2 cursor-pointer hover:bg-muted/30 ${selected ? "bg-muted/50" : ""}`}
    >
      <button
        type="button"
        data-drag-handle="true"
        {...s.attributes}
        {...s.listeners}
        className="cursor-grab rounded p-0.5 text-muted-foreground/40 hover:text-muted-foreground"
        onClick={(e) => e.stopPropagation()}
      >
        <GripVertical className="size-3 shrink-0" />
      </button>
      <ProviderIcon provider={def.provider} />
      <div className="flex-1 min-w-0 text-xs truncate">
        <span className="font-medium text-foreground">{def.provider}</span>
        {def.role && (
          <span className="ml-1 text-muted-foreground">{def.role}</span>
        )}
        {def.model && (
          <span className="ml-1 text-muted-foreground/60">{def.model}</span>
        )}
      </div>
      {!locked && (
        <button
          type="button"
          data-delete-btn="true"
          onClick={(e) => {
            e.stopPropagation();
            onRemove();
          }}
          className="rounded p-0.5 text-muted-foreground/40 hover:bg-rose-500/20 hover:text-rose-400"
        >
          <Trash2 className="size-3 shrink-0" />
        </button>
      )}
    </div>
  );
}

export function TaskSetupDialog({
  mode = "create",
  workspace: _workspace,
  open,
  onOpenChange,
  onSubmit,
  onDelete,
  initialAgents,
  providerHistory = [],
  codexModels,
  claudeModels,
}: TaskSetupDialogProps) {
  const init = initialAgents ?? [{ ...DEFAULT_FIRST }];
  const [agentDefs, setAgentDefs] = useState<AgentDef[]>(init);
  const [sortIds, setSortIds] = useState<string[]>(() =>
    init.map((d, i) => d.agentId ?? `new-${i}`),
  );
  const [selectedId, setSelectedId] = useState<string | null>(
    init.length > 0 ? (init[0].agentId ?? "new-0") : null,
  );
  const sensors = useSensors(useSensor(PointerSensor));
  const handleClose = useCallback(() => onOpenChange(false), [onOpenChange]);
  const selectedIdx = sortIds.indexOf(selectedId ?? "");
  const updateDef = (i: number, u: AgentDef) =>
    setAgentDefs((p) => p.map((d, j) => (j === i ? u : d)));
  const removeDef = (i: number) => {
    if (i === 0) return;
    const rid = sortIds[i];
    setAgentDefs((p) => p.filter((_, j) => j !== i));
    setSortIds((p) => p.filter((_, j) => j !== i));
    if (selectedId === rid) setSelectedId(null);
  };
  const addDef = () => {
    const nid = `new-${Date.now()}`;
    setAgentDefs((p) => [...p, { provider: "claude", role: "" }]);
    setSortIds((p) => [...p, nid]);
    setSelectedId(nid);
  };
  const handleDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over || active.id === over.id) return;
    const ai = sortIds.indexOf(active.id as string),
      oi = sortIds.indexOf(over.id as string);
    setAgentDefs((p) => arrayMove(p, ai, oi));
    setSortIds((p) => arrayMove(p, ai, oi));
  };
  const valid = agentDefs.filter((d) => d.role.trim().length > 0);
  const submit = (launch: boolean) => {
    const ca = valid.find((a) => a.provider === "claude"),
      cx = valid.find((a) => a.provider === "codex");
    onSubmit({
      agents: valid,
      claudeConfig: ca ? buildDraftConfigFromDef(ca) : null,
      codexConfig: cx ? buildDraftConfigFromDef(cx) : null,
      requestLaunch: launch,
    });
    onOpenChange(false);
  };

  return (
    <DialogLayout
      open={open}
      onClose={handleClose}
      width="lg"
      bodyFlex
      header={
        <h3 className="text-sm font-semibold text-foreground">
          {mode === "edit" ? "Edit Task" : "New Task"}
        </h3>
      }
      body={
        <div className="flex h-full w-full">
          <div
            data-left-pane="true"
            className="flex w-52 shrink-0 flex-col border-r border-border/30"
          >
            <div className="flex items-center justify-between px-3 py-2">
              <span className="text-xs font-medium text-muted-foreground">
                Agents
              </span>
              <button
                type="button"
                onClick={addDef}
                className="inline-flex items-center gap-0.5 rounded-md px-1.5 py-0.5 text-[10px] text-muted-foreground hover:bg-muted hover:text-foreground"
              >
                <Plus className="size-3" />
                Add
              </button>
            </div>
            <div
              data-scroll-region="true"
              className="min-h-0 flex-1 overflow-y-auto"
            >
              <DndContext
                sensors={sensors}
                collisionDetection={closestCenter}
                onDragEnd={handleDragEnd}
              >
                <SortableContext
                  items={sortIds}
                  strategy={verticalListSortingStrategy}
                >
                  {agentDefs.map((def, i) => (
                    <SortableListRow
                      key={sortIds[i]}
                      id={sortIds[i]}
                      def={def}
                      selected={selectedId === sortIds[i]}
                      onSelect={() => setSelectedId(sortIds[i])}
                      onRemove={() => removeDef(i)}
                      locked={i === 0}
                    />
                  ))}
                </SortableContext>
              </DndContext>
            </div>
          </div>
          <div
            data-right-pane="true"
            className="flex flex-1 flex-col overflow-y-auto"
          >
            {selectedIdx === -1 ? (
              <div
                data-right-pane-placeholder="true"
                className="flex flex-1 items-center justify-center text-xs text-muted-foreground/60"
              >
                Select an agent to configure
              </div>
            ) : (
              <AgentConfigForm
                def={agentDefs[selectedIdx]}
                onChange={(u) => updateDef(selectedIdx, u)}
                onRemove={() => removeDef(selectedIdx)}
                locked={selectedIdx === 0}
                providerHistory={providerHistory}
                codexModels={codexModels}
                claudeModels={claudeModels}
              />
            )}
          </div>
        </div>
      }
      footer={
        <div data-dialog-footer="true" className="flex items-center gap-2">
          {mode === "edit" && onDelete && (
            <button
              type="button"
              data-delete-task-btn="true"
              onClick={onDelete}
              className="rounded-lg border border-rose-500/30 bg-rose-500/10 px-3 py-1.5 text-xs font-medium text-rose-400 transition-colors hover:bg-rose-500/20 active:opacity-70"
            >
              Delete Task
            </button>
          )}
          <div className="flex-1" />
          <button
            type="button"
            onClick={handleClose}
            className="rounded-lg px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          >
            Cancel
          </button>
          {mode === "edit" ? (
            <>
              <button
                type="button"
                onClick={() => submit(true)}
                className="rounded-lg border border-primary/30 bg-primary/10 px-3 py-1.5 text-xs font-medium text-primary transition-colors hover:bg-primary/20"
              >
                Save &amp; Connect
              </button>
              <button
                type="button"
                onClick={() => submit(false)}
                className="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90"
              >
                Save
              </button>
            </>
          ) : (
            <>
              <button
                type="button"
                onClick={() => submit(true)}
                disabled={valid.length === 0}
                className="rounded-lg border border-primary/30 bg-primary/10 px-3 py-1.5 text-xs font-medium text-primary transition-colors hover:bg-primary/20 disabled:opacity-40"
              >
                Create &amp; Connect
              </button>
              <button
                type="button"
                onClick={() => submit(false)}
                className="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90"
              >
                Create
              </button>
            </>
          )}
        </div>
      }
    />
  );
}
