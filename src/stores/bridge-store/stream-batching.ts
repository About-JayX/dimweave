import type { BridgeState } from "./types";
import type {
  ClaudeStreamPayload,
  CodexStreamPayload,
} from "./listener-payloads";
import { handleClaudeStreamEvent } from "./stream-reducers";

const MAX_CODEX_PREVIEW_CHARS = 100_000;

interface CodexStreamBatch {
  activity: string | null;
  reasoning: string | null;
  delta: string | null;
  commandOutputAppend: string;
}

export interface PendingStreamUpdates {
  claudePreviewText: string;
  codexActivity: string | null;
  codexReasoning: string | null;
  codexDelta: string | null;
  codexCommandOutput: string;
}

export function createPendingStreamUpdates(): PendingStreamUpdates {
  return {
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
}

export function clearPendingCodexStream(
  pending: PendingStreamUpdates,
): void {
  pending.codexActivity = null;
  pending.codexReasoning = null;
  pending.codexDelta = null;
  pending.codexCommandOutput = "";
}

export function queueClaudePreviewUpdate(
  pending: PendingStreamUpdates,
  payload: ClaudeStreamPayload,
): boolean {
  if (payload.kind !== "preview" || !payload.text) {
    return false;
  }
  pending.claudePreviewText += payload.text;
  return true;
}

export function queueCodexBufferedUpdate(
  pending: PendingStreamUpdates,
  payload: CodexStreamPayload,
): boolean {
  switch (payload.kind) {
    case "activity":
      pending.codexActivity = payload.label ?? "";
      return true;
    case "reasoning":
      pending.codexReasoning = payload.text ?? "";
      return true;
    case "delta":
      pending.codexDelta = payload.text ?? "";
      return true;
    case "commandOutput":
      pending.codexCommandOutput += payload.text ?? "";
      return true;
    default:
      return false;
  }
}

export function flushClaudePreviewIfPending(
  state: BridgeState,
  pending: PendingStreamUpdates,
): Partial<BridgeState> {
  if (!pending.claudePreviewText) {
    return {};
  }
  return flushPendingStreamUpdates(state, pending);
}

export function flushPendingStreamUpdates(
  state: BridgeState,
  pending: PendingStreamUpdates,
): Partial<BridgeState> {
  let nextState = state;
  let partial: Partial<BridgeState> = {};

  if (pending.claudePreviewText) {
    const claudePartial = handleClaudeStreamEvent(nextState, {
      kind: "preview",
      text: pending.claudePreviewText,
    });
    partial = { ...partial, ...claudePartial };
    nextState = {
      ...nextState,
      ...claudePartial,
      claudeStream: claudePartial.claudeStream ?? nextState.claudeStream,
    };
  }

  if (
    pending.codexActivity !== null ||
    pending.codexReasoning !== null ||
    pending.codexDelta !== null ||
    pending.codexCommandOutput
  ) {
    const codexPartial = handleCodexStreamBatch(nextState, {
      activity: pending.codexActivity,
      reasoning: pending.codexReasoning,
      delta: pending.codexDelta,
      commandOutputAppend: pending.codexCommandOutput,
    });
    partial = { ...partial, ...codexPartial };
  }

  clearPendingClaudePreview(pending);
  clearPendingCodexStream(pending);
  return partial;
}

function handleCodexStreamBatch(
  state: BridgeState,
  batch: CodexStreamBatch,
): Partial<BridgeState> {
  const next = { ...state.codexStream };

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

  return { codexStream: next };
}
