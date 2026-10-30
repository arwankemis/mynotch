use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};
use zip::ZipArchive;

use super::crypto::hash_file_hex;

pub const MAX_TOTAL_EXTRACTED_BYTES: u64 = 256 * 1024 * 1024; // 256 MiB
pub const MAX_SINGLE_FILE_BYTES: u64 = 128 * 1024 * 1024;    // 128 MiB
pub const MAX_ARCHIVE_FILES: usize = 256;

const RESERVED_DEVICE_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

pub fn validate_entry_name(name: &str) -> Result<PathBuf, String> {
    if name.is_empty() {
        return Err("Nama entri arsip kosong.".into());
    }

    if name.contains('\0') {
        return Err("Nama entri memuat byte null.".into());
    }

    // Reject Alternate Data Streams (:) and drive letters
    if name.contains(':') {
        return Err(format!("Nama entri '{name}' memuat karakter terlarang (ADS/drive)."));
    }

    // Reject absolute paths
    if name.starts_with('/') || name.starts_with('\\') {
        return Err(format!("Nama entri '{name}' adalah path absolut."));
    }

    let mut clean_path = PathBuf::new();
    let path = Path::new(name);

    for component in path.components() {
        match component {
            Component::Normal(c) => {
                let s = c.to_string_lossy();
                let trimmed = s.trim();

                if trimmed.is_empty() {
                    return Err(format!("Komponen path tidak valid pada '{name}'."));
                }

                // Windows collapses trailing spaces and dots
                if s.ends_with(' ') || s.ends_with('.') {
                    return Err(format!(
                        "Nama entri '{name}' memiliki komponen berakhiran titik atau spasi."
                    ));
                }

                // Check reserved Windows devices
                let base_name = s.split('.').next().unwrap_or(&s);
                for &dev in RESERVED_DEVICE_NAMES {
                    if base_name.eq_ignore_ascii_case(dev) {
                        return Err(format!(
                            "Nama entri '{name}' menggunakan nama perangkat Windows terlarang '{dev}'."
                        ));
                    }
                }

                clean_path.push(c);
            }
            Component::ParentDir => {
                return Err(format!(
                    "Upaya directory traversal ('..') terdeteksi pada entri '{name}'."
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!("Path absolut terlarang pada entri '{name}'."));
            }
            Component::CurDir => {
                // Ignore '.'
            }
        }
    }

    if clean_path.as_os_str().is_empty() {
        return Err(format!("Path tidak menghasilkan file valid pada '{name}'."));
    }

    // Ensure it doesn't target protected folders
    let clean_str = clean_path.to_string_lossy().to_ascii_lowercase();
    if clean_str.starts_with("data") || clean_str.starts_with("preview-data") {
        return Err(format!("Arsip mencoba mengekstrak ke folder data terlarang: '{name}'"));
    }

    Ok(clean_path)
}

pub fn extract_zip_data_safely(zip_bytes: &[u8], staging_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let reader = Cursor::new(zip_bytes);
    extract_zip_reader_safely(reader, staging_dir)
}

pub fn extract_zip_file_safely(zip_path: &Path, staging_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let file = File::open(zip_path).map_err(|e| format!("Gagal membuka file arsip: {e}"))?;
    extract_zip_reader_safely(file, staging_dir)
}

fn extract_zip_reader_safely<R: Read + std::io::Seek>(
    reader: R,
    staging_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    fs::create_dir_all(staging_dir).map_err(|e| format!("Gagal membuat folder staging: {e}"))?;

    let canonical_staging = staging_dir
        .canonicalize()
        .map_err(|e| format!("Gagal kanonikalisasi staging dir: {e}"))?;

    let mut archive = ZipArchive::new(reader).map_err(|e| format!("Arsip ZIP tidak valid: {e}"))?;

    let count = archive.len();
    if count > MAX_ARCHIVE_FILES {
        return Err(format!(
            "Arsip memiliki terlalu banyak berkas ({count} > {MAX_ARCHIVE_FILES})."
        ));
    }

    let mut total_extracted: u64 = 0;
    let mut extracted_files = Vec::new();

    for i in 0..count {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Gagal membaca entri ZIP #{i}: {e}"))?;

        let entry_name = entry.name().to_string();

        // If it's a directory
        if entry.is_dir() || entry_name.ends_with('/') || entry_name.ends_with('\\') {
            let rel_path = validate_entry_name(entry_name.trim_end_matches(['/', '\\']))?;
            let dest_dir = canonical_staging.join(&rel_path);
            fs::create_dir_all(&dest_dir)
                .map_err(|e| format!("Gagal membuat subdirektori staging: {e}"))?;
            continue;
        }

        let rel_path = validate_entry_name(&entry_name)?;
        let dest_file = canonical_staging.join(&rel_path);

        // Security check: destination must reside under canonical staging
        if let Some(parent) = dest_file.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Gagal membuat parent directory staging: {e}"))?;
            let canonical_parent = parent
                .canonicalize()
                .map_err(|e| format!("Gagal kanonikalisasi parent dir: {e}"))?;
            if !canonical_parent.starts_with(&canonical_staging) {
                return Err(format!(
                    "Entri '{entry_name}' keluar dari batas staging directory!"
                ));
            }
        }

        let uncompressed_size = entry.size();
        if uncompressed_size > MAX_SINGLE_FILE_BYTES {
            return Err(format!(
                "Ukuran berkas '{entry_name}' melebihi batas {MAX_SINGLE_FILE_BYTES} bytes."
            ));
        }

        total_extracted += uncompressed_size;
        if total_extracted > MAX_TOTAL_EXTRACTED_BYTES {
            return Err(format!(
                "Total ekstraksi melebihi batas aman {MAX_TOTAL_EXTRACTED_BYTES} bytes."
            ));
        }

        let mut out_file = File::create(&dest_file)
            .map_err(|e| format!("Gagal membuat berkas tujuan '{dest_file:?}': {e}"))?;

        let mut buffer = [0u8; 16384];
        let mut entry_bytes_read: u64 = 0;
        loop {
            let n = entry
                .read(&mut buffer)
                .map_err(|e| format!("Gagal mengekstrak berkas '{entry_name}': {e}"))?;
            if n == 0 {
                break;
            }
            entry_bytes_read += n as u64;
            if entry_bytes_read > MAX_SINGLE_FILE_BYTES {
                return Err(format!(
                    "Dekompresi berkas '{entry_name}' melebihi ukuran yang dideklarasikan!"
                ));
            }
            out_file
                .write_all(&buffer[..n])
                .map_err(|e| format!("Gagal menulis ke berkas '{dest_file:?}': {e}"))?;
        }

        extracted_files.push(rel_path);
    }

    Ok(extracted_files)
}

