import { useState } from "react";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

function ApiKeyForm({ onDone }: { onDone: () => void }) {
  const loginWithApiKey = useCodexAccountStore((s) => s.loginWithApiKey);
  const pending = useCodexAccountStore((s) => s.apiKeyLoginPending);
  const error = useCodexAccountStore((s) => s.apiKeyLoginError);
  const [value, setValue] = useState("");
  return (
    <form
      className="mt-2 space-y-2 rounded-md border border-codex/20 bg-codex/5 px-3 py-2.5"
      onSubmit={async (e) => {
        e.preventDefault();
        if (pending) return;
        const ok = await loginWithApiKey(value);
        if (ok) {
          setValue("");
          onDone();
        }
      }}
    >
      <label className="block text-[10px] font-semibold uppercase tracking-wide text-codex/80">
        OpenAI API key
      </label>
      <input
        autoFocus
        type="password"
        spellCheck={false}
        autoComplete="off"
        placeholder="sk-..."
        value={value}
        onChange={(e) => setValue(e.target.value)}
        disabled={pending}
        className="w-full rounded-md border border-border/40 bg-background px-2 py-1.5 text-[11px] font-mono outline-none focus:border-codex/50"
      />
      {error && (
        <p className="truncate text-[10px] text-destructive" title={error}>
          {error}
        </p>
      )}
      <div className="flex gap-2">
        <Button
          type="submit"
          size="sm"
          variant="outline"
          className={cn(
            "flex-1 text-[11px] border-codex/30 text-codex hover:bg-codex/10",
            pending && "opacity-60",
          )}
          disabled={pending || !value.trim()}
        >
          {pending ? "Logging in…" : "Login with API key"}
        </Button>
        <Button
          type="button"
          size="sm"
          variant="ghost"
          className="text-[11px] text-muted-foreground"
          onClick={onDone}
          disabled={pending}
        >
          Cancel
        </Button>
      </div>
    </form>
  );
}

export function AuthActions() {
  const profile = useCodexAccountStore((s) => s.profile);
  const loginPending = useCodexAccountStore((s) => s.loginPending);
  const loginUri = useCodexAccountStore((s) => s.loginUri);
  const login = useCodexAccountStore((s) => s.login);
  const cancelLogin = useCodexAccountStore((s) => s.cancelLogin);
  const logout = useCodexAccountStore((s) => s.logout);
  const [apiKeyMode, setApiKeyMode] = useState(false);

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
    if (apiKeyMode) {
      return <ApiKeyForm onDone={() => setApiKeyMode(false)} />;
    }
    return (
      <div className="mt-2 space-y-1.5">
        <Button
          size="sm"
          variant="outline"
          className="w-full text-[11px] border-codex/30 text-codex hover:bg-codex/10 hover:border-codex/50 transition-colors"
          onClick={login}
        >
          Login to Codex (ChatGPT)
        </Button>
        <button
          type="button"
          onClick={() => setApiKeyMode(true)}
          className="w-full text-[10px] text-muted-foreground hover:text-foreground transition-colors"
        >
          Use API key instead
        </button>
      </div>
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
