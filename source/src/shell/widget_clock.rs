use crate::glass::{self, GlassEnv, LiquidGlassConfig, MaterialMode};
use crate::render::{self, Painter};
use chrono::{Local, Timelike};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, Ordering};
use windows::core::{w, PCWSTR};
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::HiDpi::GetDpiForWindow,
    UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture},
    UI::WindowsAndMessaging::*,
};

/// Posted to the main shell window when the user finishes dragging the clock widget.
pub const WIDGET_CLOCK_MOVED: u32 = WM_APP + 58;

const WIDGET_DIP: f32 = 188.0;
const SHADOW_PAD: f32 = 12.0;
const CANVAS_DIP: f32 = WIDGET_DIP + SHADOW_PAD * 2.0;
const FRAME_RADIUS: f32 = 34.0;
const DIAL_RADIUS: f32 = 82.0;
const TIMER_ID: usize = 1;

static CLASS_REGISTERED: AtomicBool = AtomicBool::new(false);
static SHELL_HWND_RAW: AtomicIsize = AtomicIsize::new(0);
static WIDGET_HWND_RAW: AtomicIsize = AtomicIsize::new(0);
static LAST_MOVED_X: AtomicI32 = AtomicI32::new(i32::MIN);
static LAST_MOVED_Y: AtomicI32 = AtomicI32::new(i32::MIN);

struct DragState {
    dragging: bool,
    start_cursor_x: i32,
    start_cursor_y: i32,
    start_win_x: i32,
    start_win_y: i32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct WidgetAppearance {
    mode: MaterialMode,
    is_light: bool,
    software: bool,
}

struct WidgetState {
    drag: DragState,
    painter: Option<Painter>,
    appearance: WidgetAppearance,
    scale: f32,
}

unsafe fn window_scale(hwnd: HWND) -> f32 {
    unsafe {
        let dpi = GetDpiForWindow(hwnd);
        if dpi == 0 {
            1.0
        } else {
            (dpi as f32 / 96.0).clamp(1.0, 3.0)
        }
    }
}

unsafe fn ensure_painter(hwnd: HWND, state: &mut WidgetState) -> bool {
    unsafe {
        let scale = window_scale(hwnd);
        let px_w = (CANVAS_DIP * scale).round().max(1.0) as u32;
        let px_h = (CANVAS_DIP * scale).round().max(1.0) as u32;

        if state.painter.is_none() {
            state.painter =
                Painter::new(hwnd, px_w, px_h, 96.0 * scale, state.appearance.software).ok();
            state.scale = scale;
        } else if (state.scale - scale).abs() > 0.01 {
            if let Some(p) = state.painter.as_mut() {
                p.resize(px_w, px_h, 96.0 * scale);
            }
            state.scale = scale;
        }
        state.painter.is_some()
    }
}

unsafe fn paint_widget(hwnd: HWND, state: &mut WidgetState) {
    unsafe {
        if !ensure_painter(hwnd, state) {
            return;
        }
        let scale = state.scale;
        let app = state.appearance;
        let Some(p) = state.painter.as_mut() else {
            return;
        };

        let env = GlassEnv {
            reduced_transparency: render::is_reduced_transparency(),
            high_contrast: render::is_high_contrast(),
            software: app.software || p.is_warp,
            backdrop_unavailable: !p.host_backdrop_supported(),
        };
        let (eff_mode, _) = glass::decide(app.mode, env);
        let eff_light = glass::effective_light(app.is_light, env);
        let cfg = LiquidGlassConfig::for_stage(eff_mode, 2, eff_light);
        let backdrop_on = eff_mode.is_glass() && p.host_backdrop_supported();

        let _ = p.sync_backdrop(
            [
                WIDGET_DIP,
                WIDGET_DIP,
                0.0,
                FRAME_RADIUS,
                1.0,
                SHADOW_PAD,
                SHADOW_PAD,
                scale,
            ],
            backdrop_on,
        );

        if p.begin().is_err() {
            state.painter = None;
            return;
        }

        p.set_translation(SHADOW_PAD, SHADOW_PAD);
        p.draw_widget_shadow(0.0, 0.0, WIDGET_DIP, WIDGET_DIP, FRAME_RADIUS);
        let _ = p.fill_notch_glass(
            WIDGET_DIP,
            WIDGET_DIP,
            0.0,
            FRAME_RADIUS,
            true,
            &cfg,
            (0.5, 0.5, 0.0),
        );

        let now = Local::now();
        let sec = now.second() as f32;
        let min = now.minute() as f32 + sec / 60.0;
        let hr = (now.hour() % 12) as f32 + min / 60.0;
        let center = WIDGET_DIP * 0.5;
        p.draw_analog_clock_face(center, center, DIAL_RADIUS, eff_light, hr, min, sec);
        p.set_translation(0.0, 0.0);

        if p.end().is_err() {
            state.painter = None;
        }
    }
}

fn clamp_to_work_area(x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
    unsafe {
        let pt = POINT {
            x: x + w / 2,
            y: y + h / 2,
        };
        let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO::default();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            let work = info.rcWork;
            (
                x.clamp(work.left, (work.right - w).max(work.left)),
                y.clamp(work.top, (work.bottom - h).max(work.top)),
            )
        } else {
            (x, y)
        }
    }
}

