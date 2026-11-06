use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::Foundation::{HANDLE, HWND};
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

#[derive(Clone, Debug, PartialEq)]
pub struct TwsInfo {
    pub name: String,
    /// `None` when Windows does not publish `System.Devices.BatteryLife` for the device.
    pub battery: Option<u8>,
}

/// RAII wrapper for one WinRT apartment init. Only an S_OK init is torn down here:
/// RAII wrapper for WinRT apartment init.
struct WinRtApartment { initialized: bool }
impl WinRtApartment {
    fn new() -> Self {
        use windows::Win32::System::WinRT::RO_INIT_MULTITHREADED;
        windows::core::link!("api-ms-win-core-winrt-l1-1-0.dll" "system" fn RoInitialize(inittype: windows::Win32::System::WinRT::RO_INIT_TYPE) -> windows::core::HRESULT);
        let hr = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
        Self { initialized: hr.0 == 0 }
    }
}
impl Drop for WinRtApartment {
    fn drop(&mut self) {
        if self.initialized {
            use windows::Win32::System::WinRT::RoUninitialize;
            unsafe { RoUninitialize(); }
        }
    }
}

/// Connected Bluetooth audio device (TWS earbuds, headset, speaker).
/// Blocking: call from a worker thread, not the paint/timer thread.
pub fn get_tws_status() -> Result<Option<TwsInfo>, String> {
    use windows::Devices::Bluetooth::{BluetoothConnectionStatus, BluetoothDevice};
    use windows::Devices::Enumeration::DeviceInformation;
    use windows::core::HSTRING;

    let _apartment = WinRtApartment::new();
    let selector = BluetoothDevice::GetDeviceSelectorFromConnectionStatus(
        BluetoothConnectionStatus::Connected,
    )
    .map_err(|e| e.to_string())?;
    let requested: windows_collections::IIterable<HSTRING> = vec![
        HSTRING::from("System.Devices.Aep.IsConnected"),
        HSTRING::from("System.Devices.Aep.Bluetooth.Cod.Services.Audio"),
        HSTRING::from("System.Devices.Aep.Bluetooth.Cod.Major"),
        HSTRING::from("System.Devices.BatteryLife"),
    ]
    .into();
    let devices =
        DeviceInformation::FindAllAsyncAqsFilterAndAdditionalProperties(&selector, &requested)
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|e| e.to_string())?;
    // The selector also matches paired-but-disconnected devices, so check the live state.
    for index in 0..devices.Size().map_err(|e| e.to_string())? {
        let Ok(info) = devices.GetAt(index) else { continue };
        let Ok(props) = info.Properties() else { continue };
        if property(&props, "System.Devices.Aep.IsConnected", |v| v.GetBoolean()) != Some(true) {
            continue;
        }
        let audio = property(&props, "System.Devices.Aep.Bluetooth.Cod.Services.Audio", |v| v.GetBoolean());
        let major = property(&props, "System.Devices.Aep.Bluetooth.Cod.Major", |v| v.GetUInt16());
        if !is_audio_device(audio, major) {
            continue;
        }
        let name = info.Name().map(|n| n.to_string()).unwrap_or_default();
        let battery = property(&props, "System.Devices.BatteryLife", |v| v.GetUInt8())
            .or_else(|| property(&props, "System.Devices.BatteryLife", |v| v.GetInt32()).map(|v| v.clamp(0, 100) as u8));
        return Ok(Some(TwsInfo {
            name: if name.is_empty() { "Bluetooth audio".into() } else { name },
            battery,
        }));
    }
    Ok(None)
}

fn is_audio_device(services_audio: Option<bool>, cod_major: Option<u16>) -> bool {
    // shortcut: LE-only earbuds with no class-of-device are skipped; add a BLE battery path if seen.
    services_audio == Some(true) || cod_major == Some(4)
}

