use super::model::*;
use std::sync::{atomic::{AtomicUsize, Ordering}, Mutex};
use std::time::Instant;

pub trait RadioAdapter: Send + Sync {
    fn enumerate_radios(&self) -> Vec<RadioDevice>;
    fn set_radio_state(&self, id: &str, on: bool) -> Result<RadioState, String>;
    fn request_access(&self, id: &str) -> Result<RadioAccess, String>;
}

pub trait BrightnessAdapter: Send + Sync {
    fn enumerate_displays(&self) -> Vec<DisplayDevice>;
    fn set_brightness(&self, id: &str, percent: u32) -> Result<u32, String>;
}

/// In-memory mock radio adapter for safe automated testing.
/// Does NOT touch any physical network adapter, Bluetooth controller, or Windows radio service.
pub struct MockRadioAdapter {
    radios: Mutex<Vec<RadioDevice>>,
    pub set_calls: AtomicUsize,
    pub access_calls: AtomicUsize,
}

impl MockRadioAdapter {
    pub fn new_default() -> Self {
        let radios = vec![
            RadioDevice {
                id: "w0".to_string(),
                name: "Radio 1: Intel Wi-Fi 6E AX211".to_string(),
                kind: RadioKind::WiFi,
                state: RadioState::On,
                access: RadioAccess::Allowed,
                availability: RadioAvailability::Ready,
                status_text: "Aktif (Radio menyala)".to_string(),
                sub_status_text: "On tidak berarti tersambung ke internet. Tanpa SSID/sandi.".to_string(),
            },
            RadioDevice {
                id: "w1".to_string(),
                name: "Radio 2: Realtek RTL8812BU (USB)".to_string(),
                kind: RadioKind::WiFi,
                state: RadioState::Off,
                access: RadioAccess::Allowed,
                availability: RadioAvailability::Ready,
                status_text: "Mati (Radio nonaktif)".to_string(),
                sub_status_text: "Adaptor USB sekunder".to_string(),
            },
            RadioDevice {
                id: "b0".to_string(),
                name: "Radio 1: Intel Wireless Bluetooth".to_string(),
                kind: RadioKind::Bluetooth,
                state: RadioState::On,
                access: RadioAccess::Allowed,
                availability: RadioAvailability::Ready,
                status_text: "Aktif (Radio menyala)".to_string(),
                sub_status_text: "Hanya status daya radio. Tanpa pemindaian perangkat/pairing.".to_string(),
            },
            RadioDevice {
                id: "b1".to_string(),
                name: "Radio 2: Cambridge Silicon Radio (Dongle)".to_string(),
                kind: RadioKind::Bluetooth,
                state: RadioState::Off,
                access: RadioAccess::Allowed,
                availability: RadioAvailability::Ready,
                status_text: "Mati".to_string(),
                sub_status_text: "Dongle Bluetooth eksternal".to_string(),
            },
        ];
        Self {
            radios: Mutex::new(radios),
            set_calls: AtomicUsize::new(0),
            access_calls: AtomicUsize::new(0),
        }
    }

    pub fn new_single_radio() -> Self {
        let radios = vec![
            RadioDevice {
                id: "w0".to_string(),
                name: "Wi-Fi Adapter".to_string(),
                kind: RadioKind::WiFi,
                state: RadioState::On,
                access: RadioAccess::Allowed,
                availability: RadioAvailability::Ready,
                status_text: "Aktif (Radio menyala)".to_string(),
                sub_status_text: "On tidak berarti tersambung ke internet. Tanpa SSID/sandi.".to_string(),
            },
            RadioDevice {
                id: "b0".to_string(),
                name: "Bluetooth Adapter".to_string(),
                kind: RadioKind::Bluetooth,
                state: RadioState::On,
                access: RadioAccess::Allowed,
                availability: RadioAvailability::Ready,
                status_text: "Aktif (Radio menyala)".to_string(),
                sub_status_text: "Hanya status daya radio. Tanpa pemindaian perangkat/pairing.".to_string(),
            },
        ];
        Self {
            radios: Mutex::new(radios),
            set_calls: AtomicUsize::new(0),
            access_calls: AtomicUsize::new(0),
        }
    }

