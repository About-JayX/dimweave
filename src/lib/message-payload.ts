import type { Attachment } from "@/types";

export function hasMessagePayload(
  content: string,
  attachments?: Attachment[],
): boolean {
  return content.trim().length > 0 || Boolean(attachments?.length);
}
