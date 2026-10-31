use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::model::BuildIdentity;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvailableUpdate {
    pub version: String,
    pub tag_name: String,
    pub display_name: String,
    pub html_url: String,
    pub asset_url: String,
    pub asset_name: String,
    pub asset_size: u64,
    pub release_sequence: u64,
    pub expected_sha256: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    Available(AvailableUpdate),
    Downloading {
        percent: u8,
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
    },
    Verifying,
    Staging,
    AwaitingRestart {
        version: String,
        tx_dir: PathBuf,
    },
    Applying,
    SuccessToast(String),
    UpToDate(String),
    Error(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingUpdate {
    pub tx_dir: PathBuf,
    pub version: String,
    pub profile: String,
    pub staged_at_unix: u64,
}

impl PendingUpdate {
    pub fn new(tx_dir: PathBuf, version: String, profile: String) -> Self {
        let staged_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            tx_dir,
            version,
            profile,
            staged_at_unix,
        }
    }

    pub fn file_path(storage_root: &Path) -> PathBuf {
        storage_root.join("pending_update.json")
    }

    pub fn save(&self, storage_root: &Path) -> Result<(), String> {
        let _ = fs::create_dir_all(storage_root);
        let path = Self::file_path(storage_root);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Gagal serialisasi pending update: {e}"))?;
        fs::write(&path, json).map_err(|e| format!("Gagal menulis pending update ke {path:?}: {e}"))
    }

    pub fn load(storage_root: &Path) -> Option<Self> {
        let path = Self::file_path(storage_root);
        if !path.exists() {
            return None;
        }
        let bytes = fs::read(&path).ok()?;
        serde_json::from_slice::<Self>(&bytes).ok()
    }

    pub fn clear(storage_root: &Path) {
        let path = Self::file_path(storage_root);
        let _ = fs::remove_file(path);
    }
}

pub fn extract_sha256_for_asset(
    release: &crate::online::GithubRelease,
    asset_name: &str,
) -> Option<String> {
    // 1. Try to find from asset.digest if supplied by GitHub Releases API ("sha256:<hex>")
    for asset in &release.assets {
        if asset.name.eq_ignore_ascii_case(asset_name) {
            if let Some(digest) = &asset.digest {
                let cleaned = digest.trim().strip_prefix("sha256:").unwrap_or(digest.trim());
                if cleaned.len() == 64 && cleaned.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Some(cleaned.to_ascii_lowercase());
                }
            }
        }
    }

    // 2. Fall back to parsing release body / release notes text
    // E.g.
    // "<64-hex-sha256>  mynotch-update-public-windows-x64-0.0.2.zip"
    // or
    // "`<64-hex-sha256>` mynotch-update-public-windows-x64-0.0.2.zip"
    if let Some(body) = &release.body {
        for line in body.lines() {
            let line_lower = line.to_ascii_lowercase();
            if line_lower.contains(&asset_name.to_ascii_lowercase()) {
                for token in line.split_whitespace() {
                    let token = token.trim_matches(|c: char| {
                        c == '`' || c == '*' || c == '|' || c == ':' || c == '(' || c == ')' || c == '[' || c == ']' || c == ','
                    });
                    let cleaned = token.strip_prefix("sha256:").unwrap_or(token);
                    if cleaned.len() == 64 && cleaned.bytes().all(|b| b.is_ascii_hexdigit()) {
                        return Some(cleaned.to_ascii_lowercase());
                    }
                }
            }
        }
    }

    None
}

pub fn select_release_candidate(
    identity: &BuildIdentity,
    releases: &[crate::online::GithubRelease],
) -> Result<Option<AvailableUpdate>, String> {
    let current_prof = identity.profile.to_ascii_lowercase();

    for release in releases {
        // Find matching asset
        for asset in &release.assets {
            let name_lower = asset.name.to_ascii_lowercase();
            if !name_lower.ends_with(".zip") {
                continue;
            }

            // Architecture matching
            let arch_ok = name_lower.contains("x64") || name_lower.contains("x86_64");
            if !arch_ok {
                continue;
            }

            // Profile matching
            let is_personal_asset = name_lower.contains("personal");
            let is_public_asset = name_lower.contains("public") || (!is_personal_asset && !name_lower.contains("-arm64"));

            let profile_matches = if current_prof == "personal" {
                is_personal_asset
            } else {
                is_public_asset && !is_personal_asset
            };

            if !profile_matches {
                continue;
            }

            // Candidate sequence extraction: parse version numbers
            let version_str = release.tag_name.trim_start_matches('v');
            let candidate_seq = parse_version_sequence(version_str);

            if candidate_seq <= identity.release_sequence {
                // Not newer than current
                continue;
            }

            let expected_sha256 = extract_sha256_for_asset(release, &asset.name);

            return Ok(Some(AvailableUpdate {
                version: version_str.to_string(),
                tag_name: release.tag_name.clone(),
                display_name: release.name.clone().unwrap_or_else(|| release.tag_name.clone()),
                html_url: release.html_url.clone(),
                asset_url: asset.browser_download_url.clone(),
                asset_name: asset.name.clone(),
                asset_size: asset.size,
                release_sequence: candidate_seq,
                expected_sha256,
            }));
        }
    }

    Ok(None)
}

