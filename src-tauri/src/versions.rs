use serde::Serialize;
use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::detection::DetectionSnapshot;
use crate::homebrew;
use crate::storage::OwnershipRecord;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VersionInstance {
    pub source: String,
    pub path: String,
    pub health: String,
    pub managed_by: String,
}

#[derive(Serialize, Clone, Debug)]
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
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VersionCatalog {
    pub software_id: String,
    pub versions: Vec<VersionRow>,
    pub error: Option<String>,
    pub homebrew_available: bool,
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

pub fn catalog(software_id: &str, snapshot: &DetectionSnapshot, ownership: Option<OwnershipRecord>) -> VersionCatalog {
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
        }
    };
    for instance in snapshot.instances.iter().filter(|instance| instance.software_id == software_id) {
        let version = instance.version.clone().unwrap_or_else(|| "版本未知".into());
        let row = rows.entry(version.clone()).or_insert_with(|| ensure(&version));
        row.installed = true;
        row.instances.push(VersionInstance {
            source: instance.source.clone(),
            path: instance.path.clone(),
            health: instance.health.clone(),
            managed_by: instance.managed_by.clone(),
        });
    }
    let (formulae, error) = homebrew::available(software_id);
    for formula in formulae {
        if !formula.installed.is_empty() {
            let formula_marker = format!("/Cellar/{}/", formula.name);
            let opt_marker = format!("/opt/{}/", formula.name);
            let existing = rows.iter().find(|(_, row)| row.installed && row.instances.iter().any(|instance| instance.path.contains(&formula_marker) || instance.path.contains(&opt_marker))).map(|(version, _)| version.clone());
            if let Some(version) = existing {
                if let Some(row) = rows.get_mut(&version) {
                    row.formula = Some(formula.name.clone());
                }
            } else {
                for version in &formula.installed {
                    let row = rows.entry(version.clone()).or_insert_with(|| ensure(version));
                    row.installed = true;
                    row.formula = Some(formula.name.clone());
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
    if let Some(ownership) = ownership {
        let target = rows.iter().find(|(_, row)| {
            row.installed && (row.version == ownership.version.clone().unwrap_or_default()
                || row.instances.iter().any(|instance| Some(instance.path.as_str()) == ownership.instance_path.as_deref()))
        }).map(|(version, _)| version.clone())
            .or_else(|| rows.iter().find(|(_, row)| row.installed && row.formula.is_some()).map(|(version, _)| version.clone()));
        if let Some(version) = target {
            if let Some(row) = rows.get_mut(&version) {
                row.can_uninstall = true;
                row.formula = Some(ownership.package.clone());
            }
        }
    }
    let mut versions: Vec<VersionRow> = rows.into_values().collect();
    for row in &mut versions {
        if row.installed { row.installable = false; }
    }
    versions.sort_by(|a, b| compare_versions(&b.version, &a.version));
    VersionCatalog {
        software_id: software_id.into(),
        versions,
        error,
        homebrew_available: homebrew::probe().is_ok(),
    }
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
        let catalog = catalog("git", &snapshot(vec![instance("2.50.1", "path", "/usr/bin/git"), instance("2.50.1", "devhubV1", "/tmp/v1/git")]), None);
        let row = catalog.versions.iter().find(|row| row.version == "2.50.1").expect("row");
        assert_eq!(row.instances.len(), 2);
        assert!(row.installed);
    }

    #[test]
    #[ignore]
    fn live_version_catalogs() {
        let snapshot = crate::detection::scan();
        for id in ["git", "go", "maven", "gradle", "node", "python", "jdk", "rust"] {
            println!("### {id}");
            let catalog = catalog(id, &snapshot, None);
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
