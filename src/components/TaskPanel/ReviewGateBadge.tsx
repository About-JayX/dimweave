import { cn } from "@/lib/utils";
import type { ReviewBadge } from "./view-model";

const BADGE_STYLES: Record<ReviewBadge["tone"], string> = {
  warning: "border-amber-400/30 bg-amber-400/10 text-amber-300",
  progress: "border-sky-400/30 bg-sky-400/10 text-sky-300",
  neutral: "border-border/50 bg-muted/20 text-muted-foreground",
};

export function ReviewGateBadge({
  badge,
  className,
}: {
  badge: ReviewBadge;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex rounded-full border px-2 py-0.5 text-[10px] font-medium",
        BADGE_STYLES[badge.tone],
        className,
      )}
    >
      {badge.label}
    </span>
  );
}
