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
             );
             CREATE TABLE IF NOT EXISTS install_ownership (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               software_id TEXT NOT NULL,
               package TEXT NOT NULL,
               package_type TEXT NOT NULL,
               status TEXT NOT NULL,
               preexisting INTEGER NOT NULL DEFAULT 0,
               installed_at INTEGER NOT NULL,
               uninstalled_at INTEGER,
               instance_path TEXT,
               version TEXT,
               message TEXT
             );
             CREATE TABLE IF NOT EXISTS version_catalog_cache (
               software_id TEXT PRIMARY KEY,
               payload TEXT NOT NULL,
               updated_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS uninstall_history (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               software_id TEXT NOT NULL,
               version TEXT NOT NULL,
               source TEXT NOT NULL,
               instance_path TEXT,
               uninstalled_at INTEGER NOT NULL
             );",
        )
        .map_err(|error| format!("无法初始化 V2 数据库：{error}"))?;
    connection
        .pragma_update(None, "user_version", 2)
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
    fn ownership_records_are_pending_until_verified() {
        let connection = Connection::open_in_memory().expect("memory db");
        connection.execute_batch("CREATE TABLE install_ownership (id INTEGER PRIMARY KEY AUTOINCREMENT, software_id TEXT NOT NULL, package TEXT NOT NULL, package_type TEXT NOT NULL, status TEXT NOT NULL, preexisting INTEGER NOT NULL DEFAULT 0, installed_at INTEGER NOT NULL, uninstalled_at INTEGER, instance_path TEXT, version TEXT, message TEXT);").expect("schema");
        let id = insert_pending(&connection, "git", "git", "formula", false, 100).expect("insert");
        assert!(active_for_version(&connection, "git", "2.51.0").is_none());
        update_install(&connection, id, "success", Some("/opt/homebrew/bin/git"), Some("2.51.0"), None, 101).expect("update");
        let active = active_for_version(&connection, "git", "2.51.0").expect("active");
        assert_eq!(active.version.as_deref(), Some("2.51.0"));
        assert!(!active.preexisting);
        mark_uninstalled(&connection, id, 102).expect("uninstall");
        assert!(active_for_version(&connection, "git", "2.51.0").is_none());
    }

    #[test]
    fn version_cache_and_uninstall_history_round_trip() {
        let connection = Connection::open_in_memory().expect("memory db");
        connection.execute_batch("CREATE TABLE version_catalog_cache (software_id TEXT PRIMARY KEY, payload TEXT NOT NULL, updated_at INTEGER NOT NULL); CREATE TABLE uninstall_history (id INTEGER PRIMARY KEY AUTOINCREMENT, software_id TEXT NOT NULL, version TEXT NOT NULL, source TEXT NOT NULL, instance_path TEXT, uninstalled_at INTEGER NOT NULL);").expect("schema");
        let catalog = crate::versions::VersionCatalog { software_id: "jdk".into(), versions: Vec::new(), error: None, homebrew_available: true, stale: false };
        let payload = serde_json::to_string(&catalog).expect("json");
        save_version_cache(&connection, "jdk", &payload, 10).expect("save");
        let (loaded, updated) = load_version_cache(&connection, "jdk").expect("load");
        assert_eq!(loaded.software_id, "jdk");
        assert_eq!(updated, 10);
        record_uninstall(&connection, "jdk", "17.0.19", "jdkTrash", Some("/tmp/jdk"), 20).expect("history");
        let history = uninstall_history(&connection, "jdk");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].0, "17.0.19");
    }

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

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct OwnershipRecord {
    pub id: i64,
    pub software_id: String,
    pub package: String,
    pub package_type: String,
    pub status: String,
    pub preexisting: bool,
    pub installed_at: i64,
    pub uninstalled_at: Option<i64>,
    pub instance_path: Option<String>,
    pub version: Option<String>,
    pub message: Option<String>,
}