fn parse_version_sequence(s: &str) -> u64 {
    // E.g. "1.2.3.4-alpha.1" -> 1 * 100000000 + 2 * 1000000 + 3 * 10000 + 4
    let clean = s.split('-').next().unwrap_or(s);
    let parts: Vec<u64> = clean
        .split('.')
        .filter_map(|p| p.parse::<u64>().ok())
        .collect();

    match parts.len() {
        1 => parts[0] * 10000,
        2 => parts[0] * 1000000 + parts[1] * 10000,
        3 => parts[0] * 100000000 + parts[1] * 1000000 + parts[2] * 10000,
        4 => parts[0] * 100000000 + parts[1] * 1000000 + parts[2] * 10000 + parts[3],
        _ => parts.first().copied().unwrap_or(0),
    }
}

pub fn prepare_helper_copy(install_dir: &Path, tx_dir: &Path) -> Result<PathBuf, String> {
    let helper_dir = tx_dir.join("helper");
    fs::create_dir_all(&helper_dir).map_err(|e| format!("Gagal membuat direktori helper: {e}"))?;

    let src_exe = install_dir.join("mynotch.exe");
    let dest_exe = helper_dir.join("mynotch_helper.exe");

    fs::copy(&src_exe, &dest_exe)
        .map_err(|e| format!("Gagal menyalin executable helper dari {src_exe:?}: {e}"))?;

    if let Ok(entries) = fs::read_dir(install_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.eq_ignore_ascii_case("dll") {
                        if let Some(name) = path.file_name() {
                            let _ = fs::copy(&path, helper_dir.join(name));
                        }
                    }
                }
            }
        }
    }

    Ok(dest_exe)
}

pub fn spawn_helper(helper_exe: &Path, tx_dir: &Path) -> Result<std::process::Child, String> {
    std::process::Command::new(helper_exe)
        .arg("--internal-update-helper")
        .arg(tx_dir)
        .spawn()
        .map_err(|e| format!("Gagal menjalankan update helper: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_sha256_for_asset() {
        let dummy_hash = "b4ea3358ef49ecca8f23d039b87ae56fa91fd722054d9dded8426f3c9f8ca533";
        let rel_with_digest = crate::online::GithubRelease {
            tag_name: "v0.0.2".into(),
            name: None,
            body: None,
            html_url: "".into(),
            published_at: None,
            assets: vec![crate::online::ReleaseAsset {
                name: "pkg.zip".into(),
                size: 100,
                browser_download_url: "".into(),
                digest: Some(format!("sha256:{dummy_hash}")),
            }],
        };
        assert_eq!(
            extract_sha256_for_asset(&rel_with_digest, "pkg.zip"),
            Some(dummy_hash.to_string())
        );

        let rel_with_body = crate::online::GithubRelease {
            tag_name: "v0.0.2".into(),
            name: None,
            body: Some(format!("### Checksums\n```\n{dummy_hash}  pkg.zip\n```")),
            html_url: "".into(),
            published_at: None,
            assets: vec![crate::online::ReleaseAsset {
                name: "pkg.zip".into(),
                size: 100,
                browser_download_url: "".into(),
                digest: None,
            }],
        };
        assert_eq!(
            extract_sha256_for_asset(&rel_with_body, "pkg.zip"),
            Some(dummy_hash.to_string())
        );
    }

    #[test]
    fn test_pending_update_durable() {
        let temp = std::env::temp_dir().join(format!("test_pending_dur_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp);

        assert!(PendingUpdate::load(&temp).is_none());

        let pending = PendingUpdate::new(
            temp.join("tx-001"),
            "0.0.1.4".into(),
            "public".into(),
        );

        pending.save(&temp).expect("Save pending should succeed");

        let loaded = PendingUpdate::load(&temp).expect("Loaded pending should exist");
        assert_eq!(loaded.version, "0.0.1.4");
        assert_eq!(loaded.profile, "public");
        assert_eq!(loaded.tx_dir, temp.join("tx-001"));

        PendingUpdate::clear(&temp);
        assert!(PendingUpdate::load(&temp).is_none());

        let _ = fs::remove_dir_all(&temp);
    }
}
