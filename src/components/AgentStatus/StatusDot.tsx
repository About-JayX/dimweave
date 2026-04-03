import { cn } from "@/lib/utils";

type DotVariant = "claude" | "codex" | "generic";

const statusConfig: Record<
  string,
  Record<DotVariant, { bg: string; glow: string; animation: string }>
> = {
  connected: {
    claude: {
      bg: "bg-claude",
      glow: "shadow-[0_0_6px_var(--color-claude),0_0_12px_var(--color-claude-glow)]",
      animation: "dot-glow",
    },
    codex: {
      bg: "bg-codex",
      glow: "shadow-[0_0_6px_var(--color-codex),0_0_12px_var(--color-codex-glow)]",
      animation: "dot-glow",
    },
    generic: {
      bg: "bg-codex",
      glow: "shadow-[0_0_6px_var(--color-codex),0_0_12px_var(--color-codex-glow)]",
      animation: "dot-glow",
    },
  },
  connecting: {
    claude: {
      bg: "bg-yellow-500",
      glow: "shadow-[0_0_6px_#eab308]",
      animation: "dot-ring-ping",
    },
    codex: {
      bg: "bg-yellow-500",
      glow: "shadow-[0_0_6px_#eab308]",
      animation: "dot-ring-ping",
    },
    generic: {
      bg: "bg-yellow-500",
      glow: "shadow-[0_0_6px_#eab308]",
      animation: "dot-ring-ping",
    },
  },
  disconnected: {
    claude: { bg: "bg-muted-foreground", glow: "", animation: "" },
    codex: { bg: "bg-muted-foreground", glow: "", animation: "" },
    generic: { bg: "bg-muted-foreground", glow: "", animation: "" },
  },
  error: {
    claude: {
      bg: "bg-destructive",
      glow: "shadow-[0_0_6px_var(--color-destructive),0_0_12px_color-mix(in_srgb,var(--color-destructive)_30%,transparent)]",
      animation: "animate-pulse",
    },
    codex: {
      bg: "bg-destructive",
      glow: "shadow-[0_0_6px_var(--color-destructive),0_0_12px_color-mix(in_srgb,var(--color-destructive)_30%,transparent)]",
      animation: "animate-pulse",
    },
    generic: {
      bg: "bg-destructive",
      glow: "shadow-[0_0_6px_var(--color-destructive),0_0_12px_color-mix(in_srgb,var(--color-destructive)_30%,transparent)]",
      animation: "animate-pulse",
    },
  },
};

const fallback = statusConfig.disconnected.generic;

export function StatusDot({
  status,
  variant = "generic",
}: {
  status: string;
  variant?: DotVariant;
}) {
  const config = statusConfig[status]?.[variant] ?? fallback;
  return (
    <span className="relative inline-flex size-2 shrink-0">
      {config.animation && (
        <span
          className={cn(
            "absolute inset-0 rounded-full radius-keep",
            config.bg,
            config.glow,
            config.animation,
          )}
        />
      )}
      <span
        className={cn(
          "relative inline-block size-2 rounded-full radius-keep",
          config.bg,
        )}
      />
    </span>
  );
}
