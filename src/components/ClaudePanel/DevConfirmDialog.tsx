import { Button } from "@/components/ui/button";

interface DevConfirmDialogProps {
  cwd: string;
  rememberChoice: boolean;
  onRememberChoiceChange: (checked: boolean) => void;
  onCancel: () => void;
  onConfirm: () => void;
}

export function DevConfirmDialog({
  cwd,
  rememberChoice,
  onRememberChoiceChange,
  onCancel,
  onConfirm,
}: DevConfirmDialogProps) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4">
      <div className="w-full max-w-md rounded-2xl border border-claude/30 bg-[#11131a] p-5 shadow-[0_24px_80px_rgba(0,0,0,0.45)]">
        <div className="space-y-2">
          <div className="text-[11px] uppercase tracking-[0.18em] text-claude/80">
            Dev Confirm
          </div>
          <h3 className="text-base font-semibold text-foreground">
            Start Local Claude Channel
          </h3>
          <p className="text-sm leading-6 text-muted-foreground">
            AgentBridge will start Claude with local development channel mode
            for <span className="font-mono text-foreground">server:agentbridge</span>.
            This is only intended for your current machine and project.
          </p>
          <div className="rounded-lg border border-border/60 bg-black/20 px-3 py-2 font-mono text-[11px] text-secondary-foreground">
            {cwd}
          </div>
        </div>

        <label className="mt-4 flex cursor-pointer items-start gap-3 rounded-lg border border-border/60 bg-black/20 px-3 py-2.5 text-sm text-secondary-foreground">
          <input
            type="checkbox"
            className="mt-0.5 size-4 accent-[#d97757]"
            checked={rememberChoice}
            onChange={(e) => onRememberChoiceChange(e.target.checked)}
          />
          <span>Remember for this project</span>
        </label>

        <div className="mt-5 flex justify-end gap-2">
          <Button variant="secondary" size="sm" onClick={onCancel}>
            Cancel
          </Button>
          <Button
            size="sm"
            className="bg-claude text-white hover:bg-claude/90"
            onClick={onConfirm}
          >
            Continue
          </Button>
        </div>
      </div>
    </div>
  );
}
