use std::fs;
use std::io::{Cursor, Write};
use std::path::PathBuf;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::update::archive::extract_zip_data_safely;
use crate::update::helper::{health_ack_file, write_health_ack};
use crate::update::model::BuildIdentity;
use crate::update::transaction::{
    execute_backup, execute_rollback, execute_swap, read_and_clear_receipt, write_receipt,
    ManagedFile, TransactionJournal, TransactionPhase,
};
use crate::update::ui::{prepare_helper_copy, select_release_candidate};

struct DummyInstallation {
    root: PathBuf,
    install_dir: PathBuf,
    data_dir: PathBuf,
}

impl DummyInstallation {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("mynotch_rehearsal_{}_{}", name, std::process::id()));
        let install_dir = root.join("app");
        let data_dir = install_dir.join("data");

        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&install_dir).unwrap();
        fs::create_dir_all(&data_dir).unwrap();

        // Write initial app binary (v1)
        fs::write(install_dir.join("mynotch.exe"), b"BINARY_VERSION_1").unwrap();
        fs::write(install_dir.join("libunwind.dll"), b"DLL_VERSION_1").unwrap();

        // Write user personal data that must NEVER be overwritten or touched
        fs::write(data_dir.join("notes.json"), b"{\"important_notes\": \"user private sticky\"}").unwrap();
        fs::write(data_dir.join("settings.json"), b"{\"personal_volume\": 80}").unwrap();

        Self {
            root,
            install_dir,
            data_dir,
        }
    }

    fn create_update_zip_v2(&self) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut buf);
            let options = SimpleFileOptions::default();
            writer.start_file("mynotch.exe", options).unwrap();
            writer.write_all(b"BINARY_VERSION_2").unwrap();
            writer.start_file("libunwind.dll", options).unwrap();
            writer.write_all(b"DLL_VERSION_2").unwrap();
            writer.finish().unwrap();
        }
        buf.into_inner()
    }
}

impl Drop for DummyInstallation {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn test_rehearsal_successful_update_lifecycle() {
    let fixture = DummyInstallation::new("success");
    let tx_id = "tx-success-01";
    let tx_dir = fixture.install_dir.join(".mynotch-update").join(tx_id);
    let stage_dir = tx_dir.join("stage");
    let backup_dir = tx_dir.join("backup");

    // Step 1: Safe extract into staging
    let zip_v2 = fixture.create_update_zip_v2();
    let extracted = extract_zip_data_safely(&zip_v2, &stage_dir).expect("Staging extract must succeed");
    assert_eq!(extracted.len(), 2);

    // Step 2: Initialize transaction journal
    let managed_files: Vec<ManagedFile> = extracted
        .into_iter()
        .map(|rel| ManagedFile {
            relative_path: rel,
            stage_sha256: None,
            original_sha256: None,
        })
        .collect();

    let mut journal = TransactionJournal::new(
        tx_id.into(),
        fixture.install_dir.clone(),
        stage_dir.clone(),
        backup_dir.clone(),
        fixture.install_dir.join("mynotch.exe"),
        std::process::id(),
        managed_files,
        "0.0.2.0".into(),
        "public".into(),
    );
    journal.save(&tx_dir).unwrap();
    assert_eq!(journal.phase, TransactionPhase::Prepared);

    // Step 3: Prepare trusted helper copy
    let helper_exe = prepare_helper_copy(&fixture.install_dir, &tx_dir).expect("Helper copy must succeed");
    assert!(helper_exe.exists());

    // Step 4: Backup existing files
    execute_backup(&mut journal, &tx_dir).expect("Backup must succeed");
    assert_eq!(journal.phase, TransactionPhase::BackingUp);
    assert_eq!(fs::read(backup_dir.join("mynotch.exe")).unwrap(), b"BINARY_VERSION_1");
    assert_eq!(fs::read(backup_dir.join("libunwind.dll")).unwrap(), b"DLL_VERSION_1");

    // Step 5: Swap files in-place
    execute_swap(&mut journal, &tx_dir).expect("Swap must succeed");
    assert_eq!(journal.phase, TransactionPhase::Swapping);
    assert_eq!(fs::read(fixture.install_dir.join("mynotch.exe")).unwrap(), b"BINARY_VERSION_2");
    assert_eq!(fs::read(fixture.install_dir.join("libunwind.dll")).unwrap(), b"DLL_VERSION_2");

    // Step 6: Healthcheck ACK
    write_health_ack(&tx_dir).expect("Health ACK write must succeed");
    assert!(health_ack_file(&tx_dir).exists());

    // Step 7: Commit transaction
    journal.set_phase(&tx_dir, TransactionPhase::Committed).unwrap();
    write_receipt(&fixture.install_dir, &journal.target_version, &journal.target_profile).unwrap();

