use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::net::Cancellation;

pub const MAX_UPDATE_PACKAGE_BYTES: u64 = 128 * 1024 * 1024; // 128 MiB

pub fn is_allowed_download_host(host: &str) -> bool {
    let lower = host.to_ascii_lowercase();
    #[cfg(test)]
    if lower == "127.0.0.1" || lower == "localhost" {
        return true;
    }
    lower == "github.com"
        || lower == "api.github.com"
        || lower == "objects.githubusercontent.com"
        || lower == "raw.githubusercontent.com"
        || lower == "github-releases.githubusercontent.com"
        || lower.ends_with(".github.com")
        || lower.ends_with(".githubusercontent.com")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDownloadUrl {
    pub host: String,
    pub port: u16,
    pub resource: String,
}

pub fn parse_download_url(url: &str) -> Result<ParsedDownloadUrl, String> {
    if url.len() > 16384 || !url.is_ascii() || url.bytes().any(|b| b <= 32 || b == 127) {
        return Err("URL download tidak valid atau terlalu panjang.".into());
    }
    let rest = url.strip_prefix("https://").ok_or("URL download harus menggunakan HTTPS.")?;
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() || authority.contains('@') || rest.contains('#') || authority.contains('\\') {
        return Err("URL HTTPS tidak valid; kredensial dan fragment dilarang.".into());
    }

    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().map_err(|_| "Port URL tidak valid.")?),
        None => (authority, 443),
    };

    if !is_allowed_download_host(host) {
        return Err(format!("Host download '{host}' tidak diizinkan. Hanya domain GitHub yang diperbolehkan."));
    }

    let tail = &rest[authority_end..];
    let resource = if tail.is_empty() {
        "/".to_owned()
    } else if tail.starts_with('?') {
        format!("/{tail}")
    } else {
        tail.to_owned()
    };

    Ok(ParsedDownloadUrl {
        host: host.to_string(),
        port,
        resource,
    })
}

pub fn download_to_file<F>(
    initial_url: &str,
    part_path: &Path,
    final_path: &Path,
    expected_sha256: Option<&str>,
    cancel: &Cancellation,
    mut on_progress: F,
) -> Result<u64, String>
where
    F: FnMut(u64, Option<u64>),
{
    if let Some(parent) = part_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Gagal membuat direktori download: {e}"))?;
    }

    let mut current_url = initial_url.to_string();
    let mut redirects = 0;

    let total_bytes_downloaded = loop {
        if cancel.is_cancelled() {
            let _ = fs::remove_file(part_path);
            return Err("Pekerjaan dibatalkan.".into());
        }

        if redirects > 5 {
            let _ = fs::remove_file(part_path);
            return Err("Terlalu banyak redirect saat mengunduh pembaruan (maksimal 5).".into());
        }

        let parsed = parse_download_url(&current_url)?;

        #[cfg(windows)]
        {
            match winhttp_stream_download(
                &parsed,
                part_path,
                cancel,
                &mut on_progress,
            )? {
                StreamResult::Redirect(next_url) => {
                    redirects += 1;
                    current_url = next_url;
                    continue;
                }
                StreamResult::Done(bytes) => {
                    break bytes;
                }
            }
        }

        #[cfg(not(windows))]
        {
            let _ = (parsed, part_path, &mut on_progress);
            return Err("Downloader native memerlukan platform Windows.".into());
        }
    };

    // Verify SHA-256 if expected
    if let Some(expected) = expected_sha256 {
        let actual_hash = super::crypto::hash_file_hex(part_path)
            .map_err(|e| format!("Gagal memverifikasi hash berkas unduhan: {e}"))?;
        if !actual_hash.eq_ignore_ascii_case(expected.trim()) {
            let _ = fs::remove_file(part_path);
            return Err(format!(
                "Hash SHA-256 paket tidak cocok! Diharapkan {expected}, didapat {actual_hash}"
            ));
        }
    }

    // Atomic rename from .part to final destination
    if final_path.exists() {
        let _ = fs::remove_file(final_path);
    }
    fs::rename(part_path, final_path)
        .map_err(|e| format!("Gagal mengganti nama berkas selesai unduh: {e}"))?;

    Ok(total_bytes_downloaded)
}

pub enum StreamResult {
    Redirect(String),
    Done(u64),
}

