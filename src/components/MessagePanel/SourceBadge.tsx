import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";

const sourceStyle: Record<string, { label: string; className: string }> = {
  claude: {
    label: "Claude",
    className: "border-claude/24 bg-claude/6 text-claude/90",
  },
  codex: {
    label: "Codex",
    className: "border-codex/24 bg-codex/6 text-codex/90",
  },
  user: {
    label: "You",
    className: "border-sky-500/24 bg-sky-500/6 text-sky-300",
  },
  system: {
    label: "System",
    className: "border-system/24 bg-system/8 text-system/90",
  },
  lead: {
    label: "Lead",
    className: "border-claude/24 bg-claude/6 text-claude/90",
  },
  coder: {
    label: "Coder",
    className: "border-codex/24 bg-codex/6 text-codex/90",
  },
  reviewer: {
    label: "Reviewer",
    className: "border-yellow-500/24 bg-yellow-500/8 text-yellow-300",
  },
};

export function getSourceBadgePresentation(source: string): {
  label: string;
  className: string;
} {
  return sourceStyle[source] ?? sourceStyle.system;
}

export function SourceBadge({ source }: { source: string }) {
  const style = getSourceBadgePresentation(source);
  return (
    <Badge
      variant="outline"
      className={cn(
        "uppercase border px-2 py-0.5 text-[10px] tracking-[0.14em]",
        style.className,
      )}
    >
      {style.label}
    </Badge>
  );
}
