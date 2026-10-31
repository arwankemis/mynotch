use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows::core::{w, PCWSTR};
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::*,
};

static CLASS_REGISTERED: AtomicBool = AtomicBool::new(false);

const EDIT_ID: usize = 101;

/// Posted to the main shell window when a floating note closes with edited content.
pub const NOTE_SAVED: u32 = WM_APP + 56;

/// A pop-out window lives on the same thread but has its own wndproc, so edited notes are handed
/// over through this queue instead of borrowing the shell state directly. A queue (not a single
/// slot) keeps every edit when two floats close before the shell pumps its messages.
static PENDING_EDITS: Mutex<Vec<(u64, String)>> = Mutex::new(Vec::new());

pub fn take_pending_note_edits() -> Vec<(u64, String)> {
    PENDING_EDITS.lock().map(|mut queue| std::mem::take(&mut *queue)).unwrap_or_default()
}

/// Merges a pop-out edit back into the note list. Returns false when the note was deleted while
/// the floating window was open.
pub fn apply_popout_content(notes: &mut [crate::sticky::StickyNote], id: u64, content: &str) -> bool {
    match notes.iter_mut().find(|n| n.id == id) {
        Some(note) => {
            if note.content != content {
                note.content = content.to_string();
                note.updated_at = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
            }
            true
        }
        None => false,
    }
}

