use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::core::Settings;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModType {
    #[default]
    Builtin,
    Declarative,
    Lua,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct ModCard {
    pub title: String,
    pub value: String,
    #[serde(default)]
    pub subtext: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct ModButton {
    pub label: String,
    #[serde(default)]
    pub action: String, // "open_url", "copy", "toast"
    #[serde(default)]
    pub target: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct ModManifest {
    pub id: String,
    pub name: String,
    pub tab_name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub icon: String,
    #[serde(default)]
    pub mod_type: ModType,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub cards: Vec<ModCard>,
    #[serde(default)]
    pub buttons: Vec<ModButton>,
}

pub fn builtin_mods() -> Vec<ModManifest> {
    vec![
        ModManifest {
            id: "sticky_notes".into(),
            name: "Quick Sticky Notes".into(),
            tab_name: "Catatan".into(),
            description: "Catatan cepat melayang dan ekspor file/Windows Sticky Notes".into(),
            version: "0.1.0".into(),
            author: "mynotch".into(),
            icon: "edit".into(),
            mod_type: ModType::Builtin,
            entry: None,
            endpoint: None,
            cards: vec![],
            buttons: vec![],
        },
        ModManifest {
            id: "weather".into(),
            name: "Prakiraan Cuaca".into(),
            tab_name: "Cuaca".into(),
            description: "Cuaca saat ini dan prakiraan per jam (Open-Meteo)".into(),
            version: "0.1.0".into(),
            author: "mynotch".into(),
            icon: "cloud".into(),
            mod_type: ModType::Builtin,
            entry: None,
            endpoint: None,
            cards: vec![],
            buttons: vec![],
        },
    ]
}

pub fn available_mods(root: &Path) -> Vec<ModManifest> {
    let mut mods = builtin_mods();
    let mods_dir = root.join("mods");
    if let Ok(entries) = std::fs::read_dir(&mods_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let mod_json = path.join("mod.json");
                let manifest_json = path.join("manifest.json");
                let candidate = if mod_json.exists() {
                    Some(mod_json)
                } else if manifest_json.exists() {
                    Some(manifest_json)
                } else {
                    None
                };
                if let Some(cfg) = candidate {
                    if let Ok(bytes) = std::fs::read(&cfg) {
                        if let Ok(mut manifest) = serde_json::from_slice::<ModManifest>(&bytes) {
                            if manifest.id.is_empty() {
                                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                                    manifest.id = dir_name.to_string();
                                }
                            }
                            if path.join("main.lua").exists() && manifest.entry.is_none() {
                                manifest.entry = Some("main.lua".into());
                                manifest.mod_type = ModType::Lua;
                            } else if manifest.mod_type == ModType::Builtin && (!manifest.cards.is_empty() || !manifest.buttons.is_empty()) {
                                manifest.mod_type = ModType::Declarative;
                            }
                            if !mods.iter().any(|m| m.id == manifest.id) {
                                mods.push(manifest);
                            }
                        }
                    }
                }
            } else if path.is_file() {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                if filename.ends_with(".mod.json") && filename != "weather.mod.json" && filename != "sticky_notes.mod.json" {
                    if let Ok(bytes) = std::fs::read(&path) {
                        if let Ok(manifest) = serde_json::from_slice::<ModManifest>(&bytes) {
                            if !mods.iter().any(|m| m.id == manifest.id) {
                                mods.push(manifest);
                            }
                        }
                    }
                }
            }
        }
    }
    mods
}

#[allow(dead_code)]
pub fn get_mod_manifest(root: &Path, id: &str) -> Option<ModManifest> {
    available_mods(root).into_iter().find(|m| m.id == id)
}

pub fn get_mod_dir(root: &Path, id: &str) -> PathBuf {
    root.join("mods").join(id)
}

#[allow(dead_code)]
pub fn is_mod_installed(root: &Path, id: &str) -> bool {
    let mods = available_mods(root);
    if let Some(m) = mods.iter().find(|m| m.id == id) {
        if m.mod_type == ModType::Builtin {
            let mod_file = root.join("mods").join(format!("{id}.mod.json"));
            return mod_file.exists();
        } else {
            return true;
        }
    }
    false
}

