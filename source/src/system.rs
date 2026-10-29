use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Threading::GetCurrentProcess;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn SetProcessWorkingSetSize(hprocess: HANDLE, min: usize, max: usize) -> i32;
}
use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP};
use windows::Win32::System::Registry::{
    RegOpenKeyExW, RegSetValueExW, RegDeleteValueW, RegQueryValueExW, RegCloseKey,
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ
};

#[derive(Clone, Debug, PartialEq)]
pub struct BatteryInfo {
    pub has_battery: bool,
    pub is_charging: bool,
    pub percentage: u8,
}

pub fn get_battery_status() -> Option<BatteryInfo> {
    unsafe {
        let mut status = SYSTEM_POWER_STATUS::default();
        if GetSystemPowerStatus(&mut status).is_ok() {
            // BatteryFlag: 128 = No system battery, 255 = Unknown status
            // BatteryLifePercent: 255 = Unknown percent
            if status.BatteryFlag == 128 || status.BatteryLifePercent > 100 {
                return None;
            }
            let is_charging = (status.BatteryFlag & 8) != 0 || status.ACLineStatus == 1;
            Some(BatteryInfo {
                has_battery: true,
                is_charging,
                percentage: status.BatteryLifePercent,
            })
        } else {
            None
        }
    }
}

pub fn trim_working_set() {
    unsafe {
        let _ = SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
    }
}

pub fn get_master_volume() -> Option<(f32, bool)> {
    unsafe {
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER).ok()?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia).ok()?;
        let volume: IAudioEndpointVolume = device.Activate(CLSCTX_INPROC_SERVER, None).ok()?;
        let level = volume.GetMasterVolumeLevelScalar().ok()?;
        let mute = volume.GetMute().ok()?.as_bool();
        Some((level.clamp(0.0, 1.0), mute))
    }
}

pub fn adjust_master_volume(delta: f32) -> Option<(f32, bool)> {
    unsafe {
        let enumerator: IMMDeviceEnumerator = match CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER) {
            Ok(e) => e,
            Err(_) => {
                step_volume_fallback(delta > 0.0);
                return None;
            }
        };
        let device = match enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) {
            Ok(d) => d,
            Err(_) => {
                step_volume_fallback(delta > 0.0);
                return None;
            }
        };
        let volume: IAudioEndpointVolume = match device.Activate(CLSCTX_INPROC_SERVER, None) {
            Ok(v) => v,
            Err(_) => {
                step_volume_fallback(delta > 0.0);
                return None;
            }
        };
        let level = match volume.GetMasterVolumeLevelScalar() {
            Ok(lvl) => lvl,
            Err(_) => {
                step_volume_fallback(delta > 0.0);
                return None;
            }
        };
        let new_level = (level + delta).clamp(0.0, 1.0);
        let _ = volume.SetMasterVolumeLevelScalar(new_level, std::ptr::null());
        let mut mute = volume.GetMute().map(|m| m.as_bool()).unwrap_or(false);
        if mute && delta > 0.0 {
            let _ = volume.SetMute(false, std::ptr::null());
            mute = false;
        }
        Some((new_level, mute))
    }
}

pub fn step_volume_fallback(up: bool) {
    unsafe {
        let vk = if up { 0xAF } else { 0xAE }; // VK_VOLUME_UP (0xAF) / VK_VOLUME_DOWN (0xAE)
        keybd_event(vk, 0, Default::default(), 0);
        keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
    }
}

pub fn is_run_at_startup_enabled() -> bool {
    unsafe {
        let mut key = HKEY::default();
        let subkey = windows::core::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let val_name = windows::core::w!("mynotch");
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, None, KEY_READ, &mut key).is_ok() {
            let mut cb_data = 0u32;
            let status = RegQueryValueExW(key, val_name, None, None, None, Some(&mut cb_data as *mut u32));
            let _ = RegCloseKey(key);
            status.is_ok() && cb_data > 0
        } else {
            false
        }
    }
}

pub fn set_run_at_startup(enable: bool) -> Result<(), String> {
    unsafe {
        let subkey = windows::core::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let val_name = windows::core::w!("mynotch");
        let mut key = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, None, KEY_WRITE, &mut key).is_err() {
            return Err("Gagal membuka registry Windows.".into());
        }

        let result = if enable {
            let exe_path = match std::env::current_exe() {
                Ok(p) => p,
                Err(e) => {
                    let _ = RegCloseKey(key);
                    return Err(format!("Gagal mendapatkan path executable: {e}"));
                }
            };
            let path_str = format!("\"{}\"", exe_path.to_string_lossy());
            let wide: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();
            let byte_slice = std::slice::from_raw_parts(
                wide.as_ptr() as *const u8,
                wide.len() * std::mem::size_of::<u16>(),
            );
            if RegSetValueExW(key, val_name, None, REG_SZ, Some(byte_slice)).is_ok() {
                Ok(())
            } else {
                Err("Gagal menulis nilai registry.".into())
            }
        } else {
            let status = RegDeleteValueW(key, val_name);
            if status.is_ok() || status.0 == 2 {
                Ok(())
            } else {
                Err("Gagal menghapus nilai registry.".into())
            }
        };

        let _ = RegCloseKey(key);
        result
    }
}

