use serde::{Deserialize, Serialize};
use windows::core::BOOL;
use windows::Win32::Foundation::{HANDLE, HWND};

#[derive(Clone, Serialize, Deserialize)]
pub struct ClipItem {
    pub id: u64,
    pub text: String,
    pub time: String,
    pub chars: usize,
    pub lines: usize,
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
        if !OpenClipboard(HWND::default()).as_bool() {
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
        let max_chars = size_bytes / 2;
        let mut len = 0;
        while len < max_chars && *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        let text = String::from_utf16_lossy(slice);
        let _ = GlobalUnlock(handle);
        let _ = CloseClipboard();
        Some(text)
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

        if !OpenClipboard(HWND::default()).as_bool() {
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
        };
        assert_eq!(item.chars, 11);
        assert_eq!(item.lines, 1);
        assert_eq!(item.text, "Hello world");
    }
}