fn property<T>(
    props: &windows_collections::IMapView<windows::core::HSTRING, windows::core::IInspectable>,
    key: &str,
    read: impl Fn(&windows::Foundation::IPropertyValue) -> windows::core::Result<T>,
) -> Option<T> {
    use windows::core::Interface;
    let key = windows::core::HSTRING::from(key);
    if !props.HasKey(&key).ok()? {
        return None;
    }
    read(&props.Lookup(&key).ok()?.cast::<windows::Foundation::IPropertyValue>().ok()?).ok()
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
        volume.SetMasterVolumeLevelScalar(new_level, std::ptr::null()).ok()?;
        let mut mute = volume.GetMute().map(|m| m.as_bool()).unwrap_or(false);
        if mute && delta > 0.0 {
            let _ = volume.SetMute(false, std::ptr::null());
            mute = false;
        }
        Some((new_level, mute))
    }
}

pub fn toggle_master_mute() -> Option<(f32, bool)> {
    unsafe {
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER).ok()?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia).ok()?;
        let volume: IAudioEndpointVolume = device.Activate(CLSCTX_INPROC_SERVER, None).ok()?;
        let muted = volume.GetMute().ok()?.as_bool();
        volume.SetMute(!muted, std::ptr::null()).ok()?;
        let level = volume.GetMasterVolumeLevelScalar().ok()?.clamp(0.0, 1.0);
        Some((level, !muted))
    }
}

/// Windows 11 hosts the volume/brightness flyout in one small `XamlExplorerHostIslandWindow`
/// (its siblings are the full-screen Task Switching and Start hosts); Windows 10 uses
/// `NativeHWNDHost` + `DirectUIHWND`. Keeping that host minimized stops the shell from painting
/// its own OSD, so mynotch's volume HUD is the only indicator.
static OSD_HWND: AtomicIsize = AtomicIsize::new(0);
static OSD_TRIGGERED: AtomicBool = AtomicBool::new(false);
static OSD_HIDDEN: AtomicBool = AtomicBool::new(false);

fn find_volume_osd() -> Option<HWND> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, GetWindowRect};
    use windows::core::{w, PCWSTR};
    unsafe {
        let mut pick: Option<(i64, HWND)> = None;
        let mut after = None;
        loop {
            let Ok(outer) = FindWindowExW(None, after, w!("XamlExplorerHostIslandWindow"), PCWSTR::null()) else { break };
            after = Some(outer);
            if FindWindowExW(Some(outer), None, w!("Windows.UI.Composition.DesktopWindowContentBridge"), PCWSTR::null()).is_err() {
                continue;
            }
            let mut rect = RECT::default();
            if GetWindowRect(outer, &mut rect).is_err() {
                continue;
            }
            let (osd_w, osd_h) = (rect.right - rect.left, rect.bottom - rect.top);
            // The OSD is a small pill; skip the full-screen islands used by Task Switching/Start.
            if osd_w <= 0 || osd_h <= 0 || osd_w > 1000 || osd_h > 400 {
                continue;
            }
            let area = osd_w as i64 * osd_h as i64;
            if pick.is_none_or(|(best, _)| area < best) {
                pick = Some((area, outer));
            }
        }
        if let Some((_, window)) = pick {
            return Some(window);
        }
        let mut after = None;
        loop {
            let Ok(outer) = FindWindowExW(None, after, w!("NativeHWNDHost"), PCWSTR::null()) else { break };
            after = Some(outer);
            if FindWindowExW(Some(outer), None, w!("DirectUIHWND"), PCWSTR::null()).is_err() {
                continue;
            }
            // Same pill bounds as the Windows 11 path: a full-screen NativeHWNDHost
            // (another shell surface) must never be minimized as if it were the OSD.
            let mut rect = RECT::default();
            if GetWindowRect(outer, &mut rect).is_err() {
                continue;
            }
            let (osd_w, osd_h) = (rect.right - rect.left, rect.bottom - rect.top);
            if osd_w <= 0 || osd_h <= 0 || osd_w > 1000 || osd_h > 400 {
                continue;
            }
            return Some(outer);
        }
        None
    }
}

/// Creating the OSD once at startup means the host exists before the first hardware key, so it
/// can already be minimized. Two mute taps cancel out and leave the audio state untouched.
fn trigger_volume_osd() {
    unsafe {
        keybd_event(0xAD, 0, Default::default(), 0);
        keybd_event(0xAD, 0, KEYEVENTF_KEYUP, 0);
        std::thread::sleep(std::time::Duration::from_millis(80));
        keybd_event(0xAD, 0, Default::default(), 0);
        keybd_event(0xAD, 0, KEYEVENTF_KEYUP, 0);
    }
}

