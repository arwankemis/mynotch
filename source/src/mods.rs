use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::core::Settings;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct ModManifest {
    pub id: String,
    pub name: String,
    pub tab_name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub icon: String,
}

pub fn available_mods() -> Vec<ModManifest> {
    vec![
        ModManifest {
            id: "sticky_notes".into(),
            name: "Quick Sticky Notes".into(),
            tab_name: "Catatan".into(),
            description: "Catatan cepat melayang dan ekspor file/Windows Sticky Notes".into(),
            version: "0.1.0".into(),
            author: "mynotch".into(),
            icon: "edit".into(),
        },
        ModManifest {
            id: "weather".into(),
            name: "Prakiraan Cuaca".into(),
            tab_name: "Cuaca".into(),
            description: "Cuaca saat ini dan prakiraan per jam (Open-Meteo)".into(),
            version: "0.1.0".into(),
            author: "mynotch".into(),
            icon: "cloud".into(),
        },
    ]
}

#[allow(dead_code)]
pub fn is_mod_installed(root: &Path, id: &str) -> bool {
    let mod_file = root.join("mods").join(format!("{id}.mod.json"));
    mod_file.exists()
}

pub fn install_mod(root: &Path, settings: &mut Settings, id: &str) -> Result<(), String> {
    let manifest = available_mods().into_iter().find(|m| m.id == id).ok_or_else(|| "Mod tidak ditemukan".to_string())?;
    let mods_dir = root.join("mods");
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;
    let path = mods_dir.join(format!("{id}.mod.json"));
    let bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())?;
    if !settings.installed_mods.iter().any(|m| m == id) {
        settings.installed_mods.push(id.to_string());
    }
    settings.modules.insert(manifest.tab_name.clone(), true);
    if !settings.tab_order.iter().any(|t| t == &manifest.tab_name) {
        settings.tab_order.push(manifest.tab_name.clone());
    }
    Ok(())
}

pub fn uninstall_mod(root: &Path, settings: &mut Settings, id: &str) -> Result<(), String> {
    let manifest = available_mods().into_iter().find(|m| m.id == id).ok_or_else(|| "Mod tidak ditemukan".to_string())?;
    let mod_file = root.join("mods").join(format!("{id}.mod.json"));
    if mod_file.exists() {
        std::fs::remove_file(mod_file).map_err(|e|e.to_string())?;
    }
    settings.installed_mods.retain(|m| m != id);
    settings.modules.remove(&manifest.tab_name);
    settings.tab_order.retain(|t| t != &manifest.tab_name);
    if settings.default_tab == manifest.tab_name {
        settings.default_tab = settings.active_tab_order().first().cloned().unwrap_or_else(|| "Musik".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_manifest_removal_keeps_the_mod_installed() {
        let root=std::env::temp_dir().join(format!("weather-uninstall-{}",std::process::id()));
        let manifest=root.join("mods/weather.mod.json");std::fs::create_dir_all(&manifest).unwrap();
        let mut settings=Settings::default();settings.installed_mods.push("weather".into());settings.normalize();
        assert!(uninstall_mod(&root,&mut settings,"weather").is_err());
        assert!(settings.enabled("Cuaca"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_available_mods_contains_sticky_notes_and_weather() {
        let mods = available_mods();
        assert!(mods.iter().any(|m| m.id == "sticky_notes" && m.tab_name == "Catatan"));
        assert!(mods.iter().any(|m| m.id == "weather" && m.tab_name == "Cuaca"));
    }

    #[test]
    fn test_install_and_uninstall_mod_lifecycle() {
        let temp_dir = std::env::temp_dir().join(format!("mynotch_mod_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut settings = Settings::default();
        assert!(!is_mod_installed(&temp_dir, "weather"));
        assert!(!settings.active_tab_order().contains(&"Cuaca".to_string()));

        // Install weather mod
        assert!(install_mod(&temp_dir, &mut settings, "weather").is_ok());
        assert!(is_mod_installed(&temp_dir, "weather"));
        assert!(settings.active_tab_order().contains(&"Cuaca".to_string()));

        // Uninstall weather mod
        assert!(uninstall_mod(&temp_dir, &mut settings, "weather").is_ok());
        assert!(!is_mod_installed(&temp_dir, "weather"));
        assert!(!settings.active_tab_order().contains(&"Cuaca".to_string()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
