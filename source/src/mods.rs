use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::core::Settings;

pub mod package;
pub mod catalog;
pub mod power_broker;
#[cfg(test)]
mod catalog_tests;
#[cfg(test)]
mod power_broker_tests;

/// A single archive entry may not exceed 8 MiB, and a package may not unpack past 64 MiB.
const MAX_ENTRY_BYTES: u64 = 8 * 1024 * 1024;
const MAX_UNPACKED_BYTES: u64 = 64 * 1024 * 1024;
/// A mod manifest (mod.json / manifest.json) may not exceed 1 MiB.
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

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
    #[serde(default = "default_schema_version")]
    pub schema: u32,
    pub id: String,
    #[serde(default)]
    pub publisher: String,
    pub name: String,
    #[serde(default)]
    pub package_kind: package::PackageKind,
    pub tab_name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub icon: String,
    #[serde(default)]
    pub mod_type: ModType,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default = "default_host_api")]
    pub min_host_api: u32,
    #[serde(default = "default_profiles")]
    pub profiles: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub optional_dependencies: Vec<String>,
    #[serde(default)]
    pub default_tab_enabled: bool,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub cards: Vec<ModCard>,
    #[serde(default)]
    pub buttons: Vec<ModButton>,
}

fn default_schema_version() -> u32 { 1 }
fn default_host_api() -> u32 { 1 }
fn default_profiles() -> Vec<String> { vec!["public".into(), "personal".into()] }

pub fn builtin_mods() -> Vec<ModManifest> {
    #[allow(unused_mut)]
    let mut list = vec![
        ModManifest {
            schema: 1,
            id: "sticky_notes".into(),
            publisher: "mynotch".into(),
            name: "Quick Sticky Notes".into(),
            package_kind: package::PackageKind::Mod,
            tab_name: "Catatan".into(),
            description: "Catatan cepat melayang dan ekspor file/Windows Sticky Notes".into(),
            version: "0.1.0".into(),
            author: "mynotch".into(),
            icon: "edit".into(),
            mod_type: ModType::Builtin,
            entry: None,
            min_host_api: 1,
            profiles: vec!["public".into(), "personal".into()],
            permissions: vec![],
            optional_dependencies: vec![],
            default_tab_enabled: false,
            endpoint: None,
            cards: vec![],
            buttons: vec![],
        },
        ModManifest {
            schema: 1,
            id: "weather".into(),
            publisher: "mynotch".into(),
            name: "Prakiraan Cuaca".into(),
            package_kind: package::PackageKind::Mod,
            tab_name: "Cuaca".into(),
            description: "Cuaca saat ini dan prakiraan per jam (Open-Meteo)".into(),
            version: "0.1.0".into(),
            author: "mynotch".into(),
            icon: "cloud".into(),
            mod_type: ModType::Builtin,
            entry: None,
            min_host_api: 1,
            profiles: vec!["public".into(), "personal".into()],
            permissions: vec![],
            optional_dependencies: vec![],
            default_tab_enabled: false,
            endpoint: None,
            cards: vec![],
            buttons: vec![],
        },
    ];

    list
}

