export function createAsyncUnlistenCleanup(
  register: () => Promise<() => void>,
): () => void {
  let disposed = false;
  let unlisten: (() => void) | null = null;

  void register()
    .then((nextUnlisten) => {
      if (disposed) {
        nextUnlisten();
        return;
      }
      unlisten = nextUnlisten;
    })
    .catch(() => {});

  return () => {
    if (disposed) return;
    disposed = true;
    if (!unlisten) return;
    const cleanup = unlisten;
    unlisten = null;
    cleanup();
  };
}
