//! Disposable owner-only connection/resource metadata. No messages or credentials.
use anyhow::{Result, ensure};
use lain_wire::{ConversationId, client_sync::*};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;
const MAX_BYTES: u64 = 16 * 1024;
const HINT_RETENTION_MS: u64 = 7 * 24 * 60 * 60 * 1000;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    version: u32,
    role: String,
    endpoint: String,
    instance_id: Uuid,
    saved_at_ms: u64,
    resources_until_ms: u64,
    pub conversation_id: ConversationId,
    resources: Resources,
}
pub struct Cache {
    root: PathBuf,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(u64::MAX, |d| d.as_millis().try_into().unwrap_or(u64::MAX))
}
fn known(cap: &str) -> bool {
    matches!(
        cap,
        "client_sync/1" | "memory_context/1" | "device_work/1" | "source_guarded_publication/1"
    )
}
fn valid_resources(r: &Resources) -> bool {
    r.protocol == PROTOCOL
        && r.capabilities.len() <= 4
        && r.capabilities.iter().all(|c| known(c))
        && [
            &r.configuration_revision,
            &r.persona_revision,
            &r.policy_revision,
        ]
        .iter()
        .all(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
}
impl Cache {
    pub fn open(root: PathBuf) -> Result<Self> {
        ensure!(root.is_absolute(), "cache directory must be absolute");
        ensure!(
            !root.ancestors().any(|p| p.join(".git").exists()),
            "cache must be outside Git"
        );
        if fs::symlink_metadata(&root).is_err() {
            if let Some(parent) = root.parent() {
                fs::create_dir_all(parent)?;
            }
            match fs::create_dir(&root) {
                Ok(()) => lain_local::private(&root, true, true)?,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e.into()),
            }
        }
        lain_local::private(&root, true, false)?;
        lain_local::outside_checkout(&root)?;
        Ok(Self { root })
    }
    fn path(&self) -> PathBuf {
        self.root.join("client-sync-cache.json")
    }
    pub fn load(&self, endpoint: &str, instance_id: Uuid) -> Result<Option<Record>> {
        let path = self.path();
        if fs::symlink_metadata(&path).is_err() {
            return Ok(None);
        }
        lain_local::private(&self.root, true, false)?;
        lain_local::private(&path, false, false)?;
        ensure!(
            fs::metadata(&path)?.len() <= MAX_BYTES,
            "cache exceeds size bound"
        );
        let Ok(record) = serde_json::from_slice::<Record>(&fs::read(path)?) else {
            return Ok(None);
        };
        let time = now();
        if record.version != 1
            || record.role != "owner"
            || record.endpoint != endpoint
            || record.instance_id != instance_id
            || record.instance_id.is_nil()
            || record.conversation_id.0.is_nil()
            || time < record.saved_at_ms
            || time.saturating_sub(record.saved_at_ms) > HINT_RETENTION_MS
            || !valid_resources(&record.resources)
        {
            return Ok(None);
        }
        Ok(Some(record))
    }
    pub fn save(&self, endpoint: &str, page: &BootstrapSnapshot, remaining_ms: u64) -> Result<()> {
        page.validate().map_err(anyhow::Error::msg)?;
        ensure!(endpoint.len() <= 2048, "cache endpoint too long");
        let Some(conversation_id) = page.conversation_id else {
            return Ok(());
        };
        let mut resources = page.resources.clone();
        resources.capabilities.retain(|c| known(c));
        ensure!(valid_resources(&resources), "unsupported cache resources");
        let saved_at_ms = now();
        let record = Record {
            version: 1,
            role: "owner".into(),
            endpoint: endpoint.into(),
            instance_id: page.cursor.instance_id,
            saved_at_ms,
            resources_until_ms: saved_at_ms.saturating_add(remaining_ms.min(MAX_LEASE_MS)),
            conversation_id,
            resources,
        };
        let bytes = serde_json::to_vec(&record)?;
        ensure!(bytes.len() as u64 <= MAX_BYTES, "cache exceeds size bound");
        lain_local::atomic_private(&self.path(), &bytes)
    }
    pub fn invalidate(&self) -> Result<()> {
        let path = self.path();
        if fs::symlink_metadata(&path).is_ok() {
            lain_local::private(&self.root, true, false)?;
            lain_local::private(&path, false, false)?;
            fs::remove_file(path)?;
        }
        Ok(())
    }
    pub fn expire_resources(&self) -> Result<()> {
        let path = self.path();
        if fs::symlink_metadata(&path).is_err() {
            return Ok(());
        }
        lain_local::private(&self.root, true, false)?;
        lain_local::private(&path, false, false)?;
        ensure!(
            fs::metadata(&path)?.len() <= MAX_BYTES,
            "cache exceeds size bound"
        );
        let Ok(mut record) = serde_json::from_slice::<Record>(&fs::read(&path)?) else {
            return self.invalidate();
        };
        record.resources_until_ms = 0;
        lain_local::atomic_private(&path, &serde_json::to_vec(&record)?)
    }
}
impl Record {
    /// Version hints expire independently; no disk record ever makes a view fresh.
    pub fn resource_versions_valid(&self) -> bool {
        let t = now();
        t >= self.saved_at_ms
            && t < self.resources_until_ms
            && self.resources_until_ms - self.saved_at_ms <= MAX_LEASE_MS
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn page() -> BootstrapSnapshot {
        serde_json::from_value(serde_json::json!({"cursor":{"instance_id":Uuid::new_v4(),"boot_id":Uuid::new_v4(),"revision":1},"issued_at_ms":1,"valid_until_ms":300001,"resources":{"protocol":1,"configuration_revision":"ab".repeat(32),"persona_revision":"bc".repeat(32),"policy_revision":"cd".repeat(32),"capabilities":["client_sync/1","source_guarded_publication/1"]},"conversation_id":ConversationId::new(),"memory_context":null,"turns":[],"history_truncated":false,"devices":[],"projects":[],"mappings":[],"work":[],"work_truncated":false,"notices":[],"notice_cursor":0,"notices_truncated":false})).unwrap()
    }
    #[test]
    fn private_cache_is_bounded_endpoint_instance_bound_and_body_free() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::open(dir.path().join("profile")).unwrap();
        let p = page();
        let endpoint = "https://example.invalid/lain/";
        cache.save(endpoint, &p, 1000).unwrap();
        let r = cache.load(endpoint, p.cursor.instance_id).unwrap().unwrap();
        assert!(r.resource_versions_valid());
        assert_eq!(r.conversation_id, p.conversation_id.unwrap());
        assert!(
            cache
                .load("https://other.invalid/", p.cursor.instance_id)
                .unwrap()
                .is_none()
        );
        assert!(cache.load(endpoint, Uuid::new_v4()).unwrap().is_none());
        let text = fs::read_to_string(cache.path()).unwrap();
        for field in ["messages", "actions", "token", "memory_context", "work"] {
            assert!(!text.contains(&format!("\"{field}\"")));
        }
        cache.save(endpoint, &p, 0).unwrap();
        assert!(
            !cache
                .load(endpoint, p.cursor.instance_id)
                .unwrap()
                .unwrap()
                .resource_versions_valid()
        );
        cache.invalidate().unwrap();
        assert!(!cache.path().exists());
    }
    #[test]
    fn corrupt_cache_can_be_replaced_but_oversized_cache_is_not_read() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::open(dir.path().join("profile")).unwrap();
        lain_local::create_private(&cache.path(), b"broken").unwrap();
        let p = page();
        let e = "http://127.0.0.1:9/";
        assert!(cache.load(e, p.cursor.instance_id).unwrap().is_none());
        cache.save(e, &p, 1000).unwrap();
        assert!(cache.load(e, p.cursor.instance_id).unwrap().is_some());
        lain_local::atomic_private(&cache.path(), &vec![b'x'; MAX_BYTES as usize + 1]).unwrap();
        assert!(cache.load(e, p.cursor.instance_id).is_err());
    }
}
