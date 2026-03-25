const STORAGE_PREFIX = "agentbridge:claude-dev-confirm:";

function storageKey(cwd: string) {
  return `${STORAGE_PREFIX}${encodeURIComponent(cwd)}`;
}

export function shouldPromptForClaudeDevConfirm(cwd: string) {
  if (!import.meta.env.DEV || !cwd) {
    return false;
  }
  return window.localStorage.getItem(storageKey(cwd)) !== "accepted";
}

export function rememberClaudeDevConfirm(cwd: string) {
  if (!cwd) {
    return;
  }
  window.localStorage.setItem(storageKey(cwd), "accepted");
}
