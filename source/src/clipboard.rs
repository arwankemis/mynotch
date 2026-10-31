use serde::{Deserialize, Serialize};
use std::time::Duration;
use windows::core::BOOL;
use windows::Win32::Foundation::{HANDLE, HWND};

/// One clipboard history entry. History is stored **plaintext** in `state.json` (no DPAPI
/// encryption yet — see clipboard storage documentation); retention is bounded by a per-item size cap and
/// by TTL expiry in [`expire_history`], not by encryption.
#[derive(Clone, Serialize, Deserialize)]
pub struct ClipItem {
    pub id: u64,
    pub text: String,
    pub time: String,
    pub chars: usize,
    pub lines: usize,
    #[serde(default)]
    pub pinned: bool,
    /// Unix seconds when this entry was captured. `0` means "captured before timestamps
    /// existed"; [`expire_history`] stamps such legacy entries with "now" instead of
    /// deleting them, so upgrading never wipes the history.
    #[serde(default)]
    pub captured_at: u64,
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

/// Drops entries older than their TTL. Legacy entries (`captured_at == 0`) are stamped
/// with `now` and kept, so the first run after upgrading grants them a fresh TTL window
/// instead of deleting the whole history.
pub fn expire_history(items: &mut Vec<ClipItem>, now: u64) {
    for item in items.iter_mut() {
        if item.captured_at == 0 {
            item.captured_at = now;
        }
    }
    items.retain(|item| {
        let ttl = if item.pinned { PINNED_TTL_SECS } else { UNPINNED_TTL_SECS };
        now.saturating_sub(item.captured_at) < ttl
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

    #[test]
    fn clip_item_fields_work() {
        let item = ClipItem {
            id: 123,
            text: "Hello world".into(),
            time: "12:00".into(),
            chars: 11,
            lines: 1,
            pinned: true,
            captured_at: 0,
        };
        assert_eq!(item.chars, 11);
        assert_eq!(item.lines, 1);
        assert_eq!(item.text, "Hello world");
        assert!(item.pinned);
    }

    fn clip(id: u64, pinned: bool) -> ClipItem {
        ClipItem { id, text: format!("item {id}"), time: "12:00".into(), chars: 1, lines: 1, pinned, captured_at: 0 }
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
