use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::crypto::hash_file_hex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionPhase {
    Prepared,
    BackingUp,
    Swapping,
    Relaunching,
    Committed,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedFile {
    pub relative_path: PathBuf,
    pub stage_sha256: Option<String>,
    pub original_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransactionReceipt {
    pub version: String,
    pub profile: String,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionJournal {
    pub schema_version: u32,
    pub tx_id: String,
    pub phase: TransactionPhase,
    pub install_dir: PathBuf,
    pub stage_dir: PathBuf,
    pub backup_dir: PathBuf,
    pub target_exe: PathBuf,
    pub parent_pid: u32,
    pub managed_files: Vec<ManagedFile>,
    pub target_version: String,
    pub target_profile: String,
    pub created_at_unix: u64,
    pub error: Option<String>,
}

impl TransactionJournal {
    pub fn new(
        tx_id: String,
        install_dir: PathBuf,
        stage_dir: PathBuf,
        backup_dir: PathBuf,
        target_exe: PathBuf,
        parent_pid: u32,
        managed_files: Vec<ManagedFile>,
        target_version: String,
        target_profile: String,
    ) -> Self {
        let created_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            schema_version: 1,
            tx_id,
            phase: TransactionPhase::Prepared,
            install_dir,
            stage_dir,
            backup_dir,
            target_exe,
            parent_pid,
            managed_files,
            target_version,
            target_profile,
            created_at_unix,
            error: None,
        }
    }

    pub fn journal_file(tx_dir: &Path) -> PathBuf {
        tx_dir.join("journal.json")
    }

    pub fn save(&self, tx_dir: &Path) -> Result<(), String> {
        fs::create_dir_all(tx_dir).map_err(|e| format!("Gagal membuat folder transaksi: {e}"))?;
        let journal_path = Self::journal_file(tx_dir);
        let tmp_path = tx_dir.join("journal.json.tmp");

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Gagal serialisasi journal: {e}"))?;

        {
            let mut file = File::create(&tmp_path)
                .map_err(|e| format!("Gagal menulis temporary journal: {e}"))?;
            file.write_all(json.as_bytes())
                .map_err(|e| format!("Gagal flush temporary journal: {e}"))?;
            file.flush()
                .map_err(|e| format!("Gagal flush data journal: {e}"))?;
        }

        if journal_path.exists() {
            let _ = fs::remove_file(&journal_path);
        }
        fs::rename(&tmp_path, &journal_path)
            .map_err(|e| format!("Gagal memindahkan journal ke path final: {e}"))?;

        Ok(())
    }

    pub fn load(tx_dir: &Path) -> Result<Self, String> {
        let journal_path = Self::journal_file(tx_dir);
        let bytes = fs::read(&journal_path)
            .map_err(|e| format!("Gagal membaca berkas journal dari {journal_path:?}: {e}"))?;
        let journal: Self = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Format berkas journal tidak valid: {e}"))?;
        Ok(journal)
    }

    pub fn set_phase(&mut self, tx_dir: &Path, phase: TransactionPhase) -> Result<(), String> {
        self.phase = phase;
        self.save(tx_dir)
    }

    pub fn record_error(&mut self, tx_dir: &Path, error: String) -> Result<(), String> {
        self.error = Some(error);
        self.save(tx_dir)
    }
}

pub fn execute_backup(
    journal: &mut TransactionJournal,
    tx_dir: &Path,
) -> Result<(), String> {
    journal.set_phase(tx_dir, TransactionPhase::BackingUp)?;
    fs::create_dir_all(&journal.backup_dir)
        .map_err(|e| format!("Gagal membuat direktori backup: {e}"))?;

    for managed in &mut journal.managed_files {
        let live_path = journal.install_dir.join(&managed.relative_path);
        let backup_path = journal.backup_dir.join(&managed.relative_path);

        if live_path.exists() {
            if let Some(parent) = backup_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Gagal membuat subdirektori backup: {e}"))?;
            }

            // Compute original hash
            if let Ok(hash) = hash_file_hex(&live_path) {
                managed.original_sha256 = Some(hash);
            }

            fs::copy(&live_path, &backup_path).map_err(|e| {
                format!(
                    "Gagal mem-backup berkas '{:?}' ke '{:?}': {e}",
                    live_path, backup_path
                )
            })?;
        }
    }

    journal.save(tx_dir)?;
    Ok(())
}

pub fn execute_swap(
    journal: &mut TransactionJournal,
    tx_dir: &Path,
) -> Result<(), String> {
    journal.set_phase(tx_dir, TransactionPhase::Swapping)?;

    for managed in &journal.managed_files {
        let staged_path = journal.stage_dir.join(&managed.relative_path);
        let live_path = journal.install_dir.join(&managed.relative_path);

        if !staged_path.exists() {
            return Err(format!(
                "Berkas staged hilang sebelum swap: {:?}",
                staged_path
            ));
        }

        if let Some(parent) = live_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Gagal membuat direktori target: {e}"))?;
        }

        // Copy from staging to destination (replaces destination)
        fs::copy(&staged_path, &live_path).map_err(|e| {
            format!(
                "Gagal menyalin berkas baru '{:?}' ke live target '{:?}': {e}",
                staged_path, live_path
            )
        })?;
    }

    Ok(())
}

