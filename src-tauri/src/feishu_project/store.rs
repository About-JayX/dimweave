use super::types::FeishuProjectInboxItem;
use std::path::{Path, PathBuf};

/// In-memory store for Feishu Project inbox items, keyed by `work_item_id`.
#[derive(Debug, Clone, Default)]
pub struct FeishuProjectStore {
    pub items: Vec<FeishuProjectInboxItem>,
}

impl FeishuProjectStore {
    /// Insert or update an item. Matches on `work_item_id`; updates in place if found.
    pub fn upsert(&mut self, incoming: FeishuProjectInboxItem) {
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
        } else {
            self.items.push(incoming);
        }
    }

    pub fn find_by_work_item_id(&self, id: &str) -> Option<&FeishuProjectInboxItem> {
        self.items.iter().find(|i| i.work_item_id == id)
    }

    pub fn find_by_work_item_id_mut(&mut self, id: &str) -> Option<&mut FeishuProjectInboxItem> {
        self.items.iter_mut().find(|i| i.work_item_id == id)
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
