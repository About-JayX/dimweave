import { invoke } from "@tauri-apps/api/core";

export async function collectPastedAttachmentPaths(
  readPasteAttachments: () => Promise<string[]> = () =>
    invoke<string[]>("read_paste_attachments"),
): Promise<string[]> {
  try {
    return (await readPasteAttachments()).filter((path) => path.trim().length > 0);
  } catch (error) {
    console.error("[ReplyInput] paste attachments failed", error);
    return [];
  }
}