pub fn install_mod(root: &Path, settings: &mut Settings, id: &str) -> Result<(), String> {
    let manifest = available_mods(root).into_iter().find(|m| m.id == id).ok_or_else(|| "Mod tidak ditemukan".to_string())?;
    let mods_dir = root.join("mods");
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;

    if manifest.mod_type == ModType::Builtin {
        let path = mods_dir.join(format!("{id}.mod.json"));
        let bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| e.to_string())?;
    }

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
    let manifest = available_mods(root).into_iter().find(|m| m.id == id).ok_or_else(|| "Mod tidak ditemukan".to_string())?;
    let mod_file = root.join("mods").join(format!("{id}.mod.json"));
    if mod_file.exists() {
        let _ = std::fs::remove_file(mod_file);
    }
    settings.installed_mods.retain(|m| m != id);
    settings.modules.remove(&manifest.tab_name);
    settings.tab_order.retain(|t| t != &manifest.tab_name);
    if settings.default_tab == manifest.tab_name {
        settings.default_tab = settings.active_tab_order().first().cloned().unwrap_or_else(|| "Musik".to_string());
    }
    Ok(())
}

pub fn delete_custom_mod(root: &Path, settings: &mut Settings, id: &str) -> Result<(), String> {
    let _ = uninstall_mod(root, settings, id);
    let mod_dir = root.join("mods").join(id);
    if mod_dir.is_dir() {
        std::fs::remove_dir_all(&mod_dir).map_err(|e| e.to_string())?;
    }
    let mod_file = root.join("mods").join(format!("{id}.mod.json"));
    if mod_file.is_file() {
        let _ = std::fs::remove_file(mod_file);
    }
    Ok(())
}

