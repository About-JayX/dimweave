import { getAccentColor } from "./surface-styles";

const sourceLabel: Record<string, string> = {
  claude: "Claude",
  codex: "Codex",
  user: "You",
  system: "System",
  lead: "Lead",
  coder: "Coder",
  reviewer: "Reviewer",
  telegram: "Telegram",
};

export function getSourceBadgePresentation(source: string): {
  label: string;
  className: string;
} {
  return {
    label: sourceLabel[source] ?? source,
    className: getAccentColor(source),
  };
}

export function SourceBadge({ source }: { source: string }) {
  const accent = getAccentColor(source);
  const label = sourceLabel[source] ?? source;
  return (
    <span
      className={`flex items-center gap-1 text-[10px] font-semibold ${accent}`}
    >
      <span
        className={`inline-block size-2.5 rounded-full radius-keep bg-current`}
      />
      {label}
    </span>
  );
}
