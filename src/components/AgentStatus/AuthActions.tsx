import { useCodexAccountStore } from "@/stores/codex-account-store";
import { Button } from "@/components/ui/button";

export function AuthActions() {
  const profile = useCodexAccountStore((s) => s.profile);
  const loginPending = useCodexAccountStore((s) => s.loginPending);
  const loginUri = useCodexAccountStore((s) => s.loginUri);
  const login = useCodexAccountStore((s) => s.login);
  const cancelLogin = useCodexAccountStore((s) => s.cancelLogin);
  const logout = useCodexAccountStore((s) => s.logout);

  if (loginPending) {
    return (
      <div className="mt-2 rounded-md border border-codex/20 bg-codex/5 px-3 py-2.5 space-y-2">
        <div className="flex items-center gap-2 text-[11px] text-codex">
          <span className="size-3 border-2 border-codex/30 border-t-codex rounded-full radius-keep animate-spin shrink-0" />
          Waiting for browser login...
        </div>
        {loginUri && (
          <a
            href={loginUri}
            target="_blank"
            rel="noreferrer"
            className="block text-[10px] text-codex/80 hover:text-codex hover:underline truncate"
            title={loginUri}
          >
            Open login page →
          </a>
        )}
        <Button
          size="sm"
          variant="ghost"
          className="w-full text-[11px] text-muted-foreground hover:text-destructive"
          onClick={cancelLogin}
        >
          Cancel
        </Button>
      </div>
    );
  }

  if (!profile?.email) {
    return (
      <Button
        size="sm"
        variant="outline"
        className="w-full mt-2 text-[11px] border-codex/30 text-codex hover:bg-codex/10 hover:border-codex/50 transition-colors"
        onClick={login}
      >
        Login to Codex
      </Button>
    );
  }

  return (
    <div className="mt-1.5 flex items-center justify-between rounded-md bg-muted/30 px-2.5 py-1.5">
      <div className="flex items-center gap-1.5 min-w-0">
        <span className="size-1.5 rounded-full radius-keep bg-codex shrink-0" />
        <span
          className="text-[10px] text-foreground/80 truncate"
          title={profile.email}
        >
          {profile.email}
        </span>
        {profile.planType && (
          <span className="capitalize rounded bg-codex/10 px-1 py-px text-[9px] font-semibold text-codex shrink-0">
            {profile.planType}
          </span>
        )}
      </div>
      <button
        type="button"
        onClick={logout}
        className="text-[10px] text-muted-foreground hover:text-destructive transition-colors shrink-0 ml-2"
      >
        Logout
      </button>
    </div>
  );
}
