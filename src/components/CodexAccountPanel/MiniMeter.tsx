import { cn } from "@/lib/utils";
import { barColor } from "./helpers";

export function MiniMeter({
  label,
  used,
  remaining,
}: {
  label: string;
  used: number;
  remaining: number;
}) {
  const u = Math.min(used, 100);
  return (
    <div>
      <div className="flex items-center justify-between text-[10px] mb-1">
        <span className="text-muted-foreground">{label}</span>
        <span
          className={cn(
            "font-mono font-semibold",
            u >= 90 ? "text-destructive" : "text-foreground",
          )}
        >
          {Math.round(remaining)}%
        </span>
      </div>
      <div className="h-1.5 rounded-full radius-keep bg-secondary overflow-hidden relative">
        <div
          className={cn(
            "h-full rounded-full radius-keep transition-all duration-1000 ease-out relative",
            barColor(u),
          )}
          style={{
            width: `${u}%`,
            animation: "meter-fill 1s ease-out",
            boxShadow:
              u < 90
                ? "0 0 8px #22c55e40, 0 0 2px #22c55e80"
                : "0 0 8px #ef444440",
          }}
        >
          {u < 90 && (
            <div className="absolute inset-0 overflow-hidden rounded-full radius-keep">
              <div
                className="absolute inset-0"
                style={{
                  background:
                    "linear-gradient(90deg, transparent 0%, rgba(255,255,255,0.15) 50%, transparent 100%)",
                  animation: "shimmer 2.5s ease-in-out infinite",
                }}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
