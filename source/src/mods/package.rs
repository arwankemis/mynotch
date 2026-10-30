use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::core::Settings;
use super::ModManifest;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PackageKind {
    #[default]
    Mod,
    Extension,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageState {
    NotInstalled,
    Installed,
    Active,
    Disabled,
}

pub struct PackageManager<'a> {
    pub root: &'a Path,
}

impl<'a> PackageManager<'a> {
    pub fn new(root: &'a Path) -> Self {
        Self { root }
    }

    pub fn is_installed(&self, settings: &Settings, id: &str) -> bool {
        if !super::safe_mod_id(id){return false}
        settings.installed_mods.iter().any(|m| m == id)
            || self.root.join("mods").join(format!("{id}.mod.json")).is_file()
            || self.root.join("mods").join(id).join("mod.json").is_file()
            || self.root.join("mods").join(id).join("manifest.json").is_file()
    }

    pub fn is_active(&self, settings: &Settings, id: &str) -> bool {
        self.is_installed(settings, id) && !settings.disabled_mods.iter().any(|m| m == id)
    }

    pub fn is_tab_enabled(&self, settings: &Settings, tab_name: &str) -> bool {
        settings.mod_tab_enabled(tab_name)
    }

    pub fn install_package(&self, settings: &mut Settings, manifest: &ModManifest, files: &[(&str, &[u8])]) -> Result<(), String> {
        if !crate::mods::safe_mod_id(&manifest.id) {
            return Err("ID paket tidak valid".into());
        }
        let target_dir = self.root.join("mods").join(&manifest.id);
        std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

        for (rel, content) in files {
            let file_path = target_dir.join(rel);
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&file_path, content).map_err(|e| e.to_string())?;
        }

        let manifest_path = target_dir.join("mod.json");
        let manifest_json = serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())?;
        std::fs::write(manifest_path, manifest_json).map_err(|e| e.to_string())?;

        if !settings.installed_mods.iter().any(|m| m == &manifest.id) {
            settings.installed_mods.push(manifest.id.clone());
        }
        settings.disabled_mods.retain(|m| m != &manifest.id);

        if !manifest.permissions.is_empty() {
            settings.permissions.insert(manifest.id.clone(), manifest.permissions.clone());
        }

        if manifest.default_tab_enabled && !manifest.tab_name.is_empty() {
            if !settings.mod_tabs.iter().any(|t| t == &manifest.tab_name) {
                settings.mod_tabs.push(manifest.tab_name.clone());
            }
            settings.modules.insert(manifest.tab_name.clone(), true);
            if !settings.tab_order.iter().any(|t| t == &manifest.tab_name) {
                settings.tab_order.push(manifest.tab_name.clone());
            }
        }

        Ok(())
    }

    pub fn disable_package(&self, settings: &mut Settings, id: &str) {
        if !settings.disabled_mods.iter().any(|m| m == id) {
            settings.disabled_mods.push(id.to_string());
        }
    }

    pub fn enable_package(&self, settings: &mut Settings, id: &str) {
        settings.disabled_mods.retain(|m| m != id);
    }

    pub fn uninstall_package(&self, settings: &mut Settings, id: &str, keep_data: bool) -> Result<(), String> {
        settings.installed_mods.retain(|m| m != id);
        settings.disabled_mods.retain(|m| m != id);
        
        let target_dir = self.root.join("mods").join(id);
        if target_dir.exists() {
            let _ = std::fs::remove_dir_all(&target_dir);
        }

        if !keep_data {
            let data_dir = self.root.join("mod-data").join(id);
            if data_dir.exists() {
                let _ = std::fs::remove_dir_all(&data_dir);
            }
        }

        Ok(())
    }
}
