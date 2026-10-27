use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::{w, PCWSTR};
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::*,
};

static CLASS_REGISTERED: AtomicBool = AtomicBool::new(false);

const EDIT_ID: usize = 101;

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
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_NCDESTROY => {
                let _ = RemovePropW(hwnd, w!("COLOR_IDX"));
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

pub fn spawn_floating_sticky_note(title: &str, content: &str, color_idx: u8) {
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
            let _ = ShowWindow(h, SW_SHOW);
            let _ = UpdateWindow(h);
        }
    }
}
