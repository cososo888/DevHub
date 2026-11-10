use std::fs;
use std::path::Path;

use rusqlite::{params, Connection};

use crate::detection::DetectionSnapshot;

pub fn open(dir: &Path) -> Result<Connection, String> {
    fs::create_dir_all(dir).map_err(|error| format!("无法创建 V2 数据目录：{error}"))?;
    let connection = Connection::open(dir.join("devhub-v2.sqlite3"))
        .map_err(|error| format!("无法打开 V2 数据库：{error}"))?;
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS detection_cache (
               id INTEGER PRIMARY KEY CHECK (id = 1),
               payload TEXT NOT NULL,
               refreshed_at INTEGER NOT NULL,
               stale INTEGER NOT NULL DEFAULT 0
             );",
        )
        .map_err(|error| format!("无法初始化 V2 数据库：{error}"))?;
    connection
        .pragma_update(None, "user_version", 1)
        .map_err(|error| format!("无法设置 V2 数据库版本：{error}"))?;
    Ok(connection)
}

pub fn load(connection: &Connection) -> Option<DetectionSnapshot> {
    let row: Option<(String, i64)> = connection
        .query_row("SELECT payload, refreshed_at FROM detection_cache WHERE id = 1", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .ok();
    let (payload, refreshed_at) = row?;
    let mut snapshot: DetectionSnapshot = serde_json::from_str(&payload).ok()?;
    snapshot.stale = true;
    snapshot.refreshed_at = Some(refreshed_at);
    Some(snapshot)
}

pub fn save(connection: &Connection, snapshot: &DetectionSnapshot) -> Result<(), String> {
    let payload = serde_json::to_string(snapshot).map_err(|error| format!("无法序列化检测快照：{error}"))?;
    connection
        .execute(
            "INSERT INTO detection_cache (id, payload, refreshed_at, stale)
             VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET payload = excluded.payload, refreshed_at = excluded.refreshed_at, stale = excluded.stale",
            params![payload, snapshot.refreshed_at.unwrap_or_else(|| 0), if snapshot.stale { 1 } else { 0 }],
        )
        .map_err(|error| format!("无法保存检测快照：{error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detection::{DetectionError, HomebrewStatus};

    #[test]
    fn cache_round_trip_is_loaded_as_stale() {
        let connection = Connection::open_in_memory().expect("memory db");
        connection.execute_batch("CREATE TABLE detection_cache (id INTEGER PRIMARY KEY, payload TEXT NOT NULL, refreshed_at INTEGER NOT NULL, stale INTEGER NOT NULL DEFAULT 0);").expect("schema");
        let snapshot = DetectionSnapshot {
            schema_version: 1,
            refreshed_at: Some(1234),
            stale: false,
            duration_ms: 7,
            homebrew: HomebrewStatus { available: false, prefix: None, version: None, error: None },
            instances: Vec::new(),
            errors: vec![DetectionError { source: "test".into(), message: "ok".into() }],
        };
        save(&connection, &snapshot).expect("save");
        let loaded = load(&connection).expect("load");
        assert!(loaded.stale);
        assert_eq!(loaded.refreshed_at, Some(1234));
        assert_eq!(loaded.errors.len(), 1);
    }
}