pub fn verify_file_hashes(
    staging_dir: &Path,
    expected_hashes: &[(PathBuf, String)],
) -> Result<(), String> {
    for (rel_path, expected_hex) in expected_hashes {
        let full_path = staging_dir.join(rel_path);
        if !full_path.exists() {
            return Err(format!("Berkas wajib hilang di staging: {rel_path:?}"));
        }
        let actual_hex = hash_file_hex(&full_path)
            .map_err(|e| format!("Gagal menghitung hash {rel_path:?}: {e}"))?;
        if !actual_hex.eq_ignore_ascii_case(expected_hex.trim()) {
            return Err(format!(
                "Hash berkas {rel_path:?} tidak cocok! Diharapkan {expected_hex}, didapat {actual_hex}"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn create_test_zip_with_entry(entry_name: &str, content: &[u8]) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut buf);
            let options = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file(entry_name, options).unwrap();
            writer.write_all(content).unwrap();
            writer.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn test_safe_extract_rejects_parent_traversal() {
        assert!(validate_entry_name("../evil.exe").is_err());
        assert!(validate_entry_name("a/../../evil.exe").is_err());
        assert!(validate_entry_name("..\\evil.exe").is_err());
        assert!(validate_entry_name("sub/..").is_err());

        let zip_data = create_test_zip_with_entry("../evil.exe", b"malicious");
        let temp = std::env::temp_dir().join(format!("test_zip_trav_{}", std::process::id()));
        let res = extract_zip_data_safely(&zip_data, &temp);
        assert!(res.is_err());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_safe_extract_rejects_absolute_and_drive_and_ads() {
        assert!(validate_entry_name("/etc/passwd").is_err());
        assert!(validate_entry_name("\\Windows\\System32\\cmd.exe").is_err());
        assert!(validate_entry_name("C:\\evil.exe").is_err());
        assert!(validate_entry_name("D:evil.exe").is_err());
        assert!(validate_entry_name("file.txt:hidden_stream").is_err());
        assert!(validate_entry_name("CON").is_err());
        assert!(validate_entry_name("aux.txt").is_err());
        assert!(validate_entry_name("com1.exe").is_err());
        assert!(validate_entry_name("data/mod.json").is_err());
        assert!(validate_entry_name("preview-data/leak").is_err());
        assert!(validate_entry_name("file.txt.").is_err());
        assert!(validate_entry_name("file.txt ").is_err());
    }

    #[test]
    fn test_safe_extract_happy_path() {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut buf);
            let options = SimpleFileOptions::default();
            writer.start_file("mynotch.exe", options).unwrap();
            writer.write_all(b"new binary content").unwrap();
            writer.start_file("libunwind.dll", options).unwrap();
            writer.write_all(b"dll content").unwrap();
            writer.finish().unwrap();
        }
        let zip_data = buf.into_inner();

        let temp = std::env::temp_dir().join(format!("test_zip_ok_{}", std::process::id()));
        let res = extract_zip_data_safely(&zip_data, &temp).expect("Extraction should succeed");
        assert_eq!(res.len(), 2);
        assert!(temp.join("mynotch.exe").exists());
        assert!(temp.join("libunwind.dll").exists());
        let _ = fs::remove_dir_all(&temp);
    }
}