pub fn open_browser(url: &str) {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let url_w: Vec<u16> = url.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let _ = ShellExecuteW(
            None,
            windows::core::w!("open"),
            windows::core::PCWSTR(url_w.as_ptr()),
            windows::core::PCWSTR::null(),
            windows::core::PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

pub fn open_folder_in_explorer(path: &std::path::Path) -> Result<(), String> {
    if !path.exists() {
        let _ = std::fs::create_dir_all(path);
    }
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let path_str = path.to_string_lossy();
    let path_w: Vec<u16> = path_str.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let res = ShellExecuteW(
            None,
            windows::core::w!("explore"),
            windows::core::PCWSTR(path_w.as_ptr()),
            windows::core::PCWSTR::null(),
            windows::core::PCWSTR::null(),
            SW_SHOWNORMAL,
        );
        if (res.0 as usize) > 32 {
            Ok(())
        } else {
            let res2 = ShellExecuteW(
                None,
                windows::core::w!("open"),
                windows::core::PCWSTR(path_w.as_ptr()),
                windows::core::PCWSTR::null(),
                windows::core::PCWSTR::null(),
                SW_SHOWNORMAL,
            );
            if (res2.0 as usize) > 32 {
                Ok(())
            } else {
                Err(format!("Gagal membuka explorer: {:?}", res2.0))
            }
        }
    }
}

pub fn pick_mod_file() -> Option<std::path::PathBuf> {
    use windows::Win32::UI::Controls::Dialogs::{
        GetOpenFileNameW, OPENFILENAMEW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR,
        OFN_PATHMUSTEXIST,
    };

    let mut file_buf = [0u16; 1024];

    let mut filter: Vec<u16> = Vec::new();
    filter.extend("Mod Package (*.zip, *.json)\0".encode_utf16());
    filter.extend("*.zip;*.json;*.mod.json\0".encode_utf16());
    filter.extend("ZIP Package (*.zip)\0".encode_utf16());
    filter.extend("*.zip\0".encode_utf16());
    filter.extend("JSON Manifest (*.json)\0".encode_utf16());
    filter.extend("*.json;*.mod.json\0".encode_utf16());
    filter.extend("All Files (*.*)\0".encode_utf16());
    filter.extend("*.*\0".encode_utf16());
    filter.push(0);

    let title: Vec<u16> = "Pilih Mod MyNotch (.zip / .json)\0".encode_utf16().collect();

    let mut ofn = OPENFILENAMEW::default();
    ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.lpstrFilter = windows::core::PCWSTR(filter.as_ptr());
    ofn.lpstrFile = windows::core::PWSTR(file_buf.as_mut_ptr());
    ofn.nMaxFile = file_buf.len() as u32;
    ofn.lpstrTitle = windows::core::PCWSTR(title.as_ptr());
    ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER;

    unsafe {
        if GetOpenFileNameW(&mut ofn).as_bool() {
            let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
            let path_str = String::from_utf16_lossy(&file_buf[..len]);
            if !path_str.trim().is_empty() {
                return Some(std::path::PathBuf::from(path_str));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battery_info_struct() {
        let b = BatteryInfo {
            has_battery: true,
            is_charging: true,
            percentage: 95,
        };
        assert!(b.has_battery);
        assert!(b.is_charging);
        assert_eq!(b.percentage, 95);
    }

    #[test]
    fn test_run_at_startup_check() {
        // Calling is_run_at_startup_enabled should safely succeed without panic
        let _ = is_run_at_startup_enabled();
    }

    #[test]
    fn test_open_browser_url_valid() {
        let url = "https://github.com/arwankemis/mynotch";
        assert!(url.starts_with("https://"));
    }

    #[test]
    fn test_open_folder_creates_nonexistent() {
        let temp_dir = std::env::temp_dir().join(format!("mynotch_test_explorer_{}", std::process::id()));
        assert!(!temp_dir.exists());
        let _ = open_folder_in_explorer(&temp_dir);
        assert!(temp_dir.exists());
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}