pub fn execute_rollback(
    journal: &mut TransactionJournal,
    tx_dir: &Path,
) -> Result<(), String> {
    for managed in &journal.managed_files {
        let backup_path = journal.backup_dir.join(&managed.relative_path);
        let live_path = journal.install_dir.join(&managed.relative_path);

        if backup_path.exists() {
            if let Some(parent) = live_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::copy(&backup_path, &live_path);
        } else if live_path.exists() {
            // File was newly added in update, remove it during rollback
            let _ = fs::remove_file(&live_path);
        }
    }

    let _ = journal.set_phase(tx_dir, TransactionPhase::RolledBack);
    Ok(())
}

pub fn write_receipt(install_dir: &Path, target_version: &str, target_profile: &str) -> Result<(), String> {
    let update_root = install_dir.join(".mynotch-update");
    let _ = fs::create_dir_all(&update_root);
    let receipt_path = update_root.join("receipt.json");
    let receipt = TransactionReceipt {
        version: target_version.to_string(),
        profile: target_profile.to_string(),
        updated_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    };
    let json = serde_json::to_string_pretty(&receipt)
        .map_err(|e| format!("Gagal serialisasi receipt: {e}"))?;
    fs::write(receipt_path, json).map_err(|e| format!("Gagal menulis receipt: {e}"))?;
    Ok(())
}

pub fn read_and_clear_receipt(install_dir: &Path) -> Option<TransactionReceipt> {
    let receipt_path = install_dir.join(".mynotch-update").join("receipt.json");
    if !receipt_path.exists() {
        return None;
    }
    let bytes = fs::read(&receipt_path).ok()?;
    let receipt = serde_json::from_slice::<TransactionReceipt>(&bytes).ok()?;
    let _ = fs::remove_file(&receipt_path);
    Some(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_journal_roundtrip() {
        let temp = std::env::temp_dir().join(format!("test_tx_rt_{}", std::process::id()));
        let install_dir = temp.join("install");
        let stage_dir = temp.join("stage");
        let backup_dir = temp.join("backup");
        let target_exe = install_dir.join("mynotch.exe");

        let journal = TransactionJournal::new(
            "tx-12345".into(),
            install_dir.clone(),
            stage_dir.clone(),
            backup_dir.clone(),
            target_exe.clone(),
            1234,
            vec![
                ManagedFile {
                    relative_path: PathBuf::from("mynotch.exe"),
                    stage_sha256: Some("sha_new".into()),
                    original_sha256: None,
                },
                ManagedFile {
                    relative_path: PathBuf::from("libunwind.dll"),
                    stage_sha256: Some("sha_dll".into()),
                    original_sha256: None,
                },
            ],
            "0.0.1.4".into(),
            "public".into(),
        );

        journal.save(&temp).expect("Save journal should succeed");
        let loaded = TransactionJournal::load(&temp).expect("Load journal should succeed");

        assert_eq!(loaded.tx_id, "tx-12345");
        assert_eq!(loaded.phase, TransactionPhase::Prepared);
        assert_eq!(loaded.managed_files.len(), 2);
        assert_eq!(loaded.target_version, "0.0.1.4");
        assert_eq!(loaded.target_profile, "public");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_atomic_swap_and_rollback() {
        let temp = std::env::temp_dir().join(format!("test_tx_swap_{}", std::process::id()));
        let install_dir = temp.join("install");
        let stage_dir = temp.join("stage");
        let backup_dir = temp.join("backup");
        let target_exe = install_dir.join("app.exe");

        fs::create_dir_all(&install_dir).unwrap();
        fs::create_dir_all(&stage_dir).unwrap();

        // Create original file
        fs::write(install_dir.join("app.exe"), b"version 1.0").unwrap();
        // Create staged new file
        fs::write(stage_dir.join("app.exe"), b"version 2.0").unwrap();

        let mut journal = TransactionJournal::new(
            "tx-test".into(),
            install_dir.clone(),
            stage_dir.clone(),
            backup_dir.clone(),
            target_exe.clone(),
            std::process::id(),
            vec![ManagedFile {
                relative_path: PathBuf::from("app.exe"),
                stage_sha256: None,
                original_sha256: None,
            }],
            "2.0".into(),
            "public".into(),
        );

        // Step 1: Backup
        execute_backup(&mut journal, &temp).expect("Backup should succeed");
        assert_eq!(journal.phase, TransactionPhase::BackingUp);
        assert!(backup_dir.join("app.exe").exists());
        assert_eq!(
            fs::read_to_string(backup_dir.join("app.exe")).unwrap(),
            "version 1.0"
        );

        // Step 2: Swap
        execute_swap(&mut journal, &temp).expect("Swap should succeed");
        assert_eq!(journal.phase, TransactionPhase::Swapping);
        assert_eq!(
            fs::read_to_string(install_dir.join("app.exe")).unwrap(),
            "version 2.0"
        );

        // Step 3: Simulate failure -> Rollback
        execute_rollback(&mut journal, &temp).expect("Rollback should succeed");
        assert_eq!(journal.phase, TransactionPhase::RolledBack);
        assert_eq!(
            fs::read_to_string(install_dir.join("app.exe")).unwrap(),
            "version 1.0"
        );

        let _ = fs::remove_dir_all(&temp);
    }
}
