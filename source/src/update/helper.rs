use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use super::transaction::{
    execute_backup, execute_rollback, execute_swap, write_receipt, TransactionJournal,
    TransactionPhase,
};

pub fn is_helper_invocation(args: &[String]) -> Option<PathBuf> {
    for i in 0..args.len() {
        if args[i] == "--internal-update-helper" {
            if i + 1 < args.len() {
                return Some(PathBuf::from(&args[i + 1]));
            }
        }
    }
    None
}

pub fn health_ack_file(tx_dir: &Path) -> PathBuf {
    tx_dir.join("health_ack")
}

pub fn write_health_ack(tx_dir: &Path) -> Result<(), String> {
    let path = health_ack_file(tx_dir);
    fs::write(&path, b"HEALTH_OK").map_err(|e| format!("Gagal menulis health ACK: {e}"))
}

#[cfg(windows)]
fn wait_for_pid_exit(pid: u32, timeout_secs: u64) -> bool {
    if pid == 0 {
        return true;
    }
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, false, pid);
        if let Ok(h) = handle {
            if h.is_invalid() || h.0.is_null() {
                // Process already terminated or inaccessible
                return true;
            }
            let wait_ms = (timeout_secs * 1000) as u32;
            let res = WaitForSingleObject(h, wait_ms);
            let _ = CloseHandle(h);
            // WAIT_OBJECT_0 is 0
            res.0 == 0
        } else {
            // Process does not exist
            true
        }
    }
}

#[cfg(not(windows))]
fn wait_for_pid_exit(_pid: u32, _timeout_secs: u64) -> bool {
    true
}

pub fn run_helper(tx_dir: &Path) -> Result<TransactionPhase, String> {
    run_helper_with_timeout(tx_dir, 30)
}

pub fn run_helper_with_timeout(
    tx_dir: &Path,
    healthcheck_timeout_secs: u64,
) -> Result<TransactionPhase, String> {
    let mut journal = TransactionJournal::load(tx_dir)?;

    // Step 1: Wait for parent process to exit
    if !wait_for_pid_exit(journal.parent_pid, 20) {
        let err = format!(
            "Parent process (PID {}) tidak berhenti dalam batas waktu 20 detik.",
            journal.parent_pid
        );
        journal.record_error(tx_dir, err.clone())?;
        return Err(err);
    }

    // Step 2: Backup files if not yet done
    if journal.phase == TransactionPhase::Prepared {
        if let Err(e) = execute_backup(&mut journal, tx_dir) {
            journal.record_error(tx_dir, format!("Backup gagal: {e}"))?;
            return Err(e);
        }
    }

    // Step 3: Swap files
    if let Err(e) = execute_swap(&mut journal, tx_dir) {
        journal.record_error(tx_dir, format!("Swap gagal: {e}. Menjalankan rollback..."))?;
        let _ = execute_rollback(&mut journal, tx_dir);
        let _ = relaunch_binary(&journal.target_exe, &journal.install_dir, None);
        return Ok(TransactionPhase::RolledBack);
    }

    // Remove any previous health ACK
    let ack_path = health_ack_file(tx_dir);
    let _ = fs::remove_file(&ack_path);

    // Step 4: Relaunch new binary
    journal.set_phase(tx_dir, TransactionPhase::Relaunching)?;
    let child_res = relaunch_binary(
        &journal.target_exe,
        &journal.install_dir,
        Some(tx_dir),
    );

    let mut child = match child_res {
        Ok(c) => c,
        Err(e) => {
            journal.record_error(
                tx_dir,
                format!("Gagal meluncurkan binary baru: {e}. Menjalankan rollback..."),
            )?;
            let _ = execute_rollback(&mut journal, tx_dir);
            let _ = relaunch_binary(&journal.target_exe, &journal.install_dir, None);
            return Ok(TransactionPhase::RolledBack);
        }
    };

    // Step 5: Wait for healthcheck ACK
    let deadline = Instant::now() + Duration::from_secs(healthcheck_timeout_secs);
    let mut health_ok = false;

    while Instant::now() < deadline {
        if ack_path.exists() {
            health_ok = true;
            break;
        }

        // Check if child crashed or exited prematurely
        match child.try_wait() {
            Ok(Some(status)) => {
                let err_msg = format!("Binary baru keluar sebelum health ACK dengan status: {status:?}");
                journal.record_error(tx_dir, err_msg)?;
                break;
            }
            Ok(None) => {
                // Still running
            }
            Err(e) => {
                let err_msg = format!("Gagal memantau child process baru: {e}");
                journal.record_error(tx_dir, err_msg)?;
                break;
            }
        }

        thread::sleep(Duration::from_millis(100));
    }

    if !health_ok {
        // Kill child process if still alive
        let _ = child.kill();
        let _ = child.wait();

        // Rollback managed files
        let _ = execute_rollback(&mut journal, tx_dir);

        // Relaunch old version
        let _ = relaunch_binary(&journal.target_exe, &journal.install_dir, None);
        return Ok(TransactionPhase::RolledBack);
    }

    // Step 6: Commit
    journal.set_phase(tx_dir, TransactionPhase::Committed)?;
    let _ = write_receipt(
        &journal.install_dir,
        &journal.target_version,
        &journal.target_profile,
    );

    // Clean up temporary stage and backup directories
    let _ = fs::remove_dir_all(&journal.stage_dir);
    let _ = fs::remove_dir_all(&journal.backup_dir);
    let _ = fs::remove_file(&ack_path);

    Ok(TransactionPhase::Committed)
}

fn relaunch_binary(
    target_exe: &Path,
    install_dir: &Path,
    ack_tx_dir: Option<&Path>,
) -> std::io::Result<std::process::Child> {
    let mut cmd = Command::new(target_exe);
    cmd.current_dir(install_dir);
    if let Some(tx_dir) = ack_tx_dir {
        cmd.arg("--update-ack-tx");
        cmd.arg(tx_dir);
    }
    cmd.spawn()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_helper_invocation() {
        let args = vec![
            "mynotch.exe".into(),
            "--internal-update-helper".into(),
            "C:\\path\\to\\tx".into(),
        ];
        assert_eq!(
            is_helper_invocation(&args),
            Some(PathBuf::from("C:\\path\\to\\tx"))
        );

        let regular_args = vec!["mynotch.exe".into(), "--preview".into()];
        assert_eq!(is_helper_invocation(&regular_args), None);
    }

    #[test]
    fn test_health_ack_write_and_detect() {
        let temp = std::env::temp_dir().join(format!("test_health_ack_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp);

        assert!(!health_ack_file(&temp).exists());
        write_health_ack(&temp).expect("Writing health ack should succeed");
        assert!(health_ack_file(&temp).exists());

        let _ = fs::remove_dir_all(&temp);
    }
}
