use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::detection::DetectionSnapshot;
use crate::homebrew;
use crate::jdk;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VersionInstance {
    pub source: String,
    pub path: String,
    pub health: String,
    pub managed_by: String,
    pub can_uninstall: bool,
    pub mechanism: Option<String>,
    pub reason: Option<String>,
    pub root_path: Option<String>,
    pub manager: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VersionRow {
    pub version: String,
    pub installed: bool,
    pub installable: bool,
    pub can_uninstall: bool,
    pub methods: Vec<String>,
    pub formula: Option<String>,
    pub instances: Vec<VersionInstance>,
    pub reason: Option<String>,
    pub uninstall_reason: Option<String>,
    pub historical: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VersionCatalog {
    pub software_id: String,
    pub versions: Vec<VersionRow>,
    pub error: Option<String>,
    pub homebrew_available: bool,
    pub stale: bool,
}

fn compare_versions(left: &str, right: &str) -> Ordering {
    let parse = |value: &str| value.split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect::<Vec<_>>();
    let a = parse(left);
    let b = parse(right);
    for index in 0..a.len().max(b.len()) {
        let av = a.get(index).copied().unwrap_or(0);
        let bv = b.get(index).copied().unwrap_or(0);
        match av.cmp(&bv) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    left.cmp(right)
}

fn manager_for(software_id: &str, path: &str) -> (String, Option<String>) {
    if path.contains("/Application Support/com.yujian.devhub") {
        return ("旧版 DevHub".into(), Some("旧版 DevHub 受管安装保持只读".into()));
    }
    if path.contains("/Cellar/") || path.contains("/opt/homebrew/") || path.contains("/usr/local/Cellar/") {
        return ("Homebrew".into(), None);
    }
    if path.contains("/anaconda3/") || path.contains("/miniconda3/") {
        return ("Conda (base)".into(), Some("Conda base 环境不支持自动卸载".into()));
    }
    if path.contains("/.cargo/") || path.contains("/.rustup/") {
        return ("rustup".into(), Some("当前 rustup 活动工具链，暂不支持自动卸载".into()));
    }
    if path.starts_with("/usr/bin/") || path.starts_with("/bin/") {
        return ("macOS 系统".into(), Some("macOS 系统软件不可卸载".into()));
    }
    if path.starts_with("/Applications/") {
        return ("官方应用".into(), Some("应用包未提供安全的文件级卸载".into()));
    }
    if path.starts_with("/usr/local/mysql") {
        return ("MySQL 官方安装".into(), Some("官方安装未提供受控卸载".into()));
    }
    if path.starts_with("/usr/local/maven") || path.contains("/apache-maven-") {
        return ("手动 Maven 安装".into(), Some("无法确认完整安装边界与共享引用".into()));
    }
    if path.contains("/Library/Frameworks/Python.framework/") {
        return ("Python.org 官方安装".into(), Some("官方框架安装未提供受控卸载".into()));
    }
    if path.starts_with("/usr/local/bin/node") {
        return ("独立 Node 安装".into(), Some("无法确认所属安装管理器".into()));
    }
    if software_id == "mysql" {
        return ("MySQL 官方安装".into(), Some("官方安装未提供受控卸载".into()));
    }
    ("未知来源".into(), Some("无法确认完整安装范围与安全删除方式".into()))
}

pub fn catalog(software_id: &str, snapshot: &DetectionSnapshot) -> VersionCatalog {
    let mut rows: BTreeMap<String, VersionRow> = BTreeMap::new();
    let ensure = |version: &str| -> VersionRow {
        VersionRow {
            version: version.to_string(),
            installed: false,
            installable: false,
            can_uninstall: false,
            methods: Vec::new(),
            formula: None,
            instances: Vec::new(),
            reason: None,
            uninstall_reason: None,
            historical: false,
        }
    };
    for instance in snapshot.instances.iter().filter(|instance| instance.software_id == software_id) {
        let version = instance.version.clone().unwrap_or_else(|| "版本未知".into());
        let row = rows.entry(version.clone()).or_insert_with(|| ensure(&version));
        row.installed = true;
        let jdk_info = if software_id == "jdk" { Some(jdk::analyze(&instance.path)) } else { None };
        row.instances.push(VersionInstance {
            source: instance.source.clone(),
            path: instance.path.clone(),
            health: instance.health.clone(),
            managed_by: instance.managed_by.clone(),
            can_uninstall: jdk_info.as_ref().map(|value| value.can_uninstall).unwrap_or(false),
            mechanism: jdk_info.as_ref().filter(|value| value.can_uninstall).map(|_| "jdkTrash".to_string()),
            reason: jdk_info.as_ref().and_then(|value| value.reason.clone()),
            root_path: jdk_info.as_ref().map(|value| value.root.clone()).filter(|value| !value.is_empty()),
            manager: jdk_info.as_ref().map(|value| if value.vendor.is_empty() { "JDK".into() } else { value.vendor.clone() }).unwrap_or_else(|| manager_for(software_id, &instance.path).0),
        });
        if jdk_info.as_ref().map(|value| value.can_uninstall).unwrap_or(false) {
            row.can_uninstall = true;
        }
        if jdk_info.is_none() {
            let (manager, manager_reason) = manager_for(software_id, &instance.path);
            let last = row.instances.last_mut().expect("instance just pushed");
            last.manager = manager;
            if last.reason.is_none() { last.reason = manager_reason; }
        }
    }
    let (formulae, error) = homebrew::available(software_id);
    for formula in formulae {
        if !formula.installed.is_empty() {
            let formula_marker = format!("/Cellar/{}/", formula.name);
            let opt_marker = format!("/opt/{}/", formula.name);
            let existing = rows.iter().find(|(_, row)| row.installed && row.instances.iter().any(|instance| instance.path.contains(&formula_marker) || instance.path.contains(&opt_marker))).map(|(version, _)| version.clone());
            let mut keys = Vec::new();
            if let Some(version) = existing {
                if let Some(row) = rows.get_mut(&version) {
                    row.formula = Some(formula.name.clone());
                }
                keys.push(version);
            } else {
                for version in &formula.installed {
                    let row = rows.entry(version.clone()).or_insert_with(|| ensure(version));
                    row.installed = true;
                    row.formula = Some(formula.name.clone());
                    keys.push(version.clone());
                }
            }
            let dependents = homebrew::dependents(&formula.name);
            let has_instance = keys.iter().any(|key| rows.get(key).map(|row| !row.instances.is_empty()).unwrap_or(false));
            let blocked = match &dependents {
                Ok(values) if values.is_empty() => None,
                Ok(values) => Some(format!("被其他软件依赖：{}", values.join("、"))),
                Err(_) => Some("无法确认依赖影响，暂不提供卸载".into()),
            };
            for key in keys {
                if let Some(row) = rows.get_mut(&key) {
                    if has_instance && blocked.is_none() {
                        row.can_uninstall = true;
                    } else {
                        row.uninstall_reason = blocked.clone().or_else(|| Some("未能定位可安全卸载的安装实例".into()));
                    }
                }
            }
            continue;
        }
        let already_installed = rows.get(&formula.version).map(|row| row.installed).unwrap_or(false);
        if already_installed { continue; }
        let row = rows.entry(formula.version.clone()).or_insert_with(|| ensure(&formula.version));
        row.installable = true;
        row.formula = Some(formula.name.clone());
        row.methods = vec!["homebrew".into()];
        if !formula.dependencies.is_empty() {
            row.reason = Some("安装时可能拉取依赖".into());
        }
    }
    let mut versions: Vec<VersionRow> = rows.into_values().collect();
    for row in &mut versions {
        if row.installed {
            row.installable = false;
            if !row.can_uninstall && row.uninstall_reason.is_none() {
                row.uninstall_reason = row.instances.iter().find_map(|instance| instance.reason.clone())
                    .or_else(|| Some("未识别到可安全卸载的安装管理器".into()));
            }
        }
    }
    versions.sort_by(|a, b| compare_versions(&b.version, &a.version));
    VersionCatalog {
        software_id: software_id.into(),
        versions,
        error,
        homebrew_available: homebrew::probe().is_ok(),
        stale: false,
    }
}

pub fn historical_row(version: &str, source: &str, instance_path: Option<&str>) -> VersionRow {
    VersionRow {
        version: version.into(),
        installed: false,
        installable: false,
        can_uninstall: false,
        methods: vec![source.into()],
        formula: None,
        instances: instance_path.map(|path| vec![VersionInstance {
            source: source.into(),
            path: path.into(),
            health: "uninstalled".into(),
            managed_by: "external".into(),
            can_uninstall: false,
            mechanism: None,
            reason: Some("该版本已卸载，可重新安装时再提供".into()),
            root_path: None,
            manager: source.into(),
        }]).unwrap_or_default(),
        reason: Some("已卸载；当前没有安全安装渠道".into()),
        uninstall_reason: Some("已卸载；当前没有安全安装渠道".into()),
        historical: true,
    }
}

/// Merge live catalog, last successful cache and uninstall history.
/// Cached versions that are no longer live are shown as historical, never as installable.
pub fn merge_history(mut live: VersionCatalog, cached: Option<&VersionCatalog>, history: &[(String, String, Option<String>, i64)]) -> VersionCatalog {
    use std::collections::HashSet;
    let mut seen: HashSet<String> = live.versions.iter().map(|row| row.version.clone()).collect();
    if let Some(cached) = cached {
        for row in &cached.versions {
            if seen.insert(row.version.clone()) {
                let mut historical = row.clone();
                historical.installed = false;
                historical.installable = false;
                historical.can_uninstall = false;
                historical.historical = true;
                historical.reason = Some("已卸载或当前没有安全安装渠道".into());
                historical.uninstall_reason = Some("已卸载或当前没有安全安装渠道".into());
                live.versions.push(historical);
            }
        }
    }
    for (version, source, path, _) in history {
        if seen.insert(version.clone()) {
            live.versions.push(historical_row(version, source, path.as_deref()));
        }
    }
    if live.error.is_some() {
        for row in &mut live.versions { row.installable = false; }
        live.stale = true;
    }
    live.versions.sort_by(|a, b| compare_versions(&b.version, &a.version));
    live
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detection::{DetectionError, HomebrewStatus, SoftwareInstance};

    fn instance(version: &str, source: &str, path: &str) -> SoftwareInstance {
        SoftwareInstance {
            software_id: "git".into(),
            version: Some(version.into()),
            source: source.into(),
            sources: vec![source.into()],
            path: path.into(),
            kind: "cli".into(),
            executable: true,
            managed_by: "system".into(),
            health: "ok".into(),
            detail: None,
            last_detected_at: 1,
        }
    }

    fn snapshot(instances: Vec<SoftwareInstance>) -> DetectionSnapshot {
        DetectionSnapshot {
            schema_version: 1,
            refreshed_at: Some(1),
            stale: false,
            duration_ms: 0,
            homebrew: HomebrewStatus { available: false, prefix: None, version: None, error: None },
            instances,
            errors: vec![DetectionError { source: "test".into(), message: "ok".into() }],
        }
    }

    #[test]
    fn merges_same_version_instances_and_keeps_paths() {
        let catalog = catalog("git", &snapshot(vec![instance("2.50.1", "path", "/usr/bin/git"), instance("2.50.1", "devhubV1", "/tmp/v1/git")]));
        let row = catalog.versions.iter().find(|row| row.version == "2.50.1").expect("row");
        assert_eq!(row.instances.len(), 2);
        assert!(row.installed);
    }

    #[test]
    fn removed_versions_are_kept_as_history_not_installable() {
        let live = VersionCatalog {
            software_id: "jdk".into(),
            versions: vec![VersionRow { version: "21".into(), installed: true, installable: false, can_uninstall: false, methods: vec![], formula: None, instances: vec![], reason: None, uninstall_reason: Some("test".into()), historical: false }],
            error: None,
            homebrew_available: true,
            stale: false,
        };
        let cached = VersionCatalog { versions: vec![VersionRow { version: "17.0.19".into(), installed: true, installable: false, can_uninstall: true, methods: vec![], formula: None, instances: vec![], reason: None, uninstall_reason: None, historical: false }], ..live.clone() };
        let merged = merge_history(live, Some(&cached), &[("17.0.19".into(), "jdkTrash".into(), Some("/tmp/jdk".into()), 1)]);
        let row = merged.versions.iter().find(|row| row.version == "17.0.19").expect("history row");
        assert!(row.historical);
        assert!(!row.installed && !row.installable);
        assert_eq!(merged.versions.iter().filter(|row| row.version == "17.0.19").count(), 1);
    }

    #[test]
    #[ignore]
    fn live_version_catalogs() {
        let snapshot = crate::detection::scan();
        for id in ["git", "go", "maven", "gradle", "node", "python", "jdk", "rust"] {
            println!("### {id}");
            let catalog = catalog(id, &snapshot);
            for row in catalog.versions.iter().filter(|row| row.installable) {
                let resolved = homebrew::resolve_install(id, &row.version);
                assert!(resolved.is_ok(), "{} {} must be resolvable: {:?}", id, row.version, resolved.err());
            }
            println!("{}", serde_json::to_string_pretty(&catalog).expect("catalog json"));
        }
    }

    #[test]
    fn sorts_versions_newest_first() {
        let mut versions = vec!["2.9.0", "2.50.1", "2.10.0"];
        versions.sort_by(|a, b| compare_versions(b, a));
        assert_eq!(versions, vec!["2.50.1", "2.10.0", "2.9.0"]);
    }
}
