use serde::{Deserialize, Serialize};
use std::time::Duration;
use windows::core::BOOL;
use windows::Win32::Foundation::{HANDLE, HWND};

/// Plaintext exists only in memory; the persisted text is a per-user DPAPI blob.
#[derive(Clone)]
pub struct ClipItem {
    pub id: u64,
    pub text: String,
    pub time: String,
    pub chars: usize,
    pub lines: usize,
    pub pinned: bool,
    /// Keep the dev state format: Unix seconds, not milliseconds.
    pub captured_at: u64,
    pub pinned_at: u64,
    /// Preserve ciphertext that cannot be opened on this Windows account.
    pub locked: Option<String>,
    needs_sealing: bool,
}

pub const SEALED_PREFIX: &str = "dpapi:v1:";

impl Serialize for ClipItem {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let sealed = match &self.locked {
            Some(payload) => payload.clone(),
            None => {
                let hex = crate::secrets::protect(&self.text).map_err(serde::ser::Error::custom)?;
                format!("{SEALED_PREFIX}{hex}")
            }
        };
        let mut item = serializer.serialize_struct("ClipItem", 8)?;
        item.serialize_field("id", &self.id)?;
        item.serialize_field("text", &sealed)?;
        item.serialize_field("time", &self.time)?;
        item.serialize_field("chars", &self.chars)?;
        item.serialize_field("lines", &self.lines)?;
        item.serialize_field("pinned", &self.pinned)?;
        item.serialize_field("captured_at", &self.captured_at)?;
        item.serialize_field("pinned_at", &self.pinned_at)?;
        item.end()
    }
}

impl<'de> Deserialize<'de> for ClipItem {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Stored {
            id: u64,
            #[serde(default)] text: String,
            #[serde(default)] time: String,
            #[serde(default)] chars: usize,
            #[serde(default)] lines: usize,
            #[serde(default)] pinned: bool,
            #[serde(default)] captured_at: u64,
            #[serde(default)] pinned_at: u64,
        }
        let stored = Stored::deserialize(deserializer)?;
        let needs_sealing = !stored.text.starts_with(SEALED_PREFIX);
        let (text, locked) = match stored.text.strip_prefix(SEALED_PREFIX) {
            Some(hex) => match crate::secrets::unprotect(hex) {
                Ok(text) => (text, None),
                Err(_) => (String::new(), Some(stored.text.clone())),
            },
            None => (stored.text, None),
        };
        Ok(Self { id: stored.id, text, time: stored.time, chars: stored.chars,
            lines: stored.lines, pinned: stored.pinned, captured_at: stored.captured_at,
            pinned_at: stored.pinned_at, locked, needs_sealing })
    }
}

impl ClipItem {
    pub fn captured(id: u64, text: String, time: String, captured_at: u64) -> Self {
        Self { id, chars: text.chars().count(), lines: text.lines().count().max(1),
            text, time, pinned: false, captured_at, pinned_at: 0, locked: None, needs_sealing: false }
    }
}

/// Returns whether startup must save state, including plaintext entries already timestamped.
pub fn migrate(items: &mut Vec<ClipItem>, now: u64) -> bool {
    let before = items.len();
    let changed = items.iter().any(|i| i.needs_sealing || i.captured_at == 0 || (i.pinned && i.pinned_at == 0));
    expire_history(items, now);
    for item in items.iter_mut() { item.needs_sealing = false; }
    changed || before != items.len()
}

#[link(name = "user32")]
unsafe extern "system" {
    fn OpenClipboard(hWndNewOwner: HWND) -> BOOL;
    fn CloseClipboard() -> BOOL;
    fn EmptyClipboard() -> BOOL;
    fn IsClipboardFormatAvailable(format: u32) -> BOOL;
    fn RegisterClipboardFormatW(lpszFormat: *const u16) -> u32;
    fn GetClipboardData(uFormat: u32) -> HANDLE;
    fn SetClipboardData(uFormat: u32, hMem: HANDLE) -> HANDLE;
    fn GetClipboardSequenceNumber() -> u32;
    fn AddClipboardFormatListener(hwnd: HWND) -> BOOL;
    fn RemoveClipboardFormatListener(hwnd: HWND) -> BOOL;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GlobalAlloc(uFlags: u32, dwBytes: usize) -> HANDLE;
    fn GlobalFree(hMem: HANDLE) -> HANDLE;
    fn GlobalLock(hMem: HANDLE) -> *mut std::ffi::c_void;
    fn GlobalUnlock(hMem: HANDLE) -> BOOL;
    fn GlobalSize(hMem: HANDLE) -> usize;
}

