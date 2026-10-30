use super::package::*;
use super::catalog::*;
use crate::core::Settings;

#[test]
fn package_file_paths_cannot_overwrite_files_outside_package(){
    let root=std::env::temp_dir().join(format!("mynotch-package-path-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let mut s=Settings::default();let pm=PackageManager::new(&root);
    assert!(pm.install_package(&mut s,&sample_mod_package(),&[("../../state.json",b"overwrite")]).is_err());
    assert!(!root.join("state.json").exists());assert!(!s.is_mod_installed("daya"));if root.exists(){std::fs::remove_dir_all(root).unwrap();}
}

#[test]
fn package_uninstall_removes_tab_registration_without_deleting_observed_history(){
    let root=std::env::temp_dir().join(format!("mynotch-package-tab-cleanup-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let pm=PackageManager::new(&root);let mut s=Settings::default();let pkg=sample_mod_package();
    pm.install_package(&mut s,&pkg,&[("main.lua",b"-- bundled package")]).unwrap();assert!(s.enabled("Daya"));
    std::fs::create_dir_all(root.join("mod-data/daya")).unwrap();std::fs::write(root.join("mod-data/daya/history.json"),b"preserve").unwrap();
    pm.uninstall_package(&mut s,"daya",true).unwrap();
    assert!(!s.mod_tabs.iter().any(|t|t=="Daya"));assert!(!s.modules.contains_key("Daya"));assert!(!s.active_tab_order().iter().any(|t|t=="Daya"));
    assert_eq!(std::fs::read(root.join("mod-data/daya/history.json")).unwrap(),b"preserve");std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn builtin_catalog_matches_install_tab_toggle_and_uninstall_after_restart() {
    for (id,tab) in [("sticky_notes","Catatan"),("weather","Cuaca")] {
        let root=std::env::temp_dir().join(format!("mynotch-catalog-roundtrip-{id}-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&root).unwrap();
        let pm=PackageManager::new(&root);let mut settings=Settings::default();
        assert!(!pm.is_installed(&settings,id),"Available builtin {id} must not pretend to be installed");
        crate::mods::install_mod(&root,&mut settings,id).unwrap();
        let slot=settings.tab_order.iter().position(|s|s==tab).unwrap();
        assert!(pm.is_installed(&settings,id));assert!(pm.is_tab_enabled(&settings,tab));
        settings.set_mod_tab(tab,false);
        assert!(!settings.active_tab_order().iter().any(|s|s==tab));
        assert!(!pm.is_tab_enabled(&settings,tab));
        let mut settings:Settings=serde_json::from_slice(&serde_json::to_vec(&settings).unwrap()).unwrap();settings.normalize();
        assert!(!pm.is_tab_enabled(&settings,tab));
        settings.set_mod_tab(tab,true);assert!(settings.active_tab_order().iter().any(|s|s==tab));
        assert!(!settings.mod_tabs.iter().any(|s|s==tab),"Builtin tabs must retain native registration");
        std::fs::write(root.join("state.json"),b"user notes and preferences must survive").unwrap();
        crate::mods::uninstall_mod(&root,&mut settings,id).unwrap();settings.normalize();
        assert!(!pm.is_installed(&settings,id));
        assert!(!get_catalog(&root,&settings).filter(PackageKind::Mod,CatalogFilter::Installed,"").iter().any(|e|e.manifest.id==id));
        assert!(!settings.active_tab_order().iter().any(|s|s==tab));
        assert_eq!(std::fs::read(root.join("state.json")).unwrap(),b"user notes and preferences must survive");
        crate::mods::install_mod(&root,&mut settings,id).unwrap();
        assert!(pm.is_installed(&settings,id));assert!(settings.enabled(tab));
        assert_eq!(settings.tab_order.iter().position(|s|s==tab),Some(slot));
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn registered_community_tab_off_is_reported_off_in_catalog() {
    let mut settings=Settings::default();settings.set_mod_tab("Market Widget",true);
    settings.set_mod_tab("Market Widget",false);
    assert!(!PackageManager::new(std::path::Path::new("unused-test-root")).is_tab_enabled(&settings,"Market Widget"));
}

#[test]
fn builtin_active_switch_suspends_tab_without_losing_tab_preference() {
    for (id,tab) in [("sticky_notes","Catatan"),("weather","Cuaca")] {
        let mut settings=Settings::default();settings.installed_mods.push(id.into());settings.modules.insert(tab.into(),true);
        settings.disabled_mods.push(id.into());
        assert!(!settings.enabled(tab));
        assert_eq!(settings.modules.get(tab),Some(&true));
        settings.disabled_mods.clear();assert!(settings.enabled(tab));
    }
}

#[test]
fn failed_builtin_uninstall_does_not_claim_success_or_remove_registration() {
    use std::os::windows::fs::OpenOptionsExt;
    let root=std::env::temp_dir().join(format!("mynotch-uninstall-locked-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let mut settings=Settings::default();crate::mods::install_mod(&root,&mut settings,"weather").unwrap();
    let locked=std::fs::OpenOptions::new().read(true).share_mode(0).open(root.join("mods/weather.mod.json")).unwrap();
    assert!(crate::mods::uninstall_mod(&root,&mut settings,"weather").is_err());
    assert!(settings.is_mod_installed("weather"));assert!(settings.enabled("Cuaca"));
    drop(locked);std::fs::remove_dir_all(root).unwrap();
}

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
    assert!(!settings.mod_tabs.iter().any(|tab|tab=="Daya"));
    assert!(!settings.enabled("Daya"));
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
