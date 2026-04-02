interface ShellTopBarProps {
  workspaceLabel: string;
}

export function ShellTopBar({ workspaceLabel }: ShellTopBarProps) {
  return (
    <header className="flex items-center justify-between border-b border-border/45 px-4 py-3">
      <div className="flex items-center gap-2.5">
        <img
          src="/dimweave-mark.svg"
          alt="Dimweave logo"
          className="h-6 w-6 object-contain"
        />
        <div className="text-sm font-semibold text-foreground">Dimweave</div>
      </div>
      <div className="flex items-center gap-2 rounded-full border border-border/45 bg-card/45 px-3 py-1.5 text-[11px] text-muted-foreground">
        <span className="uppercase tracking-[0.18em] text-muted-foreground/55">
          Current workspace
        </span>
        <span className="max-w-[24rem] truncate text-foreground/82">
          {workspaceLabel}
        </span>
      </div>
    </header>
  );
}