pub fn available_mods(root: &Path) -> Vec<ModManifest> {
    let mut mods = builtin_mods();
    let mods_dir = root.join("mods");
    if let Ok(entries) = std::fs::read_dir(&mods_dir) {
        for entry in entries.flatten() {
            // Staging (.install-<id>) and backup (.backup-<id>) directories from an
            // interrupted install are never mods, even if they contain a mod.json.
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
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
                            // A folder manifest id becomes a directory name downstream
                            // (delete joins root/mods/<id>); an unvalidated id (../, /,
                            // C:) could escape data/mods, so it is skipped at discovery.
                            // Note: the id is validated, not forced to the directory name,
                            // so existing installs keep their identity.
                            if !safe_mod_id(&manifest.id) {
                                continue;
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
if filename.ends_with(".mod.json")
                    && filename != "weather.mod.json"
                    && filename != "sticky_notes.mod.json"
                    && filename != "daya.mod.json"
                {
                    if let Ok(bytes) = std::fs::read(&path) {
                        if let Ok(manifest) = serde_json::from_slice::<ModManifest>(&bytes) {
                            if !safe_mod_id(&manifest.id) {
                                continue;
                            }
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

/// Mod ids become directory names, so only a conservative charset is allowed. Anything else
/// (`..`, `/`, `\`, `C:`) could escape `data/mods` when joined.
pub fn safe_mod_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Rejects absolute paths and any `..`/root component inside a zip entry name.
fn safe_entry(rel: &str) -> bool {
    let path = Path::new(rel);
    !rel.is_empty()
        && !path.is_absolute()
        && path.components().all(|c| matches!(c, std::path::Component::Normal(_)))
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

fn attach_builtin_tab(settings: &mut Settings, manifest: &ModManifest) {
    if manifest.mod_type != ModType::Builtin {
        return;
    }
    settings.modules.insert(manifest.tab_name.clone(), true);
    if !settings.tab_order.iter().any(|t| t == &manifest.tab_name) {
        settings.tab_order.push(manifest.tab_name.clone());
    }
}

pub fn install_mod(root: &Path, settings: &mut Settings, id: &str) -> Result<(), String> {
    if !safe_mod_id(id) { return Err("ID mod tidak valid: gunakan huruf, angka, '-' atau '_'.".into()) }
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
    settings.disabled_mods.retain(|m|m!=id);
    attach_builtin_tab(settings, &manifest);
    Ok(())
}

pub fn uninstall_mod(root: &Path, settings: &mut Settings, id: &str) -> Result<(), String> {
    if !safe_mod_id(id){return Err("ID mod tidak valid".into())}
    let manifest = available_mods(root).into_iter().find(|m| m.id == id).ok_or_else(|| "Mod tidak ditemukan".to_string())?;
    let mod_file = root.join("mods").join(format!("{id}.mod.json"));
    if mod_file.exists() {
        std::fs::remove_file(mod_file).map_err(|e|format!("Gagal menghapus registrasi mod: {e}"))?;
    }
    if manifest.mod_type==ModType::Builtin {
        let folder=root.join("mods").join(id);
        if folder.exists(){std::fs::remove_dir_all(folder).map_err(|e|format!("Gagal menghapus paket mod: {e}"))?;}
    }
    settings.installed_mods.retain(|m| m != id);
    settings.disabled_mods.retain(|m|m!=id);
    settings.network_mods.retain(|m| m != id);
    if manifest.mod_type == ModType::Builtin {
        settings.modules.remove(&manifest.tab_name);
        // Keep the slot in `tab_order` so reinstalling restores the user's chosen position.
        if settings.default_tab == manifest.tab_name {
            settings.default_tab = settings.active_tab_order().first().cloned().unwrap_or_else(|| "Musik".to_string());
        }
    } else {
        // A deleted community mod must not leave a phantom Stage 2 tab behind.
        settings.mod_tabs.retain(|t| t != &manifest.tab_name);
        settings.modules.remove(&manifest.tab_name);
        settings.tab_order.retain(|t| t != &manifest.tab_name);
        if settings.default_tab == manifest.tab_name {
            settings.default_tab = settings.active_tab_order().first().cloned().unwrap_or_else(|| "Musik".to_string());
        }
    }
    Ok(())
}

pub fn delete_custom_mod(root: &Path, settings: &mut Settings, id: &str) -> Result<(), String> {
    // Validate before any filesystem use: the old code ignored uninstall_mod's
    // validation error and then ran remove_dir_all(root/mods/<id>) unvalidated.
    if !safe_mod_id(id) { return Err("ID mod tidak valid: gunakan huruf, angka, '-' atau '_'.".into()) }
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
                if entry.size() > MAX_MANIFEST_BYTES {
                    return Err("Manifest mod melebihi batas 1 MiB.".into());
                }
                let mut buf = Vec::new();
                let mut capped = std::io::Read::take(&mut entry, MAX_MANIFEST_BYTES + 1);
                std::io::Read::read_to_end(&mut capped, &mut buf).map_err(|e| e.to_string())?;
                if buf.len() as u64 > MAX_MANIFEST_BYTES {
                    return Err("Manifest mod melebihi batas 1 MiB.".into());
                }
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
        if !safe_mod_id(&manifest.id) { return Err("ID mod tidak valid: gunakan huruf, angka, '-' atau '_'.".into()) }

        let target_dir = mods_dir.join(&manifest.id);
        // Extract into a staging directory first: aborting on an invalid or oversize
        // entry must not leave a half-installed mod behind for discovery to pick up,
        // and a failed reinstall must not destroy the previous install.
        let staging_dir = mods_dir.join(format!(".install-{}", manifest.id));
        let _ = std::fs::remove_dir_all(&staging_dir);
        std::fs::create_dir_all(&staging_dir).map_err(|e| e.to_string())?;

        let extract_result = (|| -> Result<(), String> {
            let mut extracted = 0_u64;
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
                if rel_name.ends_with('/') { continue }
                if !safe_entry(rel_name) {
                    return Err(format!("Entri zip tidak valid (di luar paket): {rel_name}"));
                }
                if entry.size() > MAX_ENTRY_BYTES || extracted.saturating_add(entry.size()) > MAX_UNPACKED_BYTES {
                    return Err("Entri zip melebihi batas ukuran (8 MiB per entri, 64 MiB total).".into());
                }
                let out_path = staging_dir.join(rel_name);
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut outfile = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
                let before = extracted;
                extracted += std::io::copy(&mut entry, &mut outfile).map_err(|e| e.to_string())?;
                // The header size can lie; enforce the caps on bytes actually written too.
                if extracted - before > MAX_ENTRY_BYTES || extracted > MAX_UNPACKED_BYTES {
                    return Err("Entri zip melebihi batas ukuran (8 MiB per entri, 64 MiB total).".into());
                }
            }
            Ok(())
        })();
        if let Err(e) = extract_result {
            let _ = std::fs::remove_dir_all(&staging_dir);
            return Err(e);
        }

        if staging_dir.join("main.lua").exists() && manifest.entry.is_none() {
            manifest.entry = Some("main.lua".into());
            manifest.mod_type = ModType::Lua;
        } else if manifest.mod_type == ModType::Builtin && (!manifest.cards.is_empty() || !manifest.buttons.is_empty()) {
            manifest.mod_type = ModType::Declarative;
        }

        // Swap the staged install into place; the previous install is kept as a backup
        // until the rename succeeds, so a failed reinstall never loses the old version.
        // Note: a reinstall now replaces the old install wholesale instead of merging
        // files into it, so stale files from the previous version no longer linger.
        let backup_dir = mods_dir.join(format!(".backup-{}", manifest.id));
        let _ = std::fs::remove_dir_all(&backup_dir);
        if target_dir.exists() {
            std::fs::rename(&target_dir, &backup_dir).map_err(|e| e.to_string())?;
        }
        if let Err(e) = std::fs::rename(&staging_dir, &target_dir) {
            let _ = std::fs::rename(&backup_dir, &target_dir);
            return Err(e.to_string());
        }
        let _ = std::fs::remove_dir_all(&backup_dir);

        if !settings.installed_mods.iter().any(|m| m == &manifest.id) {
            settings.installed_mods.push(manifest.id.clone());
        }
        attach_builtin_tab(settings, &manifest);

        Ok(manifest)
    } else if ext == "json" {
        let meta = std::fs::metadata(package_path).map_err(|e| format!("Gagal membaca berkas json: {e}"))?;
        if meta.len() > MAX_MANIFEST_BYTES {
            return Err("Manifest mod melebihi batas 1 MiB.".into());
        }
        let bytes = std::fs::read(package_path).map_err(|e| format!("Gagal membaca berkas json: {e}"))?;
        let mut manifest: ModManifest = serde_json::from_slice(&bytes).map_err(|e| format!("Isi berkas json tidak valid: {e}"))?;
        if manifest.id.is_empty() {
            let stem = package_path.file_stem().and_then(|s| s.to_str()).unwrap_or("custom_mod");
            manifest.id = stem.to_string();
        }
        if manifest.mod_type == ModType::Builtin && (!manifest.cards.is_empty() || !manifest.buttons.is_empty()) {
            manifest.mod_type = ModType::Declarative;
        }

        if !safe_mod_id(&manifest.id) { return Err("ID mod tidak valid: gunakan huruf, angka, '-' atau '_'.".into()) }

        let target_dir = mods_dir.join(&manifest.id);
        std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
        std::fs::write(target_dir.join("mod.json"), &bytes).map_err(|e| e.to_string())?;

        if !settings.installed_mods.iter().any(|m| m == &manifest.id) {
            settings.installed_mods.push(manifest.id.clone());
        }
        attach_builtin_tab(settings, &manifest);

        Ok(manifest)
    } else {
        Err("Format berkas harus .zip atau .json".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_ids_and_archive_entries_cannot_escape_the_mods_folder() {
        for bad in ["../evil", "a/b", "a\\b", "C:/evil", "", ".."] { assert!(!safe_mod_id(bad), "{bad} must be rejected"); }
        for good in ["my_counter", "kurs-usd", "weather2"] { assert!(safe_mod_id(good), "{good} must be accepted"); }
        for bad in ["../x.lua", "/x.lua", "C:/x.lua", "a/../../x.lua"] { assert!(!safe_entry(bad), "{bad} must be rejected"); }
        for good in ["main.lua", "lib/helper.lua"] { assert!(safe_entry(good), "{good} must be accepted"); }
    }

    #[test]
    fn install_rejects_a_traversal_id_without_touching_disk() {
        let temp_dir = std::env::temp_dir().join(format!("mynotch_mod_escape_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let manifest = temp_dir.join("evil.json");
        std::fs::write(&manifest, r#"{"id":"../escaped","name":"x","tab_name":"X","description":"x","version":"1","author":"x","icon":"star"}"#).unwrap();
        let mut settings = Settings::default();
        assert!(install_mod_package(&temp_dir, &mut settings, &manifest).is_err());
        assert!(!temp_dir.parent().unwrap().join("escaped").exists());
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

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
        assert!(!settings.tab_order.contains(&"Widget".to_string()));
        assert!(!settings.active_tab_order().contains(&"Widget".to_string()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn reinstalling_a_builtin_mod_keeps_its_tab_position() {
        let temp_dir = std::env::temp_dir().join(format!("mynotch_mod_slot_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let mut settings = Settings::default();
        install_mod(&temp_dir, &mut settings, "weather").unwrap();
        settings.normalize();
        settings.move_tab("Cuaca", -3);
        let pos = settings.tab_order.iter().position(|t| t == "Cuaca").unwrap();

        uninstall_mod(&temp_dir, &mut settings, "weather").unwrap();
        settings.normalize();
        assert!(!settings.active_tab_order().contains(&"Cuaca".to_string()));
        assert_eq!(settings.tab_order.iter().position(|t| t == "Cuaca"), Some(pos));

        install_mod(&temp_dir, &mut settings, "weather").unwrap();
        settings.normalize();
        assert_eq!(settings.tab_order.iter().position(|t| t == "Cuaca"), Some(pos));
        assert!(settings.active_tab_order().contains(&"Cuaca".to_string()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn deleting_a_community_mod_removes_its_opted_in_tab() {
        let temp_dir = std::env::temp_dir().join(format!("mynotch_mod_tab_del_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let json_file = temp_dir.join("widget.json");
        std::fs::write(&json_file, r#"{
            "id": "my_widget", "name": "My Widget", "tab_name": "Widget",
            "description": "x", "version": "1.0.0", "author": "x", "icon": "grid",
            "cards": [{ "title": "A", "value": "B" }]
        }"#).unwrap();

        let mut settings = Settings::default();
        install_mod_package(&temp_dir, &mut settings, &json_file).unwrap();
        settings.set_mod_tab("Widget", true);
        assert!(settings.active_tab_order().contains(&"Widget".to_string()));

        delete_custom_mod(&temp_dir, &mut settings, "my_widget").unwrap();
        assert!(!settings.mod_tabs.contains(&"Widget".to_string()));
        assert!(!settings.active_tab_order().contains(&"Widget".to_string()));
        assert!(!settings.tab_order.contains(&"Widget".to_string()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn installing_a_custom_mod_never_adds_a_tab() {
        let temp_dir = std::env::temp_dir().join(format!("mynotch_mod_notab_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let json_file = temp_dir.join("counter.json");
        std::fs::write(&json_file, r#"{
            "id": "counter_lua",
            "name": "Pusat Kontrol",
            "tab_name": "Kontrol",
            "description": "x",
            "version": "1.0.0",
            "author": "x",
            "icon": "terminal",
            "mod_type": "lua",
            "entry": "main.lua"
        }"#).unwrap();

        let mut settings = Settings::default();
        install_mod_package(&temp_dir, &mut settings, &json_file).unwrap();
        assert!(settings.installed_mods.contains(&"counter_lua".to_string()));
        assert!(!settings.modules.contains_key("Kontrol"));
        assert!(!settings.tab_order.contains(&"Kontrol".to_string()));
        assert!(!settings.active_tab_order().contains(&"Kontrol".to_string()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