unsafe extern "system" fn sticky_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_CREATE => {
                let cs = *(lparam.0 as *const CREATESTRUCTW);
                let text_ptr = cs.lpCreateParams as *const u16;
                let instance = cs.hInstance;
                let edit = CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("EDIT"),
                    PCWSTR(text_ptr),
                    WS_CHILD | WS_VISIBLE | WS_VSCROLL | WINDOW_STYLE(0x0004 | 0x0040), // ES_MULTILINE | ES_AUTOVSCROLL
                    8,
                    8,
                    cs.cx - 16,
                    cs.cy - 16,
                    Some(hwnd),
                    Some(HMENU(EDIT_ID as *mut _)),
                    Some(instance),
                    None,
                );
                if let Ok(edit_hwnd) = edit {
                    let font = GetStockObject(DEFAULT_GUI_FONT);
                    let _ = SendMessageW(edit_hwnd, WM_SETFONT, Some(WPARAM(font.0 as usize)), Some(LPARAM(1)));
                }
                LRESULT(0)
            }
            WM_SIZE => {
                let w = (lparam.0 & 0xffff) as i32;
                let h = ((lparam.0 >> 16) & 0xffff) as i32;
                if let Ok(edit_hwnd) = GetDlgItem(Some(hwnd), EDIT_ID as i32) {
                    let _ = MoveWindow(edit_hwnd, 8, 8, (w - 16).max(20), (h - 16).max(20), true);
                }
                LRESULT(0)
            }
            WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => {
                let dc = HDC(wparam.0 as *mut _);
                let color_idx = GetPropW(hwnd, w!("COLOR_IDX")).0 as usize;
                let (hex_color, _) = crate::sticky::NOTE_COLORS.get(color_idx).copied().unwrap_or((0xfef08a, "Kuning"));
                let r = (hex_color >> 16) & 0xff;
                let g = (hex_color >> 8) & 0xff;
                let b = hex_color & 0xff;
                let win_color = COLORREF((b << 16) | (g << 8) | r);

                SetTextColor(dc, COLORREF(0x001e293b));
                SetBkColor(dc, win_color);
                SetDCBrushColor(dc, win_color);
                LRESULT(GetStockObject(DC_BRUSH).0 as isize)
            }
            WM_ERASEBKGND => {
                let dc = HDC(wparam.0 as *mut _);
                let color_idx = GetPropW(hwnd, w!("COLOR_IDX")).0 as usize;
                let (hex_color, _) = crate::sticky::NOTE_COLORS.get(color_idx).copied().unwrap_or((0xfef08a, "Kuning"));
                let r = (hex_color >> 16) & 0xff;
                let g = (hex_color >> 8) & 0xff;
                let b = hex_color & 0xff;
                let win_color = COLORREF((b << 16) | (g << 8) | r);

                let mut rc = RECT::default();
                let _ = GetClientRect(hwnd, &mut rc);
                SetDCBrushColor(dc, win_color);
                let brush = GetStockObject(DC_BRUSH);
                let _ = FillRect(dc, &rc, HBRUSH(brush.0));
                LRESULT(1)
            }
            WM_CLOSE => {
                // Push the edited text back to the shell before the window disappears.
                if let Ok(edit) = GetDlgItem(Some(hwnd), EDIT_ID as i32) {
                    let len = GetWindowTextLengthW(edit).max(0) as usize;
                    let mut buf = vec![0u16; len + 1];
                    let copied = GetWindowTextW(edit, &mut buf).max(0) as usize;
                    let content = String::from_utf16_lossy(&buf[..copied]);
                    let note_id = GetPropW(hwnd, w!("NOTE_ID")).0 as usize as u64;
                    let shell = HWND(GetPropW(hwnd, w!("SHELL_HWND")).0);
                    if !shell.0.is_null() {
                        if let Ok(mut queue) = PENDING_EDITS.lock() {
                            queue.push((note_id, content));
                        }
                        let _ = PostMessageW(Some(shell), NOTE_SAVED, WPARAM(0), LPARAM(0));
                    }
                }
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_NCDESTROY => {
                let _ = RemovePropW(hwnd, w!("COLOR_IDX"));
                let _ = RemovePropW(hwnd, w!("NOTE_ID"));
                let _ = RemovePropW(hwnd, w!("SHELL_HWND"));
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// Clamps a pop-out rectangle inside the work area of the monitor nearest the cursor,
/// so a note popped out near a screen edge (or on a small monitor) cannot land
/// partially or fully off-screen. Coordinates are physical pixels, the same space the
/// window itself is created in, so no extra DPI scaling is needed here.
fn clamp_to_work_area(x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
    unsafe {
        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO::default();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            let work = info.rcWork;
            let clamped_x = x.clamp(work.left, (work.right - w).max(work.left));
            let clamped_y = y.clamp(work.top, (work.bottom - h).max(work.top));
            (clamped_x, clamped_y)
        } else {
            (x, y)
        }
    }
}

pub fn spawn_floating_sticky_note(note_id: u64, title: &str, content: &str, color_idx: u8, shell: HWND) {
    unsafe {
        let instance = match GetModuleHandleW(None) {
            Ok(h) => h,
            Err(_) => return,
        };

        let class_name = w!("mynotch.FloatingStickyNote");

        if !CLASS_REGISTERED.load(Ordering::SeqCst) {
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(sticky_wndproc),
                hInstance: HINSTANCE(instance.0),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
                lpszClassName: class_name,
                ..Default::default()
            };
            if RegisterClassW(&wc) != 0 {
                CLASS_REGISTERED.store(true, Ordering::SeqCst);
            }
        }

        let title_w: Vec<u16> = format!("📌 {}", if title.trim().is_empty() { "Catatan Sticky" } else { title })
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let content_w: Vec<u16> = content.encode_utf16().chain(std::iter::once(0)).collect();

        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        let x = if cursor.x > 100 { cursor.x - 120 } else { 100 };
        let y = if cursor.y > 100 { cursor.y + 20 } else { 100 };
        let (x, y) = clamp_to_work_area(x, y, 280, 240);

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name,
            PCWSTR(title_w.as_ptr()),
            WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_VISIBLE,
            x,
            y,
            280,
            240,
            None,
            None,
            Some(HINSTANCE(instance.0)),
            Some(content_w.as_ptr() as *const _),
        );

        if let Ok(h) = hwnd {
            let _ = SetPropW(h, w!("COLOR_IDX"), Some(HANDLE(color_idx as isize as *mut _)));
            let _ = SetPropW(h, w!("NOTE_ID"), Some(HANDLE(note_id as isize as *mut _)));
            let _ = SetPropW(h, w!("SHELL_HWND"), Some(HANDLE(shell.0)));
            let _ = ShowWindow(h, SW_SHOW);
            let _ = UpdateWindow(h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sticky::StickyNote;

    fn note(id: u64, content: &str) -> StickyNote {
        StickyNote { id, title: "N".into(), content: content.into(), color_idx: 0, updated_at: String::new() }
    }

    #[test]
    fn popout_content_is_merged_back_into_the_matching_note() {
        let mut notes = vec![note(1, "first"), note(2, "second")];
        assert!(apply_popout_content(&mut notes, 2, "edited in the pop-out"));
        assert_eq!(notes[1].content, "edited in the pop-out");
        assert!(!notes[1].updated_at.is_empty());
        assert_eq!(notes[0].content, "first");
        assert!(!apply_popout_content(&mut notes, 404, "orphan"));
    }

    #[test]
    fn pending_popout_edits_are_drained_in_order() {
        {
            let mut queue = PENDING_EDITS.lock().unwrap();
            queue.push((1, "first".into()));
            queue.push((2, "second".into()));
        }
        assert_eq!(take_pending_note_edits(), vec![(1, "first".to_string()), (2, "second".to_string())]);
        assert!(take_pending_note_edits().is_empty());
    }
}
