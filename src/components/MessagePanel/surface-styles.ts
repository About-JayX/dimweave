export function getMessageSurfacePresentation(isUser: boolean): {
  containerClass: string;
} {
  return {
    containerClass: isUser
      ? "bg-sky-500/8 border border-sky-500/18"
      : "bg-card/45 border border-border/35",
  };
}

export function getStreamSurfacePresentation(
  provider: "claude" | "codex",
): {
  containerClass: string;
  statusClass: string;
  metaClass: string;
  commandClass: string;
} {
  if (provider === "claude") {
    return {
      containerClass:
        "max-w-[82%] rounded-xl border border-dashed border-claude/18 bg-claude/6 px-3 py-2.5",
      statusClass: "text-[11px] text-claude/80",
      metaClass: "text-[10px] text-muted-foreground/55",
      commandClass:
        "text-[13px] text-card-foreground/88 leading-relaxed whitespace-pre-wrap max-h-60 overflow-y-auto",
    };
  }

  return {
    containerClass:
      "max-w-[78%] rounded-xl border border-dashed border-codex/18 bg-codex/6 px-3 py-2.5",
    statusClass: "text-[11px] text-codex/80",
    metaClass: "text-[11px] text-muted-foreground/60 italic",
    commandClass:
      "text-[11px] font-mono whitespace-pre-wrap max-h-20 overflow-y-auto rounded-md border border-border/35 bg-background/55 px-1.5 py-1 text-foreground/70",
  };
}
