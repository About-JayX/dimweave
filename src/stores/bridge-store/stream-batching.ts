import type { BridgeState } from "./types";
import type {
  ClaudeStreamPayload,
  CodexStreamPayload,
} from "./listener-payloads";
import {
  defaultClaudeStreamState,
  defaultCodexStreamState,
  handleClaudeStreamEvent,
  reduceClaudeStreamSlice,
} from "./stream-reducers";

const MAX_CODEX_PREVIEW_CHARS = 100_000;

interface CodexStreamBatch {
  activity: string | null;
  reasoning: string | null;
  delta: string | null;
  commandOutputAppend: string;
}

export interface PendingStreamUpdates {
  bucketTaskId: string | null;
  claudePreviewText: string;
  codexActivity: string | null;
  codexReasoning: string | null;
  codexDelta: string | null;
  codexCommandOutput: string;
}

export function createPendingStreamUpdates(): PendingStreamUpdates {
  return {
    bucketTaskId: null,
    claudePreviewText: "",
    codexActivity: null,
    codexReasoning: null,
    codexDelta: null,
    codexCommandOutput: "",
  };
}

export function hasPendingStreamUpdates(
  pending: PendingStreamUpdates,
): boolean {
  return Boolean(
    pending.claudePreviewText ||
      pending.codexActivity !== null ||
      pending.codexReasoning !== null ||
      pending.codexDelta !== null ||
      pending.codexCommandOutput,
  );
}

export function clearPendingClaudePreview(
  pending: PendingStreamUpdates,
): void {
  pending.claudePreviewText = "";
  clearPendingBucketIfEmpty(pending);
}

export function clearPendingCodexStream(
  pending: PendingStreamUpdates,
): void {
  pending.codexActivity = null;
  pending.codexReasoning = null;
  pending.codexDelta = null;
  pending.codexCommandOutput = "";
  clearPendingBucketIfEmpty(pending);
}

export function queueClaudePreviewUpdate(
  pending: PendingStreamUpdates,
  payload: ClaudeStreamPayload,
  bucketTaskId?: string | null,
): boolean {
  if (payload.kind !== "preview" || !payload.text) {
    return false;
  }
  rememberPendingBucket(pending, bucketTaskId);
  pending.claudePreviewText += payload.text;
  return true;
}

export function queueCodexBufferedUpdate(
  pending: PendingStreamUpdates,
  payload: CodexStreamPayload,
  bucketTaskId?: string | null,
): boolean {
  switch (payload.kind) {
    case "activity":
      rememberPendingBucket(pending, bucketTaskId);
      pending.codexActivity = payload.label ?? "";
      return true;
    case "reasoning":
      rememberPendingBucket(pending, bucketTaskId);
      pending.codexReasoning = payload.text ?? "";
      return true;
    case "delta":
      rememberPendingBucket(pending, bucketTaskId);
      pending.codexDelta = payload.text ?? "";
      return true;
    case "commandOutput":
      rememberPendingBucket(pending, bucketTaskId);
      pending.codexCommandOutput += payload.text ?? "";
      return true;
    default:
      return false;
  }
}

export function flushClaudePreviewIfPending(
  state: BridgeState,
  pending: PendingStreamUpdates,
  activeTaskId: string | null = null,
): Partial<BridgeState> {
  if (!pending.claudePreviewText) {
    return {};
  }
  return flushPendingStreamUpdates(state, pending, activeTaskId);
}