#[cfg(windows)]
fn winhttp_stream_download<F>(
    url: &ParsedDownloadUrl,
    part_path: &Path,
    cancel: &Cancellation,
    on_progress: &mut F,
) -> Result<StreamResult, String>
where
    F: FnMut(u64, Option<u64>),
{
    use std::{ffi::c_void, ptr};
    type Handle = *mut c_void;

    #[link(name = "winhttp", kind = "raw-dylib")]
    unsafe extern "system" {
        fn WinHttpOpen(agent: *const u16, access: u32, proxy: *const u16, bypass: *const u16, flags: u32) -> Handle;
        fn WinHttpConnect(session: Handle, server: *const u16, port: u16, reserved: u32) -> Handle;
        fn WinHttpOpenRequest(connect: Handle, verb: *const u16, resource: *const u16, version: *const u16, referer: *const u16, accept: *const *const u16, flags: u32) -> Handle;
        fn WinHttpSetTimeouts(handle: Handle, resolve: i32, connect: i32, send: i32, receive: i32) -> i32;
        fn WinHttpSetOption(handle: Handle, option: u32, buffer: *const c_void, length: u32) -> i32;
        fn WinHttpSendRequest(request: Handle, headers: *const u16, length: u32, optional: *const c_void, optional_length: u32, total_length: u32, context: usize) -> i32;
        fn WinHttpReceiveResponse(request: Handle, reserved: *const c_void) -> i32;
        fn WinHttpQueryHeaders(request: Handle, info: u32, name: *const u16, buffer: *mut c_void, length: *mut u32, index: *mut u32) -> i32;
        fn WinHttpReadData(request: Handle, buffer: *mut c_void, length: u32, read: *mut u32) -> i32;
        fn WinHttpCloseHandle(handle: Handle) -> i32;
    }

    struct Owned(Handle);
    impl Drop for Owned {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { WinHttpCloseHandle(self.0); }
            }
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    fn error(op: &str) -> String {
        format!(
            "Download HTTPS gagal saat {op} (Windows {}).",
            std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
        )
    }

    unsafe {
        let agent = wide("NotchUpdater/0.1");
        let session = Owned(WinHttpOpen(agent.as_ptr(), 4, ptr::null(), ptr::null(), 0));
        if session.0.is_null() {
            return Err(error("membuka sesi WinHTTP"));
        }
        if WinHttpSetTimeouts(session.0, 15000, 15000, 15000, 15000) == 0 {
            return Err(error("mengatur timeout"));
        }

        let host = wide(&url.host);
        let connection = Owned(WinHttpConnect(session.0, host.as_ptr(), url.port, 0));
        if connection.0.is_null() {
            return Err(error("menghubungkan server"));
        }

        let verb = wide("GET");
        let resource = wide(&url.resource);
        let request = Owned(WinHttpOpenRequest(
            connection.0,
            verb.as_ptr(),
            resource.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            0x00800000, // WINHTTP_FLAG_SECURE
        ));
        if request.0.is_null() {
            return Err(error("membuat request"));
        }

        // Disable automatic redirect so we can audit and validate every hop
        let policy = 0_u32; // WINHTTP_OPTION_REDIRECT_POLICY_NEVER
        let _ = WinHttpSetOption(request.0, 88, &policy as *const _ as *const c_void, 4);

        let headers = wide("Accept: application/octet-stream, */*\r\nUser-Agent: NotchUpdater/0.1\r\n");
        if WinHttpSendRequest(
            request.0,
            headers.as_ptr(),
            (headers.len() - 1) as u32,
            ptr::null(),
            0,
            0,
            0,
        ) == 0 {
            return Err(error("mengirim request"));
        }

        if WinHttpReceiveResponse(request.0, ptr::null()) == 0 {
            return Err(error("menerima respons"));
        }

        // Check HTTP status code
        let mut status = 0_u32;
        let mut size = 4_u32;
        if WinHttpQueryHeaders(
            request.0,
            19 | 0x20000000, // WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER
            ptr::null(),
            &mut status as *mut _ as _,
            &mut size,
            ptr::null_mut(),
        ) == 0 {
            return Err(error("membaca status HTTP"));
        }

        // Handle redirects (301, 302, 303, 307, 308)
        if [301, 302, 303, 307, 308].contains(&status) {
            let mut loc_size = 0_u32;
            WinHttpQueryHeaders(request.0, 33, ptr::null(), ptr::null_mut(), &mut loc_size, ptr::null_mut());
            if loc_size > 0 {
                let mut buf: Vec<u16> = vec![0; (loc_size as usize / 2) + 1];
                if WinHttpQueryHeaders(
                    request.0,
                    33, // WINHTTP_QUERY_LOCATION
                    ptr::null(),
                    buf.as_mut_ptr() as _,
                    &mut loc_size,
                    ptr::null_mut(),
                ) != 0 {
                    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                    let location = String::from_utf16_lossy(&buf[..len]);
                    return Ok(StreamResult::Redirect(location));
                }
            }
            return Err("Redirect HTTP tanpa header Location yang valid.".into());
        }

        if !(200..300).contains(&status) {
            return Err(format!("Server mengembalikan status HTTP {status}."));
        }

        // Try reading Content-Length
        let mut total_expected: Option<u64> = None;
        let mut cl_size = 0_u32;
        WinHttpQueryHeaders(request.0, 5, ptr::null(), ptr::null_mut(), &mut cl_size, ptr::null_mut());
        if cl_size > 0 {
            let mut cl_buf: Vec<u16> = vec![0; (cl_size as usize / 2) + 1];
            if WinHttpQueryHeaders(
                request.0,
                5, // WINHTTP_QUERY_CONTENT_LENGTH
                ptr::null(),
                cl_buf.as_mut_ptr() as _,
                &mut cl_size,
                ptr::null_mut(),
            ) != 0 {
                let len = cl_buf.iter().position(|&c| c == 0).unwrap_or(cl_buf.len());
                let cl_str = String::from_utf16_lossy(&cl_buf[..len]);
                if let Ok(val) = cl_str.trim().parse::<u64>() {
                    total_expected = Some(val);
                }
            }
        }

        let mut out_file = File::create(part_path)
            .map_err(|e| format!("Gagal membuat berkas sementara '{part_path:?}': {e}"))?;

        let mut buffer = [0u8; 32768];
        let mut downloaded_bytes: u64 = 0;
        let started = Instant::now();

        loop {
            if cancel.is_cancelled() {
                drop(out_file);
                let _ = fs::remove_file(part_path);
                return Err("Pekerjaan dibatalkan.".into());
            }

            if started.elapsed() > Duration::from_secs(300) {
                drop(out_file);
                let _ = fs::remove_file(part_path);
                return Err("Waktu unduh melebihi batas 300 detik.".into());
            }

            let mut read = 0_u32;
            if WinHttpReadData(
                request.0,
                buffer.as_mut_ptr() as _,
                buffer.len() as u32,
                &mut read,
            ) == 0 {
                drop(out_file);
                let _ = fs::remove_file(part_path);
                return Err(error("membaca data chunk"));
            }

            if read == 0 {
                break;
            }

            downloaded_bytes += read as u64;
            if downloaded_bytes > MAX_UPDATE_PACKAGE_BYTES {
                drop(out_file);
                let _ = fs::remove_file(part_path);
                return Err(format!(
                    "Ukuran paket melebihi batas maksimum {MAX_UPDATE_PACKAGE_BYTES} bytes."
                ));
            }

            out_file
                .write_all(&buffer[..read as usize])
                .map_err(|e| format!("Gagal menulis data ke disk: {e}"))?;

            on_progress(downloaded_bytes, total_expected);
        }

        out_file.flush().map_err(|e| format!("Gagal flush berkas unduhan: {e}"))?;
        Ok(StreamResult::Done(downloaded_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowed_hosts() {
        assert!(is_allowed_download_host("github.com"));
        assert!(is_allowed_download_host("api.github.com"));
        assert!(is_allowed_download_host("objects.githubusercontent.com"));
        assert!(is_allowed_download_host("raw.githubusercontent.com"));
        assert!(is_allowed_download_host("release-assets.githubusercontent.com"));

        assert!(!is_allowed_download_host("evil.com"));
        assert!(!is_allowed_download_host("github.com.evil.com"));
        assert!(!is_allowed_download_host("notgithub.com"));
    }

    #[test]
    fn test_url_parsing() {
        let parsed = parse_download_url("https://github.com/arwankemis/mynotch/releases/download/v1/update.zip").unwrap();
        assert_eq!(parsed.host, "github.com");
        assert_eq!(parsed.port, 443);
        assert_eq!(parsed.resource, "/arwankemis/mynotch/releases/download/v1/update.zip");

        // Non-HTTPS rejected
        assert!(parse_download_url("http://github.com/update.zip").is_err());
        // Disallowed host rejected
        assert!(parse_download_url("https://evil.com/update.zip").is_err());
        // Userinfo rejected
        assert!(parse_download_url("https://user:pass@github.com/update.zip").is_err());
    }
}