    pub fn new_with_scenario(scenario: usize) -> Self {
        let adapter = Self::new_default();
        if let Ok(mut radios) = adapter.radios.lock() {
            match scenario {
                2 => { // Unknown / Loading
                    for r in radios.iter_mut() {
                        r.state = RadioState::Unknown;
                        r.status_text = "Memuat status radio...".to_string();
                    }
                }
                3 => { // Pending transition
                    if let Some(r) = radios.first_mut() {
                        r.availability = RadioAvailability::PendingTransition {
                            target_on: false,
                            requested_at: Instant::now(),
                        };
                        r.status_text = "Mematikan radio...".to_string();
                    }
                }
                4 => { // Consent pending
                    if let Some(r) = radios.first_mut() {
                        r.availability = RadioAvailability::PendingConsent;
                        r.status_text = "Menunggu izin akses radio...".to_string();
                    }
                }
                5 => { // Denied
                    for r in radios.iter_mut() {
                        r.access = RadioAccess::DeniedUser;
                        r.status_text = "Akses radio ditolak. Buka Pengaturan untuk mengizinkan.".to_string();
                    }
                }
                6 => { // Hardware disabled
                    if let Some(r) = radios.first_mut() {
                        r.availability = RadioAvailability::HardwareDisabled;
                        r.state = RadioState::Off;
                        r.status_text = "Sakelar fisik / mode pesawat aktif".to_string();
                    }
                }
                7 => { // Absent
                    if let Some(r) = radios.first_mut() {
                        r.availability = RadioAvailability::Absent;
                        r.state = RadioState::Unknown;
                        r.status_text = "Perangkat adaptor tidak ditemukan".to_string();
                    }
                }
                8 => { // Timeout
                    if let Some(r) = radios.first_mut() {
                        r.availability = RadioAvailability::Timeout;
                        r.status_text = "Batas waktu konfirmasi habis (10 detik)".to_string();
                    }
                }
                _ => {}
            }
        }
        adapter
    }
}

impl RadioAdapter for MockRadioAdapter {
    fn enumerate_radios(&self) -> Vec<RadioDevice> {
        self.radios.lock().map(|r| r.clone()).unwrap_or_default()
    }

    fn set_radio_state(&self, id: &str, on: bool) -> Result<RadioState, String> {
        self.set_calls.fetch_add(1, Ordering::SeqCst);
        let mut list = self.radios.lock().map_err(|e| e.to_string())?;
        if let Some(radio) = list.iter_mut().find(|r| r.id == id) {
            match radio.access {
                RadioAccess::DeniedUser | RadioAccess::DeniedSystem => {
                    return Err("Access denied by system/user policy".to_string());
                }
                RadioAccess::Unspecified => {
                    return Err("Access unspecified".to_string());
                }
                RadioAccess::Allowed => {}
            }
            if radio.availability == RadioAvailability::HardwareDisabled {
                return Err("Cannot toggle hardware disabled radio".to_string());
            }
            let new_state = if on { RadioState::On } else { RadioState::Off };
            radio.state = new_state;
            radio.availability = RadioAvailability::Ready;
            radio.status_text = if on {
                "Aktif (Radio menyala)".to_string()
            } else {
                "Mati (Radio nonaktif)".to_string()
            };
            Ok(new_state)
        } else {
            Err(format!("Radio {id} not found"))
        }
    }

    fn request_access(&self, id: &str) -> Result<RadioAccess, String> {
        self.access_calls.fetch_add(1, Ordering::SeqCst);
        let list = self.radios.lock().map_err(|e| e.to_string())?;
        list.iter()
            .find(|r| r.id == id)
            .map(|r| r.access)
            .ok_or_else(|| format!("Radio {id} not found"))
    }
}

/// In-memory mock brightness adapter for safe automated testing.
pub struct MockBrightnessAdapter {
    displays: Mutex<Vec<DisplayDevice>>,
    pub set_calls: AtomicUsize,
}

