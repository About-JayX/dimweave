import { useMemo, useState, type ReactNode } from "react";
import { ChevronDown } from "lucide-react";
import { TelegramPanel } from "@/components/AgentStatus/TelegramPanel";
import { BugInboxPanel } from "@/components/BugInboxPanel";
import { TelegramIcon, FeishuIcon } from "@/components/AgentStatus/BrandIcons";
import { useFeishuProjectStore } from "@/stores/feishu-project-store";
import { activeItemCount } from "@/components/BugInboxPanel/view-model";
import { cn } from "@/lib/utils";

interface DisclosureSectionProps {
  title: string;
  icon?: ReactNode;
  badge?: number;
  defaultOpen: boolean;
  children: ReactNode;
}

function DisclosureSection({
  title,
  icon,
  badge,
  defaultOpen,
  children,
}: DisclosureSectionProps) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div className="border-b border-border/30 last:border-b-0">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center gap-2 px-4 py-2.5 text-[12px] font-medium text-muted-foreground/80 transition-colors hover:bg-card/40 hover:text-foreground/90"
      >
        <ChevronDown
          className={cn(
            "size-3.5 shrink-0 transition-transform duration-150",
            !open && "-rotate-90",
          )}
        />
        {icon}
        {title}
        {badge != null && badge > 0 && (
          <span className="ml-auto min-w-4 rounded-full bg-rose-500 px-1 text-center text-[9px] font-semibold leading-4 text-background">
            {badge > 9 ? "9+" : badge}
          </span>
        )}
      </button>
      {open && <div className="px-4 pb-3">{children}</div>}
    </div>
  );
}

export function ToolsPanel() {
  const bugItems = useFeishuProjectStore((s) => s.items);
  const bugCount = useMemo(() => activeItemCount(bugItems), [bugItems]);

  return (
    <section className="flex h-full flex-col -mx-4 -my-4 overflow-y-auto">
      <DisclosureSection
        title="Telegram"
        icon={<TelegramIcon className="size-3.5 shrink-0" />}
        defaultOpen={false}
      >
        <TelegramPanel />
      </DisclosureSection>
      <DisclosureSection
        title="飞书"
        icon={<FeishuIcon className="size-3.5 shrink-0" />}
        badge={bugCount}
        defaultOpen
      >
        <BugInboxPanel />
      </DisclosureSection>
    </section>
  );
}
