use std::fs;
use std::path::{Path, PathBuf};
use crate::net::Cancellation;
use super::archive::extract_zip_file_safely;
use super::download::download_to_file;
use super::ui::extract_sha256_for_asset;

/// Checks whether the companion power provider helper executable exists in the application directory.
pub fn is_power_provider_installed() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("power-provider").join("mynotch-power.exe")))
        .map(|p| p.is_file())
        .unwrap_or(false)
}

/// Finds the companion power-provider asset from the given GitHub release.
pub fn locate_provider_asset<'a>(
    release: &'a crate::online::GithubRelease,
) -> Result<&'a crate::online::ReleaseAsset, String> {
    release
        .assets
        .iter()
        .find(|a| {
            let n = a.name.to_ascii_lowercase();
            n.contains("power-provider")
                && n.ends_with(".zip")
                && (n.contains("x64") || n.contains("x86_64"))
        })
        .ok_or_else(|| "Paket companion power-provider tidak ditemukan pada rilis ini di GitHub.".to_string())
}

/// Downloads, verifies SHA-256 digest, safely extracts, and installs the companion power provider.
pub fn download_and_install_provider<F>(
    release: &crate::online::GithubRelease,
    app_dir: &Path,
    cancel: &Cancellation,
    mut on_progress: F,
) -> Result<PathBuf, String>
where
    F: FnMut(u8),
{
    // 1. Locate matching companion asset
    let asset = locate_provider_asset(release)?;

    // 2. Locate SHA-256 digest (enforcing strict digest verification)
    let expected_sha256 = extract_sha256_for_asset(release, &asset.name).ok_or_else(|| {
        format!(
            "Digest SHA-256 untuk asset '{}' tidak ditemukan pada rilis GitHub. Pemasangan dibatalkan demi keamanan.",
            asset.name
        )
    })?;

    // 3. Prepare temporary staging directory
    let temp_root = std::env::temp_dir().join(format!(
        "mynotch-provider-dl-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = fs::remove_dir_all(&temp_root);
    fs::create_dir_all(&temp_root)
        .map_err(|e| format!("Gagal membuat folder unduhan sementara: {e}"))?;

    struct TempCleanup(PathBuf);
    impl Drop for TempCleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = TempCleanup(temp_root.clone());

    let part_path = temp_root.join("companion.part");
    let zip_path = temp_root.join("companion.zip");

    // 4. Download file with verification
    let mut last_pct = 0u8;
    download_to_file(
        &asset.browser_download_url,
        &part_path,
        &zip_path,
        Some(&expected_sha256),
        cancel,
        |downloaded, total| {
            if let Some(tot) = total {
                if tot > 0 {
                    let pct = ((downloaded as f64 / tot as f64) * 100.0).clamp(0.0, 100.0) as u8;
                    if pct != last_pct {
                        last_pct = pct;
                        on_progress(pct);
                    }
                }
            }
        },
    )?;

    // 5. Extract safely
    let extract_dir = temp_root.join("extracted");
    extract_zip_file_safely(&zip_path, &extract_dir)?;

    // 6. Install extracted provider
    install_extracted_provider(&extract_dir, app_dir)
}

/// Installs the extracted provider files into `<app_dir>/power-provider`.
pub fn install_extracted_provider(extract_dir: &Path, app_dir: &Path) -> Result<PathBuf, String> {
    let source_provider_dir = if extract_dir.join("power-provider").join("mynotch-power.exe").is_file() {
        extract_dir.join("power-provider")
    } else if extract_dir.join("mynotch-power.exe").is_file() {
        extract_dir.to_path_buf()
    } else {
        return Err("Isi paket provider tidak memuat file mynotch-power.exe.".into());
    };

    let target_provider_dir = app_dir.join("power-provider");
    if !target_provider_dir.exists() {
        fs::create_dir_all(&target_provider_dir)
            .map_err(|e| format!("Gagal membuat folder target power-provider: {e}"))?;
    }

    copy_dir_recursive(&source_provider_dir, &target_provider_dir)?;

    let installed_exe = target_provider_dir.join("mynotch-power.exe");
    if !installed_exe.is_file() {
        return Err("Pemasangan power-provider gagal: mynotch-power.exe tidak ditemukan di folder tujuan.".into());
    }

    Ok(installed_exe)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    if !dst.exists() {
        fs::create_dir_all(dst).map_err(|e| format!("Gagal membuat direktori {dst:?}: {e}"))?;
    }
    for entry in fs::read_dir(src).map_err(|e| format!("Gagal membaca direktori {src:?}: {e}"))? {
        let entry = entry.map_err(|e| format!("Error membaca entri direktori: {e}"))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Gagal menyalin {src_path:?} ke {dst_path:?}: {e}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locate_provider_asset() {
        let release = crate::online::GithubRelease {
            tag_name: "v0.0.2.2".into(),
            name: Some("v0.0.2.2".into()),
            body: Some("Release notes".into()),
            html_url: "https://github.com/arwankemis/mynotch/releases/tag/v0.0.2.2".into(),
            published_at: None,
            assets: vec![
                crate::online::ReleaseAsset {
                    name: "mynotch-update-public-windows-x64-0.0.2.2.zip".into(),
                    size: 1500000,
                    browser_download_url: "https://github.com/arwankemis/mynotch/releases/download/v0.0.2.2/mynotch-update-public-windows-x64-0.0.2.2.zip".into(),
                    digest: None,
                },
                crate::online::ReleaseAsset {
                    name: "mynotch-power-provider-windows-x64-0.0.2.2.zip".into(),
                    size: 45000000,
                    browser_download_url: "https://github.com/arwankemis/mynotch/releases/download/v0.0.2.2/mynotch-power-provider-windows-x64-0.0.2.2.zip".into(),
                    digest: Some("sha256:11223344556677889900aabbccddeeff11223344556677889900aabbccddeeff".into()),
                },
            ],
        };

        let asset = locate_provider_asset(&release).expect("Provider asset should be found");
        assert_eq!(asset.name, "mynotch-power-provider-windows-x64-0.0.2.2.zip");

        let empty_release = crate::online::GithubRelease {
            tag_name: "v0.0.2.2".into(),
            name: None,
            body: None,
            html_url: "".into(),
            published_at: None,
            assets: vec![],
        };
        assert!(locate_provider_asset(&empty_release).is_err());
    }

    #[test]
    fn test_install_extracted_provider() {
        let temp = std::env::temp_dir().join(format!("test_install_provider_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let extract_dir = temp.join("extracted");
        let app_dir = temp.join("app");

        // Format 1: nested power-provider/ folder
        let nested = extract_dir.join("power-provider");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("mynotch-power.exe"), b"mock-binary").unwrap();
        fs::write(nested.join("LibreHardwareMonitorLib.dll"), b"mock-dll").unwrap();

        let installed = install_extracted_provider(&extract_dir, &app_dir).unwrap();
        assert_eq!(installed, app_dir.join("power-provider").join("mynotch-power.exe"));
        assert!(app_dir.join("power-provider").join("mynotch-power.exe").is_file());
        assert!(app_dir.join("power-provider").join("LibreHardwareMonitorLib.dll").is_file());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_install_extracted_provider_missing_binary() {
        let temp = std::env::temp_dir().join(format!("test_install_provider_missing_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let extract_dir = temp.join("extracted");
        let app_dir = temp.join("app");
        fs::create_dir_all(&extract_dir).unwrap();
        fs::write(extract_dir.join("some_other.dll"), b"mock-dll").unwrap();

        let err = install_extracted_provider(&extract_dir, &app_dir);
        assert!(err.is_err());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_download_provider_missing_digest_rejected() {
        let release = crate::online::GithubRelease {
            tag_name: "v0.0.2.2".into(),
            name: Some("v0.0.2.2".into()),
            body: Some("Release notes without digest".into()),
            html_url: "".into(),
            published_at: None,
            assets: vec![crate::online::ReleaseAsset {
                name: "mynotch-power-provider-windows-x64-0.0.2.2.zip".into(),
                size: 100,
                browser_download_url: "https://github.com/foo/bar.zip".into(),
                digest: None,
            }],
        };
        let app_dir = std::env::temp_dir();
        let cancel = Cancellation::default();
        let res = download_and_install_provider(&release, &app_dir, &cancel, |_| {});
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Digest SHA-256"));
    }
}