impl MockBrightnessAdapter {
    pub fn new_default() -> Self {
        let displays = vec![
            DisplayDevice {
                id: "disp_internal".to_string(),
                name: "Layar 1 (Internal - Panel Laptop)".to_string(),
                kind: DisplayKind::InternalWmi,
                current_percent: 75,
                target_percent: None,
                supported_levels: vec![0, 10, 25, 50, 75, 100],
                min_percent: 0,
                max_percent: 100,
                status_text: "Aktual: 75% (Level tersnap WMI)".to_string(),
                sub_status_text: "WMI metode WmiSetBrightness • Nilai tersnap [0,10,25,50,75,100]".to_string(),
            },
            DisplayDevice {
                id: "disp_external".to_string(),
                name: "Layar 2 (Eksternal - DDC/CI MCCS)".to_string(),
                kind: DisplayKind::ExternalMccs,
                current_percent: 60,
                target_percent: None,
                supported_levels: vec![],
                min_percent: 0,
                max_percent: 100,
                status_text: "Aktual: 60% (DDC/CI MCCS)".to_string(),
                sub_status_text: "Rentang kontinu 0–100%".to_string(),
            },
            DisplayDevice {
                id: "disp_unsupported".to_string(),
                name: "Layar 3 (Eksternal - DDC Tidak Didukung)".to_string(),
                kind: DisplayKind::Unsupported,
                current_percent: 50,
                target_percent: None,
                supported_levels: vec![],
                min_percent: 0,
                max_percent: 100,
                status_text: "Kontrol perangkat lunak tidak didukung".to_string(),
                sub_status_text: "Gunakan tombol fisik/OSD pada monitor eksternal.".to_string(),
            },
        ];
        Self {
            displays: Mutex::new(displays),
            set_calls: AtomicUsize::new(0),
        }
    }
}

impl BrightnessAdapter for MockBrightnessAdapter {
    fn enumerate_displays(&self) -> Vec<DisplayDevice> {
        self.displays.lock().map(|d| d.clone()).unwrap_or_default()
    }

    fn set_brightness(&self, id: &str, percent: u32) -> Result<u32, String> {
        self.set_calls.fetch_add(1, Ordering::SeqCst);
        let mut list = self.displays.lock().map_err(|e| e.to_string())?;
        if let Some(disp) = list.iter_mut().find(|d| d.id == id) {
            if disp.kind == DisplayKind::Unsupported {
                return Err("Display does not support software brightness".to_string());
            }
            let snapped = snap_brightness(percent, &disp.supported_levels);
            disp.current_percent = snapped;
            disp.target_percent = None;
            disp.status_text = match disp.kind {
                DisplayKind::InternalWmi => format!("Aktual: {snapped}% (Level tersnap WMI)"),
                DisplayKind::ExternalMccs => format!("Aktual: {snapped}% (DDC/CI MCCS)"),
                DisplayKind::Unsupported => "Kontrol perangkat lunak tidak didukung".to_string(),
            };
            Ok(snapped)
        } else {
            Err(format!("Display {id} not found"))
        }
    }
}

/// Windows Runtime radio adapter that communicates with physical radios via `Windows.Devices.Radios`.
pub struct WindowsRadioAdapter;

impl WindowsRadioAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl RadioAdapter for WindowsRadioAdapter {
    fn enumerate_radios(&self) -> Vec<RadioDevice> {
        use windows::Devices::Radios::{Radio, RadioKind as WinRadioKind, RadioState as WinRadioState};
        let _apartment = crate::system::WinRtApartment::new();
        let radios_async = match Radio::GetRadiosAsync() {
            Ok(op) => op,
            Err(_) => return Vec::new(),
        };
        let radio_list = match radios_async.join() {
            Ok(list) => list,
            Err(_) => return Vec::new(),
        };
        let count = match radio_list.Size() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut result = Vec::new();
        for i in 0..count {
            let Ok(radio) = radio_list.GetAt(i) else { continue };
            let Ok(kind) = radio.Kind() else { continue };
            let model_kind = match kind {
                WinRadioKind::WiFi => RadioKind::WiFi,
                WinRadioKind::Bluetooth => RadioKind::Bluetooth,
                _ => continue,
            };
            let name = radio.Name().map(|h| h.to_string()).unwrap_or_else(|_| "Radio".to_string());
            let state = match radio.State() {
                Ok(WinRadioState::On) => RadioState::On,
                Ok(WinRadioState::Off) => RadioState::Off,
                _ => RadioState::Unknown,
            };
            let avail = if let Ok(WinRadioState::Disabled) = radio.State() {
                RadioAvailability::HardwareDisabled
            } else {
                RadioAvailability::Ready
            };
            let id = format!("{}_{}", if model_kind == RadioKind::WiFi { "wifi" } else { "bt" }, i);
            let (status_text, sub_status_text) = if avail == RadioAvailability::HardwareDisabled {
                ("Sakelar fisik / mode pesawat aktif".to_string(), "Tidak dapat diaktifkan dari software".to_string())
            } else if state == RadioState::On {
                (
                    "Aktif (Radio menyala)".to_string(),
                    if model_kind == RadioKind::WiFi {
                        "On tidak berarti tersambung ke internet. Tanpa SSID/sandi.".to_string()
                    } else {
                        "Hanya status daya radio. Tanpa pemindaian perangkat/pairing.".to_string()
                    },
                )
            } else {
                (
                    "Mati (Radio nonaktif)".to_string(),
                    if model_kind == RadioKind::WiFi {
                        "Adaptor Wi-Fi nonaktif".to_string()
                    } else {
                        "Adaptor Bluetooth nonaktif".to_string()
                    },
                )
            };
            result.push(RadioDevice {
                id,
                name,
                kind: model_kind,
                state,
                access: RadioAccess::Allowed,
                availability: avail,
                status_text,
                sub_status_text,
            });
        }
        result
    }

