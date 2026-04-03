import { cn } from "@/lib/utils";

export function TabBtn({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "relative text-sm font-semibold transition-all duration-200 pb-1",
        active
          ? "text-foreground"
          : "text-muted-foreground hover:text-foreground/80",
      )}
    >
      {children}
      <span
        className={cn(
          "absolute bottom-0 left-0 right-0 h-0.5 rounded-full radius-keep transition-all duration-300",
          active
            ? "bg-primary opacity-100 shadow-[0_0_8px_#8b5cf660]"
            : "bg-transparent opacity-0",
        )}
      />
    </button>
  );
}
