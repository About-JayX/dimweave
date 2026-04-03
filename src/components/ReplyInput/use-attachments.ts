import { useState, useCallback } from "react";
import type { Attachment } from "@/types";

function fileNameFromPath(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

export function useAttachments() {
  const [attachments, setAttachments] = useState<Attachment[]>([]);

  const addFiles = useCallback((paths: string[]) => {
    setAttachments((prev) => [
      ...prev,
      ...paths.map((p) => ({ filePath: p, fileName: fileNameFromPath(p) })),
    ]);
  }, []);

  const removeAt = useCallback((index: number) => {
    setAttachments((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const clear = useCallback(() => setAttachments([]), []);

  return { attachments, addFiles, removeAt, clear } as const;
}