    fn set_radio_state(&self, id: &str, on: bool) -> Result<RadioState, String> {
        use windows::Devices::Radios::{Radio, RadioKind as WinRadioKind, RadioState as WinRadioState, RadioAccessStatus};
        let _apartment = crate::system::WinRtApartment::new();
        let radios_async = Radio::GetRadiosAsync().map_err(|e| e.to_string())?;
        let radio_list = radios_async.join().map_err(|e| e.to_string())?;
        let count = radio_list.Size().map_err(|e| e.to_string())?;
        for i in 0..count {
            let Ok(radio) = radio_list.GetAt(i) else { continue };
            let Ok(kind) = radio.Kind() else { continue };
            let model_kind = match kind {
                WinRadioKind::WiFi => RadioKind::WiFi,
                WinRadioKind::Bluetooth => RadioKind::Bluetooth,
                _ => continue,
            };
            let expected_id = format!("{}_{}", if model_kind == RadioKind::WiFi { "wifi" } else { "bt" }, i);
            if expected_id == id {
                let target_state = if on { WinRadioState::On } else { WinRadioState::Off };
                let op = radio.SetStateAsync(target_state).map_err(|e| e.to_string())?;
                let status = op.join().map_err(|e| e.to_string())?;
                match status {
                    RadioAccessStatus::Allowed => {
                        return Ok(if on { RadioState::On } else { RadioState::Off });
                    }
                    RadioAccessStatus::DeniedByUser | RadioAccessStatus::DeniedBySystem => {
                        return Err("Akses radio ditolak oleh sistem atau pengguna".to_string());
                    }
                    _ => {
                        return Err("Gagal mengubah status radio perangkat".to_string());
                    }
                }
            }
        }
        Err(format!("Radio {id} tidak ditemukan"))
    }

    fn request_access(&self, _id: &str) -> Result<RadioAccess, String> {
        use windows::Devices::Radios::{Radio, RadioAccessStatus};
        let _apartment = crate::system::WinRtApartment::new();
        let op = Radio::RequestAccessAsync().map_err(|e| e.to_string())?;
        let status = op.join().map_err(|e| e.to_string())?;
        match status {
            RadioAccessStatus::Allowed => Ok(RadioAccess::Allowed),
            RadioAccessStatus::DeniedByUser => Ok(RadioAccess::DeniedUser),
            RadioAccessStatus::DeniedBySystem => Ok(RadioAccess::DeniedSystem),
            RadioAccessStatus::Unspecified => Ok(RadioAccess::Unspecified),
            _ => Ok(RadioAccess::Unspecified),
        }
    }
}

/// Reads and adjusts the internal panel brightness through WMI (`ROOT\\WMI`).
pub struct WindowsBrightnessAdapter;

