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

/// Preview-only radio adapter. No Windows radio API is implemented yet, so it reports no
/// devices instead of presenting a mock list as if real hardware were detected.
pub struct WindowsRadioAdapter;

impl WindowsRadioAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl RadioAdapter for WindowsRadioAdapter {
    fn enumerate_radios(&self) -> Vec<RadioDevice> {
        Vec::new()
    }

    fn set_radio_state(&self, _id: &str, _on: bool) -> Result<RadioState, String> {
        Err("Kontrol radio belum diimplementasikan".to_string())
    }

    fn request_access(&self, _id: &str) -> Result<RadioAccess, String> {
        Err("Kontrol radio belum diimplementasikan".to_string())
    }
}

/// Reads the internal panel brightness through WMI (`ROOT\\WMI`). Changing brightness is not
/// implemented yet, so the value is read-only and unavailable displays are not fabricated.
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
            CoSetProxyBlanket(
                &services,
                10, 0,
                PCWSTR::null(),
                RPC_C_AUTHN_LEVEL_CALL,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_NONE,
            ).ok()?;

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

            let key_lvl: Vec<u16> = "Level".encode_utf16().chain(Some(0)).collect();
            let mut v_lvl = VARIANT::default();
            let levels = vec![0, 10, 25, 50, 75, 100];
            if obj.Get(PCWSTR(key_lvl.as_ptr()), 0, &mut v_lvl, None, None).is_ok() {
                // If Level array is extracted, use it, otherwise use standard levels
            }
            Some((current, levels))
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
            sub_status_text: "Nilai dibaca dari WMI. Pengaturan kecerahan belum diimplementasikan.".to_string(),
        }]
    }

    fn set_brightness(&self, _id: &str, _percent: u32) -> Result<u32, String> {
        Err("Pengaturan kecerahan belum diimplementasikan".to_string())
    }
}