    // Verify user data is 100% UNTOUCHED
    assert_eq!(
        fs::read(fixture.data_dir.join("notes.json")).unwrap(),
        b"{\"important_notes\": \"user private sticky\"}"
    );
    assert_eq!(
        fs::read(fixture.data_dir.join("settings.json")).unwrap(),
        b"{\"personal_volume\": 80}"
    );

    // Verify receipt read
    let receipt = read_and_clear_receipt(&fixture.install_dir).expect("Receipt must be present");
    assert_eq!(receipt.version, "0.0.2.0");
    assert_eq!(receipt.profile, "public");
}

#[test]
fn test_rehearsal_failure_triggers_complete_rollback() {
    let fixture = DummyInstallation::new("rollback");
    let tx_id = "tx-rollback-01";
    let tx_dir = fixture.install_dir.join(".mynotch-update").join(tx_id);
    let stage_dir = tx_dir.join("stage");
    let backup_dir = tx_dir.join("backup");

    // Staging and journal setup
    let zip_v2 = fixture.create_update_zip_v2();
    let extracted = extract_zip_data_safely(&zip_v2, &stage_dir).unwrap();

    let managed_files: Vec<ManagedFile> = extracted
        .into_iter()
        .map(|rel| ManagedFile {
            relative_path: rel,
            stage_sha256: None,
            original_sha256: None,
        })
        .collect();

    let mut journal = TransactionJournal::new(
        tx_id.into(),
        fixture.install_dir.clone(),
        stage_dir.clone(),
        backup_dir.clone(),
        fixture.install_dir.join("mynotch.exe"),
        std::process::id(),
        managed_files,
        "0.0.2.0".into(),
        "public".into(),
    );
    journal.save(&tx_dir).unwrap();

    // Backup & Swap to v2
    execute_backup(&mut journal, &tx_dir).unwrap();
    execute_swap(&mut journal, &tx_dir).unwrap();

    // Verify files became v2 momentarily
    assert_eq!(fs::read(fixture.install_dir.join("mynotch.exe")).unwrap(), b"BINARY_VERSION_2");

    // Simulate startup healthcheck crash/failure -> Trigger rollback!
    execute_rollback(&mut journal, &tx_dir).expect("Rollback must succeed");
    assert_eq!(journal.phase, TransactionPhase::RolledBack);

    // Verify ALL files are cleanly restored to original v1!
    assert_eq!(fs::read(fixture.install_dir.join("mynotch.exe")).unwrap(), b"BINARY_VERSION_1");
    assert_eq!(fs::read(fixture.install_dir.join("libunwind.dll")).unwrap(), b"DLL_VERSION_1");

    // Verify user data is completely untouched
    assert_eq!(
        fs::read(fixture.data_dir.join("notes.json")).unwrap(),
        b"{\"important_notes\": \"user private sticky\"}"
    );
    assert_eq!(
        fs::read(fixture.data_dir.join("settings.json")).unwrap(),
        b"{\"personal_volume\": 80}"
    );
}

#[test]
fn test_rehearsal_profile_enforcement_in_candidate_selection() {
    let public_id = BuildIdentity::new("public", 100);
    let personal_id = BuildIdentity::new("personal", 100);

    let release = crate::online::GithubRelease {
        tag_name: "v0.0.2.0".into(),
        name: Some("Release 0.0.2.0".into()),
        body: None,
        html_url: "https://github.com/arwankemis/mynotch/releases/v0.0.2.0".into(),
        published_at: None,
        assets: vec![
            crate::online::ReleaseAsset {
                name: "mynotch-update-public-windows-x64-0.0.2.0.zip".into(),
                size: 5000000,
                browser_download_url: "https://github.com/arwankemis/mynotch/releases/download/v0.0.2.0/public.zip".into(),
            },
            crate::online::ReleaseAsset {
                name: "mynotch-update-personal-windows-x64-0.0.2.0.zip".into(),
                size: 6000000,
                browser_download_url: "https://github.com/arwankemis/mynotch/releases/download/v0.0.2.0/personal.zip".into(),
            },
        ],
    };

    // Public selects public asset
    let pub_candidate = select_release_candidate(&public_id, &[release.clone()])
        .unwrap()
        .expect("Public must find candidate");
    assert!(pub_candidate.asset_name.contains("public"));
    assert!(!pub_candidate.asset_name.contains("personal"));

    // Personal selects personal asset
    let pers_candidate = select_release_candidate(&personal_id, &[release])
        .unwrap()
        .expect("Personal must find candidate");
    assert!(pers_candidate.asset_name.contains("personal"));
}