impl WindowsBrightnessAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Attempts to query WMI `WmiMonitorBrightness` from `ROOT\\WMI`.
    pub fn query_wmi_brightness() -> Option<(u32, Vec<u8>)> {
        use windows::core::{BSTR, PCWSTR};
        use windows::Win32::System::Com::*;
        use windows::Win32::System::Wmi::*;
        use windows::Win32::System::Variant::VARIANT;

        unsafe {
            let com_ok = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
            struct Apartment(bool);
            impl Drop for Apartment {
                fn drop(&mut self) {
                    if self.0 { unsafe { CoUninitialize(); } }
                }
            }
            let _apt = Apartment(com_ok);

            let locator: IWbemLocator = CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER).ok()?;
            let empty = BSTR::new();
            let services = locator.ConnectServer(
                &BSTR::from(r"ROOT\WMI"),
                &empty, &empty, &empty,
                128, &empty, None,
            ).ok()?;
            let _ = CoSetProxyBlanket(
                &services,
                10, 0,
                PCWSTR::null(),
                RPC_C_AUTHN_LEVEL_CALL,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_NONE,
            );

            let iter = services.ExecQuery(
                &BSTR::from("WQL"),
                &BSTR::from("SELECT CurrentBrightness, Level, Active FROM WmiMonitorBrightness WHERE Active = True"),
                WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY,
                None,
            ).ok()?;

            let mut row = [None];
            let mut count = 0;
            if iter.Next(500, &mut row, &mut count).is_err() || count == 0 {
                return None;
            }
            let obj = row[0].take()?;

            let key_cur: Vec<u16> = "CurrentBrightness".encode_utf16().chain(Some(0)).collect();
            let mut v_cur = VARIANT::default();
            obj.Get(PCWSTR(key_cur.as_ptr()), 0, &mut v_cur, None, None).ok()?;
            let current = f64::try_from(&v_cur).ok().map(|n| n as u32)?;
            let _ = windows::Win32::System::Variant::VariantClear(&mut v_cur);

            let key_lvl: Vec<u16> = "Level".encode_utf16().chain(Some(0)).collect();
            let mut v_lvl = VARIANT::default();
            let mut levels = vec![0, 10, 25, 50, 75, 100];
            if obj.Get(PCWSTR(key_lvl.as_ptr()), 0, &mut v_lvl, None, None).is_ok() {
                if let Some(parsed) = parse_safearray_levels(&v_lvl) {
                    if !parsed.is_empty() {
                        levels = parsed;
                    }
                }
                let _ = windows::Win32::System::Variant::VariantClear(&mut v_lvl);
            }
            Some((current, levels))
        }
    }

    /// Sets brightness via WMI method `WmiSetBrightness` on `WmiMonitorBrightnessMethods`.
    pub fn set_wmi_brightness(percent: u32) -> Result<u32, String> {
        use windows::core::{BSTR, PCWSTR};
        use windows::Win32::System::Com::*;
        use windows::Win32::System::Wmi::*;
        use windows::Win32::System::Variant::VARIANT;

        unsafe {
            let com_ok = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
            struct Apartment(bool);
            impl Drop for Apartment {
                fn drop(&mut self) {
                    if self.0 { unsafe { CoUninitialize(); } }
                }
            }
            let _apt = Apartment(com_ok);

            let locator: IWbemLocator = CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER).map_err(|e| e.to_string())?;
            let empty = BSTR::new();
            let services = locator.ConnectServer(
                &BSTR::from(r"ROOT\WMI"),
                &empty, &empty, &empty,
                128, &empty, None,
            ).map_err(|e| e.to_string())?;
            let _ = CoSetProxyBlanket(
                &services,
                10, 0,
                PCWSTR::null(),
                RPC_C_AUTHN_LEVEL_CALL,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_NONE,
            );

            let iter = services.ExecQuery(
                &BSTR::from("WQL"),
                &BSTR::from("SELECT __PATH FROM WmiMonitorBrightnessMethods WHERE Active = True"),
                WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY,
                None,
            ).map_err(|e| e.to_string())?;

            let mut row = [None];
            let mut count = 0;
            if iter.Next(1000, &mut row, &mut count).is_err() || count == 0 {
                return Err("Tidak ada interface WMI monitor brightness yang aktif".to_string());
            }
            let obj = row[0].take().ok_or_else(|| "Gagal membaca objek WMI".to_string())?;

            let key_path: Vec<u16> = "__PATH".encode_utf16().chain(Some(0)).collect();
            let mut v_path = VARIANT::default();
            obj.Get(PCWSTR(key_path.as_ptr()), 0, &mut v_path, None, None).map_err(|e| e.to_string())?;
            let inst_path = BSTR::try_from(&v_path).map_err(|e| e.to_string())?;
            let _ = windows::Win32::System::Variant::VariantClear(&mut v_path);

            let mut class_opt = None;
            services.GetObject(
                &BSTR::from("WmiMonitorBrightnessMethods"),
                windows::Win32::System::Wmi::WBEM_GENERIC_FLAG_TYPE(0),
                None,
                Some(&mut class_opt),
                None,
            ).map_err(|e| e.to_string())?;
            let class_obj = class_opt.ok_or_else(|| "Gagal mendapatkan kelas WmiMonitorBrightnessMethods".to_string())?;

            let mut in_params_def: Option<IWbemClassObject> = None;
            class_obj.GetMethod(
                &BSTR::from("WmiSetBrightness"),
                0,
                &mut in_params_def as *mut Option<IWbemClassObject>,
                std::ptr::null_mut(),
            ).map_err(|e| e.to_string())?;
            let in_params_class = in_params_def.ok_or_else(|| "Gagal mendapatkan definisi parameter WmiSetBrightness".to_string())?;

            let in_params = in_params_class.SpawnInstance(0).map_err(|e| e.to_string())?;

            let key_timeout: Vec<u16> = "Timeout".encode_utf16().chain(Some(0)).collect();
            let mut v_timeout = VARIANT::default();
            (*v_timeout.Anonymous.Anonymous).vt = windows::Win32::System::Variant::VT_I4;
            (*v_timeout.Anonymous.Anonymous).Anonymous.lVal = 1;
            in_params.Put(PCWSTR(key_timeout.as_ptr()), 0, &v_timeout, 0).map_err(|e| e.to_string())?;

            let key_brightness: Vec<u16> = "Brightness".encode_utf16().chain(Some(0)).collect();
            let mut v_brightness = VARIANT::default();
            (*v_brightness.Anonymous.Anonymous).vt = windows::Win32::System::Variant::VT_UI1;
            (*v_brightness.Anonymous.Anonymous).Anonymous.bVal = percent.clamp(0, 100) as u8;
            in_params.Put(PCWSTR(key_brightness.as_ptr()), 0, &v_brightness, 0).map_err(|e| e.to_string())?;

            services.ExecMethod(
                &inst_path,
                &BSTR::from("WmiSetBrightness"),
                windows::Win32::System::Wmi::WBEM_GENERIC_FLAG_TYPE(0),
                None,
                &in_params,
                None,
                None,
            ).map_err(|e| e.to_string())?;

            let _ = windows::Win32::System::Variant::VariantClear(&mut v_timeout);
            let _ = windows::Win32::System::Variant::VariantClear(&mut v_brightness);

            Ok(percent.clamp(0, 100))
        }
    }
}

