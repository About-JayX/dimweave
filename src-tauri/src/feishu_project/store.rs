use super::types::FeishuProjectInboxItem;
use std::path::{Path, PathBuf};

/// In-memory store for Feishu Project inbox items, keyed by `work_item_id`.
#[derive(Debug, Clone, Default)]
pub struct FeishuProjectStore {
    pub items: Vec<FeishuProjectInboxItem>,
}

impl FeishuProjectStore {
    /// Insert or update an item. Matches on `work_item_id`; updates in place if found.
    /// Returns `true` if this was a new item (inserted), `false` if updated in place.
    pub fn upsert(&mut self, incoming: FeishuProjectInboxItem) -> bool {
        if let Some(existing) = self
            .items
            .iter_mut()
            .find(|i| i.work_item_id == incoming.work_item_id)
        {
            existing.title = incoming.title;
            existing.status_label = incoming.status_label;
            existing.assignee_label = incoming.assignee_label;
            existing.updated_at = incoming.updated_at;
            existing.source_url = incoming.source_url;
            existing.raw_snapshot_ref = incoming.raw_snapshot_ref;
            existing.last_ingress = incoming.last_ingress;
            existing.last_event_uuid = incoming.last_event_uuid;
            // Preserve: record_id, ignored, linked_task_id
            false
        } else {
            self.items.push(incoming);
            true
        }
    }

    pub fn find_by_work_item_id(&self, id: &str) -> Option<&FeishuProjectInboxItem> {
        self.items.iter().find(|i| i.work_item_id == id)
    }

    pub fn find_by_work_item_id_mut(&mut self, id: &str) -> Option<&mut FeishuProjectInboxItem> {
        self.items.iter_mut().find(|i| i.work_item_id == id)
    }

    /// Full-refresh reconciliation: upsert all incoming items (preserving local
    /// state like `ignored` / `linked_task_id`), then remove items absent from
    /// the incoming set. Returns work_item_ids of newly inserted items.
    pub fn sync_replace(&mut self, incoming: Vec<FeishuProjectInboxItem>) -> Vec<String> {
        let incoming_ids: std::collections::HashSet<&str> =
            incoming.iter().map(|i| i.work_item_id.as_str()).collect();
        // Remove items no longer present in remote
        self.items.retain(|i| incoming_ids.contains(i.work_item_id.as_str()));
        // Upsert incoming (preserves ignored / linked_task_id for retained items)
        incoming
            .into_iter()
            .filter_map(|item| {
                let wid = item.work_item_id.clone();
                if self.upsert(item) { Some(wid) } else { None }
            })
            .collect()
    }

    pub fn set_ignored(&mut self, work_item_id: &str, ignored: bool) -> bool {
        if let Some(item) = self.find_by_work_item_id_mut(work_item_id) {
            item.ignored = ignored;
            true
        } else {
            false
        }
    }
}

// ── Persistence ──────────────────────────────────────────────────────────────

pub fn default_store_path() -> anyhow::Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    Ok(base
        .join("com.dimweave.app")
        .join("feishu_project_inbox.json"))
}

pub fn load_store(path: &Path) -> anyhow::Result<FeishuProjectStore> {
    if !path.exists() {
        return Ok(FeishuProjectStore::default());
    }
    let data = std::fs::read_to_string(path)?;
    let items: Vec<FeishuProjectInboxItem> = serde_json::from_str(&data)?;
    Ok(FeishuProjectStore { items })
}