fn default_position(w: i32, h: i32) -> (i32, i32) {
    unsafe {
        let mut work = RECT::default();
        if SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut work as *mut _ as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok()
        {
            let x = (work.right - w - 28).max(work.left);
            let y = (work.top + 36).min((work.bottom - h).max(work.top));
            (x, y)
        } else {
            (40, 40)
        }
    }
}

unsafe extern "system" fn widget_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WidgetState;
        match msg {
            WM_CREATE => {
                let state = Box::new(WidgetState {
                    drag: DragState {
                        dragging: false,
                        start_cursor_x: 0,
                        start_cursor_y: 0,
                        start_win_x: 0,
                        start_win_y: 0,
                    },
                    painter: None,
                    appearance: WidgetAppearance {
                        mode: MaterialMode::Solid,
                        is_light: false,
                        software: false,
                    },
                    scale: 1.0,
                });
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
                let _ = SetTimer(Some(hwnd), TIMER_ID, 1000, None);
                LRESULT(0)
            }
            WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
            WM_WINDOWPOSCHANGING => {
                let wp = lparam.0 as *mut WINDOWPOS;
                if !wp.is_null() {
                    (*wp).hwndInsertAfter = HWND_BOTTOM;
                }
                LRESULT(0)
            }
            WM_TIMER if wparam.0 == TIMER_ID => {
                if !state_ptr.is_null() {
                    paint_widget(hwnd, &mut *state_ptr);
                }
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                if !state_ptr.is_null() {
                    let state = &mut *state_ptr;
                    let mut cursor = POINT::default();
                    let _ = GetCursorPos(&mut cursor);
                    let mut rc = RECT::default();
                    let _ = GetWindowRect(hwnd, &mut rc);
                    state.drag = DragState {
                        dragging: true,
                        start_cursor_x: cursor.x,
                        start_cursor_y: cursor.y,
                        start_win_x: rc.left,
                        start_win_y: rc.top,
                    };
                    let _ = SetCapture(hwnd);
                }
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                if !state_ptr.is_null() {
                    let state = &mut *state_ptr;
                    if state.drag.dragging {
                        let mut cursor = POINT::default();
                        let _ = GetCursorPos(&mut cursor);
                        let dx = cursor.x - state.drag.start_cursor_x;
                        let dy = cursor.y - state.drag.start_cursor_y;
                        let mut rc = RECT::default();
                        let _ = GetWindowRect(hwnd, &mut rc);
                        let w = rc.right - rc.left;
                        let h = rc.bottom - rc.top;
                        let (nx, ny) = clamp_to_work_area(
                            state.drag.start_win_x + dx,
                            state.drag.start_win_y + dy,
                            w,
                            h,
                        );
                        let _ = SetWindowPos(
                            hwnd,
                            Some(HWND_BOTTOM),
                            nx,
                            ny,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                }
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                if !state_ptr.is_null() {
                    let state = &mut *state_ptr;
                    if state.drag.dragging {
                        state.drag.dragging = false;
                        let _ = ReleaseCapture();
                        let mut rc = RECT::default();
                        let _ = GetWindowRect(hwnd, &mut rc);
                        LAST_MOVED_X.store(rc.left, Ordering::Relaxed);
                        LAST_MOVED_Y.store(rc.top, Ordering::Relaxed);
                        let shell = HWND(SHELL_HWND_RAW.load(Ordering::Relaxed) as *mut _);
                        if !shell.0.is_null() {
                            let _ = PostMessageW(
                                Some(shell),
                                WIDGET_CLOCK_MOVED,
                                WPARAM(0),
                                LPARAM(0),
                            );
                        }
                    }
                }
                LRESULT(0)
            }
            WM_DPICHANGED => {
                if !state_ptr.is_null() {
                    let state = &mut *state_ptr;
                    let scale = window_scale(hwnd);
                    let px_w = (CANVAS_DIP * scale).round().max(1.0) as i32;
                    let px_h = (CANVAS_DIP * scale).round().max(1.0) as i32;
                    let _ = SetWindowPos(
                        hwnd,
                        Some(HWND_BOTTOM),
                        0,
                        0,
                        px_w,
                        px_h,
                        SWP_NOMOVE | SWP_NOACTIVATE,
                    );
                    paint_widget(hwnd, state);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                let _ = KillTimer(Some(hwnd), TIMER_ID);
                WIDGET_HWND_RAW.store(0, Ordering::SeqCst);
                let ptr = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut WidgetState;
                if !ptr.is_null() {
                    drop(Box::from_raw(ptr));
                }
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

pub fn take_last_moved_pos() -> Option<(i32, i32)> {
    let x = LAST_MOVED_X.swap(i32::MIN, Ordering::Relaxed);
    let y = LAST_MOVED_Y.swap(i32::MIN, Ordering::Relaxed);
    if x == i32::MIN || y == i32::MIN {
        None
    } else {
        Some((x, y))
    }
}

pub fn sync_widget_clock(
    shell_hwnd: HWND,
    enabled: bool,
    mode: MaterialMode,
    is_light: bool,
    software: bool,
    x: i32,
    y: i32,
) {
    SHELL_HWND_RAW.store(shell_hwnd.0 as isize, Ordering::Relaxed);

    if !enabled {
        destroy_widget_clock();
        return;
    }

    let desired = WidgetAppearance {
        mode,
        is_light,
        software,
    };

    unsafe {
        let existing = WIDGET_HWND_RAW.load(Ordering::SeqCst);
        if existing != 0 {
            let hwnd = HWND(existing as *mut _);
            if IsWindow(Some(hwnd)).as_bool() {
                let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WidgetState;
                if !state_ptr.is_null() {
                    let state = &mut *state_ptr;
                    let software_changed = state.appearance.software != desired.software;
                    let changed = state.appearance != desired;
                    state.appearance = desired;
                    if software_changed {
                        state.painter = None;
                    }
                    if changed {
                        paint_widget(hwnd, state);
                    }
                }
                return;
            }
            WIDGET_HWND_RAW.store(0, Ordering::SeqCst);
        }

        let Ok(instance) = GetModuleHandleW(None) else {
            return;
        };
        let class_name = w!("mynotch.DesktopClockWidget");
        if !CLASS_REGISTERED.load(Ordering::SeqCst) {
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(widget_wndproc),
                hInstance: HINSTANCE(instance.0),
                hCursor: LoadCursorW(None, IDC_SIZEALL).unwrap_or_default(),
                lpszClassName: class_name,
                ..Default::default()
            };
            if RegisterClassW(&wc) != 0 {
                CLASS_REGISTERED.store(true, Ordering::SeqCst);
            }
        }

        let init_w = CANVAS_DIP.round() as i32;
        let init_h = CANVAS_DIP.round() as i32;
        let (wx, wy) = if x == i32::MIN || y == i32::MIN {
            default_position(init_w, init_h)
        } else {
            clamp_to_work_area(x, y, init_w, init_h)
        };

        let Ok(hwnd) = CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name,
            PCWSTR(w!("mynotch Clock Widget").as_ptr()),
            WS_POPUP,
            wx,
            wy,
            init_w,
            init_h,
            None,
            None,
            Some(HINSTANCE(instance.0)),
            None,
        ) else {
            return;
        };

        WIDGET_HWND_RAW.store(hwnd.0 as isize, Ordering::SeqCst);
        let scale = window_scale(hwnd);
        let px_w = (CANVAS_DIP * scale).round() as i32;
        let px_h = (CANVAS_DIP * scale).round() as i32;
        let (fx, fy) = if x == i32::MIN || y == i32::MIN {
            default_position(px_w, px_h)
        } else {
            clamp_to_work_area(x, y, px_w, px_h)
        };
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_BOTTOM),
            fx,
            fy,
            px_w,
            px_h,
            SWP_NOACTIVATE,
        );
        let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WidgetState;
        if !state_ptr.is_null() {
            let state = &mut *state_ptr;
            state.appearance = desired;
            paint_widget(hwnd, state);
        }
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
}

pub fn reset_widget_clock_pos() {
    let raw = WIDGET_HWND_RAW.load(Ordering::SeqCst);
    if raw == 0 {
        return;
    }
    unsafe {
        let hwnd = HWND(raw as *mut _);
        if IsWindow(Some(hwnd)).as_bool() {
            let scale = window_scale(hwnd);
            let px_w = (CANVAS_DIP * scale).round() as i32;
            let px_h = (CANVAS_DIP * scale).round() as i32;
            let (fx, fy) = default_position(px_w, px_h);
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_BOTTOM),
                fx,
                fy,
                px_w,
                px_h,
                SWP_NOACTIVATE,
            );
        }
    }
}

pub fn destroy_widget_clock() {
    let raw = WIDGET_HWND_RAW.swap(0, Ordering::SeqCst);
    if raw != 0 {
        unsafe {
            let hwnd = HWND(raw as *mut _);
            if IsWindow(Some(hwnd)).as_bool() {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}
