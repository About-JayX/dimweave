export {};

declare global {
  interface Window {
    __TAURI_INTERNALS__?: {
      convertFileSrc?: (filePath: string, protocol?: string) => string;
      invoke?: (
        command: string,
        args?: unknown,
        options?: unknown,
      ) => Promise<unknown>;
      transformCallback?: (
        callback: (...args: unknown[]) => unknown,
        once?: boolean,
      ) => number;
      unregisterCallback?: (callbackId: number) => void;
    };
    __TAURI_EVENT_PLUGIN_INTERNALS__?: {
      unregisterListener?: (eventId: number) => void;
    };
  }
}
