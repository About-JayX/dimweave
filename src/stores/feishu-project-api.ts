import { invoke } from "@tauri-apps/api/core";
import type { FeishuProjectInboxItem } from "./feishu-project-store";

export interface IssueFilter {
  status?: string | null;
  assignee?: string | null;
}

export async function loadMoreFiltered(filter: IssueFilter): Promise<number> {
  return invoke<number>("feishu_project_load_more_filtered", { filter });
}

export async function fetchFilterOptions(): Promise<void> {
  await invoke("feishu_project_fetch_filter_options");
}

export async function listItems(): Promise<FeishuProjectInboxItem[]> {
  return invoke<FeishuProjectInboxItem[]>("feishu_project_list_items");
}