/// Worker half of the first-time OSD hide: create the host, wait for it to appear, then
/// minimize it. Never called on the UI thread: the two mute taps plus the settle sleep
/// cost ~330 ms, which used to freeze every frame and delay startup.
fn trigger_and_hide_osd_async() {
    trigger_volume_osd();
    std::thread::sleep(std::time::Duration::from_millis(250));
    if let Some(found) = find_volume_osd() {
        OSD_HWND.store(found.0 as isize, Ordering::Relaxed);
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_MINIMIZE};
            let _ = ShowWindow(found, SW_MINIMIZE);
        }
        OSD_HIDDEN.store(true, Ordering::Relaxed);
    }
}

pub fn set_volume_osd_hidden(hidden: bool) {
    use windows::Win32::UI::WindowsAndMessaging::{IsWindow, ShowWindow, SW_HIDE, SW_MINIMIZE, SW_RESTORE};
    unsafe {
        if !hidden {
            if !OSD_HIDDEN.swap(false, Ordering::Relaxed) {
                return;
            }
            let stored = OSD_HWND.swap(0, Ordering::Relaxed);
            if stored != 0 {
                let window = HWND(stored as *mut _);
                if IsWindow(Some(window)).as_bool() {
                    // Restore then hide: the stale pill must not linger, but the shell must be
                    // able to show it normally on the next volume change after mynotch exits.
                    let _ = ShowWindow(window, SW_RESTORE);
                    let _ = ShowWindow(window, SW_HIDE);
                }
            }
            return;
        }
        let stored = OSD_HWND.load(Ordering::Relaxed);
        let mut window = if stored != 0 && IsWindow(Some(HWND(stored as *mut _))).as_bool() {
            Some(HWND(stored as *mut _))
        } else {
            None
        };
        if window.is_none() {
            window = find_volume_osd();
            if window.is_none() && !OSD_TRIGGERED.swap(true, Ordering::Relaxed) {
                // First hide only: hand the trigger to a worker thread. The worker
                // minimizes the host once it appears; the next tick picks the stored
                // window up if the worker has not finished yet. The synchronous
                // find_volume_osd above is window enumeration only (no sleeps) and
                // stays cheap enough for the tick.
                let _ = std::thread::Builder::new()
                    .name("mynotch-osd".into())
                    .spawn(trigger_and_hide_osd_async);
            }
            if let Some(found) = window {
                OSD_HWND.store(found.0 as isize, Ordering::Relaxed);
            }
        }
        if let Some(window) = window {
            let _ = ShowWindow(window, SW_MINIMIZE);
            OSD_HIDDEN.store(true, Ordering::Relaxed);
        }
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

/// Only `https://` URLs with a non-empty host may be opened in the browser. A mod button
/// or a crafted target must never smuggle another scheme (`file:`, `ms-settings:`, an
/// executable path, or a control-character injection) through ShellExecuteW.
pub fn is_openable_browser_url(url: &str) -> bool {
    if !url.starts_with("https://") || url.bytes().any(|b| b <= 32 || b == 127) {
        return false;
    }
    let rest = &url["https://".len()..];
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    host_end > 0
}

pub fn open_browser(url: &str) {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    if !is_openable_browser_url(url) {
        return;
    }
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

/// Opens a Windows Settings page. Only the `ms-settings:` scheme is accepted. This is the
/// dedicated entry point for the app's own hardcoded privacy buttons, so `open_browser`
/// can stay https-only without breaking them. Never reachable from mod content.
pub fn open_settings_page(uri: &str) {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    if !uri.starts_with("ms-settings:") || uri.bytes().any(|b| b <= 32 || b == 127) {
        return;
    }
    let uri_w: Vec<u16> = uri.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let _ = ShellExecuteW(
            None,
            windows::core::w!("open"),
            windows::core::PCWSTR(uri_w.as_ptr()),
            windows::core::PCWSTR::null(),
            windows::core::PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

fn ensure_directory(path: &std::path::Path) -> Result<(), String> {
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(|e| format!("Gagal membuat folder: {e}"))?;
    }
    Ok(())
}

pub fn open_folder_in_explorer(path: &std::path::Path) -> Result<(), String> {
    ensure_directory(path)?;
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
    fn test_tws_audio_detection() {
        assert!(is_audio_device(Some(true), None));
        assert!(is_audio_device(None, Some(4)));
        assert!(!is_audio_device(Some(false), Some(1)));
        assert!(!is_audio_device(None, None));
    }

    #[test]
    fn test_run_at_startup_check() {
        // Calling is_run_at_startup_enabled should safely succeed without panic
        let _ = is_run_at_startup_enabled();
    }

    #[test]
    fn test_sample_system_storage_safe() {
        let (pct, used, total, drives) = sample_system_storage();
        if let (Some(p), Some(u), Some(t)) = (pct, used, total) {
            assert!(p >= 0.0 && p <= 100.0);
            assert!(u <= t);
            assert!(!drives.is_empty());
            for d in drives {
                assert!(d.percent >= 0.0 && d.percent <= 100.0);
                assert!(d.used_gb <= d.total_gb);
                assert!(d.letter.ends_with(':'));
            }
        }
    }

    #[test]
    fn test_open_browser_url_valid() {
        assert!(is_openable_browser_url("https://github.com/arwankemis/mynotch"));
        assert!(is_openable_browser_url("https://example.com/a?x=1"));
        for bad in [
            "http://example.com",
            "file:///C:/Windows/System32/calc.exe",
            "ms-settings:privacy-microphone",
            "https://",
            "https:///path",
            "https://example.com/\r\nInjected",
            "https://exa mple.com",
            "",
        ] {
            assert!(!is_openable_browser_url(bad), "{bad} must be rejected");
        }
    }

    #[test]
    fn test_open_folder_creates_nonexistent() {
        let temp_dir = std::env::temp_dir().join(format!("mynotch_test_explorer_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        assert!(!temp_dir.exists());
        // Do not call open_folder_in_explorer here: it would spawn a real Explorer window at a
        // folder this test deletes a moment later, leaving a "Location is not available" dialog.
        assert!(ensure_directory(&temp_dir).is_ok());
        assert!(temp_dir.exists());
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_power_snapshot_default_is_unavailable() {
        let snapshot = PowerSnapshot::default();
        assert_eq!(snapshot.source_kind, SourceKind::Unavailable);
        assert!(snapshot.active_power_w.is_none());
    }

    #[test]
    fn test_calculate_power_snapshot() {
        let snap = calculate_power(0.5, 0.2);
        assert_eq!(snap.source_kind, SourceKind::Estimated);
        assert_eq!(snap.quality, Quality::Valid);
        assert!(snap.active_power_w.is_some());
        assert!(snap.current_a.is_none());
        assert!(snap.voltage_v.is_none());
        assert!(snap.cpu_name.is_none());
        assert!(snap.gpu_name.is_none());
        assert!(snap.cpu_watts.unwrap() > 15.0);
        assert!(snap.gpu_watts.unwrap() > 10.0);
        assert!(snap.total_ac_watts.unwrap() > snap.total_dc_watts.unwrap());
        assert!(snap.hourly_cost_idr.unwrap() > 0.0);
    }
}

#[allow(dead_code)]
#[derive(Default, Clone)]
pub struct CpuSampler {
    prev_idle: u64,
    prev_total: u64,
    last_usage: f32,
}

#[allow(dead_code)]
impl CpuSampler {
    pub fn sample(&mut self) -> f32 {
        unsafe {
            use windows::Win32::Foundation::FILETIME;
            use windows::Win32::System::Threading::GetSystemTimes;

            let mut idle = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();
            if GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).is_ok() {
                let idle_val = ((idle.dwHighDateTime as u64) << 32) | (idle.dwLowDateTime as u64);
                let kernel_val = ((kernel.dwHighDateTime as u64) << 32) | (kernel.dwLowDateTime as u64);
                let user_val = ((user.dwHighDateTime as u64) << 32) | (user.dwLowDateTime as u64);
                let total_val = kernel_val + user_val;

                if self.prev_total > 0 && total_val > self.prev_total {
                    let total_delta = total_val - self.prev_total;
                    let idle_delta = idle_val.saturating_sub(self.prev_idle);
                    if total_delta > 0 {
                        let usage = 1.0 - (idle_delta as f32 / total_delta as f32);
                        self.last_usage = usage.clamp(0.0, 1.0);
                    }
                }
                self.prev_idle = idle_val;
                self.prev_total = total_val;
            }
        }
        self.last_usage
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    Measured,
    Estimated,
    Derived,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    Valid,
    Stale,
    Partial,
    Disconnected,
    Error,
}

#[allow(dead_code)]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PowerSample {
    pub provider_id: String,
    pub device_id: String,
    pub sequence: u64,
    pub timestamp_secs: i64,
    pub monotonic_ms: u64,
    pub source_kind: SourceKind,
    pub quality: Quality,
    pub active_power_w: Option<f32>,
    pub current_a: Option<f32>,
    pub voltage_v: Option<f32>,
    pub cumulative_energy_kwh: Option<f64>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PowerSnapshot {
    #[serde(default)] pub cpu_setup_required:bool,
    #[serde(default)] pub provider_note:Option<String>,
    #[serde(default)] pub today_kwh:Option<f32>,
    pub source_kind: SourceKind,
    pub quality: Quality,
    pub active_power_w: Option<f32>,
    pub current_a: Option<f32>,
    pub voltage_v: Option<f32>,
    pub cpu_watts: Option<f32>,
    pub gpu_watts: Option<f32>,
    pub rest_watts: Option<f32>,
    pub total_dc_watts: Option<f32>,
    pub total_ac_watts: Option<f32>,
    pub cpu_name: Option<String>,
    pub gpu_name: Option<String>,
    pub hourly_cost_idr: Option<f32>,
    pub daily_cost_idr: Option<f32>,
    pub monthly_cost_idr: Option<f32>,
    #[serde(default)] pub cpu_usage: Option<f32>,
    #[serde(default)] pub gpu_usage: Option<f32>,
    #[serde(default)] pub ram_usage: Option<f32>,
    #[serde(default)] pub ram_used_gb: Option<f32>,
    #[serde(default)] pub ram_total_gb: Option<f32>,
    #[serde(default)] pub storage_usage: Option<f32>,
    #[serde(default)] pub storage_used_gb: Option<f32>,
    #[serde(default)] pub storage_total_gb: Option<f32>,
    #[serde(default)] pub storage_drives: Vec<DriveInfo>,
    #[serde(default)] pub language: String,
}

impl Default for PowerSnapshot {
    fn default() -> Self {
        Self {
            cpu_setup_required:false,provider_note:None,today_kwh:None,
            source_kind: SourceKind::Unavailable,
            quality: Quality::Disconnected,
            active_power_w: None,
            current_a: None,
            voltage_v: None,
            cpu_watts: None,
            gpu_watts: None,
            rest_watts: None,
            total_dc_watts: None,
            total_ac_watts: None,
            cpu_name: None,
            gpu_name: None,
            hourly_cost_idr: None,
            daily_cost_idr: None,
            monthly_cost_idr: None,
            cpu_usage: None,
            gpu_usage: None,
            ram_usage: None,
            ram_used_gb: None,
            ram_total_gb: None,
            storage_usage: None,
            storage_used_gb: None,
            storage_total_gb: None,
            storage_drives: Vec::new(),
            language: "id".to_string(),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DriveInfo {
    pub letter: String,
    pub used_gb: f32,
    pub total_gb: f32,
    pub percent: f32,
}

pub fn sample_system_memory() -> (Option<f32>, Option<f32>, Option<f32>) {
    unsafe {
        use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        let mut mem = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        if GlobalMemoryStatusEx(&mut mem).is_ok() {
            let total_gb = (mem.ullTotalPhys as f64 / (1024.0 * 1024.0 * 1024.0)) as f32;
            let avail_gb = (mem.ullAvailPhys as f64 / (1024.0 * 1024.0 * 1024.0)) as f32;
            let used_gb = (total_gb - avail_gb).max(0.0);
            let percent = mem.dwMemoryLoad as f32;
            (Some(percent), Some(used_gb), Some(total_gb))
        } else {
            (None, None, None)
        }
    }
}

pub fn sample_system_storage() -> (Option<f32>, Option<f32>, Option<f32>, Vec<DriveInfo>) {
    unsafe {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn SetThreadErrorMode(dw_new_mode: u32, lp_old_mode: *mut u32) -> i32;
        }
        const SEM_FAILCRITICALERRORS: u32 = 0x0001;
        let mut prev_mode = 0u32;
        let _ = SetThreadErrorMode(SEM_FAILCRITICALERRORS, &mut prev_mode);

        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives};
        let mask = GetLogicalDrives();
        let mut drives = Vec::new();
        let mut sum_used_bytes: u64 = 0;
        let mut sum_total_bytes: u64 = 0;

        for i in 0..26 {
            if (mask & (1 << i)) != 0 {
                let letter_char = (b'A' + i as u8) as char;
                let path_str = format!("{}:\\\0", letter_char);
                let wide: Vec<u16> = path_str.encode_utf16().collect();
                let dtype = GetDriveTypeW(PCWSTR(wide.as_ptr()));
                if dtype == 2 || dtype == 3 || dtype == 6 {
                    let mut free_avail = 0u64;
                    let mut total = 0u64;
                    let mut total_free = 0u64;
                    if GetDiskFreeSpaceExW(
                        PCWSTR(wide.as_ptr()),
                        Some(&mut free_avail),
                        Some(&mut total),
                        Some(&mut total_free),
                    ).is_ok() && total > 0 {
                        let used = total.saturating_sub(total_free);
                        let total_gb = (total as f64 / (1024.0 * 1024.0 * 1024.0)) as f32;
                        let used_gb = (used as f64 / (1024.0 * 1024.0 * 1024.0)) as f32;
                        let pct = ((used as f64 / total as f64) * 100.0) as f32;
                        sum_used_bytes += used;
                        sum_total_bytes += total;
                        drives.push(DriveInfo {
                            letter: format!("{}:", letter_char),
                            used_gb,
                            total_gb,
                            percent: pct,
                        });
                    }
                }
            }
        }
        let mut dummy = 0u32;
        let _ = SetThreadErrorMode(prev_mode, &mut dummy);

        if sum_total_bytes > 0 {
            let total_gb = (sum_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0)) as f32;
            let used_gb = (sum_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0)) as f32;
            let pct = ((sum_used_bytes as f64 / sum_total_bytes as f64) * 100.0) as f32;
            (Some(pct), Some(used_gb), Some(total_gb), drives)
        } else {
            (None, None, None, drives)
        }
    }
}

#[allow(dead_code)]
pub fn calculate_power(cpu_usage: f32, gpu_usage: f32) -> PowerSnapshot {
    let cpu_watts = (15.0 + cpu_usage * 65.0).clamp(10.0, 120.0);
    let gpu_watts = (10.0 + gpu_usage * 150.0).clamp(5.0, 250.0);
    let rest_watts = 25.0;
    let total_dc = cpu_watts + gpu_watts + rest_watts;
    let total_ac = total_dc / 0.90;
    let tariff_per_kwh = 1444.70;
    let hourly_cost = (total_ac / 1000.0) * tariff_per_kwh;
    let daily_cost = hourly_cost * 24.0;
    let monthly_cost = daily_cost * 30.0;

    PowerSnapshot {
        cpu_setup_required:false,provider_note:Some("Estimasi lama, bukan sensor perangkat".into()),today_kwh:None,
        source_kind: SourceKind::Estimated,
        quality: Quality::Valid,
        active_power_w: Some(total_ac),
        current_a: None,
        voltage_v: None,
        cpu_watts: Some(cpu_watts),
        gpu_watts: Some(gpu_watts),
        rest_watts: Some(rest_watts),
        total_dc_watts: Some(total_dc),
        total_ac_watts: Some(total_ac),
        cpu_name: None,
        gpu_name: None,
        hourly_cost_idr: Some(hourly_cost),
        daily_cost_idr: Some(daily_cost),
        monthly_cost_idr: Some(monthly_cost),
        ..Default::default()
    }
}


pub mod power_sensor;

// --- Task 035: Hardware Monitoring Sampler (RAM, CPU, GPU) ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HardwareDataQuality {
    Valid,
    Partial,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HardwareSample {
    pub ram_percent: Option<f32>,
    pub cpu_percent: Option<f32>,
    pub gpu_percent: Option<f32>,
    pub quality: HardwareDataQuality,
    pub timestamp: std::time::Instant,
}

impl Default for HardwareSample {
    fn default() -> Self {
        Self {
            ram_percent: None,
            cpu_percent: None,
            gpu_percent: None,
            quality: HardwareDataQuality::Unavailable,
            timestamp: std::time::Instant::now(),
        }
    }
}

use std::ffi::c_void;

#[repr(C)]
struct PdhFmtCountervalueDouble {
    c_status: u32,
    double_value: f64,
}

#[repr(C)]
struct PdhFmtCountervalueItemW {
    sz_name: *mut u16,
    fmt_value: PdhFmtCountervalueDouble,
}

type PdhHQuery = *mut c_void;
type PdhHCounter = *mut c_void;
type PdhStatus = i32;

struct PdhGpuSampler {
    _h_module: windows::Win32::Foundation::HMODULE,
    fn_close_query: unsafe extern "system" fn(PdhHQuery) -> PdhStatus,
    fn_collect: unsafe extern "system" fn(PdhHQuery) -> PdhStatus,
    fn_get_array: unsafe extern "system" fn(PdhHCounter, u32, *mut u32, *mut u32, *mut u8) -> PdhStatus,
    query: PdhHQuery,
    counter: PdhHCounter,
    has_primed: bool,
}

impl Drop for PdhGpuSampler {
    fn drop(&mut self) {
        if !self.query.is_null() {
            unsafe {
                (self.fn_close_query)(self.query);
            }
            self.query = std::ptr::null_mut();
        }
    }
}

impl PdhGpuSampler {
    pub fn new() -> Option<Self> {
        use windows::core::w;
        use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

        unsafe {
            let h_module = LoadLibraryW(w!("pdh.dll")).ok()?;

            let fn_open_query: unsafe extern "system" fn(*const u16, usize, *mut PdhHQuery) -> PdhStatus =
                std::mem::transmute(GetProcAddress(h_module, windows::core::s!("PdhOpenQueryW"))?);
            let fn_close_query: unsafe extern "system" fn(PdhHQuery) -> PdhStatus =
                std::mem::transmute(GetProcAddress(h_module, windows::core::s!("PdhCloseQuery"))?);
            let fn_collect: unsafe extern "system" fn(PdhHQuery) -> PdhStatus =
                std::mem::transmute(GetProcAddress(h_module, windows::core::s!("PdhCollectQueryData"))?);
            let fn_get_array: unsafe extern "system" fn(PdhHCounter, u32, *mut u32, *mut u32, *mut u8) -> PdhStatus =
                std::mem::transmute(GetProcAddress(h_module, windows::core::s!("PdhGetFormattedCounterArrayW"))?);

            let fn_add_english: Option<unsafe extern "system" fn(PdhHQuery, *const u16, usize, *mut PdhHCounter) -> PdhStatus> =
                GetProcAddress(h_module, windows::core::s!("PdhAddEnglishCounterW")).map(|p| std::mem::transmute(p));
            let fn_add_counter: Option<unsafe extern "system" fn(PdhHQuery, *const u16, usize, *mut PdhHCounter) -> PdhStatus> =
                GetProcAddress(h_module, windows::core::s!("PdhAddCounterW")).map(|p| std::mem::transmute(p));

            let mut query: PdhHQuery = std::ptr::null_mut();
            if fn_open_query(std::ptr::null(), 0, &mut query) != 0 || query.is_null() {
                return None;
            }

            let path: Vec<u16> = "\\GPU Engine(*)\\Utilization Percentage\0".encode_utf16().collect();
            let mut counter: PdhHCounter = std::ptr::null_mut();

            let mut added = false;
            if let Some(add_eng) = fn_add_english {
                if add_eng(query, path.as_ptr(), 0, &mut counter) == 0 && !counter.is_null() {
                    added = true;
                }
            }
            if !added {
                if let Some(add_cnt) = fn_add_counter {
                    if add_cnt(query, path.as_ptr(), 0, &mut counter) == 0 && !counter.is_null() {
                        added = true;
                    }
                }
            }

            if !added || counter.is_null() {
                fn_close_query(query);
                return None;
            }

            // Prime the initial query
            let _ = fn_collect(query);

            Some(Self {
                _h_module: h_module,
                fn_close_query,
                fn_collect,
                fn_get_array,
                query,
                counter,
                has_primed: false,
            })
        }
    }

    pub fn sample(&mut self) -> Option<f32> {
        if self.query.is_null() || self.counter.is_null() {
            return None;
        }

        unsafe {
            let res = (self.fn_collect)(self.query);
            if res != 0 {
                return None;
            }

            if !self.has_primed {
                self.has_primed = true;
                return None;
            }

            let mut buffer_size: u32 = 0;
            let mut item_count: u32 = 0;
            const PDH_FMT_DOUBLE: u32 = 0x00000200;

            // Query buffer size needed
            let _ = (self.fn_get_array)(self.counter, PDH_FMT_DOUBLE, &mut buffer_size, &mut item_count, std::ptr::null_mut());
            if buffer_size == 0 || item_count == 0 {
                return None;
            }

            let mut buffer: Vec<u8> = vec![0u8; buffer_size as usize];
            let res2 = (self.fn_get_array)(self.counter, PDH_FMT_DOUBLE, &mut buffer_size, &mut item_count, buffer.as_mut_ptr());
            if res2 != 0 {
                return None;
            }

            let items = buffer.as_ptr() as *const PdhFmtCountervalueItemW;
            let mut max_utilization: f64 = 0.0;
            let mut found_any = false;

            for i in 0..item_count {
                let item = &*items.add(i as usize);
                if item.sz_name.is_null() {
                    continue;
                }

                // Check sz_name for engtype_3D or Compute
                let mut len = 0;
                while *item.sz_name.add(len) != 0 {
                    len += 1;
                }
                let slice = std::slice::from_raw_parts(item.sz_name, len);
                let name = String::from_utf16_lossy(slice);

                if name.contains("engtype_3D") || name.contains("Compute") {
                    if item.fmt_value.c_status == 0 {
                        found_any = true;
                        let val = item.fmt_value.double_value;
                        if val > max_utilization {
                            max_utilization = val;
                        }
                    }
                }
            }

            if found_any {
                Some((max_utilization as f32).clamp(0.0, 100.0))
            } else {
                None
            }
        }
    }
}

pub struct HardwareSampler {
    pub cpu: CpuSampler,
    gpu: Option<PdhGpuSampler>,
    pub last_sample: Option<HardwareSample>,
    cpu_primed: bool,
}

impl Default for HardwareSampler {
    fn default() -> Self {
        Self {
            cpu: CpuSampler::default(),
            gpu: PdhGpuSampler::new(),
            last_sample: None,
            cpu_primed: false,
        }
    }
}

impl HardwareSampler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sample(&mut self) -> HardwareSample {
        let (ram_pct, _used, _total) = sample_system_memory();

        let raw_cpu = self.cpu.sample() * 100.0;
        let cpu_pct = if !self.cpu_primed {
            self.cpu_primed = true;
            None
        } else {
            Some(raw_cpu.clamp(0.0, 100.0))
        };

        let gpu_pct = if let Some(gpu) = &mut self.gpu {
            gpu.sample()
        } else {
            None
        };

        let quality = if ram_pct.is_some() || cpu_pct.is_some() || gpu_pct.is_some() {
            HardwareDataQuality::Valid
        } else {
            HardwareDataQuality::Unavailable
        };

        let sample = HardwareSample {
            ram_percent: ram_pct,
            cpu_percent: cpu_pct,
            gpu_percent: gpu_pct,
            quality,
            timestamp: std::time::Instant::now(),
        };

        self.last_sample = Some(sample.clone());
        sample
    }
}