pub fn save_store(path: &Path, store: &FeishuProjectStore) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&store.items)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feishu_project::types::IngressSource;

    fn sample_item() -> FeishuProjectInboxItem {
        FeishuProjectInboxItem {
            record_id: "rec_1".into(),
            project_key: "proj".into(),
            work_item_id: "1001".into(),
            work_item_type_key: "bug".into(),
            title: "Crash on launch".into(),
            status_label: Some("Open".into()),
            assignee_label: Some("alice".into()),
            updated_at: 10,
            source_url: "https://feishu.cn/proj/1001".into(),
            raw_snapshot_ref: "snap_1.json".into(),
            ignored: false,
            linked_task_id: None,
            last_ingress: IngressSource::Poll,
            last_event_uuid: None,
        }
    }

    #[test]
    fn upsert_inserts_new_item() {
        let mut store = FeishuProjectStore::default();
        store.upsert(sample_item());
        assert_eq!(store.items.len(), 1);
        assert_eq!(store.items[0].title, "Crash on launch");
    }

    #[test]
    fn upsert_updates_existing_record_instead_of_appending() {
        let mut store = FeishuProjectStore::default();
        store.upsert(sample_item());
        store.upsert(FeishuProjectInboxItem {
            title: "Crash on launch (updated)".into(),
            status_label: Some("In Progress".into()),
            updated_at: 20,
            ..sample_item()
        });
        assert_eq!(store.items.len(), 1);
        assert_eq!(store.items[0].title, "Crash on launch (updated)");
        assert_eq!(store.items[0].status_label.as_deref(), Some("In Progress"));
        assert_eq!(store.items[0].updated_at, 20);
    }

    #[test]
    fn upsert_preserves_ignored_and_linked_task_id() {
        let mut store = FeishuProjectStore::default();
        let mut item = sample_item();
        item.ignored = true;
        item.linked_task_id = Some("task_42".into());
        store.upsert(item);

        store.upsert(FeishuProjectInboxItem {
            title: "Updated title".into(),
            updated_at: 30,
            ..sample_item()
        });
        assert_eq!(store.items.len(), 1);
        assert!(store.items[0].ignored);
        assert_eq!(store.items[0].linked_task_id.as_deref(), Some("task_42"));
        assert_eq!(store.items[0].title, "Updated title");
    }

    #[test]
    fn set_ignored_toggles_flag() {
        let mut store = FeishuProjectStore::default();
        store.upsert(sample_item());
        assert!(store.set_ignored("1001", true));
        assert!(store.items[0].ignored);
        assert!(store.set_ignored("1001", false));
        assert!(!store.items[0].ignored);
    }

    #[test]
    fn set_ignored_returns_false_for_unknown() {
        let mut store = FeishuProjectStore::default();
        assert!(!store.set_ignored("9999", true));
    }

    #[test]
    fn sync_replace_preserves_local_state_for_retained_items() {
        let mut store = FeishuProjectStore::default();
        let mut item = sample_item();
        item.ignored = true;
        item.linked_task_id = Some("task_42".into());
        store.upsert(item);

        // Sync with updated title but same work_item_id
        let refreshed = FeishuProjectInboxItem {
            title: "Crash on launch v2".into(),
            updated_at: 20,
            ..sample_item()
        };
        let new_ids = store.sync_replace(vec![refreshed]);
        assert!(new_ids.is_empty(), "existing item should not be 'new'");
        assert_eq!(store.items.len(), 1);
        assert!(store.items[0].ignored);
        assert_eq!(store.items[0].linked_task_id.as_deref(), Some("task_42"));
        assert_eq!(store.items[0].title, "Crash on launch v2");
    }

    #[test]
    fn sync_replace_removes_absent_items() {
        let mut store = FeishuProjectStore::default();
        store.upsert(sample_item()); // work_item_id = "1001"
        store.upsert(FeishuProjectInboxItem {
            work_item_id: "1002".into(),
            title: "Second bug".into(),
            ..sample_item()
        });
        assert_eq!(store.items.len(), 2);

        // Sync only returns "1002" — "1001" should be removed
        let kept = FeishuProjectInboxItem {
            work_item_id: "1002".into(),
            title: "Second bug (updated)".into(),
            ..sample_item()
        };
        let new_ids = store.sync_replace(vec![kept]);
        assert!(new_ids.is_empty());
        assert_eq!(store.items.len(), 1);
        assert_eq!(store.items[0].work_item_id, "1002");
        assert!(store.find_by_work_item_id("1001").is_none());
    }

    #[test]
    fn sync_replace_adds_new_items() {
        let mut store = FeishuProjectStore::default();
        store.upsert(sample_item()); // "1001"

        let existing = sample_item();
        let new_item = FeishuProjectInboxItem {
            work_item_id: "1003".into(),
            title: "Brand new bug".into(),
            ..sample_item()
        };
        let new_ids = store.sync_replace(vec![existing, new_item]);
        assert_eq!(new_ids, vec!["1003"]);
        assert_eq!(store.items.len(), 2);
        assert!(store.find_by_work_item_id("1001").is_some());
        assert!(store.find_by_work_item_id("1003").is_some());
    }

    #[test]
    fn store_round_trip_preserves_items() {
        let path = std::env::temp_dir().join(format!(
            "dimweave_fp_store_rt_{}_{}.json",
            std::process::id(),
            chrono::Utc::now().timestamp_millis(),
        ));
        let mut store = FeishuProjectStore::default();
        store.upsert(sample_item());
        store.upsert(FeishuProjectInboxItem {
            record_id: "rec_2".into(),
            work_item_id: "1002".into(),
            title: "Second bug".into(),
            ..sample_item()
        });
        save_store(&path, &store).unwrap();
        let loaded = load_store(&path).unwrap();
        assert_eq!(loaded.items.len(), 2);
        assert_eq!(loaded.items[0].work_item_id, "1001");
        assert_eq!(loaded.items[1].work_item_id, "1002");
        let _ = std::fs::remove_file(path);
    }
}
