use super::*;
use crate::memory::*;
#[test]
fn startup_sync_clock_is_transactional_and_idempotent_reads_never_advance_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("clock.db");
    let mut s = Store::open(&path).unwrap();
    let boot = uuid::Uuid::new_v4();
    let initial = s.sync_status(boot, now_ms().unwrap()).unwrap();
    let input = AddMemory {
        request_id: lain_wire::RequestId::new(),
        draft: MemoryDraft {
            content: "synthetic sync memory".into(),
            kind: MemoryKind::Episodic,
            scope: MemoryScope::Global,
        },
    };
    let record = s.add_memory(&input).unwrap();
    let after = s.sync_revision().unwrap();
    assert!(after > initial.cursor.revision);
    s.memory(record.id).unwrap();
    s.add_memory(&input).unwrap();
    assert_eq!(s.sync_revision().unwrap(), after);
    let tx = s.connection.transaction().unwrap();
    tx.execute(
        "INSERT INTO lain_conversations VALUES(?1,1,1)",
        [uuid::Uuid::new_v4().to_string()],
    )
    .unwrap();
    tx.rollback().unwrap();
    assert_eq!(s.sync_revision().unwrap(), after);
    drop(s);
    let s = Store::open(&path).unwrap();
    assert_eq!(
        s.sync_status(boot, now_ms().unwrap())
            .unwrap()
            .cursor
            .instance_id,
        initial.cursor.instance_id
    );
    let other = Store::open(&dir.path().join("other.db")).unwrap();
    assert_ne!(
        other
            .sync_status(boot, now_ms().unwrap())
            .unwrap()
            .cursor
            .instance_id,
        initial.cursor.instance_id
    );
}
#[test]
fn startup_sync_deadline_index_retains_future_deadlines_next_to_expired_ones() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("deadline.db")).unwrap();
    s.connection
        .execute_batch("CREATE TABLE deadline_fixture(snapshot TEXT NOT NULL);")
        .unwrap();
    let tx = s.connection.transaction().unwrap();
    install(&tx).unwrap();
    tx.commit().unwrap();
    let now = now_ms().unwrap();
    let future = now + 1500;
    s.connection.execute("INSERT INTO deadline_fixture VALUES(?1)",[serde_json::json!({"old":{"expires_at_ms":now-1},"current":{"expires_at_ms":future},"source":{"fresh_until_ms":future+1000}}).to_string()]).unwrap();
    let boot = uuid::Uuid::new_v4();
    assert_eq!(s.sync_status(boot, now).unwrap().valid_until_ms, future);
    assert_eq!(
        s.sync_status(boot, future + 1).unwrap().valid_until_ms,
        future + 1000
    );
    s.connection
        .execute("DELETE FROM deadline_fixture", [])
        .unwrap();
    assert_eq!(
        s.sync_status(boot, now).unwrap().valid_until_ms,
        now + MAX_LEASE_MS
    );
}
