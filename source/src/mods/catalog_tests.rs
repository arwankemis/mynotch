use super::package::*;
use super::catalog::*;
use crate::core::Settings;

fn sample_mod_package() -> crate::mods::ModManifest {
    crate::mods::ModManifest {
        schema: 2,
        id: "daya".into(),
        publisher: "mynotch".into(),
        name: "Daya".into(),
        package_kind: PackageKind::Mod,
        mod_type: crate::mods::ModType::Lua,
        entry: Some("main.lua".into()),
        version: "0.2.0".into(),
        min_host_api: 2,
        profiles: vec!["public".into(), "personal".into()],
        permissions: vec!["power.read".into(), "storage.self".into()],
        optional_dependencies: vec!["power-meter-provider".into()],
        tab_name: "Daya".into(),
        default_tab_enabled: true,
        description: "Monitor konsumsi energi PC".into(),
        author: "mynotch".into(),
        icon: "zap".into(),
        endpoint: None,
        cards: vec![],
        buttons: vec![],
    }
}

#[test]
fn test_package_lifecycle_distinguishes_installed_active_and_tab() {
    let temp_dir = std::env::temp_dir().join(format!("mynotch_test_pm_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let manager = PackageManager::new(&temp_dir);
    let mut settings = Settings::default();
    let pkg = sample_mod_package();

    assert!(!manager.is_installed(&settings, "daya"));
    assert!(!manager.is_active(&settings, "daya"));

    manager.install_package(&mut settings, &pkg, &[("main.lua", b"-- test lua")]).unwrap();
    assert!(manager.is_installed(&settings, "daya"));
    assert!(manager.is_active(&settings, "daya"));
    assert!(manager.is_tab_enabled(&settings, "Daya"));

    // Disabling deactivates without uninstalling
    manager.disable_package(&mut settings, "daya");
    assert!(manager.is_installed(&settings, "daya"));
    assert!(!manager.is_active(&settings, "daya"));

    // Re-enabling
    manager.enable_package(&mut settings, "daya");
    assert!(manager.is_active(&settings, "daya"));

    // Toggling tab off does not deactivate
    settings.mod_tabs.retain(|t| t != "Daya");
    assert!(!manager.is_tab_enabled(&settings, "Daya"));
    assert!(manager.is_active(&settings, "daya"));

    // Uninstalling retains user data folder by default
    let mod_data_dir = temp_dir.join("mod-data").join("daya");
    std::fs::create_dir_all(&mod_data_dir).unwrap();
    std::fs::write(mod_data_dir.join("history.json"), b"{}").unwrap();

    manager.uninstall_package(&mut settings, "daya", true).unwrap();
    assert!(!manager.is_installed(&settings, "daya"));
    assert!(!manager.is_active(&settings, "daya"));
    assert!(mod_data_dir.join("history.json").exists(), "User data must be retained by default!");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_catalog_separates_mods_and_extensions_and_marks_planned() {
    let temp_dir = std::env::temp_dir().join(format!("mynotch_test_cat_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let settings = Settings::default();
    
    let catalog = get_catalog(&temp_dir, &settings);
    let mods = catalog.filter(PackageKind::Mod, CatalogFilter::All, "");
    let extensions = catalog.filter(PackageKind::Extension, CatalogFilter::All, "");

    assert!(mods.iter().any(|p| p.manifest.id == "weather"));
    assert!(mods.iter().any(|p| p.manifest.id == "sticky_notes"));
    assert!(mods.iter().any(|p| p.manifest.id == "daya"));

    // Extensions that have no real implementation must display as Planned (not available for install)
    assert!(extensions.iter().any(|p| p.manifest.id == "meter" && !p.available));
    assert!(extensions.iter().any(|p| p.manifest.id == "telemetry" && !p.available));

    let _ = std::fs::remove_dir_all(&temp_dir);
}
