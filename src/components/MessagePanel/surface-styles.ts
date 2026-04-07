const accentMap: Record<string, string> = {
  claude: "text-claude",
  lead: "text-claude",
  codex: "text-codex",
  coder: "text-codex",
  user: "text-sky-400",
  system: "text-muted-foreground",
  telegram: "text-sky-400",
};

export function getAccentColor(source: string): string {
  return accentMap[source] ?? accentMap.system;
}

const bgMap: Record<string, string> = {
  claude: "bg-claude/8",
  lead: "bg-claude/8",
  codex: "bg-codex/8",
  coder: "bg-codex/8",
  user: "bg-sky-500/10",
  system: "bg-muted/40",
  telegram: "bg-sky-500/10",
};

export function getMessageSurfacePresentation(source: string): {
  containerClass: string;
} {
  return { containerClass: bgMap[source] ?? bgMap.system };
}

export function getStreamSurfacePresentation(provider: "claude" | "codex"): {
  statusClass: string;
  metaClass: string;
  commandClass: string;
} {
  if (provider === "claude") {
    return {
      statusClass: "text-[11px] text-claude/80",
      metaClass: "text-[10px] text-muted-foreground/55",
      commandClass:
        "text-[13px] text-card-foreground/88 leading-relaxed whitespace-pre-wrap max-h-60 overflow-y-auto",
    };
  }

  return {
    statusClass: "text-[11px] text-codex/80",
    metaClass: "text-[11px] text-muted-foreground/60 italic",
    commandClass:
      "text-[11px] font-mono whitespace-pre-wrap max-h-20 overflow-y-auto rounded-md border border-border/35 bg-background/55 px-1.5 py-1 text-foreground/70",
  };
}