export function flushPendingStreamUpdates(
  state: BridgeState,
  pending: PendingStreamUpdates,
  activeTaskId: string | null = null,
): Partial<BridgeState> {
  let nextState = state;
  let partial: Partial<BridgeState> = {};
  const targetTaskId = pending.bucketTaskId ?? activeTaskId;

  if (pending.claudePreviewText) {
    const payload: ClaudeStreamPayload = {
      kind: "preview",
      text: pending.claudePreviewText,
    };
    if (targetTaskId) {
      const prevBucket =
        nextState.claudeStreamsByTask[targetTaskId] ??
        defaultClaudeStreamState();
      const nextBucket = reduceClaudeStreamSlice(prevBucket, payload);
      const claudePartial: Partial<BridgeState> = {
        claudeStream: nextBucket,
        claudeStreamsByTask: {
          ...nextState.claudeStreamsByTask,
          [targetTaskId]: nextBucket,
        },
      };
      partial = { ...partial, ...claudePartial };
      nextState = {
        ...nextState,
        ...claudePartial,
        claudeStream: nextBucket,
        claudeStreamsByTask: claudePartial.claudeStreamsByTask!,
      };
    } else {
      const claudePartial = handleClaudeStreamEvent(nextState, payload);
      partial = { ...partial, ...claudePartial };
      nextState = {
        ...nextState,
        ...claudePartial,
        claudeStream: claudePartial.claudeStream ?? nextState.claudeStream,
      };
    }
  }

  if (
    pending.codexActivity !== null ||
    pending.codexReasoning !== null ||
    pending.codexDelta !== null ||
    pending.codexCommandOutput
  ) {
    const batch = {
      activity: pending.codexActivity,
      reasoning: pending.codexReasoning,
      delta: pending.codexDelta,
      commandOutputAppend: pending.codexCommandOutput,
    };
    const codexPartial = targetTaskId
      ? handleCodexTaskStreamBatch(nextState, targetTaskId, batch)
      : handleCodexStreamBatch(nextState, batch);
    partial = { ...partial, ...codexPartial };
  }

  clearPendingClaudePreview(pending);
  clearPendingCodexStream(pending);
  return partial;
}

function rememberPendingBucket(
  pending: PendingStreamUpdates,
  bucketTaskId: string | null | undefined,
): void {
  if (bucketTaskId === undefined) return;
  if (pending.bucketTaskId && pending.bucketTaskId !== bucketTaskId) {
    pending.claudePreviewText = "";
    pending.codexActivity = null;
    pending.codexReasoning = null;
    pending.codexDelta = null;
    pending.codexCommandOutput = "";
  }
  pending.bucketTaskId = bucketTaskId;
}

function clearPendingBucketIfEmpty(pending: PendingStreamUpdates): void {
  if (!hasPendingStreamUpdates(pending)) {
    pending.bucketTaskId = null;
  }
}

function handleCodexStreamBatch(
  state: BridgeState,
  batch: CodexStreamBatch,
): Partial<BridgeState> {
  return { codexStream: reduceCodexStreamBatch(state.codexStream, batch) };
}

function handleCodexTaskStreamBatch(
  state: BridgeState,
  activeTaskId: string,
  batch: CodexStreamBatch,
): Partial<BridgeState> {
  const prevBucket =
    state.codexStreamsByTask[activeTaskId] ?? defaultCodexStreamState();
  const nextBucket = reduceCodexStreamBatch(prevBucket, batch);
  return {
    codexStream: nextBucket,
    codexStreamsByTask: {
      ...state.codexStreamsByTask,
      [activeTaskId]: nextBucket,
    },
  };
}

function reduceCodexStreamBatch(
  prev: BridgeState["codexStream"],
  batch: CodexStreamBatch,
): BridgeState["codexStream"] {
  const next = { ...prev };
  if (batch.activity !== null) {
    next.activity = batch.activity;
    next.commandOutput = "";
  }
  if (batch.reasoning !== null) {
    next.reasoning = batch.reasoning.slice(-MAX_CODEX_PREVIEW_CHARS);
  }
  if (batch.delta !== null) {
    next.currentDelta = batch.delta.slice(-MAX_CODEX_PREVIEW_CHARS);
  }
  if (batch.commandOutputAppend) {
    next.commandOutput += batch.commandOutputAppend;
  }

  return next;
}