impl BrightnessAdapter for WindowsBrightnessAdapter {
    fn enumerate_displays(&self) -> Vec<DisplayDevice> {
        let Some((current, levels)) = Self::query_wmi_brightness() else {
            return Vec::new();
        };
        vec![DisplayDevice {
            id: "disp_internal".to_string(),
            name: "Layar internal".to_string(),
            kind: DisplayKind::InternalWmi,
            current_percent: current,
            target_percent: None,
            supported_levels: levels,
            min_percent: 0,
            max_percent: 100,
            status_text: format!("Aktual: {current}%"),
            sub_status_text: "Layar internal (WMI)".to_string(),
        }]
    }

    fn set_brightness(&self, id: &str, percent: u32) -> Result<u32, String> {
        if id != "disp_internal" {
            return Err("Display not found".to_string());
        }
        Self::set_wmi_brightness(percent)
    }
}

unsafe fn parse_safearray_levels(v: &windows::Win32::System::Variant::VARIANT) -> Option<Vec<u8>> {
    use windows::Win32::System::Ole::*;
    use windows::Win32::System::Variant::*;
    unsafe {
        let vt = (*v.Anonymous.Anonymous).vt;
        if (vt.0 & VT_ARRAY.0) == 0 {
            return None;
        }
        let psa = (*v.Anonymous.Anonymous).Anonymous.parray;
        if psa.is_null() {
            return None;
        }
        let lbound = SafeArrayGetLBound(psa, 1).ok()?;
        let ubound = SafeArrayGetUBound(psa, 1).ok()?;
        if ubound < lbound {
            return None;
        }
        let count = (ubound - lbound + 1) as usize;
        let mut data_ptr: *mut core::ffi::c_void = std::ptr::null_mut();
        if SafeArrayAccessData(psa, &mut data_ptr).is_err() || data_ptr.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(data_ptr as *const u8, count);
        let mut res = bytes.to_vec();
        let _ = SafeArrayUnaccessData(psa);
        res.sort_unstable();
        res.dedup();
        Some(res)
    }
}
