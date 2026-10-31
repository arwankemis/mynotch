use std::path::Path;
use crate::core::Settings;
use super::ModManifest;
use super::package::{PackageKind, PackageManager};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogFilter {
    All,
    Installed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CatalogEntry {
    pub manifest: ModManifest,
    pub available: bool,
    pub is_installed: bool,
    pub is_active: bool,
    pub is_tab_enabled: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub entries: Vec<CatalogEntry>,
}

impl Catalog {
    pub fn filter(&self, kind: PackageKind, filter: CatalogFilter, query: &str) -> Vec<CatalogEntry> {
        let q = query.trim().to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.manifest.package_kind == kind)
            .filter(|e| match filter {
                CatalogFilter::All => true,
                CatalogFilter::Installed => e.is_installed,
            })
            .filter(|e| {
                if q.is_empty() {
                    true
                } else {
                    e.manifest.name.to_lowercase().contains(&q)
                        || e.manifest.description.to_lowercase().contains(&q)
                        || e.manifest.id.to_lowercase().contains(&q)
                }
            })
            .cloned()
            .collect()
    }
}

pub fn get_catalog(root: &Path, settings: &Settings) -> Catalog {
    let pm = PackageManager::new(root);
    let mut entries = Vec::new();

    // 1. Official Daya Mod package (available locally or installable)
    let daya_installed = pm.is_installed(settings, "daya");
    let daya_manifest = ModManifest {
        schema: 2,
        id: "daya".into(),
        publisher: "mynotch".into(),
        name: "Daya".into(),
        package_kind: PackageKind::Mod,
        mod_type: super::ModType::Lua,
        entry: Some("main.lua".into()),
        version: "0.3.0".into(),
        min_host_api: 2,
        profiles: vec!["public".into(), "personal".into()],
        permissions: vec!["power.read".into(), "storage.self".into()],
        optional_dependencies: vec!["power-meter-provider".into()],
        tab_name: "Daya".into(),
        default_tab_enabled: true,
        description: "Sensor CPU/GPU bawaan dan energi komponen teramati. Sensor CPU tertentu memerlukan izin admin; listrik seluruh PC membutuhkan meter.".into(),
        author: "mynotch".into(),
        icon: "zap".into(),
        endpoint: None,
        cards: vec![],
        buttons: vec![],
    };
    entries.push(CatalogEntry {
        manifest: daya_manifest,
        available: true,
        is_installed: daya_installed,
        is_active: pm.is_active(settings, "daya"),
        is_tab_enabled: pm.is_tab_enabled(settings, "Daya"),
    });

    for m in super::available_mods(root) {
        if m.id == "daya" {
            continue;
        }
        let is_inst = pm.is_installed(settings, &m.id);
        let is_act = pm.is_active(settings, &m.id);
        let is_tab = pm.is_tab_enabled(settings, &m.tab_name);
        entries.push(CatalogEntry {
            manifest: m,
            available: true,
            is_installed: is_inst,
            is_active: is_act,
            is_tab_enabled: is_tab,
        });
    }

    // 3. Planned Extensions (meter provider, hardware telemetry)
    entries.push(CatalogEntry {
        manifest: ModManifest {
            schema: 2,
            id: "meter".into(),
            publisher: "mynotch".into(),
            name: "Provider meter".into(),
            package_kind: PackageKind::Extension,
            mod_type: super::ModType::Builtin,
            entry: None,
            version: "0.1.0-planned".into(),
            min_host_api: 2,
            profiles: vec!["public".into(), "personal".into()],
            permissions: vec!["meter.read".into()],
            optional_dependencies: vec![],
            tab_name: "".into(),
            default_tab_enabled: false,
            description: "Pengukuran arus & tegangan via smart plug / hardware meter".into(),
            author: "mynotch".into(),
            icon: "sliders".into(),
            endpoint: None,
            cards: vec![],
            buttons: vec![],
        },
        available: false, // Planned: cannot be installed yet
        is_installed: false,
        is_active: false,
        is_tab_enabled: false,
    });

    entries.push(CatalogEntry {
        manifest: ModManifest {
            schema: 2,
            id: "telemetry".into(),
            publisher: "mynotch".into(),
            name: "Hardware telemetry".into(),
            package_kind: PackageKind::Extension,
            mod_type: super::ModType::Builtin,
            entry: None,
            version: "0.1.0-planned".into(),
            min_host_api: 2,
            profiles: vec!["public".into(), "personal".into()],
            permissions: vec!["hardware.telemetry".into()],
            optional_dependencies: vec![],
            tab_name: "".into(),
            default_tab_enabled: false,
            description: "Telemetri daya komponen CPU dan GPU internal".into(),
            author: "mynotch".into(),
            icon: "sliders".into(),
            endpoint: None,
            cards: vec![],
            buttons: vec![],
        },
        available: false, // Planned: cannot be installed yet
        is_installed: false,
        is_active: false,
        is_tab_enabled: false,
    });

    Catalog { entries }
}
