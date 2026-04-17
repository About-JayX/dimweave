import type { Attachment } from "@/types";

export function hasMessagePayload(
  message: string,
  attachments?: Attachment[],
): boolean {
  return message.trim().length > 0 || Boolean(attachments?.length);
}