pub fn insert_pending(connection: &Connection, software_id: &str, package: &str, package_type: &str, preexisting: bool, now: i64) -> Result<i64, String> {
    connection
        .execute(
            "INSERT INTO install_ownership (software_id, package, package_type, status, preexisting, installed_at) VALUES (?1, ?2, ?3, 'pending', ?4, ?5)",
            params![software_id, package, package_type, if preexisting { 1 } else { 0 }, now],
        )
        .map_err(|error| format!("无法写入安装归属记录：{error}"))?;
    Ok(connection.last_insert_rowid())
}

pub fn update_install(connection: &Connection, id: i64, status: &str, instance_path: Option<&str>, version: Option<&str>, message: Option<&str>, now: i64) -> Result<(), String> {
    connection
        .execute(
            "UPDATE install_ownership SET status = ?1, instance_path = ?2, version = ?3, message = ?4, installed_at = ?5 WHERE id = ?6",
            params![status, instance_path, version, message, now, id],
        )
        .map_err(|error| format!("无法更新安装归属记录：{error}"))?;
    Ok(())
}

pub fn mark_uninstalled(connection: &Connection, id: i64, now: i64) -> Result<(), String> {
    connection
        .execute("UPDATE install_ownership SET uninstalled_at = ?1 WHERE id = ?2", params![now, id])
        .map_err(|error| format!("无法记录卸载：{error}"))?;
    Ok(())
}

pub fn active_for_version(connection: &Connection, software_id: &str, version: &str) -> Option<OwnershipRecord> {
    connection.query_row(
        "SELECT id, software_id, package, package_type, status, preexisting, installed_at, uninstalled_at, instance_path, version, message
         FROM install_ownership WHERE software_id = ?1 AND version = ?2 AND status = 'success' AND uninstalled_at IS NULL ORDER BY id DESC LIMIT 1",
        params![software_id, version],
        |row| Ok(OwnershipRecord {
            id: row.get(0)?,
            software_id: row.get(1)?,
            package: row.get(2)?,
            package_type: row.get(3)?,
            status: row.get(4)?,
            preexisting: row.get::<_, i64>(5)? != 0,
            installed_at: row.get(6)?,
            uninstalled_at: row.get(7)?,
            instance_path: row.get(8)?,
            version: row.get(9)?,
            message: row.get(10)?,
        }),
    ).ok()
}

pub fn save_version_cache(connection: &Connection, software_id: &str, payload: &str, now: i64) -> Result<(), String> {
    connection.execute(
        "INSERT INTO version_catalog_cache (software_id, payload, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(software_id) DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at",
        params![software_id, payload, now],
    ).map_err(|error| format!("无法保存版本目录缓存：{error}"))?;
    Ok(())
}

pub fn load_version_cache(connection: &Connection, software_id: &str) -> Option<(crate::versions::VersionCatalog, i64)> {
    connection.query_row(
        "SELECT payload, updated_at FROM version_catalog_cache WHERE software_id = ?1",
        params![software_id],
        |row| {
            let payload: String = row.get(0)?;
            let updated: i64 = row.get(1)?;
            Ok((payload, updated))
        },
    ).ok().and_then(|(payload, updated)| serde_json::from_str::<crate::versions::VersionCatalog>(&payload).ok().map(|catalog| (catalog, updated)))
}

pub fn record_uninstall(connection: &Connection, software_id: &str, version: &str, source: &str, instance_path: Option<&str>, now: i64) -> Result<(), String> {
    connection.execute(
        "INSERT INTO uninstall_history (software_id, version, source, instance_path, uninstalled_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![software_id, version, source, instance_path, now],
    ).map_err(|error| format!("无法写入卸载历史：{error}"))?;
    Ok(())
}

pub fn uninstall_history(connection: &Connection, software_id: &str) -> Vec<(String, String, Option<String>, i64)> {
    let Ok(mut statement) = connection.prepare("SELECT version, source, instance_path, uninstalled_at FROM uninstall_history WHERE software_id = ?1 ORDER BY id DESC") else { return Vec::new() };
    let Ok(rows) = statement.query_map(params![software_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))) else { return Vec::new() };
    rows.flatten().collect()
}