const CF_UNICODETEXT: u32 = 13;
const GMEM_MOVEABLE: u32 = 0x0002;
const OPEN_ATTEMPTS: usize = 5;
/// A single clipboard capture is kept at most 64 KiB; anything larger is truncated to a
/// char boundary instead of growing `state.json` without bound.
pub const MAX_CLIP_BYTES: usize = 64 * 1024;
/// Unpinned entries expire after 24 h, pinned entries after 30 days. Pinned is an explicit
/// user action, so it gets a longer window — but no entry lives forever anymore.
pub const UNPINNED_TTL_SECS: u64 = 24 * 3600;
pub const PINNED_TTL_SECS: u64 = 30 * 24 * 3600;

/// Another process can hold the clipboard for a few milliseconds. Retry briefly instead of
/// dropping the event, so a copy made while another app is writing is still captured.
fn open_clipboard_with_retry() -> bool {
    unsafe {
        for attempt in 0..OPEN_ATTEMPTS {
            if OpenClipboard(HWND::default()).as_bool() {
                return true;
            }
            if attempt + 1 < OPEN_ATTEMPTS {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        false
    }
}

/// Keeps clipboard history inside `max`, but never discards a pinned entry: unpinned items are
/// removed first and a fully pinned history is left untouched.
pub fn trim_history(items: &mut Vec<ClipItem>, max: usize) {
    while items.len() > max {
        match items.iter().rposition(|c| !c.pinned) {
            Some(pos) => { items.remove(pos); }
            None => break,
        }
    }
}

pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Truncates to at most `max_bytes` bytes without splitting a UTF-8 character.
pub fn truncate_to_bytes(mut text: String, max_bytes: usize) -> String {
    if text.len() > max_bytes {
        let mut end = max_bytes;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    text
}

/// Apply the 003 retention policy using dev's seconds-based timestamps.
pub fn expire_history(items: &mut Vec<ClipItem>, now: u64) {
    for item in items.iter_mut() {
        if item.captured_at == 0 {
            // Historical IDs were Unix milliseconds. Unknown IDs receive a fresh anchor.
            item.captured_at = if item.id >= 1_000_000_000_000 { item.id / 1000 } else { now };
        }
        if item.pinned && item.pinned_at == 0 { item.pinned_at = item.captured_at; }
    }
    items.retain(|item| {
        if item.locked.is_none() && item.text.len() > MAX_CLIP_BYTES { return false; }
        let ttl = if item.pinned || item.locked.is_some() { PINNED_TTL_SECS } else { UNPINNED_TTL_SECS };
        let anchor = if item.pinned { item.pinned_at } else { item.captured_at };
        now.saturating_sub(anchor) < ttl
    });
}

fn is_sensitive_clipboard() -> bool {
    unsafe {
        for name in ["ExcludeClipboardContentFromMonitorProcessing", "ClipboardHistoryIgnore"] {
            let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            let fmt = RegisterClipboardFormatW(wide.as_ptr());
            if fmt != 0 && IsClipboardFormatAvailable(fmt).as_bool() {
                return true;
            }
        }
        false
    }
}

pub fn read_clipboard_text() -> Option<String> {
    unsafe {
        if !IsClipboardFormatAvailable(CF_UNICODETEXT).as_bool() || is_sensitive_clipboard() {
            return None;
        }
        if !open_clipboard_with_retry() {
            return None;
        }
        let handle = GetClipboardData(CF_UNICODETEXT);
        if handle.is_invalid() || handle.0.is_null() {
            let _ = CloseClipboard();
            return None;
        }
        let ptr = GlobalLock(handle) as *const u16;
        if ptr.is_null() {
            let _ = CloseClipboard();
            return None;
        }
        let size_bytes = GlobalSize(handle);
        if size_bytes < 2 {
            let _ = GlobalUnlock(handle);
            let _ = CloseClipboard();
            return None;
        }
        let max_chars = size_bytes / 2;
        let mut len = 0;
        while len < max_chars && *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        let text = truncate_to_bytes(String::from_utf16_lossy(slice), MAX_CLIP_BYTES);
        let _ = GlobalUnlock(handle);
        let _ = CloseClipboard();
        if text.is_empty() { None } else { Some(text) }
    }
}

pub fn set_clipboard_text(text: &str) -> bool {
    unsafe {
        let mut utf16: Vec<u16> = text.encode_utf16().collect();
        utf16.push(0);
        let bytes = utf16.len() * 2;
        let h_mem = GlobalAlloc(GMEM_MOVEABLE, bytes);
        if h_mem.is_invalid() || h_mem.0.is_null() {
            return false;
        }
        let ptr = GlobalLock(h_mem) as *mut u16;
        if ptr.is_null() {
            let _ = GlobalFree(h_mem);
            return false;
        }
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());
        let _ = GlobalUnlock(h_mem);

        if !open_clipboard_with_retry() {
            let _ = GlobalFree(h_mem);
            return false;
        }
        let _ = EmptyClipboard();
        let res = SetClipboardData(CF_UNICODETEXT, h_mem);
        let ok = !res.is_invalid() && !res.0.is_null();
        if !ok {
            let _ = GlobalFree(h_mem);
        }
        let _ = CloseClipboard();
        ok
    }
}

pub fn get_sequence_number() -> u32 {
    unsafe { GetClipboardSequenceNumber() }
}

pub fn listen_clipboard(hwnd: HWND) -> bool {
    unsafe { AddClipboardFormatListener(hwnd).as_bool() }
}

pub fn unlisten_clipboard(hwnd: HWND) -> bool {
    unsafe { RemoveClipboardFormatListener(hwnd).as_bool() }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_800_000_000;

    #[test]
    fn dpapi_roundtrip_preserves_dev_seconds_and_max_size() {
        let text = "k".repeat(MAX_CLIP_BYTES);
        let item = ClipItem::captured(NOW * 1000, text.clone(), "12:00".into(), NOW);
        let raw = serde_json::to_string(&item).unwrap();
        assert!(!raw.contains(&text));
        assert!(raw.contains(SEALED_PREFIX));
        let restored: ClipItem = serde_json::from_str(&raw).unwrap();
        assert_eq!(restored.text, text);
        assert_eq!(restored.captured_at, NOW);
        assert!(restored.locked.is_none());
    }

    #[test]
    fn migration_prunes_old_ids_and_reseals_already_timestamped_plaintext() {
        let raw = format!(r#"[{{"id":{},"text":"old"}},{{"id":2,"text":"new","captured_at":{NOW}}}]"#, (NOW - UNPINNED_TTL_SECS) * 1000);
        let mut items: Vec<ClipItem> = serde_json::from_str(&raw).unwrap();
        assert!(migrate(&mut items, NOW));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text, "new");
        let saved = serde_json::to_string(&items).unwrap();
        assert!(!saved.contains("\"new\""));
        let mut reopened: Vec<ClipItem> = serde_json::from_str(&saved).unwrap();
        assert!(!migrate(&mut reopened, NOW));
        assert_eq!(reopened[0].captured_at, NOW);
    }

    #[test]
    fn locked_ciphertext_survives_save_until_retention_limit() {
        let payload = format!("{SEALED_PREFIX}00112233445566778899aabbccddeeff");
        let raw = format!(r#"[{{"id":1,"text":"{payload}","captured_at":{NOW}}}]"#);
        let mut items: Vec<ClipItem> = serde_json::from_str(&raw).unwrap();
        assert!(items[0].text.is_empty());
        assert_eq!(items[0].locked.as_deref(), Some(payload.as_str()));
        expire_history(&mut items, NOW + UNPINNED_TTL_SECS);
        assert_eq!(items.len(), 1);
        assert!(serde_json::to_string(&items).unwrap().contains(&payload));
        expire_history(&mut items, NOW + PINNED_TTL_SECS);
        assert!(items.is_empty());
    }

    #[test]
    fn pin_time_extends_retention_and_size_limit_applies_to_loaded_history() {
        let mut pin = ClipItem::captured(1, "test".into(), "12:00".into(), NOW - 3600);
        pin.pinned = true;
        pin.pinned_at = NOW;
        let mut items = vec![pin];
        expire_history(&mut items, NOW + PINNED_TTL_SECS - 1);
        assert_eq!(items.len(), 1);
        expire_history(&mut items, NOW + PINNED_TTL_SECS);
        assert!(items.is_empty());
        items.push(ClipItem::captured(2, "k".repeat(MAX_CLIP_BYTES + 1), "12:00".into(), NOW));
        expire_history(&mut items, NOW);
        assert!(items.is_empty());
    }

    #[test]
    fn migration_store_roundtrip_preserves_other_dev_state_fields() {
        let root = std::env::temp_dir().join(format!("mynotch-012-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mut store = crate::core::Store::new(root.clone()).unwrap();
        let legacy = format!(r#"{{"notes":"synthetic-note","power_month":"2026-11","monthly_kwh":1.5,"monthly_cost_idr":2167,"power_days_active":["2026-11-04"],"clipboard_history":[{{"id":1,"text":"synthetic-012-secret","captured_at":{NOW}}}]}}"#);
        std::fs::write(root.join("state.json"), legacy).unwrap();
        let mut state: crate::core::State = store.load("state.json");
        assert!(migrate(&mut state.clipboard_history, NOW));
        store.save("state.json", &state).unwrap();
        let raw = std::fs::read_to_string(root.join("state.json")).unwrap();
        assert!(!raw.contains("synthetic-012-secret"));
        let reopened: crate::core::State = store.load("state.json");
        assert_eq!(reopened.clipboard_history[0].text, "synthetic-012-secret");
        assert_eq!(reopened.notes, "synthetic-note");
        assert_eq!(reopened.monthly_kwh, 1.5);
        assert_eq!(reopened.monthly_cost_idr, 2167);
        assert_eq!(reopened.power_days_active, vec!["2026-11-04"]);
        // Keep only synthetic fixtures under the isolated test root; no user files.
    }

    #[test]
    fn clip_item_fields_work() {
        let item = ClipItem {
            id: 123,
            text: "Hello world".into(),
            time: "12:00".into(),
            chars: 11,
            lines: 1,
            pinned: true,
            captured_at: 0, pinned_at: 0, locked: None, needs_sealing: false,
        };
        assert_eq!(item.chars, 11);
        assert_eq!(item.lines, 1);
        assert_eq!(item.text, "Hello world");
        assert!(item.pinned);
    }

    fn clip(id: u64, pinned: bool) -> ClipItem {
        ClipItem { id, text: format!("item {id}"), time: "12:00".into(), chars: 1, lines: 1, pinned, captured_at: 0, pinned_at: 0, locked: None, needs_sealing: false }
    }

    #[test]
    fn history_expiry_applies_ttl_and_migrates_legacy_entries() {
        let now = 1_800_000_000u64;
        let mut items = vec![
            ClipItem { captured_at: now - 3600, ..clip(1, false) },
            ClipItem { captured_at: now - UNPINNED_TTL_SECS - 1, ..clip(2, false) },
            ClipItem { captured_at: now - PINNED_TTL_SECS + 3600, ..clip(3, true) },
            ClipItem { captured_at: now - PINNED_TTL_SECS - 1, ..clip(4, true) },
            ClipItem { captured_at: 0, ..clip(5, false) },
        ];
        expire_history(&mut items, now);
        let ids: Vec<u64> = items.iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![1, 3, 5]);
        assert_eq!(items.iter().find(|c| c.id == 5).unwrap().captured_at, now);
    }

    #[test]
    fn truncation_never_splits_a_character() {
        let text = "aé🎉".to_string();
        assert_eq!(truncate_to_bytes(text.clone(), 100), text);
        assert_eq!(truncate_to_bytes(text.clone(), 3), "aé");
        assert_eq!(truncate_to_bytes(text, 2), "a");
    }

    #[test]
    fn history_trim_never_discards_pinned_items() {
        let mut all_pinned: Vec<ClipItem> = (0..50).map(|id| clip(id, true)).collect();
        trim_history(&mut all_pinned, 50);
        assert_eq!(all_pinned.len(), 50);
        all_pinned.push(clip(99, false));
        trim_history(&mut all_pinned, 50);
        assert_eq!(all_pinned.len(), 50);
        assert!(all_pinned.iter().all(|c| c.pinned), "pinned items must survive a full history");

        let mut mixed: Vec<ClipItem> = (0..49).map(|id| clip(id, true)).collect();
        mixed.push(clip(100, false));
        mixed.push(clip(101, false));
        trim_history(&mut mixed, 50);
        assert_eq!(mixed.len(), 50);
        assert_eq!(mixed.iter().filter(|c| c.pinned).count(), 49);
        assert_eq!(mixed.iter().filter(|c| !c.pinned).count(), 1);
    }
}
