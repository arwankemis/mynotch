use windows::core::{Interface, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS};
use windows::Win32::Media::Audio::{
    AudioSessionStateActive, eCapture, DEVICE_STATE_ACTIVE, IAudioSessionControl2,
    IAudioSessionManager2, IMMDeviceEnumerator, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
use crate::privacy::model::Coverage;

struct ComApartment {
    should_uninitialize: bool,
}

impl ComApartment {
    fn enter_mta() -> Self {
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        // S_OK (0) or S_FALSE (1) means initialized on this thread
        // RPC_E_CHANGED_MODE means already initialized with different apartment, don't uninit
        let should_uninitialize = hr.is_ok();
        Self {
            should_uninitialize,
        }
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.should_uninitialize {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioEndpointProbe {
    pub total_capture_endpoints: usize,
    pub active_sessions_count: usize,
    pub active_applications: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsentStoreProbe {
    pub key_exists: bool,
    pub entries_found: usize,
    pub active_candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    pub mic_supported: bool,
    pub mic_endpoints: usize,
    pub mic_active_sessions: usize,
    pub camera_accessible: bool,
    pub camera_entries: usize,
    pub camera_active_candidates: usize,
    pub screen_status: Coverage,
}

/// Helper to get process executable name given a PID.
pub fn get_process_name_by_pid(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut path_buf = [0u16; 512];
        let mut len = path_buf.len() as u32;
        let success = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            PWSTR(path_buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        if success.is_ok() && len > 0 {
            let full_path = String::from_utf16_lossy(&path_buf[..len as usize]);
            let exe_name = full_path
                .rsplit(&['\\', '/'][..])
                .next()
                .unwrap_or(&full_path)
                .to_string();
            Some(exe_name)
        } else {
            None
        }
    }
}

/// Probe CoreAudio capture endpoints and active sessions safely.
/// NEVER opens an audio stream or captures buffers.
pub fn probe_audio_endpoints() -> Result<AudioEndpointProbe, String> {
    let _com = ComApartment::enter_mta();

    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;

        let endpoints = enumerator
            .EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)
            .map_err(|e| e.to_string())?;

        let count = endpoints.GetCount().map_err(|e| e.to_string())? as usize;
        let mut active_sessions_count = 0usize;
        let mut active_applications = Vec::new();

        for i in 0..count {
            let device = match endpoints.Item(i as u32) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let session_manager: IAudioSessionManager2 = match device.Activate(CLSCTX_ALL, None) {
                Ok(mgr) => mgr,
                Err(_) => continue,
            };

            let session_enum = match session_manager.GetSessionEnumerator() {
                Ok(se) => se,
                Err(_) => continue,
            };

            let sessions_count = session_enum.GetCount().unwrap_or(0);
            for s in 0..sessions_count {
                let session = match session_enum.GetSession(s) {
                    Ok(sess) => sess,
                    Err(_) => continue,
                };

                let state = match session.GetState() {
                    Ok(st) => st,
                    Err(_) => continue,
                };

                if state == AudioSessionStateActive {
                    active_sessions_count += 1;
                    if let Ok(control2) = session.cast::<IAudioSessionControl2>() {
                        if let Ok(pid) = control2.GetProcessId() {
                            if pid != 0 {
                                let name = get_process_name_by_pid(pid)
                                    .unwrap_or_else(|| format!("PID: {pid}"));
                                if !active_applications.iter().any(|existing: &String| existing.eq_ignore_ascii_case(&name)) {
                                    active_applications.push(name);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(AudioEndpointProbe {
            total_capture_endpoints: count,
            active_sessions_count,
            active_applications,
        })
    }
}

fn open_registry_key(root: HKEY, subkey: &str) -> Option<HKEY> {
    let subkey_utf16: Vec<u16> = subkey.encode_utf16().chain(Some(0)).collect();
    let mut hkey = HKEY::default();
    unsafe {
        let res = RegOpenKeyExW(
            root,
            PCWSTR(subkey_utf16.as_ptr()),
            None,
            KEY_READ,
            &mut hkey,
        );
        if res == ERROR_SUCCESS {
            Some(hkey)
        } else {
            None
        }
    }
}

fn enumerate_registry_subkeys(key: HKEY) -> Vec<String> {
    let mut subkeys = Vec::new();
    let mut index = 0u32;
    let mut name_buf = [0u16; 256];

    loop {
        let mut name_len = name_buf.len() as u32;
        let res = unsafe {
            RegEnumKeyExW(
                key,
                index,
                Some(PWSTR(name_buf.as_mut_ptr())),
                &mut name_len,
                None,
                None,
                None,
                None,
            )
        };

        if res == ERROR_NO_MORE_ITEMS {
            break;
        }
        if res == ERROR_SUCCESS {
            let name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
            subkeys.push(name);
        }
        index += 1;
    }

    subkeys
}

fn read_u64_registry_value(key: HKEY, value_name: &str) -> Option<u64> {
    let val_utf16: Vec<u16> = value_name.encode_utf16().chain(Some(0)).collect();
    let mut data: u64 = 0;
    let mut data_size = std::mem::size_of::<u64>() as u32;

    unsafe {
        let res = RegQueryValueExW(
            key,
            PCWSTR(val_utf16.as_ptr()),
            None,
            None,
            Some(&mut data as *mut u64 as *mut u8),
            Some(&mut data_size as *mut u32),
        );
        if res == ERROR_SUCCESS && data_size == 8 {
            Some(data)
        } else {
            None
        }
    }
}

/// Inspect webcam ConsentStore safely and read-only.
/// NEVER opens camera device or touches DirectShow/MediaFoundation capture.
pub fn probe_consent_store_webcam() -> Result<ConsentStoreProbe, String> {
    const WEBCAM_PATH: &str =
        "Software\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\webcam";

    let root_key = match open_registry_key(HKEY_CURRENT_USER, WEBCAM_PATH) {
        Some(k) => k,
        None => {
            return Ok(ConsentStoreProbe {
                key_exists: false,
                entries_found: 0,
                active_candidates: Vec::new(),
            });
        }
    };

    let mut entries_found = 0usize;
    let mut active_candidates = Vec::new();

    let direct_subkeys = enumerate_registry_subkeys(root_key);

    for subkey_name in &direct_subkeys {
        if subkey_name.eq_ignore_ascii_case("NonPackaged") {
            // Check non-packaged applications
            if let Some(non_pkg_key) = open_registry_key(root_key, subkey_name) {
                let non_pkg_apps = enumerate_registry_subkeys(non_pkg_key);
                for app_key_name in non_pkg_apps {
                    entries_found += 1;
                    if let Some(app_key) = open_registry_key(non_pkg_key, &app_key_name) {
                        let start = read_u64_registry_value(app_key, "LastUsedTimeStart").unwrap_or(0);
                        let stop = read_u64_registry_value(app_key, "LastUsedTimeStop").unwrap_or(0);
                        if start > 0 && (stop == 0 || start > stop) {
                            let clean_name = app_key_name
                                .replace('#', "\\")
                                .rsplit(&['\\', '/'][..])
                                .next()
                                .unwrap_or(&app_key_name)
                                .to_string();
                            active_candidates.push(clean_name);
                        }
                        unsafe {
                            let _ = RegCloseKey(app_key);
                        }
                    }
                }
                unsafe {
                    let _ = RegCloseKey(non_pkg_key);
                }
            }
        } else {
            // Packaged UWP app (e.g. Microsoft.WindowsCamera_8wekyb3d8bbwe)
            entries_found += 1;
            if let Some(app_key) = open_registry_key(root_key, subkey_name) {
                let start = read_u64_registry_value(app_key, "LastUsedTimeStart").unwrap_or(0);
                let stop = read_u64_registry_value(app_key, "LastUsedTimeStop").unwrap_or(0);
                if start > 0 && (stop == 0 || start > stop) {
                    active_candidates.push(subkey_name.clone());
                }
                unsafe {
                    let _ = RegCloseKey(app_key);
                }
            }
        }
    }

    unsafe {
        let _ = RegCloseKey(root_key);
    }

    Ok(ConsentStoreProbe {
        key_exists: true,
        entries_found,
        active_candidates,
    })
}

pub fn run_full_probe() -> ProbeReport {
    let mic_probe = probe_audio_endpoints();
    let (mic_supported, mic_endpoints, mic_active_sessions) = match mic_probe {
        Ok(p) => (true, p.total_capture_endpoints, p.active_sessions_count),
        Err(_) => (false, 0, 0),
    };

    let cam_probe = probe_consent_store_webcam();
    let (camera_accessible, camera_entries, camera_active_candidates) = match cam_probe {
        Ok(p) => (p.key_exists, p.entries_found, p.active_candidates.len()),
        Err(_) => (false, 0, 0),
    };

    ProbeReport {
        mic_supported,
        mic_endpoints,
        mic_active_sessions,
        camera_accessible,
        camera_entries,
        camera_active_candidates,
        screen_status: Coverage::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_session_enumeration_without_stream_open() {
        let report = probe_audio_endpoints();
        assert!(
            report.is_ok(),
            "CoreAudio endpoint enumeration must succeed without opening audio stream: {:?}",
            report.err()
        );
        let res = report.unwrap();
        // Just verify it enumerated safely without panicking or hanging
        println!(
            "Probe audio endpoints: count={}, active_sessions={}",
            res.total_capture_endpoints, res.active_sessions_count
        );
    }

    #[test]
    fn test_consent_store_webcam_read_only_safe() {
        let report = probe_consent_store_webcam();
        assert!(
            report.is_ok(),
            "Reading ConsentStore registry must succeed without errors: {:?}",
            report.err()
        );
        let res = report.unwrap();
        println!(
            "Probe ConsentStore webcam: key_exists={}, entries_found={}",
            res.key_exists, res.entries_found
        );
    }

    #[test]
    fn test_full_probe_honest_reporting() {
        let report = run_full_probe();
        assert!(report.mic_supported, "CoreAudio should be supported on Windows");
        // Screen capture is unsupported / capability gap
        assert_eq!(report.screen_status, Coverage::Unsupported);
    }
}