pub fn install_mod_package(root: &Path, settings: &mut Settings, package_path: &Path) -> Result<ModManifest, String> {
    if !package_path.exists() {
        return Err("Berkas mod tidak ditemukan.".into());
    }
    let ext = package_path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_lowercase();
    let mods_dir = root.join("mods");
    std::fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;

    if ext == "zip" {
        let file = std::fs::File::open(package_path).map_err(|e| format!("Gagal membuka arsip zip: {e}"))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Format zip tidak valid: {e}"))?;

        let mut manifest_info: Option<(String, Vec<u8>)> = None;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = entry.name().replace('\\', "/");
            let file_part = name.split('/').last().unwrap_or("");
            if file_part == "mod.json" || file_part == "manifest.json" {
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut entry, &mut buf).map_err(|e| e.to_string())?;
                let prefix = if let Some(pos) = name.rfind('/') {
                    name[..=pos].to_string()
                } else {
                    String::new()
                };
                manifest_info = Some((prefix, buf));
                break;
            }
        }

        let (prefix, bytes) = manifest_info.ok_or_else(|| "Berkas mod.json tidak ditemukan di dalam arsip zip.".to_string())?;
        let mut manifest: ModManifest = serde_json::from_slice(&bytes).map_err(|e| format!("Isi mod.json tidak valid: {e}"))?;
        if manifest.id.is_empty() {
            let stem = package_path.file_stem().and_then(|s| s.to_str()).unwrap_or("custom_mod");
            manifest.id = stem.to_string();
        }

        let target_dir = mods_dir.join(&manifest.id);
        std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            let name = entry.name().replace('\\', "/");
            if !prefix.is_empty() && !name.starts_with(&prefix) {
                continue;
            }
            let rel_name = if !prefix.is_empty() {
                &name[prefix.len()..]
            } else {
                &name
            };
            if rel_name.is_empty() || rel_name.starts_with('/') || rel_name.contains("..") {
                continue;
            }
            let out_path = target_dir.join(rel_name);
            if entry.is_dir() || rel_name.ends_with('/') {
                std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
            } else {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut outfile = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
                std::io::copy(&mut entry, &mut outfile).map_err(|e| e.to_string())?;
            }
        }

        if target_dir.join("main.lua").exists() && manifest.entry.is_none() {
            manifest.entry = Some("main.lua".into());
            manifest.mod_type = ModType::Lua;
        } else if manifest.mod_type == ModType::Builtin && (!manifest.cards.is_empty() || !manifest.buttons.is_empty()) {
            manifest.mod_type = ModType::Declarative;
        }

        if !settings.installed_mods.iter().any(|m| m == &manifest.id) {
            settings.installed_mods.push(manifest.id.clone());
        }
        settings.modules.insert(manifest.tab_name.clone(), true);
        if !settings.tab_order.iter().any(|t| t == &manifest.tab_name) {
            settings.tab_order.push(manifest.tab_name.clone());
        }

        Ok(manifest)
    } else if ext == "json" {
        let bytes = std::fs::read(package_path).map_err(|e| format!("Gagal membaca berkas json: {e}"))?;
        let mut manifest: ModManifest = serde_json::from_slice(&bytes).map_err(|e| format!("Isi berkas json tidak valid: {e}"))?;
        if manifest.id.is_empty() {
            let stem = package_path.file_stem().and_then(|s| s.to_str()).unwrap_or("custom_mod");
            manifest.id = stem.to_string();
        }
        if manifest.mod_type == ModType::Builtin && (!manifest.cards.is_empty() || !manifest.buttons.is_empty()) {
            manifest.mod_type = ModType::Declarative;
        }

        let target_dir = mods_dir.join(&manifest.id);
        std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
        std::fs::write(target_dir.join("mod.json"), &bytes).map_err(|e| e.to_string())?;

        if !settings.installed_mods.iter().any(|m| m == &manifest.id) {
            settings.installed_mods.push(manifest.id.clone());
        }
        settings.modules.insert(manifest.tab_name.clone(), true);
        if !settings.tab_order.iter().any(|t| t == &manifest.tab_name) {
            settings.tab_order.push(manifest.tab_name.clone());
        }

        Ok(manifest)
    } else {
        Err("Format berkas harus .zip atau .json".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_mods_contains_sticky_notes_and_weather() {
        let temp_dir = std::env::temp_dir().join(format!("mynotch_mod_avail_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mods = available_mods(&temp_dir);
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

    #[test]
    fn test_discover_custom_folder_mod() {
        let temp_dir = std::env::temp_dir().join(format!("mynotch_mod_custom_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mod_dir = temp_dir.join("mods/my_counter");
        std::fs::create_dir_all(&mod_dir).unwrap();

        let json = r#"{
            "id": "my_counter",
            "name": "My Counter",
            "tab_name": "Counter",
            "description": "Counter description",
            "version": "1.0.0",
            "author": "tester",
            "icon": "star",
            "mod_type": "lua",
            "entry": "main.lua"
        }"#;
        std::fs::write(mod_dir.join("mod.json"), json).unwrap();
        std::fs::write(mod_dir.join("main.lua"), "-- lua script").unwrap();

        let mods = available_mods(&temp_dir);
        let found = mods.iter().find(|m| m.id == "my_counter");
        assert!(found.is_some());
        let m = found.unwrap();
        assert_eq!(m.tab_name, "Counter");
        assert_eq!(m.mod_type, ModType::Lua);
        assert_eq!(m.entry.as_deref(), Some("main.lua"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_install_json_mod_package() {
        let temp_dir = std::env::temp_dir().join(format!("mynotch_mod_pkg_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let json_file = temp_dir.join("my_widget.json");
        let json = r#"{
            "id": "my_widget",
            "name": "My Widget",
            "tab_name": "Widget",
            "description": "Widget description",
            "version": "1.0.0",
            "author": "tester",
            "icon": "grid",
            "cards": [
                { "title": "Speed", "value": "100 km/h", "subtext": "Optimal" }
            ],
            "buttons": [
                { "label": "Visit", "action": "open_url", "target": "https://example.com" }
            ]
        }"#;
        std::fs::write(&json_file, json).unwrap();

        let mut settings = Settings::default();
        let manifest = install_mod_package(&temp_dir, &mut settings, &json_file).unwrap();
        assert_eq!(manifest.id, "my_widget");
        assert_eq!(manifest.mod_type, ModType::Declarative);
        assert!(settings.installed_mods.contains(&"my_widget".to_string()));
        assert!(settings.active_tab_order().contains(&"Widget".to_string()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
