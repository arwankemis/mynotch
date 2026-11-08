use crate::{core::*, ui, media::{MediaService,MediaEvent,MediaSnapshot,MediaSessions}, visualizer::Visualizer, render::{self,Painter}, online::{OnlineService,WeatherReport,LyricsResult}, ical::CalendarEvent};
use crate::glass;
use chrono::{Local,Datelike,Timelike,NaiveDate};
use std::{cell::RefCell,collections::BTreeMap,sync::{Arc,Mutex,mpsc,atomic::{AtomicIsize,Ordering}},time::{Duration,Instant},thread};
use windows::core::{w,PCWSTR};
use windows::Win32::{Foundation::*,System::{LibraryLoader::GetModuleHandleW,Com::*},Graphics::Gdi::*,Media::{timeSetEvent,timeKillEvent,TIME_PERIODIC},UI::{Controls::WM_MOUSELEAVE,WindowsAndMessaging::*,Input::KeyboardAndMouse::*,HiDpi::*}};

mod notes_host;
mod editor_host;
mod preset_host;
mod activity_host;
mod pages;
mod reminder;
mod settings;
mod catalog_ui;
mod settings_layout;
mod sticky_window;
mod weather;
mod material;
pub mod privacy;
mod quick_controls_ui;

const WAKE:u32=WM_APP+1;const TRAY:u32=WM_APP+2;const MORPH_MSG:u32=WM_APP+5;const EM_SETSEL:u32=0x00B1;
const WM_APP_VOLUME_CHANGED:u32=WM_APP+55;
const WM_APP_SCREENSHOT:u32=WM_APP+57;
/// A single rendered frame is not proof of health; the updated build must stay alive this long.
const HEALTH_ACK_STABILITY:Duration=Duration::from_secs(5);
/// Duration of the Stage 2 lyric-sheet vertical slide (mockup: transform .55s cubic-bezier(.33,.8,.35,1)).
const SHEET_SLIDE:Duration=Duration::from_millis(550);
static HOOK_HWND: AtomicIsize = AtomicIsize::new(0);
static HOOK_HANDLE: AtomicIsize = AtomicIsize::new(0);

#[derive(Clone, Copy, PartialEq, Debug)]
enum VolumeKey { Up, Down, Mute }
fn volume_key(vk: u16) -> Option<VolumeKey> {
    match vk {
        0xAF => Some(VolumeKey::Up),
        0xAE => Some(VolumeKey::Down),
        0xAD => Some(VolumeKey::Mute),
        _ => None,
    }
}

fn install_volume_hook(preview:bool)->bool{!preview}

unsafe extern "system" fn ll_keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && lparam.0 != 0 {
        let msg = wparam.0 as u32;
        unsafe {
            let kbd = *(lparam.0 as *const KBDLLHOOKSTRUCT);
            let hwnd_val = HOOK_HWND.load(Ordering::Relaxed);
            if hwnd_val != 0 && (msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN) {
                if kbd.vkCode == 0x53 { // 'S' key for screenshot
                    let is_shift = (GetAsyncKeyState(VK_SHIFT.0 as i32) as i32 & 0x8000) != 0
                        || (GetKeyState(VK_SHIFT.0 as i32) as i32 & 0x8000) != 0;
                    let is_ctrl = (GetAsyncKeyState(VK_CONTROL.0 as i32) as i32 & 0x8000) != 0
                        || (GetKeyState(VK_CONTROL.0 as i32) as i32 & 0x8000) != 0;
                    let is_win = (GetAsyncKeyState(VK_LWIN.0 as i32) as i32 & 0x8000) != 0
                        || (GetAsyncKeyState(VK_RWIN.0 as i32) as i32 & 0x8000) != 0;
                    if is_shift && (is_ctrl || is_win) {
                        let _ = PostMessageW(Some(HWND(hwnd_val as *mut _)), WM_APP_SCREENSHOT, WPARAM(0), LPARAM(0));
                    }
                } else if kbd.vkCode == 0x2C { // VK_SNAPSHOT (PrintScreen)
                    let _ = PostMessageW(Some(HWND(hwnd_val as *mut _)), WM_APP_SCREENSHOT, WPARAM(0), LPARAM(0));
                }
            }
            // Injected keys and fullscreen-hidden windows keep the Windows handling path.
            if volume_key(kbd.vkCode as u16).is_some()
                && kbd.flags.0 & LLKHF_INJECTED.0 == 0
                && hwnd_val != 0
                && IsWindowVisible(HWND(hwnd_val as *mut _)).as_bool()
            {
                if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                    let _ = PostMessageW(Some(HWND(hwnd_val as *mut _)), WM_APP_VOLUME_CHANGED, WPARAM(kbd.vkCode as usize), LPARAM(0));
                }
                return LRESULT(1);
            }
        }
    }
    let hook_val = HOOK_HANDLE.load(Ordering::Relaxed);
    let hook = if hook_val != 0 { Some(HHOOK(hook_val as *mut _)) } else { None };
    unsafe { CallNextHookEx(hook, code, wparam, lparam) }
}
unsafe extern "system" {
    fn MessageBeep(utype: u32) -> i32;
}
/// Display version comes from the same embedded release identity as the updater.
pub fn app_version() -> &'static str {
    static IDENTITY: std::sync::OnceLock<crate::update::model::BuildIdentity> = std::sync::OnceLock::new();
    &IDENTITY.get_or_init(crate::update::model::BuildIdentity::current).version
}
/// Single source for the window title and the tray tooltip.
pub fn app_title() -> String { format!("mynotch {}", app_version()) }

#[test]
fn app_title_uses_release_manifest_without_extra_suffix() {
    let identity = crate::update::model::BuildIdentity::current();
    assert_eq!(app_version(), identity.version);
    assert_eq!(app_title(), format!("mynotch {}", identity.version));
}

pub use crate::update::UpdateStatus;

thread_local!{static APP:RefCell<Option<Shell>>=const{RefCell::new(None)};}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToastKind {
    Water,
    Stretch,
    Prayer,
    Focus,
    Generic,
    UpdateReady,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct ToastNotification {
    pub kind: ToastKind,
    pub message: String,
    pub expires_at: Instant,
    pub confirmed: bool,
    pub confirmed_at: Option<Instant>,
}

#[allow(dead_code)]
#[derive(Clone,PartialEq)]
enum Action {

    NotesSubview(bool),ConvertNote(u64),NoteSelectId(u64),NoteCommand(u64,u8),NoteExportMenu(u64),NoteExportPick(u64,u8),RetryNoteSave,TaskSelect(u64,u64),TaskDone(u64,bool,u64),OpenTodoNote(u64,u64),DetachTodoNote(u64,u64),TasksPage(i32),
    Preset(crate::presets::PresetAction),PresetSettings,
    ActivityExpired,OpenActivity(crate::activity::ActivityId,u64),DismissActivity(crate::activity::ActivityId,u64),ConfirmActivity(crate::activity::ActivityId,u64),
    Tab(String),Settings,Back,Close,Exit,Bottom(bool),Minimal,Gap(i32),Scale(f32),Opacity(i32),ToggleModule(String),Toggle(&'static str),
    Play,Next,Previous,Seek(f64),SeekPointer,VolumeStep(i32),MasterVolumeStep(i32),LyricsFollow,LyricsRetry,Lyric(usize),FocusStart,FocusPause,FocusReset,FocusBreak,FocusLength(i32),BreakLength(i32),
    AddTodo,Todo(u64),DeleteTodo(u64,u64),TodoPage(i32),SaveNote,Water,UndoWater,WaterTarget(i32),WaterServing(i32),Breath,
    Month(i32),Day(u32),JumpToDate(i32,u32,u32),CalendarSave,CalendarRefresh,WeatherSave,WeatherRefresh,WeatherToggleMenu,AiSave,AiSend,AiClear,
    ToggleLyrics,ToggleShuffle,CycleRepeat,SelectMode(usize),MediaSourceMenu,SelectMediaSource(Option<String>),
    CalendarToday,CalendarLeaveToggle,
    TogglePin,ToStage0,ToStage1,ExpandMusic,ExpandHardware,ExpandWeather,SimpanSub(u8),
    WaterInterval(i32),MoveInterval(i32),ReminderSelect(usize),ReminderStep(i32),StretchLength(i32),StretchStart,StretchDone,PrayerLead(i32),PrayerRefresh,ReminderMenu(&'static str),ReminderValue(&'static str,u32),
    OpenSettingsAudio,ConfirmWater,SelectClip(usize),CopyClip(usize),DeleteClip(u64),ClearClips,ClipPage(i32),TogglePinClip(u64),ClearClipFilter,ToggleClipSearch,WeatherSetCity(&'static str,f64,f64),WeatherToggleCustom,CheckUpdate,DownloadUpdate,PerbaruiUpdate,RestartNow,RestartLater,OpenReleaseNotes,OpenRepoUrl,
    MoveTab(String, i32),
    SettingsCategory(usize),SetDefaultTab(String),SettingsMenu(&'static str,f32,f32,f32),SettingsValue(&'static str,u32),SettingsOpenTab(String,usize),
    PowerSensorSetup,InstallMod(String),UninstallMod(String),DeleteCustomMod(String),PickAndInstallMod,OpenModsFolder,ModButtonClick(String),NoteAdd,NoteSelect(usize),NoteDelete(u64),NoteColor(u8),NoteExport(u8),NotePopout,
    SelectMod(String),BackModsList,OpenMod(String),CloseModView,RefreshMods,ToggleModTab(String),ToggleModNetwork(String),
    SetModKind(crate::mods::package::PackageKind),SetModFilter(crate::mods::catalog::CatalogFilter),ToggleModActive(String),
    PrivacyDetail,PrivacyClose,OpenWindowsMicSettings,OpenWindowsCameraSettings,OpenPrivacySettings,ClosePrivacySettings,
    ToggleStorageDropdown,KeepStorageDropdown,CycleStoragePage,
    PromptDownloadPowerProvider,ConfirmDownloadPowerProvider,CancelDownloadPowerProvider,
    QuickControlsOpen,QuickControlsBack,
    QuickControlsToggleRadio(crate::quick_controls::RadioKind),
    QuickControlsSelectRadio(crate::quick_controls::RadioKind,usize),
    QuickControlsSelectDisplay(usize),
    QuickControlsSetBrightness(u32,bool),
    QuickBrightnessPointer,
    OpenSettingsPage(&'static str),
}
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct TabDragState {
    pub tab: String,
    pub start_pos: (f32, f32),
    pub current_pos: (f32, f32),
    pub grab_offset_x: f32,
    pub is_dragging: bool,
    pub started_at: Instant,
}
#[derive(Clone)]struct Hit{rect:render::D2D_RECT_F,action:Action}
#[allow(dead_code)]
#[derive(Clone)]struct LyricAnim{prev_text:String,started:Instant}
// Controls are lossless FIFO; streaming contributes at most one latest sample.
#[derive(Default)]
struct EventQueue { events: Vec<Event>, wake_pending: bool, closed: bool }
impl EventQueue {
    fn push(&mut self, event: Event) -> bool {
        if self.closed { return false; }
        if matches!(&event, Event::Levels(_)) {
            if let Some(index) = self.events.iter().position(|e| matches!(e, Event::Levels(_))) {
                self.events.remove(index);
            }
        }
        self.events.push(event);
        if self.wake_pending { false } else { self.wake_pending = true; true }
    }
    fn drain(&mut self) -> Vec<Event> {
        self.wake_pending = false;
        std::mem::take(&mut self.events)
    }
    fn close(&mut self) {
        self.closed = true;
        self.wake_pending = false;
        self.events.clear();
    }
}
#[derive(Clone)]struct Poster{queue:Arc<Mutex<EventQueue>>,hwnd:usize}
impl Poster {
    fn send(&self, event: Event) {
        if let Ok(mut q) = self.queue.lock() {
            // PostMessage never waits for the UI while we hold this lock.
            if q.push(event) {
                let posted = unsafe { PostMessageW(Some(HWND(self.hwnd as *mut _)),WAKE,WPARAM(0),LPARAM(0)) };
                // Retry notification on the next send if the OS rejected it.
                if posted.is_err() { q.wake_pending = false; }
            }
        }
    }
}
#[allow(dead_code)]
enum Event {
    Prayers(String,String,Result<crate::online::PrayerSchedule,String>),

    Artwork(u64,Result<crate::artwork::Artwork,String>),
    Completed(&'static str,u64,Box<Event>),
    Media(MediaEvent),Levels(Vec<f32>),Lyrics(crate::lyrics_gate::LyricsPermission,String,Result<LyricsResult,String>),Weather(Result<WeatherReport,String>, bool),
    Agenda(Result<Vec<CalendarEvent>,String>),Ai(Result<String,String>),Notice(String),Update(Result<Option<crate::online::GithubRelease>,String>),
    UpdateProgress(u8, u64, Option<u64>),
    UpdateStaged(Result<(String, std::path::PathBuf), String>),
    ModHttp(String,Result<String,String>),
    Tws(Result<Option<crate::system::TwsInfo>,String>),
    Storage(Option<f32>, Option<f32>, Option<f32>, Vec<crate::system::DriveInfo>),
    Hardware(crate::system::HardwareSample),
    ProviderDownloadProgress(u8),
    ProviderDownloadResult(Result<String, String>),
}

#[allow(dead_code)]
pub(super) struct Shell {
    notes_host:notes_host::NotesHost,
    pub simpan_sub:u8,
    pub weather_stage1_until:Option<Instant>,
    editors:editor_host::Editors,preset_host:preset_host::PresetHost,
    activities:activity_host::Activities,
    suspended:bool,

    power_sensors:Option<crate::system::power_sensor::PowerSensors>,power_integrator:crate::mods::power_broker::EnergyIntegrator,power_epoch:Instant,power_day:String,power_energy:crate::mods::power_broker::ObservedEnergy,power_flushed:Instant,
    cpu_sampler:crate::system::CpuSampler,power_snapshot:crate::system::PowerSnapshot,last_power_tick:Instant,session_energy_kwh:f32,
    last_storage_tick:Option<Instant>,storage_job_started:Option<Instant>,cached_storage:(Option<f32>,Option<f32>,Option<f32>,Vec<crate::system::DriveInfo>),
    render_ready:bool,
    cover:Option<crate::artwork::Artwork>,cover_key:u64,preview_silent:bool,
    online:OnlineService,
    drafts:BTreeMap<(String,u16),String>,input_page:String,
    jobs:BTreeMap<&'static str,(u64,crate::net::Cancellation)>,next_job:u64,
    hwnd:HWND,inputs_hwnd:HWND,input_x:i32,input_y:i32,canvas_w:i32,canvas_h:i32,win_x:i32,win_y:i32,painter:Option<Painter>,frame_contour:Option<glass::FrameContour>,presented_contour:Option<glass::FrameContour>,store:Store,settings:Settings,state:State,
    expanded:bool,settings_open:bool,settings_cat:usize,settings_scroll:[f32;7],settings_popup:Option<settings::SettingsPopup>,tab:String,scale:f32,width:f32,height:f32,hits:Vec<Hit>,inputs:BTreeMap<u16,HWND>,
    morph:Option<ui::Morph>,pointer:Option<(f32,f32)>,hit_enabled:bool,pressed:Option<Action>,focused:Option<Action>,tab_fade:Option<(String,Instant,i8)>,painting:bool,morph_timer:u32,
    media:Option<MediaService>,track:MediaSnapshot,media_sessions:MediaSessions,media_selection_revision:u64,media_source_open:bool,media_source_cursor:usize,visualizer:Option<Visualizer>,levels:Vec<f32>,
    lyrics_consent:crate::lyrics_gate::LyricsConsent,lyrics_data:Option<LyricsResult>,
    lyrics:Vec<crate::lyrics::LyricLine>,plain_lyrics:String,lyrics_key:String,lyric_offset:usize,lyrics_error:Option<String>,lyrics_retry:crate::lyrics::RetryState,lyrics_from_youtube:bool,
    focus:FocusClock,breathing:Option<Instant>,last_water:Instant,last_move:Instant,last_eye:Instant,
    toast:Option<ToastNotification>,queue:Arc<Mutex<EventQueue>>,fullscreen:bool,fullscreen_hwnd:Option<HWND>,toast_from_stage0:bool,
    last_clipboard_seq:u32,selected_clip:usize,clip_offset:usize,
    year:i32,month:u32,day:u32,system_date:NaiveDate,holidays:Vec<HolidayRecord>,agenda:Vec<CalendarEvent>,weather:Option<WeatherReport>,
    ai_messages:Vec<crate::online::ChatMessage>,busy:bool,todo_offset:usize,
    prayer_schedule:Option<crate::online::PrayerSchedule>,prayer_attempt:Option<(String,Instant)>,prayer_error:Option<String>,reminder_menu:Option<&'static str>,reminder_menu_cursor:usize,pub reminder_menu_scroll:usize,
    reminder_index:usize,reminder_anim:Option<(f32,Instant)>,stretch_deadline:Option<Instant>,toast_queue:std::collections::VecDeque<(ToastKind,String,u32)>,last_schedule:String,weather_updated:Option<Instant>,weather_fetched_time:Option<String>,agenda_updated:Option<Instant>,preview:bool,
    lyric_mode:bool,hover_stage:u8,pinned:bool,
    pub screenshot_stage2_freeze:Option<Instant>,
    pub screenshot_hide_until:Option<Instant>,
    hover_blocked_until:Instant,
    last_lyric_line:String,lyric_anim:Option<LyricAnim>,
    /// Stage 2 lyric sheet (mockup stage2-musik): last rendered active-line index and the
    /// running 550 ms slide (offset in rows + start) used to animate scroll between lines.
    sheet_index:usize,sheet_slide:Option<(f32,std::time::Instant)>,
    tab_drag:Option<TabDragState>,
    /// Startup fade-in: ramps 0→1 over ~300 ms (20 ticks × 16 ms via timer 11).
    /// Permanently 1.0 once fully visible; timer 11 is killed at that point.
    startup_alpha:f32,
    /// F1 sekuens boot: mulai Stage 0 (fade + turun kecil), lalu timer 13 (2,5 dtk)
    /// morph ke Stage 1 idle. Some(t) = animasi turun 600 ms masih berjalan sejak t.
    /// None saat preview, reduced-motion, atau setelah animasi selesai.
    boot_drop:Option<Instant>,
    /// F2 sweep aksen Nyala: anchor siklus 3 dtk + aksen yang sedang berjalan.
    /// Diset draw_activity saat menggambar aksen, di-wrap morph_step tiap periode,
    /// dan dikosongkan paint() bila frame terakhir tidak menampilkan pill aksen.
    activity_sweep:Option<(Instant,u32)>,
    /// Penanda "frame ini menggambar pill aksen" — diperiksa paint() setelah draw().
    activity_sweep_live:bool,
    /// Repaint sweep terakhir, untuk decimasi ~40 fps pada timer morph 8 ms.
    activity_sweep_paint:Instant,
    pub stage1_temporary_until: Option<Instant>,
    stage1_media_until:Option<Instant>,
    pub volume_hud: Option<(f32, bool, Instant)>,
    pub battery: Option<crate::system::BatteryInfo>,
    pub last_battery_check: Instant,
    tws: Option<crate::system::TwsInfo>,
    pub cached_idle_clock_width: f32,
    pub cached_lyric_geometry: (f32, f32, f32),
    pub hardware_sampler: crate::system::HardwareSampler,
    pub hardware_sample: Option<crate::system::HardwareSample>,
    pub last_hardware_tick: Instant,
    pub cached_hardware_width: f32,
    last_tws_check: Instant,
    tws_pending: bool,
    /// First poll only primes the state; it must not toast for earbuds that were already on.
    tws_ready: bool,
    pub last_mod_button_clicked: Option<String>,
    /// Cached Lua output per mod id. Painted from here; the VM never runs inside `paint()`.
    pub mod_output: Option<(String, Result<crate::mod_script::LuaModOutput, String>)>,
    /// Community mod currently open in its own Stage 2 view (opened from Settings → Mod).
    pub mod_view: Option<String>,
    mods_catalog_cache:Option<crate::mods::catalog::Catalog>,
    mods_selected: Option<String>,mods_detail_open:bool,mods_detail_scroll:f32,
    mods_scroll: usize,
    pub mods_kind: crate::mods::package::PackageKind,
    pub mods_filter: crate::mods::catalog::CatalogFilter,
    mods_cache: Option<Vec<crate::mods::ModManifest>>,
    /// URL -> (fetched_at, body or error) for community-mod HTTP requests.
    mod_http_cache: BTreeMap<String, (Instant, Result<String, String>)>,
    mod_http_ttl: Duration,
    pub last_ac_status: Option<bool>,
    pub clip_filter: String,
    pub clip_search_open: bool,
    pub update_status: UpdateStatus,
    pub weather_custom_open: bool,
    weather_menu_open: bool,
    note_export_open: Option<u64>,
    weather_refresh: crate::weather::RefreshState,
    pub last_known_volume: Option<(f32, bool)>,
    pub privacy_mailbox_tx: crate::privacy::MailboxSender,
    pub privacy_mailbox_rx: crate::privacy::MailboxReceiver,
    pub privacy_snapshot: crate::privacy::PrivacySnapshot,
    pub privacy_monitor: Option<crate::privacy::PrivacyMonitorHandle>,
    pub quick_controls: crate::quick_controls::QuickControlsController,
    pub quick_controls_open: bool,
    pub privacy_detail_open: bool,
    pub settings_subpage_privacy: bool,
    pub storage_dropdown_open: bool,
    pub storage_dropdown_page: usize,
    pub track_is_youtube: bool,
    pub pending_ack_tx: Option<std::path::PathBuf>,
    /// Earliest moment the updated build may report itself healthy.
    pub pending_ack_ready_at: Option<Instant>,
    pub provider_downloading: bool,
    pub provider_download_progress: u8,
    pub confirm_provider_download: bool,
    pub palette: crate::theme::ResolvedPalette,
    glass: material::GlassRt,
}
#[derive(Clone,serde::Deserialize)]struct HolidayRecord{date:NaiveDate,name:String,#[serde(default)]collective_leave:bool}
#[derive(serde::Deserialize)]struct HolidayFile{holidays:Vec<HolidayRecord>}

pub fn run(args:&[String])->Result<(),String>{unsafe{
    let _=SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    if args.iter().any(|a|a=="--fullscreen-fixture"){return fullscreen_fixture()}
    struct Instance(HANDLE);impl Drop for Instance{fn drop(&mut self){unsafe{let _=CloseHandle(self.0);}}}
    let preview_instance=args.iter().any(|s|matches!(s.as_str(),"--preview"|"--live-preview"));
    // GetLastError is only meaningful immediately after a failed-or-set call. Earlier DPI/window
    // calls may leave ERROR_ALREADY_EXISTS behind, which would make the first instance exit.
    SetLastError(ERROR_SUCCESS);
    let handle=windows::Win32::System::Threading::CreateMutexW(None,false,if preview_instance{w!("Local\\mynotch.Preview.v0.0.1")}else{w!("Local\\mynotch.Native.v0.0.1")}).map_err(|e|e.to_string())?;
    let duplicate=GetLastError()==ERROR_ALREADY_EXISTS;let _instance=Instance(handle);
    if duplicate{
        if should_focus_existing_instance(preview_instance,duplicate){
            if let Ok(existing)=FindWindowW(w!("mynotch.Native"),PCWSTR::null()){
                let _=ShowWindow(existing,SW_SHOWNOACTIVATE);
                let _=PostMessageW(Some(existing),WM_COMMAND,WPARAM(900),LPARAM(0));
            }
        }
        return Ok(())
    }
    // A failed COM init or class registration used to be swallowed (let _= / ignored
    // return), so the later CreateWindowExW failure blamed the wrong thing. Fail here
    // with a clear message instead; run()'s caller shows it in a message box.
    if CoInitializeEx(None,COINIT_APARTMENTTHREADED).is_err(){
        return Err("Inisialisasi COM gagal; aplikasi tidak dapat dimulai.".into());
    }
    let module=GetModuleHandleW(None).map_err(|e|e.to_string())?;
    let class=WNDCLASSW{lpfnWndProc:Some(wndproc),hInstance:module.into(),lpszClassName:w!("mynotch.Native"),hCursor:LoadCursorW(None,IDC_ARROW).unwrap_or_default(),..Default::default()};
    if RegisterClassW(&class)==0 && GetLastError()!=ERROR_ALREADY_EXISTS {
        return Err("Registrasi kelas window utama gagal; aplikasi tidak dapat dimulai.".into());
    }
    // The shell presents through a DirectComposition swap chain, and a window with
    // WS_EX_NOREDIRECTIONBITMAP cannot show child HWNDs. The EDIT controls therefore live in
    // this separate standard window, which is clipped by a region to exactly the input
    // rectangles: everything outside the region is neither painted nor clickable, so the shell
    // behind it keeps receiving every other click.
    let input_class=WNDCLASSW{lpfnWndProc:Some(inputs_wndproc),hInstance:module.into(),lpszClassName:w!("mynotch.Inputs"),..Default::default()};
    if RegisterClassW(&input_class)==0 && GetLastError()!=ERROR_ALREADY_EXISTS {
        return Err("Registrasi kelas window input gagal; aplikasi tidak dapat dimulai.".into());
    }
    let inputs_hwnd=CreateWindowExW(input_host_ex_styles(),w!("mynotch.Inputs"),None,WS_POPUP,0,0,0,0,None,None,Some(module.into()),None).map_err(|e|e.to_string())?;
    let live_preview=args.iter().any(|s|s=="--live-preview");

    let preview=live_preview||args.iter().any(|s|s=="--preview");
    let window_title:Vec<u16>=app_title().encode_utf16().chain(std::iter::once(0)).collect();
    let hwnd=CreateWindowExW(shell_ex_styles(preview),w!("mynotch.Native"),PCWSTR(window_title.as_ptr()),WS_POPUP,0,0,400,112,None,None,Some(module.into()),None).map_err(|e|e.to_string())?;
    SetWindowLongPtrW(inputs_hwnd,GWLP_HWNDPARENT,hwnd.0 as isize);
    let executable=std::env::current_exe().map_err(|_|"Lokasi aplikasi tidak ditemukan.")?;
    let root=crate::app::storage_root(&executable,preview)?;
    let mut store=Store::new(root)?;
    let settings_existed=store.root.join("settings.json").exists();
    let mut settings:Settings=store.load("settings.json");
    // Custom (JSON) themes are removed from the public build; always use the built-in palette.
    let palette = crate::theme::ResolvedPalette::default();
    if !settings_existed {

        let check_mods = [("sticky_notes","Catatan"),("weather","Cuaca")];
        for (id,tab) in check_mods {
            if store.root.join("mods").join(format!("{id}.mod.json")).exists(){
                settings.installed_mods.push(id.into());
                settings.modules.insert(tab.into(),true);
                settings.tab_order.push(tab.into());
            }
        }
    }
    // Drop tab registrations for mods removed outside the app (folder deleted, zip replaced).
    let available_mods=crate::mods::available_mods(&store.root);
    let known_tabs:Vec<String>=available_mods.iter().map(|m|m.tab_name.clone()).collect();
    settings.mod_tabs.retain(|t|known_tabs.iter().any(|k|k==t));
    settings.normalize();
    crate::net::set_allowed_hosts(&settings.net_allowed_hosts);
    if !preview{settings.run_at_startup=crate::system::is_run_at_startup_enabled();}
    if preview&&args.iter().any(|a|a=="--edge-check"){settings.bottom=false;settings.edge_gap=0;if let Some(scale)=args.iter().find_map(|a|a.strip_prefix("--ui-scale=").and_then(|v|v.parse::<f32>().ok())){settings.scale=scale;settings.normalize();}}
    let mut state:State=store.load("state.json");
    state.normalize_note_links();state.normalize_activities();
    // Migrate legacy clipboard text before UI startup; failed sealing never falls back.
    let clipboard_migration = if crate::clipboard::migrate(&mut state.clipboard_history, crate::clipboard::now_unix()) {
        store.save("state.json", &state).err()
    } else { None };
    // Keep legacy totals until the new broker owns persistence end-to-end.
    // Never clear the only live accumulator during application startup.
    let now=Local::now();
    let power_energy=crate::mods::power_broker::ObservedEnergy::load(&store.root);
    let queue=Arc::new(Mutex::new(EventQueue::default()));let online=OnlineService::new(store.root.clone());
    // F1: non-preview selalu boot dari Stage 0; lihat boot_start_stage()/boot_completes_to_stage().
    let (privacy_mailbox_tx, privacy_mailbox_rx) = crate::privacy::mailbox(1);
    let privacy_snapshot = crate::privacy::PrivacySnapshot::new(1);
    let privacy_monitor = if settings.privacy_enabled && !preview {
        Some(crate::privacy::PrivacyMonitorHandle::start(privacy_mailbox_tx.clone()))
    } else {
        None
    };
    let lyrics_consent=crate::lyrics_gate::LyricsConsent::default();
    lyrics_consent.set(settings.lyrics_online,settings.lyrics_youtube_fallback);
    // F1: animasi "turun kecil" boot dihitung sebelum `settings` dipindah ke Shell.
    let boot_drop=if preview||settings.effective_presentation().reduced_motion{None}else{Some(Instant::now())};
    let mut app=Shell{
        lyrics_consent,lyrics_data:None,
        notes_host:Default::default(),simpan_sub:0,weather_stage1_until:None,
        editors:Default::default(),preset_host:Default::default(),
        activities:Default::default(),
        suspended:false,
        power_sensors:None,power_integrator:Default::default(),power_epoch:Instant::now(),power_day:now.format("%Y-%m-%d").to_string(),power_energy,power_flushed:Instant::now(),
        cpu_sampler:crate::system::CpuSampler::default(),power_snapshot:crate::system::PowerSnapshot::default(),last_power_tick:Instant::now(),session_energy_kwh:0.0,
        last_storage_tick:None,storage_job_started:None,cached_storage:(None,None,None,Vec::new()),
        render_ready:false,
        cover:None,cover_key:0,preview_silent:args.iter().any(|s|s=="--silent"||s=="--preview-silent"),
        online,drafts:BTreeMap::new(),input_page:String::new(),jobs:BTreeMap::new(),next_job:0,
        hwnd,inputs_hwnd,input_x:0,input_y:0,canvas_w:0,canvas_h:0,win_x:0,win_y:0,painter:None,frame_contour:None,presented_contour:None,store,settings,state,expanded:preview,settings_open:false,settings_cat:0,settings_scroll:[0.;7],settings_popup:None,tab:"Musik".into(),scale:1.,width:if preview{ui::PLAYER_WIDTH}else{220.},height:if preview{ui::PLAYER_HEIGHT}else{30.},hits:vec![],inputs:BTreeMap::new(),morph:None,pointer:None,hit_enabled:true,pressed:None,focused:None,tab_fade:None,painting:false,morph_timer:0,media:None,track:MediaSnapshot::default(),media_sessions:MediaSessions::default(),media_selection_revision:0,media_source_open:false,media_source_cursor:0,visualizer:None,levels:vec![0.;24],lyrics:vec![],plain_lyrics:String::new(),lyrics_key:String::new(),lyric_offset:0,lyrics_error:None,lyrics_retry:Default::default(),lyrics_from_youtube:false,focus:FocusClock::default(),breathing:None,last_water:Instant::now(),last_move:Instant::now(),last_eye:Instant::now(),toast:None,toast_from_stage0:false,last_clipboard_seq:0,selected_clip:0,clip_offset:0,queue,fullscreen:false,fullscreen_hwnd:None,year:now.year(),month:now.month(),day:now.day(),system_date:now.date_naive(),holidays:vec![],agenda:vec![],weather:None,ai_messages:vec![],busy:false,todo_offset:0,prayer_schedule:None,prayer_attempt:None,prayer_error:None,reminder_menu:None,reminder_menu_cursor:0,reminder_menu_scroll:0,reminder_index:1,reminder_anim:None,stretch_deadline:None,toast_queue:Default::default(),last_schedule:String::new(),weather_updated:None,weather_fetched_time:None,agenda_updated:None,preview,lyric_mode:false,hover_stage:boot_start_stage(preview),pinned:false,screenshot_stage2_freeze:None,screenshot_hide_until:None,hover_blocked_until:Instant::now(),startup_alpha:if preview{1.}else{0.},boot_drop,activity_sweep:None,activity_sweep_live:false,activity_sweep_paint:Instant::now(),last_lyric_line:String::new(),lyric_anim:None,sheet_index:0,sheet_slide:None,tab_drag:None,stage1_temporary_until:None,stage1_media_until:None,volume_hud:None,battery:crate::system::get_battery_status(),last_battery_check:Instant::now(),tws:None,cached_idle_clock_width:150.,cached_lyric_geometry:(270.,30.,15.),hardware_sampler:crate::system::HardwareSampler::new(),hardware_sample:None,last_hardware_tick:Instant::now(),cached_hardware_width:330.,last_tws_check:Instant::now(),tws_pending:false,tws_ready:false,last_ac_status:crate::system::get_battery_status().map(|b|b.is_charging),clip_filter:String::new(),clip_search_open:false,update_status:UpdateStatus::Idle,weather_custom_open:false,weather_menu_open:false,weather_refresh:crate::weather::RefreshState::default(),last_known_volume:crate::system::get_master_volume(),last_mod_button_clicked:None,mod_output:None,mod_view:None,mods_catalog_cache:None,mods_selected:None,mods_detail_open:false,mods_detail_scroll:0.,mods_scroll:0,mods_kind:crate::mods::package::PackageKind::Mod,mods_filter:crate::mods::catalog::CatalogFilter::All,mods_cache:Some(available_mods),mod_http_cache:BTreeMap::new(),mod_http_ttl:Duration::from_secs(60),
privacy_mailbox_tx,privacy_mailbox_rx,privacy_snapshot,privacy_monitor,quick_controls:crate::quick_controls::QuickControlsController::new_windows(),quick_controls_open:false,privacy_detail_open:false,settings_subpage_privacy:false,storage_dropdown_open:false,note_export_open:None,storage_dropdown_page:0,track_is_youtube:false,pending_ack_tx:None,pending_ack_ready_at:None,provider_downloading:false,provider_download_progress:0,confirm_provider_download:false,palette,glass:material::GlassRt::new(args),};

    app.tab=app.settings.default_tab.clone();
    if !app.settings.active_tab_order().contains(&app.tab){app.tab=app.settings.active_tab_order().first().cloned().unwrap_or_default();}
    app.load_holidays();
    let init_pw=(ui::CANVAS_WIDTH*app.scale).max(1.)as u32;
    let init_ph=(ui::CANVAS_HEIGHT*app.scale).max(1.)as u32;
    app.painter=Painter::new(hwnd,init_pw,init_ph,96.*app.scale,app.settings.software_renderer).ok();
    if app.painter.is_none(){return Err("Renderer tidak tersedia. Perbarui driver grafis atau jalankan dengan --software-renderer.".into())}
    app.glass.warp=app.painter.as_ref().is_some_and(|p|p.is_warp);
    if preview{
        // QA hooks (preview instances only): pin material/colour/stage so screenshots are reproducible.
        if let Some(v)=args.iter().find_map(|a|a.strip_prefix("--qa-material=")){app.settings.theme_material=glass::MaterialMode::from_str(v).as_str().into();}
        if let Some(v)=args.iter().find_map(|a|a.strip_prefix("--qa-color=")){app.settings.theme_color=if glass::is_light_mode(v){"light".into()}else{"dark".into()};}
        if let Some(v)=args.iter().find_map(|a|a.strip_prefix("--qa-gap=")).and_then(|v|v.parse::<i32>().ok()){app.settings.edge_gap=v.clamp(0,160);}
        if let Some(v)=args.iter().find_map(|a|a.strip_prefix("--qa-tab=")){if app.settings.enabled(v){app.tab=v.into();}}
        if args.iter().any(|a|a=="--qa-settings"){app.settings_open=true;app.settings_cat=1;}
        if let Some(stage)=args.iter().find_map(|a|a.strip_prefix("--qa-stage=")).and_then(|v|v.parse::<u8>().ok()){
            app.hover_stage=stage.min(2);app.expanded=stage>=2;
            let g=app.target_geometry();app.width=g.0;app.height=g.1;
        }
    }
    app.layout();
    app.sync_glass();
    app.update_window_region();    app.make_inputs();app.tray(true);
    // A corrupt settings/state file must not pass silently: the load already copied it
    // to a .recovery file and held its next save, and this notice tells the user which
    // files are held. Shell::save already surfaces the held-save error on every later
    // save attempt, so the hold cannot be missed either.
    let held = app.store.held_files();
    if !held.is_empty() {
        app.notice(format!("Data gagal dibaca ({}); data asli dipertahankan sebagai recovery dan penyimpanan ditahan sampai dipulihkan.", held.join(", ")));
    }
    crate::system::set_volume_osd_hidden(true);
    crate::clipboard::listen_clipboard(hwnd);
    if install_volume_hook(app.preview){
        HOOK_HWND.store(hwnd.0 as isize, Ordering::Relaxed);
        if let Ok(hook) = SetWindowsHookExW(WH_KEYBOARD_LL, Some(ll_keyboard_proc), None, 0) {
            HOOK_HANDLE.store(hook.0 as isize, Ordering::Relaxed);
        }
    }
    app.sync_clipboard();
    app.register_hotkey();
    app.start_media();

    if let Some(error) = clipboard_migration { app.notice(error); }
    app.clock_timer();
    if !app.preview && app.hover_stage == 1 && app.should_revert_stage1_to_calendar() {
        app.trigger_stage1_temporary(5);
    }
    app.poll_weather();
    if !app.preview&&!app.settings.effective_presentation().reduced_motion{SetTimer(Some(hwnd),11,16,None);}else{app.startup_alpha=1.;}
    // F1 boot: 2,5 dtk di Stage 0, lalu morph spring ke Stage 1 idle (timer 13).
    if !app.preview&&!app.settings.minimal{SetTimer(Some(hwnd),13,BOOT_STAGE1_DELAY_MS,None);}
    SetTimer(Some(hwnd),5,150,None);
    if args.iter().any(|s|s=="--suspend-fixture"){SetTimer(Some(hwnd),7,4000,None);SetTimer(Some(hwnd),8,7500,None);}
    if let Ok(exe) = std::env::current_exe() {
        if let Some(install_dir) = exe.parent() {
            if let Some(receipt) = crate::update::transaction::read_and_clear_receipt(install_dir) {
                let msg = format!("{} {}", crate::i18n::t("update.updated_to", &app.settings.language), receipt.version);
                app.show_toast(ToastKind::Generic, msg, 8000);
            }
        }
        if let Ok(root) = crate::app::storage_root(&exe, app.preview) {
            if let Some(pending) = crate::update::ui::PendingUpdate::load(&root) {
                app.update_status = UpdateStatus::AwaitingRestart { version: pending.version, tx_dir: pending.tx_dir };
            }
        }
    }
    for i in 0..args.len() {
        if args[i] == "--update-ack-tx" && i + 1 < args.len() {
            app.pending_ack_tx = Some(std::path::PathBuf::from(&args[i + 1]));
        }
    }
    APP.with(|cell|*cell.borrow_mut()=Some(app));
    let _=ShowWindow(hwnd,SW_SHOWNOACTIVATE);let _=InvalidateRect(Some(hwnd),None,false);
    let mut message=MSG::default();
    while GetMessageW(&mut message,None,0,0).as_bool(){
        if message.message == WM_KEYDOWN && message.wParam.0 == 0x41 {
            let ctrl = (GetKeyState(0x11) as u16 & 0x8000) != 0;
            if ctrl && is_edit_control(message.hwnd) {
                let _ = SendMessageW(message.hwnd, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
                continue;
            }
        }
        let _=TranslateMessage(&message);
        DispatchMessageW(&message);
    }
    APP.with(|cell|{cell.borrow_mut().take();});CoUninitialize();Ok(())
}}

fn is_edit_control(hwnd: HWND) -> bool {
    unsafe {
        let mut buf = [0u16; 16];
        let len = GetClassNameW(hwnd, &mut buf);
        if len > 0 {
            let s = String::from_utf16_lossy(&buf[..len as usize]);
            s.eq_ignore_ascii_case("EDIT")
        } else {
            false
        }
    }
}

unsafe extern "system" fn wndproc(hwnd:HWND,msg:u32,wparam:WPARAM,lparam:LPARAM)->LRESULT{
    // DestroyWindow dispatches WM_DESTROY reentrantly while the command handler
    // already borrows APP. Quit must not depend on obtaining that borrow again.
    if msg==WM_DESTROY{crate::system::set_volume_osd_hidden(false);unsafe{PostQuitMessage(0);}return LRESULT(0)}
    APP.with(|cell|{if let Ok(mut app)=cell.try_borrow_mut(){if let Some(app)=app.as_mut(){return app.handle(msg,wparam,lparam);}}unsafe{DefWindowProcW(hwnd,msg,wparam,lparam)}})
}
/// The input host only exists to carry the EDIT controls. It has no behaviour of its own:
/// its region makes everything outside the inputs inert, and its own background is never
/// painted, so the shell behind it stays fully visible and clickable. The one thing it does
/// own is the edit-control colours, because WM_CTLCOLOREDIT goes to the control's parent —
/// which is this window, not the composition shell.
unsafe extern "system" fn inputs_wndproc(hwnd:HWND,msg:u32,wparam:WPARAM,lparam:LPARAM)->LRESULT{
    unsafe{
        match msg{
            WM_CTLCOLOREDIT=>{
                let dc=HDC(wparam.0 as *mut _);
                let edit_hwnd=HWND(lparam.0 as *mut _);
                let ctrl_id=GetDlgCtrlID(edit_hwnd);
                if ctrl_id==60{
                    SetTextColor(dc,COLORREF(0x00ffffff));
                    SetBkColor(dc,COLORREF(0x00000000));
                    SetDCBrushColor(dc,COLORREF(0x00000000));
                }else{
                    SetTextColor(dc,COLORREF(0x00f5f1e9));
                    SetBkColor(dc,COLORREF(0x002d2319));
                    SetDCBrushColor(dc,COLORREF(0x002d2319));
                }
                return LRESULT(GetStockObject(DC_BRUSH).0 as isize)
            },

            WM_ERASEBKGND=>LRESULT(1),
            WM_COMMAND=>{
                let id=(wparam.0&0xffff)as u16;
                let code=(wparam.0>>16)as u16;
                if code==0x0300&&matches!(id,50|51){
                    APP.with(|cell|{if let Ok(mut app)=cell.try_borrow_mut(){if let Some(app)=app.as_mut(){if let Some(lease)=app.editors.panel{let _=app.editors.source.borrow_mut().mark_changed(lease);app.editors.unsaved=true;}}}});return LRESULT(0);
                }
                if code==0x0300&&id==60{
                    APP.with(|cell|{
                        if let Ok(mut app)=cell.try_borrow_mut(){
                            if let Some(app)=app.as_mut(){
                                app.clip_filter=app.input(60);
                                app.clip_offset=0;
                                app.invalidate();
                            }
                        }
                    });
                    return LRESULT(0);
                }
                DefWindowProcW(hwnd,msg,wparam,lparam)
            },
            _=>DefWindowProcW(hwnd,msg,wparam,lparam),
        }
    }
}

/// timeSetEvent invokes this on its own worker thread. It must not touch any UI state — it only
/// nudges the message pump, which then runs the morph on the UI thread where painting is safe.
unsafe extern "system" fn morph_tick(_id:u32,_msg:u32,user:usize,_dw1:usize,_dw2:usize){
    unsafe{let _=PostMessageW(Some(HWND(user as *mut _)),MORPH_MSG,WPARAM(0),LPARAM(0));}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CollapsedRoute {
    Toast,
    LyricStretch,
    NowPlaying,
    HardwareMonitor,
    IdleClock,
    Activity,
    Empty,
}

#[cfg(test)]
pub fn resolve_collapsed_route(
    hover_stage: u8,
    expanded: bool,
    toast: Option<&ToastNotification>,
    quiet_mode: bool,
    snapshot: Option<&crate::activity::ActivitySnapshot>,
    lyric_active: bool,
    hw_monitor_active: bool,
) -> CollapsedRoute {
    resolve_collapsed_route_ex(hover_stage, expanded, toast, quiet_mode, snapshot, lyric_active, false, hw_monitor_active)
}

pub fn resolve_collapsed_route_ex(
    hover_stage: u8,
    expanded: bool,
    toast: Option<&ToastNotification>,
    quiet_mode: bool,
    snapshot: Option<&crate::activity::ActivitySnapshot>,
    lyric_active: bool,
    now_playing: bool,
    hw_monitor_active: bool,
) -> CollapsedRoute {
    let visible_toast = toast.filter(|_| !quiet_mode);
    if visible_toast.is_some() {
        CollapsedRoute::Toast
    } else if hover_stage == 1 && !expanded {
        let is_higher_priority = snapshot.map(|s| s.primary.priority < 4).unwrap_or(false);
        if is_higher_priority {
            CollapsedRoute::Activity
        } else if lyric_active && !quiet_mode {
            CollapsedRoute::LyricStretch
        } else if now_playing && !quiet_mode {
            CollapsedRoute::NowPlaying
        } else if hw_monitor_active && !quiet_mode {
            CollapsedRoute::HardwareMonitor
        } else {
            // A playing media session is surfaced only through the short NowPlaying
            // transient (and, in lyric mode, the lyric stretch). It must not hold
            // Stage 1 hostage as a persistent activity: with no active transient the
            // pill returns to the idle clock (calendar).
            let show_idle = snapshot
                .map(|s| matches!(s.primary.id, crate::activity::ActivityId::Clock | crate::activity::ActivityId::Media(_)))
                .unwrap_or(false);
            if show_idle {
                CollapsedRoute::IdleClock
            } else {
                CollapsedRoute::Activity
            }
        }
    } else if hover_stage > 0 {
        CollapsedRoute::Activity
    } else {
        CollapsedRoute::Empty
    }
}

pub trait CollapsedSink {
    fn push_alpha(&mut self, alpha: f32);
    fn pop_alpha(&mut self);
    fn on_toast(&mut self, toast: &ToastNotification);
    fn on_lyric_stretch(&mut self);
    fn on_now_playing(&mut self);
    fn on_hardware_monitor(&mut self);
    fn on_idle_clock(&mut self);
    fn on_activity(&mut self);
    fn on_empty(&mut self);
}

pub fn dispatch_collapsed<S: CollapsedSink>(
    sink: &mut S,
    alpha: f32,
    hover_stage: u8,
    expanded: bool,
    toast: Option<&ToastNotification>,
    quiet_mode: bool,
    snapshot: Option<&crate::activity::ActivitySnapshot>,
    lyric_active: bool,
    now_playing: bool,
    hw_monitor_active: bool,
) -> CollapsedRoute {
    sink.push_alpha(alpha);
    let route = resolve_collapsed_route_ex(hover_stage, expanded, toast, quiet_mode, snapshot, lyric_active, now_playing, hw_monitor_active);
    match route {
        CollapsedRoute::Toast => {
            if let Some(t) = toast.filter(|_| !quiet_mode) {
                sink.on_toast(t);
            }
        }
        CollapsedRoute::LyricStretch => {
            sink.on_lyric_stretch();
        }
        CollapsedRoute::NowPlaying => {
            sink.on_now_playing();
        }
        CollapsedRoute::HardwareMonitor => {
            sink.on_hardware_monitor();
        }
        CollapsedRoute::IdleClock => {
            sink.on_idle_clock();
        }
        CollapsedRoute::Activity => {
            sink.on_activity();
        }
        CollapsedRoute::Empty => {
            sink.on_empty();
        }
    }
    sink.pop_alpha();
    route
}

#[derive(Default, Debug, Clone)]
#[allow(dead_code)]
pub struct RecordingCollapsedSink {
    pub routes: Vec<CollapsedRoute>,
    pub alpha_depth: i32,
    pub max_alpha_depth: i32,
    pub toast_kinds: Vec<ToastKind>,
    pub lyric_stretch_calls: usize,
    pub now_playing_calls: usize,
    pub hardware_monitor_calls: usize,
    pub idle_clock_calls: usize,
    pub activity_calls: usize,
    pub empty_calls: usize,
}

impl CollapsedSink for RecordingCollapsedSink {
    fn on_lyric_stretch(&mut self) {
        self.routes.push(CollapsedRoute::LyricStretch);
        self.lyric_stretch_calls += 1;
    }
    fn on_now_playing(&mut self) {
        self.routes.push(CollapsedRoute::NowPlaying);
        self.now_playing_calls += 1;
    }
    fn on_hardware_monitor(&mut self) {
        self.routes.push(CollapsedRoute::HardwareMonitor);
        self.hardware_monitor_calls += 1;
    }
    fn push_alpha(&mut self, _alpha: f32) {
        self.alpha_depth += 1;
        if self.alpha_depth > self.max_alpha_depth {
            self.max_alpha_depth = self.alpha_depth;
        }
    }
    fn pop_alpha(&mut self) {
        self.alpha_depth -= 1;
    }
    fn on_toast(&mut self, toast: &ToastNotification) {
        self.routes.push(CollapsedRoute::Toast);
        self.toast_kinds.push(toast.kind.clone());
    }
    fn on_idle_clock(&mut self) {
        self.routes.push(CollapsedRoute::IdleClock);
        self.idle_clock_calls += 1;
    }
    fn on_activity(&mut self) {
        self.routes.push(CollapsedRoute::Activity);
        self.activity_calls += 1;
    }
    fn on_empty(&mut self) {
        self.routes.push(CollapsedRoute::Empty);
        self.empty_calls += 1;
    }
}

struct NativeCollapsedSink<'a, 'b> {
    shell: &'a mut Shell,
    p: &'b mut Painter,
}

impl<'a, 'b> CollapsedSink for NativeCollapsedSink<'a, 'b> {
    fn push_alpha(&mut self, alpha: f32) {
        unsafe { self.p.push_alpha(alpha); }
    }
    fn pop_alpha(&mut self) {
        unsafe { self.p.pop_alpha(); }
    }
    fn on_toast(&mut self, toast: &ToastNotification) {
        unsafe {
            self.p.clip(0., 0., self.shell.width, self.shell.height);
            self.shell.draw_toast_content(self.p, toast);
            self.p.unclip();
        }
    }
    fn on_lyric_stretch(&mut self) {
        self.shell.draw_lyric_stretch(self.p);
    }
    fn on_now_playing(&mut self) {
        self.shell.draw_now_playing(self.p);
    }
    fn on_hardware_monitor(&mut self) {
        self.shell.draw_hardware_monitor(self.p);
    }
    fn on_idle_clock(&mut self) {
        self.shell.draw_idle_clock(self.p);
    }
    fn on_activity(&mut self) {
        self.shell.draw_activity(self.p, 0., false);
    }
    fn on_empty(&mut self) {}
}

impl Shell {
    fn power_state(&mut self,suspended:bool){
        if suspended==self.suspended{return}self.activities.runtime.pause_presentation();self.suspended=suspended;
        if suspended{
            self.cancel_jobs();
            unsafe{
                for id in [1,2,3,4,5,6,10,11,12]{let _=KillTimer(Some(self.hwnd),id);}
                if self.morph_timer!=0{let _=timeKillEvent(self.morph_timer);self.morph_timer=0;}
            }
            self.morph=None;self.tab_fade=None;self.activity_sweep=None;self.activity_sweep_live=false;

            if let Some(mut media)=self.media.take(){media.stop();}
            if let Some(mut capture)=self.visualizer.take(){capture.stop();}
            if let Some(mut p_mon)=self.privacy_monitor.take(){p_mon.stop();}
            self.track.playing=false;self.levels.fill(0.);self.save();
            crate::system::trim_working_set();
        }else{
            self.lyrics_key.clear();
            self.start_media();

            if self.settings.privacy_enabled && self.privacy_monitor.is_none() && !self.preview {
                self.privacy_monitor = Some(crate::privacy::PrivacyMonitorHandle::start(self.privacy_mailbox_tx.clone()));
            }
            self.clock_timer();
            unsafe{SetTimer(Some(self.hwnd),5,150,None);}
            self.tick();
        }
        // A morph that was cut short by suspend must not leave the window at an in-between size.
        self.layout();
        if self.preview{eprintln!("suspended={} capture={}",self.suspended,self.visualizer.is_some());}
    }

    fn job(&mut self,module:&'static str,work:impl FnOnce()->Event+Send+'static)->bool{
        if self.fullscreen||self.suspended{return false;}self.cancel_job(module);self.next_job+=1;let id=self.next_job;
        let token=crate::net::Cancellation::default();self.jobs.insert(module,(id,token.clone()));self.busy=self.jobs.keys().any(|m|matches!(*m,"AI"|"Cuaca"|"Kalender"));
        let post=self.poster();thread::spawn(move||{let result=crate::net::with_cancel(token.clone(),work);if !token.is_cancelled(){post.send(Event::Completed(module,id,Box::new(result)));}});
        true
    }
    fn cancel_job(&mut self,module:&str){if let Some((_,token))=self.jobs.remove(module){token.cancel();}self.busy=self.jobs.keys().any(|m|matches!(*m,"AI"|"Cuaca"|"Kalender"));}
    fn cancel_jobs(&mut self){self.weather_refresh.cancel_pending(self.jobs.contains_key("Cuaca"));for(_,(_,token))in std::mem::take(&mut self.jobs){token.cancel();}self.busy=false;}

    fn poster(&self)->Poster{Poster{queue:self.queue.clone(),hwnd:self.hwnd.0 as usize}}
    fn relay<T:Send+'static>(&self,receiver:mpsc::Receiver<T>,map:fn(T)->Event){let post=self.poster();thread::spawn(move||while let Ok(event)=receiver.recv(){post.send(map(event));});}
    fn maybe_write_health_ack(&mut self){
        if self.pending_ack_tx.is_none()||!self.render_ready{return;}
        let now=Instant::now();
        let ready_at=*self.pending_ack_ready_at.get_or_insert_with(||now+HEALTH_ACK_STABILITY);
        if now<ready_at{return;}
        if let Some(tx_dir)=self.pending_ack_tx.take(){
            if let Err(error)=crate::update::helper::write_health_ack(&tx_dir){eprintln!("Gagal menulis health ACK: {error}");}
        }
        self.pending_ack_ready_at=None;
    }
    fn save(&mut self){
        if !self.commit_editors(){self.editors.unsaved=true;return;}
        crate::clipboard::expire_history(&mut self.state.clipboard_history,crate::clipboard::now_unix());
        if !self.power_energy.days.is_empty(){if let Err(e)=self.power_energy.flush(&self.store.root){self.notice(e);}}
        let settings_saved=self.store.save("settings.json",&self.settings).is_ok();
        let state_saved=self.store.save("state.json",&self.state).is_ok();
        self.preset_host.settings_unsaved=!settings_saved;self.editors.unsaved=!state_saved;
        if self.editors.unsaved||self.preset_host.settings_unsaved{self.notice("Belum tersimpan — coba simpan lagi");}
    }
    fn notice(&mut self,text:impl Into<String>){
        let msg=text.into();
        let kind = classify_notice_kind(&msg);
        self.show_toast(kind, msg, 8000);
    }
    fn show_toast(&mut self, kind: ToastKind, text: impl Into<String>, timeout_ms: u32){
        let msg=text.into();
        if self.settings.quiet_mode{return;}
        if kind!=ToastKind::UpdateReady{self.activity_feedback(msg,timeout_ms);return;}
        if duplicate_toast(self.toast.as_ref(),&self.toast_queue,kind,&msg,Instant::now()){return}
        if self.toast.as_ref().is_some_and(|t|t.expires_at>Instant::now()){
            if self.toast_queue.len()<16{self.toast_queue.push_back((kind,msg,timeout_ms));}return
        }
        let from_stage0 = self.hover_stage == 0;
        self.toast_from_stage0 = from_stage0;
        self.toast=Some(ToastNotification {
            kind,
            message: msg,
            expires_at: Instant::now() + Duration::from_millis(timeout_ms as u64),
            confirmed: false,
            confirmed_at: None,
        });
        if (kind!=ToastKind::Prayer||self.settings.prayer_sound) && !(self.fullscreen && self.settings.hide_fullscreen){unsafe{let _=MessageBeep(0x00000040);}}
        if from_stage0 {
            self.set_hover_stage(1);
        } else {
            self.retarget_morph();
        }
        unsafe{SetTimer(Some(self.hwnd),3,timeout_ms,None);}
        self.clock_timer();
        self.layout();
        self.invalidate();
    }
    fn expire_toast(&mut self){
        let now=Instant::now();
        // A delayed WM_TIMER from a previous notice must not expire a newer one.
        if let Some(t)=self.toast.as_ref().filter(|t|t.expires_at>now){unsafe{SetTimer(Some(self.hwnd),3,t.expires_at.duration_since(now).as_millis().clamp(1,u32::MAX as u128)as u32,None);}return}
        unsafe{let _=KillTimer(Some(self.hwnd),3);}
        let from0=self.toast_from_stage0;self.toast=None;
        if let Some((kind,msg,timeout))=self.toast_queue.pop_front(){self.show_toast(kind,msg,timeout);self.toast_from_stage0=from0;return}
        self.toast_from_stage0=false;
        if from0&&self.hover_stage==1{self.set_hover_stage(0);}else{self.retarget_morph();self.layout();self.invalidate();}
        self.clock_timer();
    }
    fn sync_clipboard(&mut self){
        if self.preview_silent||!self.settings.enabled("Clipboard"){return}
        self.last_clipboard_seq = crate::clipboard::get_sequence_number();
        if let Some(text) = crate::clipboard::read_clipboard_text() {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                if self.state.clipboard_history.first().map(|c| c.text.as_str()) != Some(&text) {
                    self.state.clipboard_history.retain(|c| c.text != text);
                    let now_str = Local::now().format("%H:%M").to_string();
                    self.state.clipboard_history.insert(0, crate::clipboard::ClipItem::captured(
                        Local::now().timestamp_millis() as u64, text, now_str, crate::clipboard::now_unix(),
                    ));
                    crate::clipboard::expire_history(&mut self.state.clipboard_history, crate::clipboard::now_unix());
                    crate::clipboard::trim_history(&mut self.state.clipboard_history, 50);
                    let _ = self.store.save("state.json", &self.state);
                    if self.hover_stage == 1 && self.tab == "Clipboard" {
                        self.trigger_stage1_temporary(5);
                        self.retarget_morph();
                    }
                    self.invalidate();
                }
            }
        }
    }
    fn invalidate(&mut self){if self.suspended{return}self.paint();}
    /// Alt/Ctrl + key toggle registered from settings. Re-registering is safe: the old
    /// registration for id 1 is released first. A failed registration is reported, not swallowed.
    fn register_hotkey(&mut self){unsafe{
        let _=UnregisterHotKey(Some(self.hwnd),1);
        let mut modifiers=MOD_NOREPEAT;
        if self.settings.hotkey_alt{modifiers|=MOD_ALT;}
        if self.settings.hotkey_ctrl{modifiers|=MOD_CONTROL;}
        let key=self.settings.hotkey_key.clamp(1,255);
        if RegisterHotKey(Some(self.hwnd),1,modifiers,key).is_err(){
            let label=if self.settings.hotkey_ctrl&&self.settings.hotkey_alt{"Ctrl+Alt"}else if self.settings.hotkey_ctrl{"Ctrl"}else if self.settings.hotkey_alt{"Alt"}else{""};
            let hint=crate::i18n::t("notice.hotkey_conflict", &self.settings.language);
            self.notice(format!("{hint} ({label}+{})",(key as u8 as char).to_ascii_uppercase()));
        }
    }}
    fn detect_fullscreen(&mut self)->bool{unsafe{
        if self.screenshot_stage2_freeze.is_some_and(|until| Instant::now() < until) {
            return false;
        }
        let my_monitor=MonitorFromWindow(self.hwnd,MONITOR_DEFAULTTONEAREST);
        let mut info=MONITORINFO{cbSize:std::mem::size_of::<MONITORINFO>()as u32,..Default::default()};
        if !GetMonitorInfoW(my_monitor,&mut info).as_bool(){
            self.fullscreen_hwnd=None;
            return false;
        }
        let raw_fg=GetForegroundWindow();
        if raw_fg==self.inputs_hwnd && self.expanded {
            return false;
        }
        let fg=if raw_fg.0.is_null()||raw_fg==self.hwnd||raw_fg==self.inputs_hwnd{
            match top_standard_window_on_monitor(self.hwnd,self.inputs_hwnd,None,my_monitor){
                Some(h)=>h,
                None=>return self.fullscreen && self.fullscreen_hwnd.is_some_and(|h|IsWindowVisible(h).as_bool()&&!IsIconic(h).as_bool()),
            }
        }else{
            raw_fg
        };
        if is_shell_or_desktop_window(fg)||IsIconic(fg).as_bool(){
            self.fullscreen_hwnd=None;
            return false;
        }
        if check_hwnd_fullscreen(fg,my_monitor,&info){
            self.fullscreen_hwnd=Some(fg);
            return true;
        }
        for flag in [GA_ROOT,GA_ROOTOWNER]{
            let anc=GetAncestor(fg,flag);
            if !anc.0.is_null()&&anc!=fg&&check_hwnd_fullscreen(anc,my_monitor,&info){
                self.fullscreen_hwnd=Some(anc);
                return true;
            }
        }
        if let Some(prev)=self.fullscreen_hwnd{
            if prev==fg {
                if check_hwnd_fullscreen(prev,my_monitor,&info){
                    return true;
                }else{
                    self.fullscreen_hwnd=None;
                }
            }
        }
        let mut fg_pid:u32=0;
        let _=GetWindowThreadProcessId(fg,Some(&mut fg_pid));
        if fg_pid!=0&&fg_pid!=std::process::id(){
            let mut cur=GetTopWindow(None).unwrap_or_default();
            for _ in 0..256{
                if cur.0.is_null(){break;}
                if cur!=fg&&cur!=self.hwnd&&cur!=self.inputs_hwnd&&IsWindowVisible(cur).as_bool(){
                    let mut pid:u32=0;
                    let _=GetWindowThreadProcessId(cur,Some(&mut pid));
                    if pid==fg_pid&&check_hwnd_fullscreen(cur,my_monitor,&info){
                        self.fullscreen_hwnd=Some(cur);
                        return true;
                    }
                }
                cur=GetWindow(cur,GW_HWNDNEXT).unwrap_or_default();
            }
        }
        let style=GetWindowLongW(fg,GWL_STYLE)as u32;
        let ex_style=GetWindowLongW(fg,GWL_EXSTYLE)as u32;
        let mut fg_rect=RECT::default();
        let _=GetWindowRect(fg,&mut fg_rect);
        let fg_w=fg_rect.right-fg_rect.left;
        let fg_h=fg_rect.bottom-fg_rect.top;
        let is_overlay=(ex_style&(WS_EX_TOOLWINDOW.0|WS_EX_TRANSPARENT.0|WS_EX_NOACTIVATE.0))!=0
            ||((style&WS_CAPTION.0)!=WS_CAPTION.0&&(fg_w<640||fg_h<480));
        if is_overlay{
            if let Some(behind)=top_standard_window_on_monitor(self.hwnd,self.inputs_hwnd,Some(fg),my_monitor){
                if check_hwnd_fullscreen(behind,my_monitor,&info){
                    self.fullscreen_hwnd=Some(behind);
                    return true;
                }
            }
        }
        if let Some(top_win)=top_standard_window_on_monitor(self.hwnd,self.inputs_hwnd,None,my_monitor){
            if check_hwnd_fullscreen(top_win,my_monitor,&info){
                self.fullscreen_hwnd=Some(top_win);
                return true;
            }
        }
        self.fullscreen_hwnd=None;
        false
    }}
    fn sync_foreground(&mut self){unsafe{
        let full=self.detect_fullscreen();
        let changed=full!=self.fullscreen;
        if changed{
            self.fullscreen=full;
            if full{
                self.cancel_jobs();
                self.morph=None;
                self.tab_fade=None;
                if self.morph_timer!=0{let _=timeKillEvent(self.morph_timer);self.morph_timer=0;}
                for id in [2,4,6,10,12]{let _=KillTimer(Some(self.hwnd),id);}
                self.pointer=None;
                self.pressed=None;
                self.tab_drag=None;
                self.volume_hud=None;
                if self.settings.hide_fullscreen{
                    let target_stage=collapse_target_stage(self.settings.minimal);
                    self.hover_stage=target_stage;
                    self.expanded=false;
                    self.settings_open=false;
                    self.settings_popup=None;
                    self.reminder_menu=None;self.reminder_menu_scroll=0;
                    self.focused=None;
                    self.clip_search_open=false;
                    self.clip_filter.clear();
                    self.weather_custom_open=false;
                    self.weather_menu_open=false;
                    self.mod_view=None;
                    let (w,h,_)=self.target_geometry();
                    self.width=w;
                    self.height=h;
                    self.make_inputs();
                }
            }else{
                self.lyrics_key.clear();
                self.poster().send(Event::Media(MediaEvent::Changed(self.track.clone())));
            }
            self.sync_visualizer();
            self.clock_timer();
            self.layout();
        }
        let privacy_fullscreen_override = self.settings.privacy_enabled
            && self.settings.privacy_show_fullscreen
            && self.privacy_snapshot.any_active();
        let fullscreen_hide = self.fullscreen && self.settings.hide_fullscreen && !privacy_fullscreen_override;
        let hidden = fullscreen_hide || self.screenshot_hide_until.is_some_and(|until| Instant::now() < until);
        if hidden{self.activities.runtime.pause_presentation();}
        if IsWindowVisible(self.hwnd).as_bool()==hidden{let _=ShowWindow(self.hwnd,if hidden{SW_HIDE}else{SW_SHOWNOACTIVATE});if !hidden{self.layout();self.invalidate();}self.sync_input_host();}
        if self.fullscreen && self.settings.hide_fullscreen {
            if let Some(tray) = FindWindowW(w!("Shell_TrayWnd"), None).ok() {
                if IsWindowVisible(tray).as_bool() {
                    let _ = ShowWindow(tray, SW_HIDE);
                }
            }
            if let Some(sec_tray) = FindWindowW(w!("Shell_SecondaryTrayWnd"), None).ok() {
                if IsWindowVisible(sec_tray).as_bool() {
                    let _ = ShowWindow(sec_tray, SW_HIDE);
                }
            }
            if changed {
                if let Some(fg) = self.fullscreen_hwnd {
                    let _ = SetWindowPos(fg, Some(HWND_TOP), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
                }
            }
        } else if changed && !self.fullscreen {
            if let Some(tray) = FindWindowW(w!("Shell_TrayWnd"), None).ok() {
                if !IsWindowVisible(tray).as_bool() {
                    let _ = ShowWindow(tray, SW_SHOW);
                }
            }
            if let Some(sec_tray) = FindWindowW(w!("Shell_SecondaryTrayWnd"), None).ok() {
                if !IsWindowVisible(sec_tray).as_bool() {
                    let _ = ShowWindow(sec_tray, SW_SHOW);
                }
            }
        }
        if changed&&self.preview{eprintln!("lifecycle fullscreen={} visible={} capture={} jobs={}",self.fullscreen,IsWindowVisible(self.hwnd).as_bool(),self.visualizer.is_some(),self.jobs.len());}
    }}
    fn start_media(&mut self){
        if self.suspended||!self.settings.enabled("Musik")||self.media.is_some(){return}
        self.media_selection_revision=0;self.media_sessions=MediaSessions::default();
        let(tx,rx)=mpsc::channel();match MediaService::start(tx){Ok(service)=>{self.media=Some(service);self.relay(rx,Event::Media);},Err(e)=>self.notice(format!("{} {e}",crate::i18n::t("notice.media_unavailable", &self.settings.language)))}
    }
    fn sync_visualizer(&mut self){
        let policy=WorkPolicy{enabled:self.settings.enabled("Musik")&&self.settings.effective_presentation().visualizer,visible:!self.fullscreen&&!(self.settings.minimal&&!self.expanded),fullscreen:self.fullscreen||self.suspended,playing:self.track.playing};
        if policy.visualize()&&!self.preview_silent{if self.visualizer.is_none(){let(tx,rx)=mpsc::channel();match Visualizer::start(tx){Ok(v)=>{self.visualizer=Some(v);self.relay(rx,Event::Levels);if self.preview{eprintln!("capture started playing={} fullscreen={}",self.track.playing,self.fullscreen);}},Err(e)=>{self.levels.fill(0.);self.notice(format!("{} {e}",crate::i18n::t("notice.visualizer_unavailable", &self.settings.language)));}}}}
        else if let Some(mut v)=self.visualizer.take(){v.stop();self.levels.fill(0.);if self.preview{eprintln!("capture stopped playing={} fullscreen={}",self.track.playing,self.fullscreen);}}
    }
    /// Stable geometry the shell is heading towards. The pill radius is half its height,
    /// which is what makes the compact state a capsule instead of a rounded rectangle.
    fn target_geometry(&self)->(f32,f32,f32){let(w,h,r)=self.base_geometry();(w,if self.expanded&&(self.toast.is_some()||self.has_activity_bar()){h+30.}else{h},r)}
    fn base_geometry(&self)->(f32,f32,f32){
        if self.hover_stage>=2||self.expanded{
            if self.quick_controls_open {
                (crate::quick_controls::QC_PANEL_WIDTH, crate::quick_controls::QC_PANEL_HEIGHT, 18.)
            } else if self.privacy_detail_open{
                (560.,330.,24.)
            }else if self.settings_open{
                (560.,360.,27.)
            }else if self.mod_view.is_some(){
                (if self.mod_view.as_deref()==Some("daya"){560.}else{520.},if self.mod_view.as_deref()==Some("daya"){360.}else{320.},24.)
            }else if self.tab=="Musik"{
                if self.lyric_mode{
                    // Mockup stage2-musik: the lyric sheet widens the panel to the full
                    // canvas (620), left column 40% detail + right 60% lyrics.
                    (620.,186.,28.)
                }else{
                    // Mockup stage2-musik: layout player identik di ketiga material
                    // (Solid/Glass Lite/Glass Adaptive) — panel 460x180 untuk semua.
                    (460.,180.,28.)
                }
            }else if self.tab=="Kalender"{
                (540.,300.,24.)
            }else if self.tab=="Reminder"{
                (550.,if self.reminder_index==3{362.}else{346.},27.)
            }else if self.tab=="Clipboard"{
                (530.,236.,24.)
            }else if self.tab=="Catatan"{
                if self.simpan_sub==1{(530.,280.,24.)}else{(540.,280.,24.)}
            }else if self.tab=="Cuaca"{
                (560.,if self.weather_custom_open{320.}else{240.},24.)
            }else if self.tab=="Daya" || self.tab=="Power"{
                (560.,360.,24.)
            }else if self.settings.mod_tabs.iter().any(|t|t==&self.tab){
                (520.,320.,24.)
            }else{
                (ui::EXPANDED_WIDTH,ui::EXPANDED_HEIGHT,24.)
            }
        }else if self.hover_stage == 1 || self.toast.is_some() {
            if self.should_show_weather_stage1() { return (300., 30., 15.); }
            let route = resolve_collapsed_route_ex(
                self.hover_stage,
                self.expanded,
                self.toast.as_ref(),
                self.settings.quiet_mode,
                self.activities.runtime.snapshot.as_ref(),
                self.is_lyric_active(),
                self.stage1_now_playing_visible(),
                self.settings.hw_monitor_enabled,
            );
            match route {
                CollapsedRoute::Toast => {
                    let toast = self.toast.as_ref().unwrap();
                    if toast.kind == ToastKind::Water {
                        if toast.confirmed { (280., 30., 15.) } else { (320., 30., 15.) }
                    } else {
                        let tw = (toast.message.chars().count() as f32 * 7.5 + 48.).clamp(160., 440.);
                        (tw, 30., 15.)
                    }
                }
                CollapsedRoute::LyricStretch => {
                    self.lyric_stretch_target_geometry()
                }
                CollapsedRoute::NowPlaying => {
                    self.now_playing_target_geometry()
                }
                CollapsedRoute::HardwareMonitor => {
                    (self.hardware_monitor_target_width(), 30., 15.)
                }
                CollapsedRoute::IdleClock => {
                    (self.idle_clock_target_width(), 30., 15.)
                }
                CollapsedRoute::Activity => {
                    (self.activity_target_width(), 30., 15.)
                }
                CollapsedRoute::Empty => {
                    if self.settings.privacy_enabled && self.privacy_snapshot.any_active() {
                        let geom = crate::shell::privacy::compute_stage0_geometry(&self.privacy_snapshot);
                        (geom.width, geom.height, geom.radius)
                    } else {
                        (50., 10., 5.)
                    }
                }
            }
        }else{
            if self.settings.privacy_enabled && self.privacy_snapshot.any_active() {
                let geom = crate::shell::privacy::compute_stage0_geometry(&self.privacy_snapshot);
                (geom.width, geom.height, geom.radius)
            } else {
                (50.,10.,5.)
            }
        }
    }
    fn target_ear(&self)->f32{
        if !self.settings.bottom && self.settings.edge_gap == 0 {
            if self.hover_stage == 0 && !self.expanded {
                if self.settings.privacy_enabled && self.privacy_snapshot.any_active() {
                    8.
                } else {
                    6.
                }
            } else if self.target_geometry().1 >= 24. {
                12.
            } else {
                (self.target_geometry().1 * 0.5).max(4.)
            }
        } else {
            0.
        }
    }
    pub fn current_lyric_line(&self) -> Option<String> {
        if !self.track.playing || self.settings.quiet_mode {
            return None;
        }
        if let Some(l) = self.active_lyric() {
            if !l.trim().is_empty() {
                return Some(l);
            }
        }
        if !self.last_lyric_line.trim().is_empty() {
            return Some(self.last_lyric_line.clone());
        }
        if !self.lyrics.is_empty() {
            if let Some(first) = self.lyrics.first() {
                return Some(first.text.clone());
            }
        }
        if !self.plain_lyrics.trim().is_empty() {
            return Some(self.plain_lyrics.clone());
        }
        None
    }

    pub fn is_lyric_active(&self) -> bool {
        self.lyric_mode && self.current_lyric_line().is_some()
    }
    /// True while the short now-playing transient (a few seconds after a track
    /// change) should replace the idle clock. Lyrics in Stage 1 are only shown
    /// when the user explicitly enabled lyric mode from Stage 2.
    pub fn stage1_now_playing_visible(&self) -> bool {
        !self.lyric_mode
            && self.settings.enabled("Musik")
            && self.track.playing
            && self.stage1_media_until.is_some_and(|until| Instant::now() < until)
            && !self.track.title.trim().is_empty()
    }
    fn stage1_now_playing(&self) -> Option<(String, Option<String>)> {
        if !self.stage1_now_playing_visible() { return None; }
        let artist = self.track.artist.trim();
        Some((self.track.title.clone(), if artist.is_empty() { None } else { Some(self.track.artist.clone()) }))
    }

    pub fn lyric_stretch_target_geometry(&self) -> (f32, f32, f32) {
        self.cached_lyric_geometry
    }

    pub fn refresh_lyric_stretch_metrics(&mut self, p: &mut Painter) {
        if let Some(text) = self.current_lyric_line() {
            let wrapped = unsafe {
                ui::wrap_lyric_lines(&text, 346.0, |s| p.measure_text(s, 12.0, true))
            };
            let radius = if wrapped.is_multiline { 16.0 } else { 15.0 };
            self.cached_lyric_geometry = (wrapped.target_width, wrapped.target_height, radius);
        } else {
            self.cached_lyric_geometry = (ui::LYRIC_MIN_WIDTH, ui::LYRIC_BASELINE_HEIGHT, 15.0);
        }
    }

    /// Single-line Stage 1 status for the now-playing transient: app icon on the
    /// left, "Title · Artist" info, the clock footer, and the dropdown chevron.
    pub fn now_playing_target_geometry(&self) -> (f32, f32, f32) {
        let len = match self.stage1_now_playing() {
            Some((title, artist)) => title.chars().count() + artist.as_ref().map(|a| a.chars().count() + 3).unwrap_or(0),
            None => 0,
        };
        let width = (len as f32 * 7.2 + 110.).clamp(190., 360.);
        (width, 30., 15.)
    }

    fn retarget_morph(&mut self) {
        let target = self.target_geometry();
        if (self.width - target.0).abs() > 4.0 || (self.height - target.1).abs() > 2.0 {
            self.retarget_to(target);
        }
    }
    fn retarget_to(&mut self, target: (f32, f32, f32)) {
        let ear = self.target_ear();
        let reduced = self.settings.effective_presentation().reduced_motion;
        if let Some(morph) = &mut self.morph {
            morph.set_reduced_motion(reduced);
            morph.retarget(target, ear);
        } else {
            let mut m = ui::Morph::from_presented(self.presented_contour,target,ear);
            m.set_reduced_motion(reduced);
            self.morph = Some(m);
        }
        self.update_window_region();
        self.start_morph_timer();
    }
    fn stage1_tab(&self)->&str{ui::stage1_tab(&self.tab,self.settings.enabled("Musik")&&self.stage1_media_until.is_some_and(|until|Instant::now()<until))}
    pub fn idle_clock_target_width(&self) -> f32 {
        if self.cached_idle_clock_width > 0. {
            self.cached_idle_clock_width
        } else {
            150.
        }
    }
    pub fn refresh_idle_clock_metrics(&mut self, p: &mut Painter) {
        let now = Local::now();
        let today = now.date_naive();
        let month_str = crate::i18n::month_name(today.month(), &self.settings.language);
        let date_text = format!("{}, {} {}", crate::i18n::weekday_short(today.weekday().num_days_from_monday(), &self.settings.language), today.day(), month_str);
        let time_str = now.format("%H:%M").to_string();
        let power = ui::PowerPresentation::from_battery(self.battery.as_ref());
        let tws = ui::TwsPresentation::from_tws(self.tws.as_ref());

        unsafe {
            let date_w = p.measure_text(&date_text, ui::STAGE1_TEXT_SIZE, true);
            // Slack 8 px: format baru dengan nama hari lebih lebar — tanpa slack, plan_idle_clock
            // memotong tanggal tepat di batas lebar (date ter-ellipsis "8 N...").
            let date_w = date_w + 8.0;
            let time_w = p.measure_text(&time_str, ui::STAGE1_TEXT_SIZE, true);
            let (power_glyph_w, power_percent_w) = match power {
                ui::PowerPresentation::Charging(pct) => {
                    let lbl = format!("{pct}%");
                    (ui::STAGE1_BOLT_W, p.measure_text(&lbl, ui::STAGE1_TEXT_SIZE, true))
                }
                ui::PowerPresentation::Low(pct) => {
                    let lbl = format!("{pct}%");
                    (p.measure_text("🪫", ui::STAGE1_TEXT_SIZE, false), p.measure_text(&lbl, ui::STAGE1_TEXT_SIZE, true))
                }
                ui::PowerPresentation::Normal(pct) => {
                    let lbl = format!("{pct}%");
                    (p.measure_text("🔋", ui::STAGE1_TEXT_SIZE, false), p.measure_text(&lbl, ui::STAGE1_TEXT_SIZE, true))
                }
                ui::PowerPresentation::Unavailable => (0., 0.),
            };
            let (tws_glyph_w, tws_label_w) = match tws {
                ui::TwsPresentation::Known(pct) => {
                    let lbl = format!("{pct}%");
                    (p.measure_text("🎧", ui::STAGE1_TEXT_SIZE, false), p.measure_text(&lbl, ui::STAGE1_TEXT_SIZE, true))
                }
                ui::TwsPresentation::Unknown => {
                    (p.measure_text("🎧", ui::STAGE1_TEXT_SIZE, false), p.measure_text("—", ui::STAGE1_TEXT_SIZE, true))
                }
                ui::TwsPresentation::Absent => (0., 0.),
            };
            let privacy_rail_w = if self.settings.privacy_enabled && self.privacy_snapshot.any_active() {
                crate::shell::privacy::compute_privacy_rail_width(&self.privacy_snapshot)
            } else {
                0.
            };
            let preset_active = self.settings.quiet_mode || self.settings.power_saver_mode;
            // +6 px slack: format tanggal baru (dengan nama hari) lebih lebar — tanpa slack,
            // plan_idle_clock memotongnya tepat di batas (date ter-ellipsis "8 N...").
            self.cached_idle_clock_width = ui::calculate_idle_clock_target_width(
                date_w,
                time_w,
                power,
                power_glyph_w,
                power_percent_w,
                tws,
                tws_glyph_w,
                tws_label_w,
                privacy_rail_w,
                preset_active,
            ) + 6.0;
        }
    }
    pub fn hardware_monitor_target_width(&self) -> f32 {
        if self.cached_hardware_width > 0. {
            self.cached_hardware_width
        } else {
            330.
        }
    }

    pub fn refresh_hardware_monitor_metrics(&mut self, p: &mut Painter) {
        let now = Local::now();
        let time_str = now.format("%H:%M").to_string();
        unsafe {
            let time_w = p.measure_text(&time_str, ui::STAGE1_TEXT_SIZE, true);
            let privacy_rail_w = if self.settings.privacy_enabled && self.privacy_snapshot.any_active() {
                crate::shell::privacy::compute_privacy_rail_width(&self.privacy_snapshot)
            } else {
                0.
            };
            let preset_active = self.settings.quiet_mode || self.settings.power_saver_mode;
            self.cached_hardware_width = ui::calculate_hardware_monitor_width(
                &self.settings.hw_metrics,
                self.hardware_sample.as_ref(),
                time_w,
                privacy_rail_w,
                preset_active,
                |text, size, bold| p.measure_text(text, size, bold),
            );
        }
    }

    fn draw_hardware_monitor(&mut self, p: &mut Painter) {
        unsafe {
            p.clip(0., 0., self.width, self.height);
            self.refresh_hardware_monitor_metrics(p);

            let now = Local::now();
            let time_str = now.format("%H:%M").to_string();
            let time_w = p.measure_text(&time_str, ui::STAGE1_TEXT_SIZE, true);

            let privacy_rail_w = if self.settings.privacy_enabled && self.privacy_snapshot.any_active() {
                crate::shell::privacy::compute_privacy_rail_width(&self.privacy_snapshot)
            } else {
                0.
            };
            let preset_active = self.settings.quiet_mode || self.settings.power_saver_mode;

            let plan = ui::plan_hardware_monitor(
                self.width,
                self.height,
                &self.settings.hw_metrics,
                self.hardware_sample.as_ref(),
                self.settings.hw_threshold_high,
                self.settings.hw_threshold_critical,
                time_w,
                privacy_rail_w,
                preset_active,
                |text, size, bold| p.measure_text(text, size, bold),
            );

            // 1. Slot Kiri: multi-level circular gauge (one coloured ring per metric).
            // Normal rings keep their metric colour; high/critical rings turn amber/red.
            for ring in &plan.rings {
                let pct = ring.ratio * 100.0;
                let col = if pct >= self.settings.hw_threshold_critical as f32 { 0xf87171 }
                    else if pct >= self.settings.hw_threshold_high as f32 { 0xf98a47 }
                    else { ring.color };
                p.ring_arc(plan.gauge_center.0, plan.gauge_center.1, ring.radius, ring.ratio, 0x2a2d35, col, 1.6);
            }

            // 2. Slot Tengah: Chip metrik + separator vertikal
            for sep in &plan.separators {
                p.fill_alpha(sep.0, sep.1, sep.2, sep.3, 0.4, 0x4a4f57, 1.0);
            }

            for chip in &plan.chips {
                // Label (abu redup #8b8f98)
                p.text_centered(chip.label, chip.label_rect.0, chip.label_rect.1, chip.label_rect.2, chip.label_rect.3, 11.5, 0x8b8f98, 1.0, false);
                // Value: metric colour normally, amber/red past the configured thresholds.
                let val_color = match chip.threshold {
                    ui::HwThresholdState::Normal => ui::hw_metric_color(chip.metric),
                    other => other.color_u32(),
                };
                p.text_centered(&chip.percent_text, chip.percent_rect.0, chip.percent_rect.1, chip.percent_rect.2, chip.percent_rect.3, 12.5, val_color, 1.0, true);
            }

            // 3. Slot Kanan: Jam HH:MM rata kanan + chevron di paling kanan (width - 24)
            self.button_chevron_up(p, plan.chevron_hit.0, plan.chevron_hit.1, plan.chevron_hit.2, plan.chevron_hit.3, Action::ToStage0, 0x88888e);
            p.text_centered(&time_str, plan.time_rect.0, plan.time_rect.1, plan.time_rect.2, plan.time_rect.3, ui::STAGE1_TEXT_SIZE, 0xffffff, 1.0, true);

            if let Some(rect) = plan.preset_slot {
                self.preset_indicator(p, rect.0, rect.1);
            }

            if let Some(rect) = plan.privacy_slot {
                self.draw_privacy_rail(p, rect.0, rect.1, rect.2, rect.3);
            }

            // Whole pill opens Stage 2 (hardware monitor → Daya page when available).
            if self.hit_enabled {
                let hit_w = (plan.chevron_hit.0 - 8.0).max(10.0);
                self.hits.push(Hit {
                    rect: render::rect(8.0, 0.0, hit_w, self.height),
                    action: Action::ExpandHardware,
                });
            }

            p.unclip();
        }
    }

    pub fn should_revert_stage1_to_calendar(&self) -> bool {
        should_revert_stage1(self.stage1_tab(), self.reminder_index, self.lyric_mode)
    }
    pub fn is_stage1_temporary_active(&self) -> bool {
        self.stage1_media_until.is_some_and(|until|Instant::now()<until)||self.stage1_temporary_until.is_some_and(|until| Instant::now() < until)
    }
    /// True during the short post-collapse weather preview window (3 s).
    fn weather_stage1_window(&self)->bool{ self.weather_stage1_until.is_some_and(|until|Instant::now()<until) }
    pub fn should_show_weather_stage1(&self)->bool{
        self.weather_stage1_window() && self.hover_stage==1 && !self.expanded && self.settings.enabled("Cuaca")
    }
    fn sync_stage1_timer(&self){unsafe{
        let now=Instant::now();
        let mut ms=ui::stage1_deadline_ms(self.stage1_media_until,self.stage1_temporary_until,now);
        if let Some(w)=self.weather_stage1_until.filter(|u|*u>now){
            let d=w.duration_since(now).as_millis().clamp(1,u32::MAX as u128) as u32;
            ms=Some(ms.map_or(d,|m|m.min(d)));
        }
        if let Some(ms)=ms{
            SetTimer(Some(self.hwnd),6,ms,None);
        }else{let _=KillTimer(Some(self.hwnd),6);}
    }}
    pub fn trigger_stage1_temporary(&mut self,duration_secs:u64){
        self.stage1_temporary_until=Some(Instant::now()+Duration::from_secs(duration_secs));
        self.sync_stage1_timer();
    }

    pub fn show_volume_hud(&mut self, vol: f32, muted: bool) {
        self.last_known_volume = Some((vol, muted));
        if self.fullscreen && self.settings.hide_fullscreen {
            return;
        }
        let is_new = self.volume_hud.is_none();
        self.volume_hud = Some((vol, muted, Instant::now()));
        self.trigger_stage1_temporary(2);
        if self.hover_stage == 0 {
            self.set_hover_stage(1);
        } else if is_new && self.hover_stage == 1 {
            self.retarget_morph();
        }
        self.reconcile_activities();self.invalidate();
    }
    fn set_hover_stage(&mut self,stage:u8){
        if stage<2&&self.hover_stage>=2&&!self.flush_editors_barrier(){return;}
        if stage==0{self.activities.runtime.pause_presentation();}
        if self.fullscreen{return}
        // Keep the short weather preview visible even if the cursor leaves during it.
        let stage = if stage==0 && self.weather_stage1_window() { 1 } else { stage };
        if self.hover_stage==stage{return}
        if stage==0{
            self.hover_blocked_until=Instant::now()+Duration::from_millis(600);
        }
        if stage == 1 && self.should_revert_stage1_to_calendar() {
            self.trigger_stage1_temporary(5);
        }
        self.hover_stage=stage;
        self.expanded=stage>=2;
        if stage<2 && self.tab=="Cuaca" && self.settings.enabled("Cuaca") {
            self.weather_stage1_until=Some(Instant::now()+Duration::from_secs(3));
            if self.hover_stage==0 { self.hover_stage=1; }
            self.sync_stage1_timer();
        }
        if stage<2{
            self.media_source_open=false;
            self.note_export_open=None;
            self.settings_open=false;
            self.settings_popup=None;
            self.reminder_menu=None;self.reminder_menu_scroll=0;
            self.focused=None;
            self.clip_search_open=false;
            self.clip_filter.clear();
            self.weather_custom_open=false;
            self.weather_menu_open=false;
            self.mod_view=None;
            let today = Local::now().date_naive();
            if self.year != today.year() || self.month != today.month() || self.day != today.day() {
                self.year = today.year();
                self.month = today.month();
                self.day = today.day();
                self.load_holidays();
            }
        }
        self.retarget_morph();
        if self.hover_stage<2{self.make_inputs();}
        self.poll_weather();self.sync_visualizer();self.clock_timer();self.invalidate();
    }
    fn geometry(&self)->(f32,f32,f32){if let Some(c)=self.frame_contour{return(c.width,c.height,c.radius)}match &self.morph{Some(morph)=>morph.sample(),None=>self.target_geometry()}}
    /// Offset vertikal animasi boot (F1); 0. di luar sekuens boot. Hanya untuk notch
    /// menempel tepi atas (bottom=false, edge_gap=0) — satu-satunya konfigurasi yang
    /// memotong mulus di tepi layar, sehingga "turun dari balik bezel" hanya di sana.
    fn boot_drop_offset(&self)->f32{
        if self.settings.bottom||self.settings.edge_gap!=0{return 0.}
        self.boot_drop.as_ref().map(|t|boot_drop_offset_at(t.elapsed())).unwrap_or(0.)
    }
    fn capsule_origin(&self)->(f32,f32){
        let(width,height,_)=self.geometry();
        let capsule_x=(ui::CANVAS_WIDTH-width)/2.;
        let capsule_y=ui::panel_origin_y(self.settings.bottom,height,self.scale);
        (capsule_x,capsule_y)
    }
    fn client_to_capsule(&self,lparam:LPARAM)->(f32,f32){
        let px=(lparam.0 as i16)as f32/self.scale;
        let py=((lparam.0>>16)as i16)as f32/self.scale;
        let(ox,oy)=self.capsule_origin();
        (px-ox,py-oy)
    }
    /// True while the physical cursor is still inside the notch contour. The EDIT host is a
    /// separate top-level window, so moving into it makes the shell receive WM_MOUSELEAVE even
    /// though the pointer never left the notch. Timer 2 uses this to avoid collapsing then.
    fn cursor_over_notch(&self)->bool{unsafe{
        let mut pt=POINT::default();if GetCursorPos(&mut pt).is_err(){return false;}
        let mut origin=POINT::default();let _=ClientToScreen(self.hwnd,&mut origin);
        let(ox,oy)=self.capsule_origin();let(width,height,_)=self.geometry();
        let left=origin.x+(ox*self.scale).round()as i32;
        let top=origin.y+(oy*self.scale).round()as i32;
        let right=left+(width*self.scale).ceil()as i32;
        let bottom=top+(height*self.scale).ceil()as i32;
        pt.x>=left&&pt.x<right&&pt.y>=top&&pt.y<bottom
    }}
    fn layout(&mut self){unsafe{
        let mut area=RECT::default();let _=SystemParametersInfoW(SPI_GETWORKAREA,0,Some((&mut area as *mut RECT).cast()),SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
        let dpi=GetDpiForWindow(self.hwnd).max(96)as f32/96.;self.scale=(dpi*self.settings.scale).min(((area.bottom-area.top-24)as f32/ui::EXPANDED_HEIGHT).max(0.6));
        let(width,height,_radius)=self.geometry();
        self.width=width;self.height=height;
        let monitor=MonitorFromWindow(self.hwnd,MONITOR_DEFAULTTONEAREST);
        let mut monitor_info=MONITORINFO{cbSize:std::mem::size_of::<MONITORINFO>()as u32,..Default::default()};
        let (work_area, monitor_top) = if GetMonitorInfoW(monitor,&mut monitor_info).as_bool(){(monitor_info.rcWork, monitor_info.rcMonitor.top)}else{(area, area.top)};
        let pixel_width=((ui::CANVAS_WIDTH)*self.scale).max(1.)as i32;
        let pixel_height=((ui::CANVAS_HEIGHT)*self.scale).max(1.)as i32;
        let x=work_area.left+(work_area.right-work_area.left-pixel_width)/2;let y=ui::window_top(self.settings.bottom,self.settings.edge_gap,self.scale,work_area.top,work_area.bottom,monitor_top);
        if pixel_width!=self.canvas_w||pixel_height!=self.canvas_h||x!=self.win_x||y!=self.win_y{
            if self.preview{eprintln!("layout work_top={} monitor_top={} scale={} window_top={} panel_top={}",area.top,monitor_top,self.scale,y,y as f32+ui::panel_origin_y(self.settings.bottom,height,self.scale)*self.scale);}
            self.canvas_w=pixel_width;self.canvas_h=pixel_height;self.win_x=x;self.win_y=y;
            self.input_x=x;self.input_y=y;
            let _=SetWindowPos(self.hwnd,Some(HWND_TOPMOST),x,y,pixel_width,pixel_height,SWP_NOACTIVATE);
            if let Some(p)=&mut self.painter{p.resize(pixel_width as u32,pixel_height as u32,96.*self.scale);}
        }
        self.sync_input_host();
        self.update_window_region();
        self.invalidate();
    }}
    fn update_window_region(&self){unsafe{
        if self.preview&&!std::env::args().any(|a|a=="--edge-check"){return}
        if self.glass_active(){self.apply_glass_region();return}
        let (width, height, ear) = match &self.morph {
            Some(morph) => {
                let from = morph.from();
                let to = morph.to();
                let cur = morph.sample();
                // Envelope window region: must include body width + 2 * ear_size + margin for overshoot so shape never clips.
                let max_w = from.0.max(to.0).max(cur.0) + 24.0;
                let max_h = from.1.max(to.1).max(cur.1) + 16.0;
                let e = if !self.settings.bottom && self.settings.edge_gap == 0 {
                    16.0
                } else {
                    0.0
                };
                (max_w, max_h, e)
            }
            None => {
                let g = self.geometry();
                let e = self.target_ear();
                (g.0, g.1, e)
            }
        };
        let shadow = if height >= 100. {
            14.
        } else {
            0.
        };
        let capsule_x = (ui::CANVAS_WIDTH - width) / 2.;
        let capsule_y=ui::panel_origin_y(self.settings.bottom,height,self.scale);
        let left = (((capsule_x - ear - shadow) * self.scale).floor() as i32).max(0);
        let top = if self.settings.bottom {
            (((capsule_y - shadow) * self.scale).floor() as i32).max(0)
        } else if self.settings.edge_gap == 0 {
            ui::attached_clip_top(self.scale)
        } else {
            (((capsule_y - shadow) * self.scale).floor() as i32).max(0)
        };
        let right = (((capsule_x + width + ear + shadow) * self.scale).ceil() as i32).min(self.canvas_w);
        let bottom = if self.settings.bottom {
            if self.settings.edge_gap == 0 {
                (((capsule_y + height) * self.scale).ceil() as i32).min(self.canvas_h)
            } else {
                (((capsule_y + height + shadow) * self.scale).ceil() as i32).min(self.canvas_h)
            }
        } else {
            (((capsule_y + height + shadow) * self.scale).ceil() as i32).min(self.canvas_h)
        };
        let rgn = CreateRectRgn(left, top, right, bottom);
        if !rgn.0.is_null() {
            let _ = SetWindowRgn(self.hwnd, Some(rgn), false);
        }
    }}
    /// Drives the morph at high refresh rate (up to 120/144 Hz via 8ms timer).
    /// F2: timer juga hidup selama sweep aksen Nyala berjalan, dan mati lagi begitu
    /// anchor dikosongkan — tidak ada render loop permanen.
    fn start_morph_timer(&mut self){unsafe{
        let needs_timer=self.morph.is_some()||self.lyric_anim.is_some()||self.tab_fade.is_some()||self.reminder_anim.is_some()||self.sheet_slide.is_some()||self.activity_sweep.is_some();
        if needs_timer{
            if self.morph_timer==0{self.morph_timer=timeSetEvent(ui::MORPH_TICK_MS,0,Some(morph_tick),self.hwnd.0 as usize,TIME_PERIODIC);}
        }else if self.morph_timer!=0{
            let _=timeKillEvent(self.morph_timer);self.morph_timer=0;
        }
    }}
    /// Advances a running transition smoothly without resizing swap chain buffers on every frame.
    fn morph_step(&mut self){
        if self.reminder_anim.as_ref().is_some_and(|(_,t)|t.elapsed()>=Duration::from_millis(180)){self.reminder_anim=None;}
        if self.morph.as_ref().is_some_and(ui::Morph::finished){
            self.morph=None;
            self.make_inputs();
            self.sync_input_host();
            self.update_window_region();
            if self.hover_stage == 0 {
                crate::system::trim_working_set();
            }
        }
        if self.tab_fade.as_ref().is_some_and(|(_,t,_)|t.elapsed()>=ui::TAB_FADE_DURATION){
            self.tab_fade=None;
        }
        if self.sheet_slide.as_ref().is_some_and(|(_,t)|t.elapsed()>=SHEET_SLIDE){
            self.sheet_slide=None;
        }
        if self.lyric_anim.as_ref().is_some_and(|a|a.started.elapsed()>=Duration::from_millis(280)){
            self.lyric_anim=None;
        }
        // F2 sweep: wrap anchor tiap periode 3 dtk agar t terus berputar, lalu
        // decimate repaint saat sweep adalah satu-satunya animasi (~40 fps);
        // animasi lain tetap memakai tick penuh 8 ms.
        if let Some((anchor,acc))=self.activity_sweep{
            if anchor.elapsed()>=Duration::from_millis(ui::ACCENT_SWEEP_PERIOD_MS){self.activity_sweep=Some((Instant::now(),acc));}
        }
        let sweep_only=self.activity_sweep.is_some()&&self.morph.is_none()&&self.lyric_anim.is_none()&&self.tab_fade.is_none()&&self.reminder_anim.is_none()&&self.sheet_slide.is_none();
        let sweep_due=!sweep_only||self.activity_sweep_paint.elapsed()>=Duration::from_millis(ui::ACCENT_SWEEP_FRAME_MS);
        if sweep_only{self.activity_sweep_paint=Instant::now();}
        self.start_morph_timer();
        let (width,height,_)=self.geometry();
        self.width=width;self.height=height;
        if sweep_due{self.invalidate();}
    }
    fn refresh_lyrics_consent(&mut self) {
        self.lyrics_consent.set(self.settings.lyrics_online,self.settings.lyrics_youtube_fallback);
        self.cancel_job("Musik");
        self.lyrics.clear();self.plain_lyrics.clear();self.lyrics_data=None;self.lyrics_key.clear();
        self.lyrics_error=None;self.lyrics_retry=Default::default();self.lyrics_from_youtube=false;
        self.last_lyric_line.clear();self.lyric_anim=None;
        self.request_lyrics(false);self.layout();
    }
    fn apply_lyrics(&mut self,value:LyricsResult) {
        self.lyrics_data=Some(value.clone());
            let app_l=self.track.source_app.to_lowercase();
            if value.is_youtube&&!app_l.contains("spotify")&&!app_l.contains("vlc"){self.track_is_youtube=true;}self.lyrics=crate::lyrics::parse_lrc_with_options(&value.synced_lyrics,self.settings.romanize_lyrics);self.plain_lyrics=if value.instrumental{"Instrumental · nikmati musiknya.".into()}else if value.plain_lyrics.is_empty()&&self.lyrics.is_empty(){"Lirik belum tersedia untuk lagu ini.".into()}else if self.settings.romanize_lyrics{crate::lyrics::romanize_text(&value.plain_lyrics)}else{value.plain_lyrics};
    }
    fn request_lyrics(&mut self, force: bool) {
        if !self.settings.lyrics_online || self.track.title.is_empty() || !self.settings.enabled("Musik") || self.fullscreen || self.suspended { return; }
        let key = lyric_request_key(&self.track);
        let api = self.online.clone();
        let track = self.track.clone();
        let allow_yt = crate::youtube::session_allows_captions(&track.source_app, &track.title, &track.album);
        let permission=self.lyrics_consent.snapshot();
        if self.job("Musik", move || { let result=api.lyrics_lookup(&track.title, &track.artist, &track.album, track.duration_seconds, allow_yt, force, permission.clone()); Event::Lyrics(permission,key,result) }) {
            self.lyrics_key = lyric_request_key(&self.track);
            self.lyrics_error = None;
            self.lyrics_retry.started();
        }
    }
    fn sync_active_lyric(&mut self){
        if self.settings.quiet_mode||self.settings.effective_presentation().reduced_motion{self.lyric_anim=None;return;}
        let curr_text=self.active_lyric().unwrap_or_default();
        if !curr_text.is_empty()&&curr_text!=self.last_lyric_line{
            let prev=std::mem::replace(&mut self.last_lyric_line,curr_text);
            if !prev.is_empty(){
                self.lyric_anim=Some(LyricAnim{
                    prev_text:prev,
                    started:Instant::now(),
                });
                self.start_morph_timer();
            }
            // Do NOT retarget here: the cached lyric geometry still belongs to the
            // previous line, so retargeting now (then correcting again in paint after
            // DirectWrite re-measures) produced a one-frame double-morph — the visible
            // background flicker on 1-line <-> 2-line lyric changes. A single
            // invalidate lets paint() refresh the metrics and retarget exactly once.
            self.invalidate();
        }
    }
    fn set_expanded(&mut self,value:bool){
        if value{self.activate_on_explicit_open();}
        self.set_hover_stage(if value{2}else{collapse_target_stage(self.settings.minimal)});
    }
    fn activate_on_explicit_open(&self){unsafe{let _=SetForegroundWindow(self.hwnd);}}
    pub fn is_mod_active(&self, id: &str) -> bool {
        active_package(&self.store.root,&self.settings,id)
    }

    fn clock_timer(&self){unsafe{let now=Local::now();let ui_active=self.focus.running()||self.breathing.is_some()||self.expanded||self.lyric_mode||self.track.playing||self.toast.is_some()||!self.state.pending_local_reminders.is_empty()||self.activities.feedback.is_some()||self.volume_hud.is_some()||self.settings.water_reminder||self.settings.move_reminder||self.settings.prayer_enabled||self.stretch_deadline.is_some()||self.lyrics_retry.pending();let delay=clock_delay_ms(self.fullscreen,ui_active,self.is_mod_active("daya"),now.second());SetTimer(Some(self.hwnd),1,delay,None);}}
    /// Persists whatever is currently typed in the Sticky Notes EDIT controls. `make_inputs` is
    /// otherwise the only flush point, so exiting straight from the tray would drop the edit.
    fn flush_note_edits(&mut self){self.flush_editors_barrier();}
    fn make_inputs(&mut self){unsafe{
        if !self.retire_panel(){return;}
        for &id in self.inputs.keys(){
            if self.input_page != "Catatan" && self.input_page != "Clipboard" {
                self.drafts.insert((self.input_page.clone(),id),self.input(id));
            }
        }
        if self.input_page=="Tugas"{if let Some(note)=self.drafts.get(&("Tugas".into(),12)){self.state.notes=note.clone();}}
        if self.input_page=="Clipboard" {
            self.clip_filter = self.input(60);
        }
        for(_,hwnd)in std::mem::take(&mut self.inputs){let _=DestroyWindow(hwnd);}if !self.expanded{self.rebuild_input_region();self.sync_input_host();return}
        if self.settings.quiet_mode{self.input_page.clear();self.rebuild_input_region();self.sync_input_host();return;}
        self.input_page=if self.settings_open{"Pengaturan".into()}else{self.tab.clone()};
        if self.settings_open{
            self.rebuild_input_region();self.sync_input_host();return
        }
        match self.tab.as_str(){

            "Tugas"=>{self.edit(10,"",16.,46.,220.,28.,false,false);self.edit(11,"",244.,46.,100.,28.,false,false);let notes=self.state.notes.clone();self.edit(12,&notes,16.,184.,330.,68.,false,true);},
            "AI"=>{let w=self.width;self.edit(40,"",28.,124.,w-56.,50.,false,true);self.edit(41,&self.settings.ai_endpoint.clone(),28.,72.,240.,26.,false,false);self.edit(42,&self.settings.ai_model.clone(),276.,72.,140.,26.,false,false);self.edit(43,"",28.,104.,240.,26.,true,false);},
            "Clipboard"=>{
                let filter = self.clip_filter.clone();
                if self.clip_search_open || !self.clip_filter.is_empty() {
                    self.edit(60, &filter, 44., 42., 164., 19., false, false);
                }
            },
            "Cuaca"=>{
                if self.weather_custom_open {
                    let modal_w = 340.;
                    let modal_x = (self.width - modal_w) / 2.;
                    let modal_y = 65.;
                    let city=self.settings.weather_city.clone();
                    self.edit(30,&city,modal_x + 114.,modal_y + 44.,200.,24.,false,false);
                    self.edit(31,&self.settings.latitude.to_string(),modal_x + 114.,modal_y + 76.,200.,24.,false,false);
                    self.edit(32,&self.settings.longitude.to_string(),modal_x + 114.,modal_y + 108.,200.,24.,false,false);
                }
            },
            "Catatan"=>{
                if self.simpan_sub==1 {
                    let filter = self.clip_filter.clone();
                    if self.clip_search_open || !self.clip_filter.is_empty() {
                        self.edit(60, &filter, 44., 76., 164., 19., false, false);
                    }
                } else if self.simpan_sub==0 && self.note_export_open.is_none() {
                    if let Some(note)=self.state.sticky_notes.get(self.state.selected_note).filter(|n|!self.editors.registry.panel_read_only(n.id)){
                        let title=note.title.clone();
                        let content=note.content.clone();
                        self.edit(50,&title,256.,76.,self.width-276.,24.,false,false);
                        self.edit(51,&content,256.,106.,self.width-276.,106.,false,true);
                    }
                }
            },
            _=>{}
        }
        self.bind_panel();self.rebuild_input_region();self.sync_input_host();
    }}
    unsafe fn edit(&mut self,id:u16,value:&str,x:f32,y:f32,width:f32,height:f32,password:bool,multiline:bool){unsafe{
        let value=if self.input_page != "Catatan" && self.input_page != "Clipboard" {
            self.drafts.get(&(self.input_page.clone(),id)).map(|v|v.as_str()).unwrap_or(value)
        } else {
            value
        };
        let value:Vec<u16>=value.encode_utf16().chain(Some(0)).collect();
        let style=if id == 60 {
            WS_CHILD|WS_VISIBLE|WS_TABSTOP|WINDOW_STYLE(ES_AUTOHSCROLL as u32)
        } else {
            WS_CHILD|WS_VISIBLE|WS_TABSTOP|WS_BORDER|WINDOW_STYLE(if multiline{(ES_MULTILINE|ES_AUTOVSCROLL|ES_WANTRETURN)as u32|WS_VSCROLL.0}else{ES_AUTOHSCROLL as u32})|WINDOW_STYLE(if password{ES_PASSWORD as u32}else{0})
        };
        // Parent is the input host, not the composition window: child HWNDs do not render on a
        // WS_EX_NOREDIRECTIONBITMAP parent. The host sits exactly on top of the shell and is
        // clipped to these rectangles, so the shell keeps every click outside them.
        // Coordinates are shifted by the shadow margin because the shell window now starts
        // SHADOW_PAD above and left of the capsule.
        let (capsule_x,capsule_y)=self.capsule_origin();
        if let Ok(edit)=CreateWindowExW(WINDOW_EX_STYLE(0),w!("EDIT"),PCWSTR(value.as_ptr()),style,((x+capsule_x)*self.scale)as i32,((y+capsule_y)*self.scale)as i32,(width*self.scale)as i32,(height*self.scale)as i32,Some(self.inputs_hwnd),Some(HMENU(id as usize as *mut _)),None,None){
            SendMessageW(edit,WM_SETFONT,Some(WPARAM(GetStockObject(DEFAULT_GUI_FONT).0 as usize)),Some(LPARAM(1)));
            if id == 60 {
                let cue: Vec<u16> = crate::i18n::t("clipboard.search_hint", &self.settings.language).encode_utf16().chain(Some(0)).collect();
                let _ = SendMessageW(edit, 0x1501, Some(WPARAM(1)), Some(LPARAM(cue.as_ptr() as isize)));
            }
            self.inputs.insert(id,edit);
        }
    }}
    /// Clips the input host to exactly the rectangles of the EDIT controls. Everything outside
    /// the region is neither painted nor hit-tested, so those clicks fall through to the shell.
    fn rebuild_input_region(&self){unsafe{
        let mut region:Option<HRGN>=None;
        for &edit in self.inputs.values(){
            let mut rect=RECT::default();let _=GetWindowRect(edit,&mut rect);
            let mut origin=POINT::default();let _=ClientToScreen(self.inputs_hwnd,&mut origin);
            let left=rect.left-origin.x;let top=rect.top-origin.y;
            let piece=CreateRectRgn(left,top,left+(rect.right-rect.left),top+(rect.bottom-rect.top));
            region=Some(match region{None=>piece,Some(combined)=>{let _=CombineRgn(Some(combined),Some(combined),Some(piece),RGN_OR);let _=DeleteObject(piece.into());combined}});
        }
        // An empty region makes the whole window inert; hiding it is clearer.
        let _=SetWindowRgn(self.inputs_hwnd,region,true);
    }}
    /// Keeps the input host glued to the shell: same rectangle, just above it in z-order, and
    /// only visible while the panel is open with input controls on screen.
    /// The host is raised to the top of the topmost band rather than inserted after the shell:
    /// the shell re-raises itself on every layout, and an insert-after placement loses that race,
    /// which leaves the host underneath the shell and the inputs invisible.
    fn sync_input_host(&self){unsafe{
        let visible=self.expanded&&self.morph.is_none()&&!self.inputs.is_empty()&&IsWindowVisible(self.hwnd).as_bool();
        let _=SetWindowPos(self.inputs_hwnd,Some(HWND_TOPMOST),self.input_x,self.input_y,self.canvas_w,self.canvas_h,SWP_NOACTIVATE);
        if IsWindowVisible(self.inputs_hwnd).as_bool()!=visible{let _=ShowWindow(self.inputs_hwnd,if visible{SW_SHOWNOACTIVATE}else{SW_HIDE});}
    }}
    fn input(&self,id:u16)->String{unsafe{if let Some(&hwnd)=self.inputs.get(&id){let mut text=vec![0u16;(GetWindowTextLengthW(hwnd)as usize).min(65535)+1];let count=GetWindowTextW(hwnd,&mut text);String::from_utf16_lossy(&text[..count.max(0)as usize])}else{String::new()}}}
    fn clear_input(&self,id:u16){unsafe{if let Some(&hwnd)=self.inputs.get(&id){let _=SetWindowTextW(hwnd,w!(""));}}}
    fn load_holidays(&mut self){self.holidays=if self.year==2026{serde_json::from_str::<HolidayFile>(include_str!("../data/holidays-2026.json")).map(|f|f.holidays).unwrap_or_default()}else{vec![]};}
    fn handle(&mut self,msg:u32,wparam:WPARAM,lparam:LPARAM)->LRESULT{unsafe{
        match msg{
            sticky_window::NOTE_SAVED=>{if self.flush_editors_barrier(){self.make_inputs();self.invalidate();}return LRESULT(0);},
            WM_APP_VOLUME_CHANGED=>{
                let updated=match volume_key(wparam.0 as u16){
                    Some(VolumeKey::Up)=>crate::system::adjust_master_volume(0.02),
                    Some(VolumeKey::Down)=>crate::system::adjust_master_volume(-0.02),
                    Some(VolumeKey::Mute)=>crate::system::toggle_master_mute(),
                    None=>None,
                };
                // The Core Audio fallback lets Windows handle the key, so read the result back.
                let shown=updated.or_else(crate::system::get_master_volume);
                if let Some((vol,muted))=shown{
                    self.show_volume_hud(vol,muted);
                }
                return LRESULT(0);
            },
            WM_APP_SCREENSHOT=>{
                if self.hover_stage >= 2 || self.expanded {
                    self.screenshot_stage2_freeze = Some(Instant::now() + Duration::from_secs(15));
                    let _ = KillTimer(Some(self.hwnd), 2);
                } else if self.settings.hide_screenshot {
                    self.screenshot_hide_until = Some(Instant::now() + Duration::from_secs(6));
                    let _ = ShowWindow(self.hwnd, SW_HIDE);
                    SetTimer(Some(self.hwnd), 14, 6000, None);
                } else {
                    self.screenshot_stage2_freeze = Some(Instant::now() + Duration::from_secs(15));
                    let _ = KillTimer(Some(self.hwnd), 2);
                }
                return LRESULT(0);
            },
            0x031D=>{self.sync_clipboard();return LRESULT(0)},
            WM_PAINT=>{let mut ps=PAINTSTRUCT::default();BeginPaint(self.hwnd,&mut ps);self.paint();let _=EndPaint(self.hwnd,&ps);return LRESULT(0)},
            WM_ERASEBKGND=>return LRESULT(1),
            WM_MOUSEACTIVATE=>return LRESULT(MA_NOACTIVATE as isize),
            WM_NCHITTEST=>{
                if (self.fullscreen && self.settings.hide_fullscreen)
                    || (!self.preview && !self.expanded && !is_cursor_visible())
                {
                    return LRESULT(-1); // HTTRANSPARENT
                }
                let mut point=POINT{x:(lparam.0 as i16)as i32,y:((lparam.0>>16)as i16)as i32};
                let _=ScreenToClient(self.hwnd,&mut point);
                let px=point.x as f32/self.scale;let py=point.y as f32/self.scale;
                let(ox,oy)=self.capsule_origin();let(width,height,_)=self.geometry();
                let ear=if !self.settings.bottom&&self.settings.edge_gap==0{
                    if self.hover_stage==0&&!self.expanded&&!self.lyric_mode{6.}else if height>=24.{12.}else{(height*0.5).max(4.)}
                }else{0.};
                // Hit test strictly within the physical capsule boundary!
                // Zero extra margins, so 100% of clicks outside the notch pass straight to Chrome tabs.
                if px>=(ox-ear)&&px<=(ox+width+ear)&&py>=oy&&py<=(oy+height){
                    return DefWindowProcW(self.hwnd,msg,wparam,lparam)
                }
                return LRESULT(-1) // HTTRANSPARENT
            },
            WM_MOUSEMOVE=>{
                if self.pressed == Some(Action::QuickBrightnessPointer) {
                    let (x, _) = self.client_to_capsule(lparam);
                    let layout = crate::quick_controls::calculate_layout();
                    let ratio = ((x - layout.brightness_slider_hit.left) / (layout.brightness_slider_hit.right - layout.brightness_slider_hit.left)).clamp(0., 1.);
                    let pct = (ratio * 100.0).round() as u32;
                    self.action(Action::QuickControlsSetBrightness(pct, false));
                }
                if (self.fullscreen && self.settings.hide_fullscreen)
                    || (!self.preview && !self.expanded && !is_cursor_visible())
                {
                    return LRESULT(0);
                }
                let mut tracking=TRACKMOUSEEVENT{cbSize:std::mem::size_of::<TRACKMOUSEEVENT>()as u32,dwFlags:TME_LEAVE,hwndTrack:self.hwnd,dwHoverTime:0};
                let _=TrackMouseEvent(&mut tracking);
                let _=KillTimer(Some(self.hwnd),2);
                let (x,y)=self.client_to_capsule(lparam);
                let moved=self.pointer.is_none_or(|(px,py)|(px-x).abs()>0.5||(py-y).abs()>0.5);
                if moved{self.pointer=Some((x,y));self.update_glass_hover(Some((x,y)));self.invalidate();}

                if let Some(drag) = &mut self.tab_drag {
                    if self.pressed.is_some() {
                        let dx = (x - drag.start_pos.0).abs();
                        if !drag.is_dragging && dx > 4.0 {
                            let _ = KillTimer(Some(self.hwnd), 12);
                            drag.is_dragging = true;
                            if self.tab != drag.tab {
                                self.tab = drag.tab.clone();
                            }
                        }
                        if drag.is_dragging {
                            drag.current_pos = (x, y);
                            self.invalidate();
                        }
                    }
                }

                if self.hover_stage==0 && Instant::now() > self.hover_blocked_until {
                    // Debounced hover on Stage 0 reveals Stage 1 after 140 ms so incidental top-edge cursor travel doesn't flicker.
                    SetTimer(Some(self.hwnd), 4, 140, None);
                }
                return LRESULT(0)
            },
            WM_MOUSELEAVE=>{
                self.update_glass_hover(None);
                let _=KillTimer(Some(self.hwnd),4);
                let _=KillTimer(Some(self.hwnd),12);
                if self.pointer.take().is_some(){self.invalidate();}
                if let Some(drag) = self.tab_drag.take() {
                    if drag.is_dragging {
                        self.save();
                        self.invalidate();
                    }
                }
                self.pressed=None;
                if !self.preview && !self.pinned && self.hover_stage==2 && self.settings_popup.is_none() && self.reminder_menu.is_none() {
                    if !self.screenshot_stage2_freeze.is_some_and(|until| Instant::now() < until) {
                        SetTimer(Some(self.hwnd),2,800,None);
                    }
                }
                return LRESULT(0)
            },
            WM_LBUTTONDOWN=>{self.screenshot_stage2_freeze=None;self.activities.discard_release=false;let (x,y)=self.client_to_capsule(lparam);
                if self.note_export_open.is_some(){
                    let inside=self.hits.iter().rev().find(|h|x>=h.rect.left&&x<=h.rect.right&&y>=h.rect.top&&y<=h.rect.bottom).is_some_and(|h|matches!(h.action,Action::NoteExportMenu(..)|Action::NoteExportPick(..)));
                    if !inside{self.note_export_open=None;self.pressed=None;self.make_inputs();self.invalidate();return LRESULT(0);}
                }
                if self.media_source_open{
                    let inside=self.hits.iter().rev().find(|h|x>=h.rect.left&&x<=h.rect.right&&y>=h.rect.top&&y<=h.rect.bottom).is_some_and(|h|matches!(h.action,Action::MediaSourceMenu|Action::SelectMediaSource(..)));
                    if !inside{self.media_source_open=false;self.invalidate();}
                }
                if self.weather_menu_open{let inside=self.hits.iter().rev().find(|h|x>=h.rect.left&&x<=h.rect.right&&y>=h.rect.top&&y<=h.rect.bottom).is_some_and(|h|matches!(h.action,Action::WeatherSetCity(..)|Action::WeatherToggleCustom|Action::WeatherToggleMenu));if !inside{self.weather_menu_open=false;self.pressed=None;self.invalidate();return LRESULT(0)}}
                if self.settings_popup.is_some(){let inside=self.hits.iter().rev().find(|h|x>=h.rect.left&&x<=h.rect.right&&y>=h.rect.top&&y<=h.rect.bottom).is_some_and(|h|matches!(h.action,Action::SettingsMenu(..)|Action::SettingsValue(..)|Action::SettingsCategory(..)|Action::Back));if !inside{self.settings_popup=None;self.pressed=None;self.invalidate();return LRESULT(0)}}
                if self.reminder_menu.is_some(){let inside=self.hits.iter().rev().find(|h|x>=h.rect.left&&x<=h.rect.right&&y>=h.rect.top&&y<=h.rect.bottom).is_some_and(|h|matches!(h.action,Action::ReminderValue(..)|Action::ReminderMenu(..)));if !inside{self.reminder_menu=None;self.reminder_menu_scroll=0;self.pressed=None;self.invalidate();return LRESULT(0)}}
                if self.storage_dropdown_open{let inside=self.hits.iter().rev().find(|h|x>=h.rect.left&&x<=h.rect.right&&y>=h.rect.top&&y<=h.rect.bottom).is_some_and(|h|matches!(h.action,Action::ToggleStorageDropdown|Action::KeepStorageDropdown|Action::CycleStoragePage));if !inside{self.storage_dropdown_open=false;self.pressed=None;self.invalidate();return LRESULT(0)}}
                if let Some(hit)=self.hits.iter().rev().find(|h|x>=h.rect.left&&x<=h.rect.right&&y>=h.rect.top&&y<=h.rect.bottom).cloned(){
                    self.pressed=Some(hit.action.clone());
                    if let Action::Tab(ref tab_name) = hit.action {
                        let grab_offset = x - hit.rect.left;
                        self.tab_drag = Some(TabDragState {
                            tab: tab_name.clone(),
                            start_pos: (x, y),
                            current_pos: (x, y),
                            grab_offset_x: grab_offset,
                            is_dragging: false,
                            started_at: Instant::now(),
                        });
                        SetTimer(Some(self.hwnd), 12, 180, None);
                    } else {
                        self.tab_drag = None;
                    }
                    self.invalidate();
                }
                return LRESULT(0)},
            WM_LBUTTONUP=>{let (x,y)=self.client_to_capsule(lparam);
                if std::mem::take(&mut self.activities.discard_release){let _=KillTimer(Some(self.hwnd),12);self.tab_drag=None;self.pressed=None;return LRESULT(0);}
                let _=KillTimer(Some(self.hwnd), 12);
                let pressed=self.pressed.take();
                let was_dragging = self.tab_drag.as_ref().map(|d| d.is_dragging).unwrap_or(false);
                self.tab_drag = None;
                self.invalidate();
                if was_dragging {
                    self.save();
                    return LRESULT(0);
                }
                if self.morph.is_some(){return LRESULT(0)}
                if let Some(pressed_action)=pressed{
                    if let Some(hit)=self.hits.iter().rev().find(|h|x>=h.rect.left&&x<=h.rect.right&&y>=h.rect.top&&y<=h.rect.bottom).cloned(){
                        if pressed_action==hit.action{
                            match hit.action{
                                Action::SeekPointer=>{
                                    let ratio=((x-hit.rect.left)/(hit.rect.right-hit.rect.left)).clamp(0.,1.)as f64;
                                    self.action(Action::Seek(ratio*self.track.duration_seconds));
                                },

                                other=>self.action(other),
                            }
                            return LRESULT(0);
                        }
                    }
                    return LRESULT(0);
                }
                // When in Stage 0, clicking capsule reveals Stage 1. When in Stage 1, clicking capsule body opens Stage 2!

                if let Some(t) = self.toast.take() {
                    let _ = KillTimer(Some(self.hwnd), 3);
                    self.toast_from_stage0 = false;
                    if let Some(idx) = toast_target_reminder(t.kind) {
                        self.tab = "Reminder".into();
                        self.reminder_index = idx;
                        self.settings_open = false;
                    }
                    self.set_hover_stage(2);
                    self.activate_on_explicit_open();
                    return LRESULT(0);
                }
                if self.hover_stage==0{
                    self.set_hover_stage(1);
                    return LRESULT(0);
                }else if self.hover_stage==1{
                    self.set_hover_stage(2);
                    self.activate_on_explicit_open();
                    return LRESULT(0);
                }
                return LRESULT(0)},
            WM_MOUSEWHEEL if self.expanded&&self.settings_open=>{let delta=(wparam.0>>16)as i16;if self.settings_popup.is_some(){self.handle_settings_key(if delta>0{0x26}else{0x28});}else if self.settings_cat==5{self.scroll_catalog(if delta>0{-1}else{1});}else{self.scroll_settings(if delta>0{-42.}else{42.});}return LRESULT(0)},
            WM_MOUSEWHEEL if !self.expanded&&!self.settings_open=>{
                let wheel_delta=(wparam.0>>16)as i16;
                let delta=if wheel_delta>0{0.02}else{-0.02};
                if let Some((vol,muted))=crate::system::adjust_master_volume(delta){
                    self.volume_hud=Some((vol,muted,Instant::now()));
                }else{
                    crate::system::step_volume_fallback(wheel_delta>0);
                    if let Some((vol,muted))=crate::system::get_master_volume(){
                        self.volume_hud=Some((vol,muted,Instant::now()));
                    }
                }
                self.trigger_stage1_temporary(2);
                if self.hover_stage==0{self.set_hover_stage(1);}
                self.invalidate();
                return LRESULT(0);
            },
            WM_MOUSEWHEEL if self.expanded&&!self.settings_open=>{
                let mut point=POINT{x:(lparam.0 as i16)as i32,y:((lparam.0>>16)as i16)as i32};let _=ScreenToClient(self.hwnd,&mut point);
                let (ox,oy)=self.capsule_origin();
                let rel_x=point.x as f32/self.scale-ox;
                let rel_y=point.y as f32/self.scale-oy;
                let wheel_delta=(wparam.0>>16)as i16;
                if rel_y<=42.0{
                    let tabs = self.settings.active_tab_order();
                    if !tabs.is_empty(){
                        let curr_idx=tabs.iter().position(|t|t==&self.tab).unwrap_or(0);
                        let next_idx=if wheel_delta>0{
                            if curr_idx==0{tabs.len()-1}else{curr_idx-1}
                        }else{
                            (curr_idx+1)%tabs.len()
                        };
                        self.action(Action::Tab(tabs[next_idx].clone()));
                    }
                    return LRESULT(0);
                }
                if self.media_source_open&&self.tab=="Musik"&&rel_y>=70.&&rel_y<=108.{
                    self.handle_media_source_key(if wheel_delta>0{0x25}else{0x27});return LRESULT(0);
                }
                if self.tab=="Cuaca"&&rel_y>42.{self.refresh_weather(true);return LRESULT(0);}
                if self.tab=="Reminder"&&self.reminder_menu.is_some()&&rel_x>=229.{self.handle_reminder_menu_key(if wheel_delta>0{0x26}else{0x28});return LRESULT(0);}
                if self.tab=="Reminder"&&rel_x<208.&&rel_y>48.{self.action(Action::ReminderStep(if wheel_delta>0{-1}else{1}));return LRESULT(0);}
                if self.tab=="Kalender"&&rel_x<308.&&rel_y>42.{self.action(Action::Month(if wheel_delta>0{-1}else{1}));return LRESULT(0);}
                if self.tab=="Clipboard"&&rel_y>42.{self.action(Action::ClipPage(if wheel_delta>0{-1}else{1}));return LRESULT(0);}
                if self.tab=="Catatan"&&rel_y>42.{if self.notes_host.tasks{self.action(Action::TasksPage(if wheel_delta>0{-1}else{1}));return LRESULT(0);}let total=self.state.sticky_notes.len();if total>0{let delta=if wheel_delta>0{-1}else{1};let next=(self.state.selected_note as i32+delta).clamp(0,(total-1)as i32)as usize;if next!=self.state.selected_note{self.action(Action::NoteSelect(next));}}return LRESULT(0);}
                if self.tab=="Musik"&&self.lyric_mode&&rel_x>=252.{
                    if !self.lyrics.is_empty(){
                        let position=self.media.as_ref().map(|m|m.position_seconds()).unwrap_or(self.track.position_seconds);
                        let base=if self.lyric_offset==0{crate::lyrics::active_line(&self.lyrics,position).unwrap_or(0)}else{self.lyric_offset-1};
                        let delta=if wheel_delta>0{-1}else{1};
                        self.lyric_offset=(base as i32+delta).clamp(0,self.lyrics.len().saturating_sub(1)as i32)as usize+1;
                        self.invalidate();
                    }else if !self.plain_lyrics.is_empty(){
                        let count=self.plain_lyrics.lines().filter(|l|!l.trim().is_empty()).count();
                        if count>0{
                            let base=self.lyric_offset.saturating_sub(1);
                            let delta=if wheel_delta>0{-1}else{1};
                            self.lyric_offset=(base as i32+delta).clamp(0,count.saturating_sub(1)as i32)as usize+1;
                            self.invalidate();
                        }
                    }
                }
                return LRESULT(0)
            },
            WM_POWERBROADCAST=>{match wparam.0{4=>self.power_state(true),7|18=>self.power_state(false),_=>{}}return LRESULT(1)},
            MORPH_MSG=>{self.morph_step();return LRESULT(0)},
            WM_TIMER=>{match wparam.0{
                16=>self.step_glass_hover(),
                1=>self.tick(),
                2=>{
                    let _=KillTimer(Some(self.hwnd),2);
                    if self.screenshot_stage2_freeze.is_some_and(|until| Instant::now() < until) {
                        return LRESULT(0);
                    }
                    let focused=GetFocus();
                    let editing=self.inputs.values().any(|&h|h==focused);
                    if !editing && !self.pinned && self.hover_stage==2 && self.settings_popup.is_none() && self.reminder_menu.is_none() {
                        // Hovering an EDIT control triggers WM_MOUSELEAVE on the shell even though
                        // the cursor is still over the notch. Only collapse once it really left.
                        if self.cursor_over_notch(){SetTimer(Some(self.hwnd),2,400,None);}
                        else{self.set_hover_stage(collapse_target_stage(self.settings.minimal));}
                    }
                },
                3=>self.expire_toast(),
                4=>{
                    let _=KillTimer(Some(self.hwnd),4);
                    if self.hover_stage==0 && self.pointer.is_some() && Instant::now()>self.hover_blocked_until {
                        self.set_hover_stage(1);
                    }
                },
                14=>{
                    let _=KillTimer(Some(self.hwnd), 14);
                    self.screenshot_hide_until = None;
                    if !self.fullscreen {
                        let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
                        self.layout();
                        self.invalidate();
                    }
                },
                5=>{
                    self.maybe_write_health_ack();
                    if !self.suspended{
                        self.sync_foreground();

                        // The shell un-minimises its OSD host when it wants to paint it, so keep
                        // it down on every fast tick. When the notch itself is hidden in fullscreen
                        // the user needs Windows' own indicator, so let it through.
                        crate::system::set_volume_osd_hidden(!(self.fullscreen && self.settings.hide_fullscreen));
                        // Hardware volume keys can reach the shell directly (HID consumer control)
                        // instead of the keyboard hook, so mirror any master-volume change here.
                        if let Some((vol, muted)) = crate::system::get_master_volume() {
                            if self.last_known_volume.is_none_or(|(known, known_muted)| (known - vol).abs() > 0.001 || known_muted != muted) {
                                self.show_volume_hud(vol, muted);
                            }
                        }
                    }
                },
                6=>{
                    let now=Instant::now();
                    if self.stage1_temporary_until.is_some_and(|until|until<=now){self.stage1_temporary_until=None;}
                    if self.stage1_media_until.is_some_and(|until|until<=now){self.stage1_media_until=None;}
                    if self.weather_stage1_until.is_some_and(|until|until<=now){self.weather_stage1_until=None;}
                    self.sync_stage1_timer();
                    if self.hover_stage<2{
                        self.retarget_morph();
                    }
                    self.layout();
                },
                7=>{let _=KillTimer(Some(self.hwnd),7);self.power_state(true);},
                8=>{let _=KillTimer(Some(self.hwnd),8);self.power_state(false);},
                10=>{if self.tab_fade.as_ref().is_some_and(|(_,t,_)|t.elapsed()>=ui::TAB_FADE_DURATION){self.tab_fade=None;let _=KillTimer(Some(self.hwnd),10);}self.invalidate();},
                11=>{self.startup_alpha=(self.startup_alpha+0.05).min(1.);
                    // F1: timer 11 juga menggerakkan animasi turun boot (600 ms) —
                    // jangan dimatikan sebelum keduanya selesai.
                    let drop_done=self.boot_drop.as_ref().is_none_or(|t|t.elapsed()>=BOOT_DROP_DURATION);
                    if self.startup_alpha>=1.&&drop_done{self.boot_drop=None;let _=KillTimer(Some(self.hwnd),11);crate::system::trim_working_set();}
                    self.invalidate();},
                13=>{let _=KillTimer(Some(self.hwnd),13);
                    // F1: selesai menunggu — morph ke Stage 1 idle kecuali pengguna
                    // sudah berpindah stage sendiri (atau mode minimal yang tetap Stage 0).
                    if self.hover_stage==0{self.set_hover_stage(boot_completes_to_stage(self.settings.minimal));}},
                12=>{
                    let _=KillTimer(Some(self.hwnd),12);
                    if let Some(drag) = &mut self.tab_drag {
                        if self.pressed.is_some() && !drag.is_dragging {
                            drag.is_dragging = true;
                            if self.tab != drag.tab {
                                self.tab = drag.tab.clone();
                            }
                            self.invalidate();
                        }
                    }
                },
                _=>{}
            }return LRESULT(0)},
            WM_KEYDOWN=>{if self.quick_controls_open && self.quick_controls.handle_key(wparam.0 as u16){self.invalidate();return LRESULT(0)}if self.quick_controls_open && wparam.0 == 0x1B{self.action(Action::QuickControlsBack);return LRESULT(0)}if self.handle_media_source_key(wparam.0 as u16){return LRESULT(0)}if self.handle_settings_key(wparam.0 as u16){return LRESULT(0)}if self.reminder_menu.is_some()&&self.handle_reminder_menu_key(wparam.0 as u16){return LRESULT(0)}match wparam.0 as u16{
                0x26|0x28 if self.tab=="Catatan"&&self.notes_host.tasks&&!self.settings_open&&!self.settings.quiet_mode=>{
                    self.refresh_notes_host();let current=self.notes_host.selected.and_then(|id|self.state.todos.iter().position(|t|t.id==id)).unwrap_or(0);
                    let next=if wparam.0==0x26{current.saturating_sub(1)}else{(current+1).min(self.state.todos.len().saturating_sub(1))};
                    if let Some(todo)=self.state.todos.get(next){self.action(Action::TaskSelect(todo.id,self.notes_host.revision));}return LRESULT(0);
                }
                0x26|0x28|0x24|0x23 if self.tab=="Reminder"&&self.expanded&&!self.settings_open=>{
                    self.action(match wparam.0{0x24=>Action::ReminderSelect(0),0x23=>Action::ReminderSelect(3),0x26=>Action::ReminderStep(-1),_=>Action::ReminderStep(1)});return LRESULT(0)
                }
                0x09=>{ // VK_TAB
                    if self.hits.is_empty(){return LRESULT(0)}
                    let reverse=(GetKeyState(VK_SHIFT.0 as i32)as i32 & 0x8000)!=0;
                    self.focused=match&self.focused{
                        None=>Some(self.hits[0].action.clone()),
                        Some(current)=>{
                            let idx=self.hits.iter().position(|h|h.action==*current).unwrap_or(0);
                            let next=if reverse{if idx==0{self.hits.len()-1}else{idx-1}}else{if idx+1>=self.hits.len(){0}else{idx+1}};
                            Some(self.hits[next].action.clone())
                        }
                    };
                    self.invalidate();return LRESULT(0)
                }
                0x0D|0x20=>{ // VK_RETURN or VK_SPACE
                    if let Some(action)=self.focused.clone(){
                        if action!=Action::SeekPointer{self.action(action);}
                        return LRESULT(0);
                    }else if wparam.0 as u16==0x20&&self.expanded&&!self.settings_open&&self.tab=="Musik"{
                        self.action(Action::Play);
                        return LRESULT(0);
                    }
                }
                0x1B if self.note_export_open.is_some()=>{self.note_export_open=None;self.make_inputs();self.invalidate();return LRESULT(0)},
                0x1B if self.weather_menu_open=>{self.weather_menu_open=false;self.invalidate();return LRESULT(0)},
                0x1B if self.weather_custom_open=>{self.action(Action::WeatherToggleCustom);return LRESULT(0)},
                0x1B if self.reminder_menu.is_some()=>{self.reminder_menu=None;self.reminder_menu_scroll=0;self.invalidate();return LRESULT(0)},
                0x1B=>{ // VK_ESCAPE
                    if self.hover_stage>=2{
                        self.set_hover_stage(collapse_target_stage(self.settings.minimal));
                    }else if self.hover_stage==1{
                        self.hover_blocked_until=Instant::now()+Duration::from_millis(600);
                        self.set_hover_stage(0);
                    }
                    return LRESULT(0);
                }
                _=>{}
            }}
            WM_SETFOCUS|WM_KILLFOCUS=>{if msg==WM_KILLFOCUS{self.media_source_open=false;}self.focused=None;self.invalidate();return LRESULT(0)},
            WM_HOTKEY=>{if !self.fullscreen{self.set_expanded(!self.expanded);}return LRESULT(0)},
            WM_RBUTTONUP|TRAY=>{if msg==WM_RBUTTONUP&&self.fullscreen&&self.settings.hide_fullscreen{return LRESULT(0)}if msg==WM_RBUTTONUP||lparam.0 as u32==WM_RBUTTONUP{self.menu();}else if lparam.0 as u32==WM_LBUTTONUP{self.set_expanded(!self.expanded);}return LRESULT(0)},
            WAKE=>{self.events();return LRESULT(0)},
            WM_DPICHANGED|WM_DISPLAYCHANGE=>{self.layout();self.make_inputs();self.invalidate();return LRESULT(0)},
        // WM_SETTINGCHANGE / WM_THEMECHANGED / WM_DWMCOMPOSITIONCHANGED: transparency or High Contrast may have flipped.
        0x001A|0x031A|0x031E=>{self.refresh_glass_env();self.sync_glass();self.invalidate();return LRESULT(0)},
            WM_CLOSE=>{self.set_expanded(false);return LRESULT(0)},
            WM_DESTROY=>{PostQuitMessage(0);return LRESULT(0)},
            WM_CTLCOLOREDIT=>{
                let dc=HDC(wparam.0 as *mut _);
                let edit_hwnd=HWND(lparam.0 as *mut _);
                let ctrl_id=GetDlgCtrlID(edit_hwnd);
                if ctrl_id==60{
                    SetTextColor(dc,COLORREF(0x00ffffff));
                    SetBkColor(dc,COLORREF(0x00000000));
                    SetDCBrushColor(dc,COLORREF(0x00000000));
                }else{
                    SetTextColor(dc,COLORREF(0x00f5f1e9));
                    SetBkColor(dc,COLORREF(0x002d2319));
                    SetDCBrushColor(dc,COLORREF(0x002d2319));
                }
                return LRESULT(GetStockObject(DC_BRUSH).0 as isize)
            },

            WM_COMMAND=>{match wparam.0&0xffff{900=>self.set_expanded(!self.expanded),901=>self.action(Action::Settings),902=>self.action(Action::Bottom(false)),903=>self.action(Action::Bottom(true)),999=>{if self.flush_editors_barrier(){let _=DestroyWindow(self.hwnd);}},_=>{}}return LRESULT(0)},
            _=>{}
        }DefWindowProcW(self.hwnd,msg,wparam,lparam)
    }}
    fn tick(&mut self){
        if self.suspended{return}
        // Safety net: the now-playing transient must always fall back to the idle
        // clock even if its dedicated WM_TIMER was missed. The single repaint at the
        // end of tick lets paint() reconcile the geometry through one morph, so the
        // shrink to the calendar stays smooth instead of double-retargeting.
        if self.stage1_media_until.is_some_and(|until| until <= Instant::now()) {
            self.stage1_media_until = None;
        }
        if self.toast.as_ref().is_some_and(|t|t.expires_at<=Instant::now()){self.expire_toast();}
        if let Some(snapshot) = self.privacy_mailbox_rx.poll_latest() {
            let prev_to = self.target_geometry();
            self.privacy_snapshot = snapshot;
            let new_to = self.target_geometry();
            if (prev_to.0 - new_to.0).abs() > 0.1 || (prev_to.1 - new_to.1).abs() > 0.1 {
                self.retarget_to(new_to);
                self.invalidate();
            } else {
                self.invalidate();
            }
            self.sync_foreground();
        }
        let today = Local::now().date_naive();
        let (new_year, new_month, new_day, changed) = crate::core::resolve_date_rollover(
            self.year,
            self.month,
            self.day,
            self.system_date,
            today,
            self.hover_stage,
        );
        self.system_date = today;
        if changed {
            let year_changed = self.year != new_year;
            self.year = new_year;
            self.month = new_month;
            self.day = new_day;
            if year_changed {
                self.load_holidays();
            }
            self.refresh_agenda();
        }
        self.tick_reminder();
        if self.last_battery_check.elapsed() >= Duration::from_secs(4) {
            self.last_battery_check = Instant::now();
            if let Some(new_bat) = crate::system::get_battery_status() {
                if let Some(old_charging) = self.last_ac_status {
                    if old_charging != new_bat.is_charging {
                        let msg = crate::i18n::battery_status(new_bat.percentage, new_bat.is_charging, &self.settings.language);
                        self.notice(&msg);
                    }
                }
                self.last_ac_status = Some(new_bat.is_charging);
                self.battery = Some(new_bat);
            }
        }
        // Bluetooth discovery is blocking; keep it off the timer thread and never overlap probes.
        if !self.preview && !self.tws_pending && self.last_tws_check.elapsed() >= Duration::from_secs(10) {
            self.last_tws_check = Instant::now();
            self.tws_pending = true;
            let post = self.poster();
            thread::spawn(move || post.send(Event::Tws(crate::system::get_tws_status())));
        }
        if let Some((_, _, at)) = self.volume_hud {
            if at.elapsed() >= Duration::from_millis(1800) {
                self.volume_hud = None;
                if self.hover_stage == 1 {
                    self.retarget_morph();
                }
                self.invalidate();
            }
        }
        let clip_seq = crate::clipboard::get_sequence_number();
        if clip_seq != self.last_clipboard_seq {
            self.sync_clipboard();
        }
        if let Some(was_break)=self.focus.complete(){
            self.complete_activity_run(false);
            self.focus.is_break=!was_break;
            if !was_break&&self.settings.auto_break{self.focus.start(self.settings.break_minutes,true);self.start_activity_run(false);}
            if !was_break&&self.settings.stretch_after_focus{self.last_move=Instant::now();}
        }
        if self.settings.enabled("Reminder") {
            if self.settings.water_reminder&&self.state.water.iter().filter(|d|d.date==Local::now().date_naive()).map(|d|d.ml).sum::<u32>()<self.settings.water_target&&self.last_water.elapsed().as_secs()>=reminder::period_secs("water_interval",self.settings.water_interval){
                self.last_water=Instant::now();
                self.recurring_activity(crate::activity::ReminderKind::Water);
            }
            if self.settings.move_reminder&&self.last_move.elapsed().as_secs()>=reminder::period_secs("move_interval",self.settings.move_interval){
                self.last_move=Instant::now();
                self.recurring_activity(crate::activity::ReminderKind::Move);
            }
            if self.settings.move_reminder&&self.settings.eye_reminder&&self.last_eye.elapsed().as_secs()>=reminder::period_secs("move_interval",self.settings.eye_interval){
                self.last_eye=Instant::now();self.recurring_activity(crate::activity::ReminderKind::Eye);
            }


        }
        if self.breathing.is_some_and(|start|start.elapsed().as_secs()>=60){self.breathing=None;self.notice(crate::i18n::t("notice.breathing_done", &self.settings.language));}
        let schedule=Local::now().format("%Y-%m-%d %H:%M").to_string();if self.settings.enabled("Reminder")&&self.settings.focus_enabled&&self.last_schedule!=schedule{let time=Local::now().format("%H:%M").to_string();if self.settings.focus_schedule.contains(&time){self.focus.start(self.settings.focus_minutes,false);self.start_activity_run(false);self.notice(crate::i18n::t("notice.focus_scheduled", &self.settings.language));}self.last_schedule=schedule;}

        if self.is_mod_active("daya"){
            if self.power_sensors.is_none(){self.power_sensors=Some(crate::system::power_sensor::PowerSensors::start());}else if let Some(sensors)=&self.power_sensors{sensors.set_active(true);}
            if self.last_power_tick.elapsed()>=Duration::from_millis(850){
                self.last_power_tick=Instant::now();self.power_snapshot=self.power_sensors.as_ref().unwrap().snapshot();
                self.power_snapshot.language = self.settings.language.clone();
                if let Some(note) = &self.power_snapshot.provider_note {
                    self.power_snapshot.provider_note = Some(crate::system::power_sensor::localize_provider_note(note, &self.settings.language));
                }
                if self.power_snapshot.cpu_usage.is_none() {
                    self.power_snapshot.cpu_usage = Some(self.cpu_sampler.sample() * 100.0);
                }
                let (ram_pct, ram_used, ram_total) = crate::system::sample_system_memory();
                self.power_snapshot.ram_usage = ram_pct;
                self.power_snapshot.ram_used_gb = ram_used;
                self.power_snapshot.ram_total_gb = ram_total;
                self.power_snapshot.storage_usage = self.cached_storage.0;
                self.power_snapshot.storage_used_gb = self.cached_storage.1;
                self.power_snapshot.storage_total_gb = self.cached_storage.2;
                self.power_snapshot.storage_drives = self.cached_storage.3.clone();
                if let Some(started) = self.storage_job_started {
                    if started.elapsed() >= Duration::from_secs(15) {
                        self.cancel_job("Storage");
                        self.storage_job_started = None;
                    }
                }
                if self.last_storage_tick.is_none_or(|t| t.elapsed() >= Duration::from_secs(30)) && !self.jobs.contains_key("Storage") {
                    let now = Instant::now();
                    if self.job("Storage", || {
                        let (st_pct, st_used, st_total, drives) = crate::system::sample_system_storage();
                        Event::Storage(st_pct, st_used, st_total, drives)
                    }) {
                        self.last_storage_tick = Some(now);
                        self.storage_job_started = Some(now);
                    }
                }
                let day=Local::now().format("%Y-%m-%d").to_string();
                if day!=self.power_day{self.power_day=day.clone();self.power_integrator=Default::default();}
                if let Some(w)=self.power_snapshot.active_power_w{
                    let delta=self.power_integrator.add_sample(w,self.power_epoch.elapsed().as_secs_f64());
                    self.power_energy.add(&day,delta);self.session_energy_kwh+=delta;
                    self.power_snapshot.today_kwh=Some(self.power_energy.days.get(&day).copied().unwrap_or(0.));
                }else{self.power_integrator.last_power_w=None;self.power_integrator.last_sample_time_ms=None;self.power_snapshot.today_kwh=self.power_energy.days.get(&day).copied();}
                if self.power_flushed.elapsed()>=Duration::from_secs(30)&&!self.power_energy.days.is_empty(){let _=self.power_energy.flush(&self.store.root);self.power_flushed=Instant::now();}
                if self.active_mod_id().as_deref()==Some("daya"){self.refresh_mod_output();}
            }
        }else if let Some(sensors)=&self.power_sensors{sensors.set_active(false);self.power_snapshot=Default::default();self.power_integrator=Default::default();}
        self.poll_weather();
        if self.lyrics_retry.ready(Instant::now()) && !self.jobs.contains_key("Musik") { self.request_lyrics(true); }
        self.sync_active_lyric();
        // Community mods run once per entry/click — never per frame. Lua output is cached by
        // mod id, so a periodic refresh only happens when the cache is empty for this mod.
        if let Some(id)=self.active_mod_id(){
            let cached_mismatch=self.mod_output.as_ref().is_none_or(|(cached,_)| cached != &id);
            let stale=self.mod_output.as_ref().is_some_and(|(cached,result)| cached == &id && result.as_ref().is_ok_and(|output| output.http_urls.iter().any(|url| self.mod_http_cache.get(url).is_some_and(|(fetched_at,_)| fetched_at.elapsed()>=self.mod_http_ttl))));
            if cached_mismatch||(stale&&!self.jobs.contains_key("ModHttp")){
                self.refresh_mod_output();
            }
        }
        // Task 035: Power-efficient Hardware Sampling (1000 ms, active only when Stage 1 is visible)
        if self.settings.hw_monitor_enabled
            && self.hover_stage == 1
            && !self.fullscreen
            && !self.suspended
            && !self.settings.quiet_mode
            && !self.settings.power_saver_mode
            && unsafe { IsWindowVisible(self.hwnd) }.as_bool()
        {
            if self.last_hardware_tick.elapsed() >= Duration::from_millis(1000) {
                self.last_hardware_tick = Instant::now();
                let sample = self.hardware_sampler.sample();
                self.poster().send(Event::Hardware(sample));
            }
        }

        self.refresh_notes_host();self.reconcile_activities();self.clock_timer();self.invalidate();
    }
    fn events(&mut self){let events=if let Ok(mut q)=self.queue.lock(){q.drain()}else{vec![]};let mut repaint=false;for event in events{let event=match event{Event::Completed(module,id,result)=>{if self.jobs.get(module).map(|j|j.0)!=Some(id){continue}self.jobs.remove(module);self.busy=self.jobs.keys().any(|m|matches!(*m,"AI"|"Cuaca"|"Kalender"));*result},other=>other};

        repaint=true;match event{

        Event::Completed(..)=>{},

        Event::Media(MediaEvent::SessionsChanged(sessions))=>{
            if !self.settings.enabled("Musik")||sessions.selection_revision<self.media_selection_revision{continue}
            self.media_sessions=sessions;
            (self.media_source_cursor,self.focused)=pages::reconcile_media_source_focus(&self.media_sessions,self.media_source_cursor,self.focused.take());
            // The last paint's rectangles may now belong to other source IDs.
            self.hits.retain(|h|!matches!(h.action,Action::SelectMediaSource(..)));
            if matches!(self.pressed,Some(Action::SelectMediaSource(..))){self.pressed=None;}
        },
        Event::Media(MediaEvent::Changed(track))=>{if !self.settings.enabled("Musik")||track.selection_revision<self.media_selection_revision{continue}
            let key=lyric_request_key(&track);let changed=key!=self.lyrics_key;
            let cover_key=crate::artwork::fingerprint(&track.artwork);
            if cover_key!=self.cover_key||(self.cover.is_none()&&!track.artwork.is_empty()&&!self.jobs.contains_key("Artwork")&&changed){
                self.cancel_job("Artwork");self.cover_key=cover_key;self.cover=None;
                if cover_key!=0{let bytes=track.artwork.clone();self.job("Artwork",move||Event::Artwork(cover_key,crate::artwork::decode(&bytes)));}
            }
            if self.preview&&self.track.playing!=track.playing{eprintln!("media playing={} can_seek={}",track.playing,track.can_seek);}
            if !track.title.is_empty()&&(track.title!=self.track.title||track.artist!=self.track.artist)&&!self.stage1_now_playing_visible(){
                self.stage1_media_until=Some(Instant::now()+Duration::from_secs(5));
                if !self.lyric_mode{self.trigger_stage1_temporary(5);}
                self.sync_stage1_timer();
            }
            self.track_is_youtube = crate::youtube::session_allows_captions(&track.source_app, &track.title, &track.album);
            self.track=track;if changed{
                self.cancel_job("Musik");self.lyrics_key=key;self.lyrics_data=None;self.lyrics.clear();self.plain_lyrics.clear();self.lyric_offset=0;self.lyrics_error=None;
                self.lyrics_retry=Default::default();self.lyrics_from_youtube=false;
                self.last_lyric_line.clear();self.lyric_anim=None;
                self.request_lyrics(false);
            }
            if self.hover_stage==1 && !self.lyric_mode {
                let target=self.target_geometry();
                if (self.width-target.0).abs()>2.0||(self.height-target.1).abs()>2.0{
                    self.retarget_to(target);
                }
            }
            self.sync_visualizer();self.layout();self.clock_timer();},
        Event::Prayers(city,date,result)=>{if city==self.settings.prayer_city&&date==reminder::jakarta_today(){match result{Ok(s)=>{self.prayer_schedule=Some(s);self.prayer_error=None;},Err(e)=>self.prayer_error=Some(e)}}},
        Event::Artwork(key,result)=>{if key==self.cover_key&&self.settings.enabled("Musik"){self.cover=result.ok();if self.preview{eprintln!("artwork ready={}",self.cover.is_some());}}},
        Event::Levels(levels)=>{if self.visualizer.is_some(){self.levels=levels;self.sync_active_lyric();}},
        Event::Lyrics(permission,key,result)=>{if permission.check().is_ok()&&self.settings.lyrics_online&&key==self.lyrics_key&&self.settings.enabled("Musik"){match result{Ok(value)=>{
            self.lyrics_error=None;self.lyrics_retry=Default::default();self.lyrics_from_youtube=value.is_youtube;
            self.apply_lyrics(value);self.layout();},Err(_)=>{self.lyrics.clear();self.plain_lyrics.clear();self.lyrics_from_youtube=false;self.lyrics_retry.failed(Instant::now());self.lyrics_error=Some(crate::i18n::t("music.lyrics_error", &self.settings.language).to_string());self.clock_timer();self.layout();}}}},
        Event::Weather(result, is_manual)=>{if self.settings.enabled("Cuaca"){match result{Ok(value)=>{self.weather=Some(value);self.weather_updated=Some(Instant::now());self.weather_fetched_time=Some(chrono::Local::now().format("%H:%M").to_string());self.weather_refresh.failed=false;if is_manual{self.notice(crate::i18n::t("notice.weather_updated",&self.settings.language));}},Err(_)=>{self.weather_refresh.failed=true;self.notice(crate::i18n::t("weather.fetch_failed",&self.settings.language));}}}},
        Event::Agenda(result)=>{if self.settings.enabled("Kalender"){match result{Ok(value)=>{self.agenda=value;self.agenda_updated=Some(Instant::now());},Err(e)=>self.notice(e)}}},
        Event::Ai(result)=>{if self.settings.enabled("AI"){match result{Ok(text)=>self.ai_messages.push(crate::online::ChatMessage{role:"assistant".into(),content:text}),Err(e)=>self.notice(e)}}},
        Event::Update(result)=>{
            match result{
                Ok(Some(release))=>{
                    let identity = crate::update::BuildIdentity::current();
                    match crate::update::ui::select_release_candidate(&identity, &[release]) {
                        Ok(Some(candidate)) => {
                            let msg = format!("{} {}!", crate::i18n::t("notice.update_available", &self.settings.language), candidate.version);
                            self.notice(&msg);
                            self.update_status = UpdateStatus::Available(candidate);
                        }
                        _ => {
                            let msg = crate::i18n::t("notice.update_latest", &self.settings.language);
                            self.notice(msg);
                            self.update_status = UpdateStatus::UpToDate(chrono::Local::now().format("%H:%M").to_string());
                        }
                    }
                },
                Ok(None)=>{
                    self.update_status=UpdateStatus::UpToDate(chrono::Local::now().format("%H:%M").to_string());
                },
                Err(e)=>{
                    self.notice(&format!("{} {e}",crate::i18n::t("notice.update_check_failed", &self.settings.language)));
                    self.update_status=UpdateStatus::Error(e);
                }
            }
        },
        Event::UpdateProgress(percent, downloaded, total)=>{
            self.update_status = UpdateStatus::Downloading { percent, downloaded_bytes: downloaded, total_bytes: total };
            self.invalidate();
        },
        Event::UpdateStaged(result)=>{
            match result {
                Ok((version, tx_dir)) => {
                    self.update_status = UpdateStatus::AwaitingRestart { version, tx_dir };
                    let msg = format!("{}: {}", crate::i18n::t("update.ready", &self.settings.language), crate::i18n::t("update.restart_desc", &self.settings.language));
                    self.show_toast(ToastKind::UpdateReady, msg, 20000);
                    self.invalidate();
                }
                Err(e) => {
                    self.update_status = UpdateStatus::Error(e.clone());
                    self.notice(&format!("{}: {e}", crate::i18n::t("update.not_installed", &self.settings.language)));
                    self.invalidate();
                }
            }
        },Event::ModHttp(url,result)=>{
            self.mod_http_cache.insert(url,(Instant::now(),result));
            if self.mod_http_cache.len()>16{
                if let Some(oldest)=self.mod_http_cache.iter().min_by_key(|(_,(fetched_at,_))|*fetched_at).map(|(url,_)|url.clone()){
                    self.mod_http_cache.remove(&oldest);
                }
            }
            if self.active_mod_id().is_some(){self.refresh_mod_output();}
        },Event::Tws(result)=>{
            self.tws_pending=false;
            if let Ok(next)=result{
                let previous=self.tws.as_ref().map(|t|t.name.clone());
                let current=next.as_ref().map(|t|t.name.clone());
                let changed=previous!=current||self.tws.as_ref().and_then(|t|t.battery)!=next.as_ref().and_then(|t|t.battery);
                let announce=self.tws_ready;self.tws_ready=true;
                if announce&&previous!=current{
                    let lang=&self.settings.language;
                    if let Some(tws)=&next{let msg=crate::i18n::tws_connected(&tws.name,tws.battery,lang);self.notice(msg);}
                    else if let Some(name)=&previous{let msg=crate::i18n::tws_disconnected(name,lang);self.notice(msg);}
                }
                self.tws=next;
                if changed&&self.hover_stage==1{self.layout();}
                repaint=true;
            }
        },
        Event::Hardware(sample)=>{
            self.hardware_sample = Some(sample);
            if self.hover_stage == 1 {
                let target = self.target_geometry();
                if (self.width - target.0).abs() > 2.0 || (self.height - target.1).abs() > 2.0 {
                    self.retarget_to(target);
                }
            }
        },
        Event::Storage(pct, used, total, drives)=>{
            self.storage_job_started = None;
            self.cached_storage = (pct, used, total, drives.clone());
            self.power_snapshot.storage_usage = pct;
            self.power_snapshot.storage_used_gb = used;
            self.power_snapshot.storage_total_gb = total;
            self.power_snapshot.storage_drives = drives;
            if self.active_mod_id().as_deref() == Some("daya") {
                self.refresh_mod_output();
            }
        },Event::ProviderDownloadProgress(pct)=>{
            self.provider_download_progress=pct;
            self.invalidate();
        },
        Event::ProviderDownloadResult(res)=>{
            self.provider_downloading=false;
            self.confirm_provider_download=false;
            match res{
                Ok(msg)=>{
                    self.show_toast(ToastKind::Generic,msg.clone(),5000);
                    self.notice(&msg);
                    if self.is_mod_active("daya"){
                        self.power_sensors=Some(crate::system::power_sensor::PowerSensors::start());
                    }
                }
                Err(e)=>{
                    self.notice(&format!("Gagal mengunduh Power Provider: {e}"));
                    self.show_toast(ToastKind::Generic,format!("Unduh gagal: {e}"),6000);
                }
            }
            self.invalidate();
        },
        Event::Notice(text)=>{
            self.provider_downloading=false;
            self.confirm_provider_download=false;
            self.notice(text);
        },
    }}if repaint{self.reconcile_activities();self.invalidate();}}
    fn button_h(&mut self,p:&mut Painter,label:&str,x:f32,y:f32,width:f32,height:f32,action:Action,primary:bool){
        let hovered=self.pointer.is_some_and(|(px,py)|px>=x&&px<=x+width&&py>=y&&py<=y+height);
        let is_pressed=self.pressed.as_ref()==Some(&action);
        let is_focused=self.focused.as_ref()==Some(&action);
        let (bx,by,bw,bh)=if is_pressed{
            let dw=width*0.02;let dh=height*0.02;
            (x+dw/2.,y+dh/2.,width*0.98,height*0.98)
        }else{(x,y,width,height)};
        let radius=ui::RADIUS_CONTROL.min(height*0.35);
        unsafe{
            if primary {
                let fill_alpha = if is_pressed { 0.42 } else if hovered { 0.32 } else { 0.22 };
                p.fill_alpha(bx, by, bw, bh, radius, self.palette.accent, fill_alpha);
                p.stroke(bx, by, bw, bh, radius, self.palette.accent, 1.2);
                p.text_centered(label, bx, by, bw, bh, if height<26.{10.5}else{11.5}, self.palette.accent, 1.0, true);
            } else {
                let surface = if is_pressed { render::PRESSED } else if hovered { render::HOVER } else { render::DIM };
                p.fill(bx, by, bw, bh, radius, surface);
                p.text_centered(label, bx, by, bw, bh, if height<26.{10.5}else{11.5}, if hovered { self.palette.text_primary } else { self.palette.text_muted }, 1.0, false);
            }
        }
        if is_focused{unsafe{p.stroke(x-2.,y-2.,width+4.,height+4.,radius+2.,self.palette.accent,1.5);}}
        if self.hit_enabled{self.hits.push(Hit{rect:render::rect(x,y,width,height),action});}
    }
    /// Transport button in the approved Stage 2 style (mockup stage2-musik, "Hover — tanpa
    /// latar"): no background on hover, the icon brightens #9aa3ad -> #f4f4f6 and its stroke
    /// thickens; an active shuffle/repeat shows a 4 px dot under the icon; the lyrics button
    /// is a mic that turns white with a soft aura when active. Text labels (volume −/+) keep
    /// the older surface treatment because the mockup only restyles icon buttons.
    fn transport_btn(&mut self,p:&mut Painter,label:&str,x:f32,y:f32,w:f32,h:f32,action:Action,active:bool,enabled:bool){
        let hovered=enabled&&self.pointer.is_some_and(|(px,py)|px>=x&&px<=x+w&&py>=y&&py<=y+h);
        let is_pressed=enabled&&self.pressed.as_ref()==Some(&action);
        // Mockup tokens: default #9aa3ad -> hover #f4f4f6 (+stroke 1.8 -> 2.3).
        let color=if !enabled{0x555a5e}else if is_pressed{0x888888}else if hovered||active{0xf4f4f6}else{0x9aa3ad};
        let emphasis=hovered||active||is_pressed;
        unsafe{
            if self.focused.as_ref()==Some(&action){
                p.stroke(x-1.,y-1.,w+2.,h+2.,7.,0xd5d5dc,1.);
            }
            let cx=x+w*0.5;let cy=y+h*0.5;
            match label{
                "LYRICS"|"MIC"|"❝" => {
                    let size=if w<32.{17.}else{19.};
                    p.icon_mic(cx,cy,size,color,if emphasis{2.4}else{1.8},active);
                },
                "SHUFFLE" => {
                    p.icon_shuffle(cx,cy,18.,color,if emphasis{2.3}else{1.8});
                    if active{p.fill(cx-2.,y+h-6.,4.,4.,2.,color);}
                },
                "REPEAT" => {
                    p.icon_repeat(cx,cy,18.,color,if emphasis{2.3}else{1.8});
                    if active{p.fill(cx-2.,y+h-6.,4.,4.,2.,color);}
                },
                "⏮" => p.icon_prev(cx,cy,if emphasis{14.*1.1}else{14.},color),
                "⏭" => p.icon_next(cx,cy,if emphasis{14.*1.1}else{14.},color),
                // Bare play/pause glyphs (preset bar); the Stage 2 panel uses
                // spotify_play_button for the white disc instead.
                "▶" => p.icon_play(cx,cy,14.,color),
                "⏸"|"Ⅱ" => p.icon_pause(cx,cy,14.,color),
                _ => {
                    if hovered||active{
                        p.fill_alpha(x,y,w,h,6.,0xffffff,if active{0.22}else{0.10});
                    }
                    p.text(label,x,y+2.,w,h-4.,15.,color,true);
                },
            }
        }
        if enabled&&self.hit_enabled{self.hits.push(Hit{rect:render::rect(x-2.,y-2.,w+4.,h+4.),action});}
    }
    /// Play/pause as the mockup's white circle (40 px, dark 22 px glyph, hover scale 1.06,
    /// press scale .98). Replaces the old translucent disc of draw_glass_player.
    fn spotify_play_button(&mut self,p:&mut Painter,x:f32,y:f32,size:f32,playing:bool,enabled:bool){
        let action=Action::Play;
        let hovered=enabled&&self.pointer.is_some_and(|(px,py)|px>=x&&px<=x+size&&py>=y&&py<=y+size);
        let is_pressed=enabled&&self.pressed.as_ref()==Some(&action);
        let scale=if is_pressed{0.98}else if hovered{1.06}else{1.};
        let s=size*scale;let dx=x+(size-s)*0.5;let dy=y+(size-s)*0.5;
        unsafe{
            if enabled{
                // Soft shadow (mockup box-shadow 0 4px 14px rgba(0,0,0,.35)).
                p.fill_alpha(x-2.,y+1.,size+4.,size+2.,(size+3.)*0.5,0x000000,0.30);
            }
            p.fill(dx,dy,s,s,s*0.5,if enabled{0xf4f4f6}else{0x77777d});
            let cx=dx+s*0.5;let cy=dy+s*0.5;
            if playing{p.icon_pause(cx,cy,s*0.55,0x101015);}else{p.icon_play(cx,cy,s*0.55,0x101015);}
            if self.focused.as_ref()==Some(&action){
                p.stroke(x-2.,y-2.,size+4.,size+4.,size*0.5,0xd5d5dc,1.);
            }
        }
        if enabled&&self.hit_enabled{self.hits.push(Hit{rect:render::rect(x-2.,y-2.,size+4.,size+4.),action});}
    }
    /// Slot layout for the Stage 2 transport row (mockup order: [mic] … acak · prev · putar ·
    /// next · ulangi). Returns (offset,width) per slot for [mic,shuffle,prev,play,next,repeat]
    /// plus the full row width. Compact = the lyric-sheet left column (30 px buttons, 38 px
    /// play, 6 px gap); full = 34 px buttons, 40 px play, 10 px gap, mic placed separately.
    pub(super) fn spotify_transport_slots(compact:bool)->([(f32,f32);6],f32){
        if compact{
            ([(0.,30.),(36.,30.),(72.,30.),(108.,38.),(152.,30.),(188.,30.)],218.)
        }else{
            ([(0.,0.),(0.,34.),(44.,34.),(88.,40.),(138.,34.),(182.,34.)],216.)
        }
    }
    /// Draws one frame straight into the composition swap chain. There is no WM_PAINT round
    /// trip: state changes call this directly, which is what removes the coalescing jitter.
    /// Any draw or present error drops the painter so the next frame rebuilds the D3D11 stack.
    /// The guard is "suspended or not on screen", not "fullscreen": with hide-fullscreen off
    /// the notch stays visible over a game, and a resize during that state reallocates the
    /// swap chain buffers — skipping the repaint then leaves undefined (transparent) content.
    fn paint(&mut self){
        if self.suspended||self.painting{return}
        if !unsafe{IsWindowVisible(self.hwnd)}.as_bool(){return}
        if self.painter.is_none(){
            let pw=(ui::CANVAS_WIDTH*self.scale).max(1.)as u32;
            let ph=(ui::CANVAS_HEIGHT*self.scale).max(1.)as u32;
            self.painter=unsafe{Painter::new(self.hwnd,pw,ph,96.*self.scale,self.settings.software_renderer)}.ok();
        }
        let mut p_take = self.painter.take();
        if let Some(painter) = p_take.as_mut() {
            self.refresh_idle_clock_metrics(painter);
            // Measure the lyric geometry with the same DirectWrite pass the draw
            // uses, before target_geometry() reads the cached width. Using a
            // separate char-count heuristic here made some lines size differently
            // than their wrap decision and clip instead of wrapping.
            if self.is_lyric_active() {
                self.refresh_lyric_stretch_metrics(painter);
            }
        }
        self.painter = p_take;
        // Reconcile every geometry change through the same driver, including settings and
        // dynamic content changes that did not originate in a navigation action.
        let target=self.target_geometry();let target_ear=self.target_ear();
        let destination=self.morph.as_ref().map(|m|(m.to(),m.coordinator.borrow().ear.target))
            .or_else(||self.presented_contour.map(|c|((c.width,c.height,c.radius),c.ear)));
        if destination.is_some_and(|(g,ear)|[g.0-target.0,g.1-target.1,g.2-target.2,ear-target_ear].into_iter().any(|d|d.abs()>0.01)){
            self.retarget_to(target);
        }
        if let Some(mut p)=self.painter.take(){
            if !self.settings.enabled("Cuaca"){p.clear_weather_icons();}
            self.painting=true;
            // Owned by the guard: a panic in draw() below must not leave the flag set.
            let _painting_guard = PaintingGuard(std::ptr::addr_of_mut!(self.painting));
            let paint_started=std::time::Instant::now();
            let ok=unsafe{
                let mut ok=p.begin().is_ok();
                if ok{
                    self.hits.clear();
                    let(width,height,radius)=self.geometry();
                    self.width=width;
                    self.height=height;
                    let attached=!self.settings.bottom&&self.settings.edge_gap==0;
                    let ear=if attached{self.morph.as_ref().map(|m|m.coordinator.borrow().ear.current).unwrap_or_else(||self.target_ear())}else{0.};
                    self.frame_contour=Some(glass::FrameContour{width,height,ear,radius,round_top:!attached,origin_dip:((ui::CANVAS_WIDTH-width)/2.,ui::panel_origin_y(self.settings.bottom,height,self.scale)),scale:self.scale});
                    self.glass.warp=p.is_warp;
                    if !p.host_backdrop_supported(){self.glass.failed.set(true);}
                    self.sync_glass();
                    let glass_cfg=self.glass_config(height).filter(|_|self.glass_active());
                    if glass_cfg.is_some(){self.apply_glass_region();}
                    if p.sync_backdrop(self.frame_contour.unwrap().key(),glass_cfg.is_some()).is_err(){self.glass.failed.set(true);let _=p.sync_backdrop([0.;8],false);}
                    let(ox,oy)=self.capsule_origin();
                    let body_bg=if self.settings_open||self.tab=="Reminder"{if self.palette.background != 0 { self.palette.background } else { 0x060608 }}else{self.palette.background};
                    // Drop shadow blooms when hovered in Stage 1 or expanded in Stage 2
                    if glass_cfg.is_none() && self.hover_stage >= 1 && (self.pointer.is_some() || self.expanded) {
                        let opacity=(self.settings.opacity as f32/255.)*self.startup_alpha;
                        let shadow_accent=if !self.settings_open&&(!self.settings.quiet_mode&&self.settings.effective_presentation().glow)&&(self.tab=="Musik"||self.lyric_mode)&&self.track.playing{
                            self.cover.as_ref().filter(|_|!self.settings.quiet_mode).map(|c|c.accent).unwrap_or(0x000000)
                        }else{
                            0x000000
                        };
                        for ring in (1..=12).rev(){
                            let spread=ring as f32*1.0;
                            p.fill_alpha(ox-spread,oy-spread+3.,width+spread*2.,height+spread*2.,radius+spread,0x000000,0.0125*opacity);
                            if shadow_accent!=0x000000&&self.expanded{
                                p.fill_alpha(ox-spread*1.2,oy-spread*1.2+4.,width+spread*2.4,height+spread*2.4,radius+spread*1.2,shadow_accent,0.006*opacity);
                            }
                        }
                    }
                    // Everything below draws in capsule coordinates; the origin is translated to (ox, oy).
                    p.push_alpha((self.settings.opacity as f32/255.)*self.startup_alpha);
                    // F1 boot: "turun kecil" — negatif berarti masih di atas posisi akhir,
                    // terpotong mulus oleh tepi atas layar lalu turun ke posisi final.
                    p.translate(ox,oy+self.boot_drop_offset());
                    let is_attached_top=!self.settings.bottom&&self.settings.edge_gap==0;
                    let mut glass_drawn=false;
                    if let Some(cfg)=&glass_cfg{
                        // One contour (material, alpha mask, outline); the DWM blur behind it is
                        // clipped by the region built from the same contour.
                        let(_,_,gear,gradius,round_top,_)=self.contour_params();
                        glass_drawn=p.fill_notch_glass(width,height,gear,gradius,round_top,cfg,self.glass.hover.sample());
                        if glass_drawn&&!self.settings_open&&(!self.settings.quiet_mode&&self.settings.effective_presentation().glow)&&self.track.playing&&(self.lyric_mode||(self.expanded&&self.tab=="Musik")){
                            let accent=self.cover.as_ref().filter(|_|!self.settings.quiet_mode).map(|c|c.accent).unwrap_or(0x1a2e3b);
                            if round_top{p.fill_alpha(0.,0.,width,height,radius,accent,0.12);}else{p.fill_notch_gradient(width,height,gear,gradius,0x000000,accent,0.0,0.22);}
                        }
                    }
                    if glass_drawn{
                    }else if is_attached_top{                        let ear = match &self.morph {
                            Some(m) => m.ear(),
                            None => self.target_ear(),
                        };
                        // Cover the physical screen-edge row as well as the antialiased contour.
                        // This avoids a transparent hairline at the clipped HWND/DComp boundary.
                        p.fill(-ear,-1./self.scale,width+2.*ear,2./self.scale,0.,body_bg);
                        if !self.settings_open&&(!self.settings.quiet_mode&&self.settings.effective_presentation().glow)&&self.hover_stage>=2 && self.tab=="Musik"{
                            let accent=self.cover.as_ref().filter(|_|!self.settings.quiet_mode).map(|c|c.accent).unwrap_or(0x1a2e3b);
                            if self.lyric_mode{
                                p.fill_notch_half_gradient(width,height,ear,radius,accent,0.85);
                            }else if self.track.playing{
                                p.fill_notch(width,height,ear,radius,body_bg);
                                p.fill_notch_gradient(width,height,ear,radius,0x000000,accent,0.0,0.22);
                            }else{
                                p.fill_notch(width,height,ear,radius,body_bg);
                            }
                        }else{
                            p.fill_notch(width,height,ear,radius,body_bg);
                        }
                    }else{
                        p.fill(0.,0.,width,height,radius,body_bg);
                        if !self.settings_open&&(!self.settings.quiet_mode&&self.settings.effective_presentation().glow)&&self.expanded&&self.tab=="Musik"{let accent=self.cover.as_ref().filter(|_|!self.settings.quiet_mode).map(|c|c.accent).unwrap_or(0x1a2e3b);p.fill_alpha(0.,0.,width,height,radius,accent,0.12);}
                    }
                    // F2 sweep: tandai ulang tiap frame — bila draw() tidak menampilkan
                    // pill aksen (live tetap false), anchor dikosongkan sehingga timer
                    // morph mati pada tick berikutnya.
                    self.activity_sweep_live=false;
                    self.draw(&mut p);
                    if !self.activity_sweep_live{self.activity_sweep=None;}
                    p.reset_transform();
                    p.pop_alpha();
                    ok=p.end().is_ok();
                }
                ok
            };
            if ok{self.presented_contour=self.frame_contour;}
            self.frame_contour=None;
            if self.preview{let elapsed=paint_started.elapsed();if elapsed.as_millis()>10{eprintln!("[paint] {:.1}ms",elapsed.as_secs_f64()*1000.);}}
            if let Some(b)=&self.glass.bench{b.borrow_mut().frame(paint_started.elapsed().as_secs_f64()*1000.,self.height,if self.glass_active(){glass::MaterialMode::from_str(&self.settings.effective_presentation().material).as_str()}else{"solid"},match self.glass.backdrop.get(){glass::Backdrop::Off=>"off",glass::Backdrop::Blur=>"blur",glass::Backdrop::Acrylic=>"acrylic"},self.glass.region_sets.get());}
            if !ok{self.render_ready=false;self.pending_ack_ready_at=None;self.painter=None;if self.preview{eprintln!("renderer dropped (device lost?) — rebuilding next frame");}}else{self.render_ready=true;self.painter=Some(p);}
        }
    }
    fn draw(&mut self,p:&mut Painter){
        let (collapsed_alpha, expanded_alpha) = match &self.morph {
            None => {
                if self.expanded { (0.0, 1.0) } else { (1.0, 0.0) }
            }
            Some(morph) => {
                let progress = morph.raw(); // always 0.0 at start of animation, 1.0 at end
                if morph.is_inter_tab() {
                    // Switching between tabs in Stage 2:
                    // Panel stays 100% solid expanded; no Stage 1 collapsed content ever drawn!
                    (0.0, 1.0)
                } else if self.expanded {
                    // OPENING transition (progress: 0.0 -> 1.0)
                    // Collapsed content fades out quickly in the first 18% of travel (0.0 -> 0.18)
                    let c_alpha = (1.0 - progress / 0.18).clamp(0.0, 1.0);
                    // Expanded content fades in smoothly as panel grows (0.15 -> 0.75)
                    let e_alpha = ((progress - 0.15) / 0.60).clamp(0.0, 1.0);
                    (c_alpha, e_alpha)
                } else if morph.is_horizontal() {
                    // Intra-compact horizontal resize (e.g. lyric width adaptation).
                    // NEVER fade out! Collapsed content stays 100% solid throughout the morph, eliminating flicker.
                    (1.0, 0.0)
                } else if morph.to().1 < 100.0 && morph.from().1 < 100.0 {
                    // Compact Stage-1 vertical change (e.g. 1-line <-> 2-line lyric,
                    // 30 <-> 50 DIP). This is NOT a stage change: never fade expanded
                    // content in (that briefly painted the Stage 2 Music tab and read
                    // as a background flicker). Keep collapsed content fully solid and
                    // animate only the geometry.
                    (1.0, 0.0)
                } else {
                    // CLOSING transition (progress: 0.0 -> 1.0)
                    // Expanded content fades out IMMEDIATELY and cleanly in the first 18% of return (0.0 -> 0.18)
                    let e_alpha = (1.0 - progress / 0.18).clamp(0.0, 1.0);
                    // Collapsed content fades in as panel nears final compact size (0.65 -> 1.0)
                    let c_alpha = ((progress - 0.65) / 0.35).clamp(0.0, 1.0);
                    (c_alpha, e_alpha)
                }
            }
        };
        self.hit_enabled = self.morph.is_none();
        if self.hover_stage == 0 && !self.expanded {
            unsafe { self.draw_privacy_stage0(p, self.width, self.height); }
        }
        if collapsed_alpha > 0.002 { self.draw_collapsed(p, collapsed_alpha); }
        if expanded_alpha > 0.002 { self.draw_expanded(p, expanded_alpha); }
    }
    fn draw_toast_content(&mut self, p: &mut Painter, toast: &ToastNotification) {
        unsafe {
            let cy = self.height * 0.5;
            if toast.kind == ToastKind::Water {
                // Vibrant blue water icon: 0xd8d8de
                p.text_centered("💧", 10., 0., 18., self.height, 13., 0xd8d8de, 1.0, false);
                if toast.confirmed {
                    let text_w = self.width - 56.;
                    let hydration_txt = crate::i18n::t("toast.hydration_logged", &self.settings.language);
                    p.text_centered(hydration_txt, 28., 0., text_w, self.height, 11.5, 0xd8d8de, 1.0, true);
                    p.text_centered("🎉", self.width - 28., 0., 20., self.height, 13., 0xffffff, 1.0, false);
                } else {
                    let btn_w = 26.;
                    let btn_h = 20.;
                    let btn_x = self.width - 10. - btn_w;
                    let btn_y = cy - btn_h * 0.5;
                    let text_x = 30.;
                    let text_w = btn_x - 6. - text_x;
                    let label = if toast.message.starts_with("💧 ") {
                        &toast.message[4..]
                    } else {
                        &toast.message
                    };
                    p.text_centered(label, text_x, 0., text_w, self.height, 11.5, 0xffffff, 1.0, true);

                    // Checklist confirmation button: [ ✓ ]
                    let hovered = self.pointer.is_some_and(|(px, py)| px >= btn_x && px <= btn_x + btn_w && py >= btn_y && py <= btn_y + btn_h);
                    let btn_bg = if hovered { 0x0284c7 } else { 0x0369a1 };
                    p.fill_alpha(btn_x, btn_y, btn_w, btn_h, btn_h * 0.5, btn_bg, 0.85);
                    p.stroke(btn_x, btn_y, btn_w, btn_h, btn_h * 0.5, 0xd8d8de, 1.2);
                    p.text_centered("✓", btn_x, btn_y, btn_w, btn_h, 12., 0xffffff, 1.0, true);

                    if self.hit_enabled {
                        self.hits.push(Hit {
                            rect: render::rect(btn_x - 4., btn_y - 2., btn_w + 8., btn_h + 4.),
                            action: Action::ConfirmWater,
                        });
                    }
                }
            } else if toast.kind == ToastKind::UpdateReady {
                let btn_w = 26.;
                let btn_h = 20.;
                let btn_y = cy - btn_h * 0.5;
                let btn2_x = self.width - 10. - btn_w;
                let btn1_x = btn2_x - 6. - btn_w;
                let text_x = 10.;
                let text_w = btn1_x - 6. - text_x;
                p.text_centered(&toast.message, text_x, 0., text_w, self.height, 10.5, 0xffffff, 1.0, true);

                // Restart button (↻)
                let h1 = self.pointer.is_some_and(|(px, py)| px >= btn1_x && px <= btn1_x + btn_w && py >= btn_y && py <= btn_y + btn_h);
                let bg1 = if h1 { 0x0284c7 } else { 0x0369a1 };
                p.fill_alpha(btn1_x, btn_y, btn_w, btn_h, btn_h * 0.5, bg1, 0.85);
                p.stroke(btn1_x, btn_y, btn_w, btn_h, btn_h * 0.5, 0xd8d8de, 1.2);
                p.text_centered("↻", btn1_x, btn_y, btn_w, btn_h, 12., 0xffffff, 1.0, true);

                // Later button (✕)
                let h2 = self.pointer.is_some_and(|(px, py)| px >= btn2_x && px <= btn2_x + btn_w && py >= btn_y && py <= btn_y + btn_h);
                let bg2 = if h2 { 0x475569 } else { 0x334155 };
                p.fill_alpha(btn2_x, btn_y, btn_w, btn_h, btn_h * 0.5, bg2, 0.85);
                p.stroke(btn2_x, btn_y, btn_w, btn_h, btn_h * 0.5, 0x94a3b8, 1.2);
                p.text_centered("✕", btn2_x, btn_y, btn_w, btn_h, 11., 0xffffff, 1.0, true);

                if self.hit_enabled {
                    self.hits.push(Hit {
                        rect: render::rect(btn1_x - 4., btn_y - 2., btn_w + 8., btn_h + 4.),
                        action: Action::RestartNow,
                    });
                    self.hits.push(Hit {
                        rect: render::rect(btn2_x - 4., btn_y - 2., btn_w + 8., btn_h + 4.),
                        action: Action::RestartLater,
                    });
                }
            } else {
                p.text_centered(&toast.message, 0., 0., self.width, self.height, 11.5, 0xffffff, 1.0, true);
            }
        }
    }

    fn draw_idle_clock(&mut self, p: &mut Painter) {
        unsafe {
            p.clip(0., 0., self.width, self.height);

            let now = Local::now();
            let today = now.date_naive();
            let month_str = crate::i18n::month_name(today.month(), &self.settings.language);
            let date_text = format!("{}, {} {}", crate::i18n::weekday_short(today.weekday().num_days_from_monday(), &self.settings.language), today.day(), month_str);
            let time_str = now.format("%H:%M").to_string();

            let power = ui::PowerPresentation::from_battery(self.battery.as_ref());
            let tws = ui::TwsPresentation::from_tws(self.tws.as_ref());

            let date_w = p.measure_text(&date_text, ui::STAGE1_TEXT_SIZE, true);
            // Slack 8 px: format baru dengan nama hari lebih lebar — tanpa slack, plan_idle_clock
            // memotong tanggal tepat di batas lebar (date ter-ellipsis "8 N...").
            let date_w = date_w + 8.0;
            let time_w = p.measure_text(&time_str, ui::STAGE1_TEXT_SIZE, true);

            let (power_glyph_w, power_percent_w) = match power {
                ui::PowerPresentation::Charging(pct) => {
                    let lbl = format!("{pct}%");
                    (ui::STAGE1_BOLT_W, p.measure_text(&lbl, ui::STAGE1_TEXT_SIZE, true))
                }
                ui::PowerPresentation::Low(pct) => {
                    let lbl = format!("{pct}%");
                    (p.measure_text("🪫", ui::STAGE1_TEXT_SIZE, false), p.measure_text(&lbl, ui::STAGE1_TEXT_SIZE, true))
                }
                ui::PowerPresentation::Normal(pct) => {
                    let lbl = format!("{pct}%");
                    (p.measure_text("🔋", ui::STAGE1_TEXT_SIZE, false), p.measure_text(&lbl, ui::STAGE1_TEXT_SIZE, true))
                }
                ui::PowerPresentation::Unavailable => (0., 0.),
            };

            let (tws_glyph_w, tws_label_w) = match tws {
                ui::TwsPresentation::Known(pct) => {
                    let lbl = format!("{pct}%");
                    (p.measure_text("🎧", ui::STAGE1_TEXT_SIZE, false), p.measure_text(&lbl, ui::STAGE1_TEXT_SIZE, true))
                }
                ui::TwsPresentation::Unknown => {
                    (p.measure_text("🎧", ui::STAGE1_TEXT_SIZE, false), p.measure_text("—", ui::STAGE1_TEXT_SIZE, true))
                }
                ui::TwsPresentation::Absent => (0., 0.),
            };

            let privacy_rail_w = if self.settings.privacy_enabled && self.privacy_snapshot.any_active() {
                crate::shell::privacy::compute_privacy_rail_width(&self.privacy_snapshot)
            } else {
                0.
            };
            let preset_active = self.settings.quiet_mode || self.settings.power_saver_mode;

            // While settling into a compact Stage 1 pill, clamp the layout size to the
            // target so the spring's undershoot can never make the pill narrower than
            // the final width. That undershoot used to flip the date to the trimmed
            // form ("6 No…") for a frame before snapping to "6 Nov".
            let (layout_w, layout_h) = match &self.morph {
                Some(m) if m.to().1 < 100.0 => (self.width.max(m.to().0), self.height.max(m.to().1)),
                _ => (self.width, self.height),
            };

            let plan = ui::plan_idle_clock(
                layout_w,
                layout_h,
                &date_text,
                date_w,
                &time_str,
                time_w,
                power,
                power_glyph_w,
                power_percent_w,
                tws,
                tws_glyph_w,
                tws_label_w,
                privacy_rail_w,
                preset_active,
            );

            // 1. Date text (with ellipsis if trimmed)
            p.text_centered(&date_text, plan.date_rect.0, plan.date_rect.1, plan.date_rect.2, plan.date_rect.3, ui::STAGE1_TEXT_SIZE, 0xececec, 1., true);

            // 3. Separator
            p.fill_alpha(plan.separator_rect.0, plan.separator_rect.1, plan.separator_rect.2, plan.separator_rect.3, 0.7, 0x4a4f57, 1.);

            // 4. Time text
            p.text_centered(&time_str, plan.time_rect.0, plan.time_rect.1, plan.time_rect.2, plan.time_rect.3, ui::STAGE1_TEXT_SIZE, 0xececec, 1., true);

            // 5. TWS group
            if let Some((cx, cy, r)) = plan.tws_dot {
                p.fill(cx - r, cy - r, r * 2., r * 2., r, 0xffffff);
            }
            if let Some(rect) = plan.tws_glyph_rect {
                p.text_centered("🎧", rect.0, rect.1, rect.2, rect.3, ui::STAGE1_TEXT_SIZE, 0xececec, 1., false);
            }
            if let Some(rect) = plan.tws_label_rect {
                p.text_centered(&plan.tws_label, rect.0, rect.1, rect.2, rect.3, ui::STAGE1_TEXT_SIZE, 0xececec, 1., true);
            }

            // 6. Power group
            if let Some((cx, cy, r)) = plan.power_dot {
                p.fill(cx - r, cy - r, r * 2., r * 2., r, 0xffffff);
            }
            if let Some(rect) = plan.power_bolt_rect {
                let bx = rect.0;
                let by = rect.1;
                let bw = rect.2;
                let bh = rect.3;
                p.draw_triangle((bx + bw * 0.85, by), (bx + bw * 0.2, by + bh * 0.58), (bx + bw * 0.55, by + bh * 0.58), 0xf98a47);
                p.draw_triangle((bx + bw * 0.2, by + bh * 0.58), (bx + bw * 0.55, by + bh * 0.58), (bx + bw * 0.15, by + bh), 0xf98a47);
            }
            if let Some(rect) = plan.power_glyph_rect {
                p.text_centered(plan.power_glyph, rect.0, rect.1, rect.2, rect.3, ui::STAGE1_TEXT_SIZE, 0xececec, 1., false);
            }
            if let Some(rect) = plan.power_label_rect {
                p.text_centered(&plan.power_label, rect.0, rect.1, rect.2, rect.3, ui::STAGE1_TEXT_SIZE, 0xececec, 1., true);
            }

            // 7. Privacy rail slot
            if let Some(rect) = plan.privacy_slot {
                self.draw_privacy_rail(p, rect.0, rect.1, rect.2, rect.3);
            }

            // 8. Preset indicator slot
            if let Some(rect) = plan.preset_slot {
                self.preset_indicator(p, rect.0, rect.1);
            }

            // 9. Chevron up button
            self.button_chevron_up(p, plan.chevron_hit.0, plan.chevron_hit.1, plan.chevron_hit.2, plan.chevron_hit.3, Action::ToStage0, 0x88888e);

            // 10. Calendar body hit
            if self.hit_enabled {
                if let Some(hit_rect) = plan.calendar_hit {
                    if let Some(s) = &self.activities.runtime.snapshot {
                        let open = Action::OpenActivity(s.primary.id.clone(), s.revision);
                        self.hits.push(Hit {
                            rect: render::rect(hit_rect.0, hit_rect.1, hit_rect.2, hit_rect.3),
                            action: open,
                        });
                    }
                }
            }

            p.unclip();
        }
    }

    fn draw_lyric_stretch(&mut self, p: &mut Painter) {
        unsafe {
            p.clip(0., 0., self.width, self.height);

            let text = self.current_lyric_line().unwrap_or_default();
            let wrapped = ui::wrap_lyric_lines(&text, 346.0, |s| p.measure_text(s, 12.0, true));
            let radius = if wrapped.is_multiline { 16.0 } else { 15.0 };
            self.cached_lyric_geometry = (wrapped.target_width, wrapped.target_height, radius);

            let now = Local::now();
            let time_str = now.format("%H:%M").to_string();
            let time_w = p.measure_text(&time_str, ui::STAGE1_TEXT_SIZE, true);

            let privacy_rail_w = if self.settings.privacy_enabled && self.privacy_snapshot.any_active() {
                crate::shell::privacy::compute_privacy_rail_width(&self.privacy_snapshot)
            } else {
                0.
            };
            let preset_active = self.settings.quiet_mode || self.settings.power_saver_mode;

            let plan = ui::plan_lyric_stretch(
                self.width,
                self.height,
                time_w,
                privacy_rail_w,
                preset_active,
                &wrapped,
            );

            // 1. Slot Kiri: Ikon musik (mint #2fd4a3) atau mini visualizer + pemisah vertikal
            if self.settings.effective_presentation().visualizer && self.track.playing {
                let levels = if self.levels.len() >= 4 { &self.levels[0..4] } else { &[0.4, 0.8, 1.0, 0.6] };
                p.bars_mirror(levels, plan.music_icon_rect.0, (self.height - 12.) * 0.5, 14., 12., 0xd8d8de);
            } else {
                p.outline_icon("music", plan.music_icon_rect.0, plan.music_icon_rect.1, plan.music_icon_rect.2, 0xd8d8de);
            }

            // Pemisah vertikal tipis
            p.fill_alpha(plan.separator_rect.0, plan.separator_rect.1, plan.separator_rect.2, plan.separator_rect.3, 0.7, 0x4a4f57, 1.);

            // 2. Slot Kanan: Chevron di width - 24, Jam di rata kanan sebelum chevron
            self.button_chevron_up(p, plan.chevron_hit.0, plan.chevron_hit.1, plan.chevron_hit.2, plan.chevron_hit.3, Action::ToStage0, 0x88888e);

            p.text_centered(&time_str, plan.time_rect.0, plan.time_rect.1, plan.time_rect.2, plan.time_rect.3, ui::STAGE1_TEXT_SIZE, 0xffffff, 1., true);

            if let Some(rect) = plan.preset_slot {
                self.preset_indicator(p, rect.0, rect.1);
            }

            if let Some(rect) = plan.privacy_slot {
                self.draw_privacy_rail(p, rect.0, rect.1, rect.2, rect.3);
            }

            // 3. Slot Tengah: Area teks lirik aktif (Slide-Up vertikal, NO horizontal marquee)
            p.clip(plan.lyric_area_rect.0, 0., plan.lyric_area_rect.2, self.height);
            let reduced = self.settings.effective_presentation().reduced_motion;

            if reduced || self.lyric_anim.is_none() {
                if plan.is_multiline {
                    p.text_centered(&plan.line1, plan.lyric_area_rect.0, 8., plan.lyric_area_rect.2, 18., 12.0, 0xffffff, 1., true);
                    if let Some(l2) = &plan.line2 {
                        p.text_centered(l2, plan.lyric_area_rect.0, 26., plan.lyric_area_rect.2, 17., 11.5, 0xe2e8f0, 1., true);
                    }
                } else {
                    p.text_centered(&plan.line1, plan.lyric_area_rect.0, (self.height - 20.) * 0.5, plan.lyric_area_rect.2, 20., 12.0, 0xffffff, 1., true);
                }
            } else if let Some(anim) = &self.lyric_anim {
                let elapsed = anim.started.elapsed().as_secs_f32();
                let t = (elapsed / 0.280).clamp(0.0, 1.0);
                let ease = 1.0 - (1.0 - t).powi(3);

                let enter_offset_y = 10.0 * (1.0 - ease);
                let enter_alpha = ease;
                let exit_offset_y = -10.0 * ease;
                let exit_alpha = (1.0 - ease).max(0.0);

                // Baris lama meluncur ke atas dan memudar
                if exit_alpha > 0.01 {
                    let prev_wrapped = ui::wrap_lyric_lines(&anim.prev_text, 346.0, |s| p.measure_text(s, 12.0, true));
                    p.push_alpha(exit_alpha);
                    if prev_wrapped.is_multiline {
                        p.text_centered(&prev_wrapped.line1, plan.lyric_area_rect.0, 8. + exit_offset_y, plan.lyric_area_rect.2, 18., 12.0, 0xffffff, 1., true);
                        if let Some(l2) = &prev_wrapped.line2 {
                            p.text_centered(l2, plan.lyric_area_rect.0, 26. + exit_offset_y, plan.lyric_area_rect.2, 17., 11.5, 0xe2e8f0, 1., true);
                        }
                    } else {
                        p.text_centered(&prev_wrapped.line1, plan.lyric_area_rect.0, (self.height - 20.) * 0.5 + exit_offset_y, plan.lyric_area_rect.2, 20., 12.0, 0xffffff, 1., true);
                    }
                    p.pop_alpha();
                }

                // Baris baru meluncur masuk dari bawah ke atas dan menguat
                p.push_alpha(enter_alpha);
                if plan.is_multiline {
                    p.text_centered(&plan.line1, plan.lyric_area_rect.0, 8. + enter_offset_y, plan.lyric_area_rect.2, 18., 12.0, 0xffffff, 1., true);
                    if let Some(l2) = &plan.line2 {
                        p.text_centered(l2, plan.lyric_area_rect.0, 26. + enter_offset_y, plan.lyric_area_rect.2, 17., 11.5, 0xe2e8f0, 1., true);
                    }
                } else {
                    p.text_centered(&plan.line1, plan.lyric_area_rect.0, (self.height - 20.) * 0.5 + enter_offset_y, plan.lyric_area_rect.2, 20., 12.0, 0xffffff, 1., true);
                }
                p.pop_alpha();
            }
            p.unclip();

            // 4. Whole lyric pill opens the Music tab in Stage 2 (chevron excluded so it can still collapse).
            if self.hit_enabled {
                self.hits.push(Hit {
                    rect: render::rect(plan.music_icon_rect.0, 0., (plan.chevron_hit.0 - plan.music_icon_rect.0).max(10.), self.height),
                    action: Action::ExpandMusic,
                });
            }

            p.unclip();
        }
    }

    /// Short Stage 1 transient shown right after a track change; reverts to the
    /// idle clock when `stage1_media_until` expires. Single-line status pill:
    /// source icon, "Title · Artist", clock footer, chevron.
    fn draw_now_playing(&mut self, p: &mut Painter) {
        let Some((title, artist)) = self.stage1_now_playing() else { return; };
        let info = match artist {
            Some(a) => format!("{} · {}", title, a),
            None => title,
        };
        let time_str = Local::now().format("%H:%M").to_string();
        let btn_up_x = self.width - 24.0;
        unsafe { p.clip(0., 0., self.width, self.height); }
        let info_x = self.draw_stage1_status(p, &info, &time_str, btn_up_x);
        unsafe {
            p.icon_media_source((info_x - 12.).max(10.), self.height * 0.5, 20., &self.track.source_app, self.track.playing);
            self.button_chevron_up(p, btn_up_x, (self.height - 20.) * 0.5, 20., 20., Action::ToStage0, 0x88888e);
        }
        if self.hit_enabled {
            self.hits.push(Hit {
                rect: render::rect(10., 0., (btn_up_x - 10.).max(10.), self.height),
                action: Action::ExpandMusic,
            });
        }
        unsafe { p.unclip(); }
    }

    fn draw_collapsed(&mut self, p: &mut Painter, alpha: f32) {
        if self.should_show_weather_stage1() {
            unsafe { p.push_alpha(alpha); }
            self.draw_weather_stage1(p, 0.);
            unsafe { p.pop_alpha(); }
            return;
        }
        let hover_stage = self.hover_stage;
        let expanded = self.expanded;
        let quiet_mode = self.settings.quiet_mode;
        let toast = self.toast.clone();
        let snapshot = self.activities.runtime.snapshot.clone();
        let lyric_active = self.is_lyric_active();
        let now_playing = self.stage1_now_playing_visible();

        let hw_monitor_active = self.settings.hw_monitor_enabled;
        let mut sink = NativeCollapsedSink { shell: self, p };
        dispatch_collapsed(
            &mut sink,
            alpha,
            hover_stage,
            expanded,
            toast.as_ref(),
            quiet_mode,
            snapshot.as_ref(),
            lyric_active,
            now_playing,
            hw_monitor_active,
        );
    }
    fn tab_pill(&mut self,p:&mut Painter,icon:&str,label:&str,x:f32,y:f32,w:f32,h:f32,action:Action,selected:bool){
        let hovered=self.pointer.is_some_and(|(px,py)|px>=x&&px<=x+w&&py>=y&&py<=y+h);
        unsafe{
            if selected {
                // Neutral (translucent white / grey) active tab — no brand accent.
                p.fill_alpha(x,y,w,h,h*0.5,0xffffff, 0.12);
                p.stroke(x,y,w,h,h*0.5,0x50505a, 1.0);
            } else if hovered {
                p.fill(x,y,w,h,h*0.5,if self.palette.card_bg != 0x18181b { self.palette.card_bg } else { 0x1c1d22 });
            }
            let rgb = if selected {
                self.palette.text_primary
            } else if hovered {
                self.palette.text_primary
            } else {
                self.palette.text_muted
            };
            if selected&&w>40.&&!label.is_empty(){
                let text_w=p.measure_text(label,11.,false);let start=x+(w-15.-6.-text_w)/2.;
                p.outline_icon(icon,start,y+(h-15.)/2.,15.,rgb);
                p.text_centered(label,start+21.,y,text_w,h,11.,rgb,1.,false);
            }else{p.outline_icon(icon,x+(w-15.)/2.,y+(h-15.)/2.,15.,rgb);}
            if self.focused.as_ref()==Some(&action){p.stroke(x-1.,y-1.,w+2.,h+2.,h*0.5,self.palette.accent,1.);}
        }
        if self.hit_enabled{self.hits.push(Hit{rect:render::rect(x,y,w,h),action});}
    }
    fn navigation_control(&mut self,p:&mut Painter,icon:&str,x:f32,y:f32,action:Action,active:bool){unsafe{
        let hovered=self.pointer.is_some_and(|(a,b)|a>=x&&a<=x+24.&&b>=y&&b<=y+26.);
        if hovered||active{p.fill(x,y,24.,26.,12.,if self.palette.card_bg != 0x18181b { self.palette.card_bg } else { 0x252528 });}
        p.outline_icon(icon,x+5.,y+6.,14.,if active{self.palette.accent}else if hovered{self.palette.text_primary}else{self.palette.text_muted});
        if self.focused.as_ref()==Some(&action){p.stroke(x,y,24.,26.,12.,self.palette.accent,1.);}
        if self.hit_enabled{self.hits.push(Hit{rect:render::rect(x,y,24.,26.),action});}
    }}
    fn button_icon(&mut self,p:&mut Painter,icon:&str,x:f32,y:f32,w:f32,h:f32,action:Action){
        self.button_icon_colored(p,icon,x,y,w,h,action,0x8a9291);
    }
    /// Draws an actual outline icon (not text) inside a pill-shaped hit target.
    fn icon_button(&mut self,p:&mut Painter,icon:&str,x:f32,y:f32,w:f32,h:f32,action:Action,active:bool){
        let hovered=self.pointer.is_some_and(|(px,py)|px>=x&&px<=x+w&&py>=y&&py<=y+h);
        let pressed=self.pressed.as_ref()==Some(&action);
        unsafe{
            if hovered||pressed||active{
                p.fill_alpha(x,y,w,h,h/2.,0xffffff,if pressed{0.25}else if active{0.20}else{0.12});
            }
            if active{p.stroke(x-1.,y-1.,w+2.,h+2.,h/2.+1.,0x50505a,1.);}
            p.outline_icon(icon,x+(w-14.)/2.,y+(h-14.)/2.,14.,if hovered||active{0xffffff}else{0x8a9291});
        }
        if self.hit_enabled{self.hits.push(Hit{rect:render::rect(x,y,w,h),action});}
    }
    fn button_icon_colored(&mut self,p:&mut Painter,icon:&str,x:f32,y:f32,w:f32,h:f32,action:Action,fg_rgb:u32){
        let hovered=self.pointer.is_some_and(|(px,py)|px>=x&&px<=x+w&&py>=y&&py<=y+h);
        let is_pressed=self.pressed.as_ref()==Some(&action);
        let radius=h/2.0;
        unsafe{
            if hovered||is_pressed{
                p.fill_alpha(x,y,w,h,radius,0xffffff,if is_pressed{0.25}else{0.12});
            }
            if self.focused.as_ref()==Some(&action){
                p.stroke(x-1.,y-1.,w+2.,h+2.,radius+1.,0xd5d5dc,1.);
            }
            p.text_centered(icon,x,y,w,h,12.,if hovered{0xffffff}else{fg_rgb},1.0,false);
        }
        if self.hit_enabled{self.hits.push(Hit{rect:render::rect(x,y,w,h),action});}
    }
    fn button_chevron_up(&mut self,p:&mut Painter,x:f32,y:f32,w:f32,h:f32,action:Action,fg_rgb:u32){
        let hovered=self.pointer.is_some_and(|(px,py)|px>=x&&px<=x+w&&py>=y&&py<=y+h);
        let is_pressed=self.pressed.as_ref()==Some(&action);
        let radius=if (w-h).abs()<1.0{w*0.5}else{4.0};
        unsafe{
            if hovered||is_pressed{
                p.fill_alpha(x,y,w,h,radius,0xffffff,if is_pressed{0.25}else{0.14});
            }
            if self.focused.as_ref()==Some(&action){
                p.stroke(x-1.,y-1.,w+2.,h+2.,radius+1.,0xd5d5dc,1.);
            }
            let color=if hovered{0xffffff}else{fg_rgb};
            p.icon_chevron_up(x+w*0.5,y+h*0.5,if h>=24.{9.}else{8.},color,1.5);
        }
        if self.hit_enabled{self.hits.push(Hit{rect:render::rect(x,y,w,h),action});}
    }
    #[allow(dead_code)]
    fn draw_stage1_status(&mut self,p:&mut Painter,info:&str,time_str:&str,btn_up_x:f32)->f32{unsafe{
        let left_x=36.0;
        let right_x=btn_up_x-4.0;
        let available_w=(right_x-left_x).max(10.0);

        if let Some((vol, muted, _at)) = self.volume_hud {
            let vol_icon = if muted || vol == 0.0 { "\u{1F507}" } else if vol < 0.33 { "\u{1F508}" } else if vol < 0.66 { "\u{1F509}" } else { "\u{1F50A}" };
            let percent = format!("{}%", (vol * 100.0).round() as u32);
            let bar_w = 54.0;
            let bar_h = 4.0;
            let bar_y = (self.height - bar_h) * 0.5;

            let icon_w = 18.0;
            let pct_w = 30.0;
            let total_w = icon_w + 4.0 + pct_w + 6.0 + bar_w;
            let start_x = left_x + (available_w - total_w).max(0.0) * 0.5;

            p.text_centered(vol_icon, start_x, 0., icon_w, self.height, 11.5, 0xffffff, 1.0, true);
            p.text_centered(&percent, start_x + icon_w + 4.0, 0., pct_w, self.height, 11.0, 0xd8d8de, 1.0, true);
            let bar_x = start_x + icon_w + 4.0 + pct_w + 6.0;
            p.fill_alpha(bar_x, bar_y, bar_w, bar_h, 2.0, 0xffffff, 0.20);
            p.fill_alpha(bar_x, bar_y, (bar_w * vol).clamp(0.0, bar_w), bar_h, 2.0, 0xd8d8de, 0.95);
            return left_x;
        }

        let footer=ui::stage1_footer(time_str,self.battery.as_ref().filter(|b|b.has_battery).map(|b|(b.percentage as u32,b.is_charging)),self.tws.as_ref().map(|t|t.battery));
        let time_w=p.measure_text(&footer,11.5,true);
        let info_w=p.measure_text(info,11.5,true);
        let(info_x,sep_x,time_x)=ui::stage1_positions(left_x,right_x,info_w,time_w);
        p.text_centered(&footer,time_x,0.,time_w,self.height,11.5,self.palette.text_primary,1.,true);
        p.text_centered("|",sep_x,0.,6.,self.height,11.,self.palette.text_muted,0.85,false);
        p.text_ellipsis_vcenter(info,info_x,0.,(sep_x-8.-info_x).max(0.),self.height,11.5,self.palette.text_primary,true);
        info_x
    }}
    fn draw_top_tabs(&mut self,p:&mut Painter){unsafe{
        let is_rearranging = self.tab_drag.as_ref().map(|d| d.is_dragging).unwrap_or(false);
        let tabs = self.settings.active_tab_order();
        let y = 12.0;
        let h = 26.0;

        struct TabSlot {
            tab: String,
            icon: String,
            label: String,
            x: f32,
            w: f32,
            selected: bool,
        }

        let mods = self.mods();
        let mut slots: Vec<TabSlot> = Vec::new();
        let mut cur_x = 18.0;
        for tab in &tabs {
            let icon=mods.iter().find(|m| &m.tab_name==tab).map(|m|m.icon.clone()).unwrap_or_else(||ui::module_icon(tab).to_string());
            let label=crate::i18n::tab_name(tab, &self.settings.language).to_string();
            let selected = self.tab == *tab;
            let w = if is_rearranging {
                26.0
            } else if selected {
                let label_w = label.chars().count() as f32 * 7.2;
                (14.0 + 6.0 + label_w + 24.0).max(72.0)
            } else {
                26.0
            };
            slots.push(TabSlot {
                tab: tab.clone(),
                icon,
                label,
                x: cur_x,
                w,
                selected,
            });
            cur_x += w + 6.0;
        }

        let mut swap_target = None;
        let mut dragged_pill = None;

        if let Some((tab, pos)) = self.tab_drag.as_ref().filter(|d| d.is_dragging).map(|d| (d.tab.clone(), d.current_pos)) {
            if let Some(cur_idx) = slots.iter().position(|s| s.tab == tab) {
                let pill_w = 26.0;
                let max_right = (self.width - 170.0 - pill_w).max(16.0);
                let dragged_x = (pos.0 - pill_w * 0.5).clamp(16.0, max_right);
                let dragged_center = dragged_x + pill_w * 0.5;

                if cur_idx > 0 {
                    let left_center = slots[cur_idx - 1].x + slots[cur_idx - 1].w * 0.5;
                    if dragged_center < left_center {
                        swap_target = Some((tab.clone(), slots[cur_idx - 1].tab.clone()));
                    }
                }
                if swap_target.is_none() && cur_idx + 1 < slots.len() {
                    let right_center = slots[cur_idx + 1].x + slots[cur_idx + 1].w * 0.5;
                    if dragged_center > right_center {
                        swap_target = Some((tab.clone(), slots[cur_idx + 1].tab.clone()));
                    }
                }

                dragged_pill = Some((slots[cur_idx].tab.clone(), slots[cur_idx].icon.clone(), dragged_x, pill_w));
            }
        }

        if let Some((a, b)) = swap_target {
            self.settings.swap_tabs(&a, &b);
            self.invalidate();
        }

        for slot in &slots {
            if let Some((ref dtab, ..)) = dragged_pill {
                if dtab == &slot.tab {
                    p.stroke(slot.x, y, slot.w, h, h * 0.5, 0xb8b8c2, 1.2);
                    p.fill_alpha(slot.x, y, slot.w, h, h * 0.5, 0xb8b8c2, 0.08);
                    continue;
                }
            }
            if slot.x + slot.w > self.width - 170.0 { break; }
            self.tab_pill(p, &slot.icon, &slot.label, slot.x, y, slot.w, h, Action::Tab(slot.tab.clone()), slot.selected);
        }

        if let Some((tab_name, icon, dx, dw)) = dragged_pill {
            let radius = h * 0.5;
            p.fill_alpha(dx - 2.0, y - 2.0, dw + 4.0, h + 4.0, radius + 2.0, 0xb8b8c2, 0.35);
            p.fill_alpha(dx, y, dw, h, radius, 0x18181b, 0.95);
            p.stroke(dx, y, dw, h, radius, 0xb8b8c2, 1.8);
            p.outline_icon(&icon,dx+(dw-15.)/2.,y+(h-15.)/2.,15.,0xefeff3);

            if self.hit_enabled {
                self.hits.push(Hit { rect: render::rect(dx, y, dw, h), action: Action::Tab(tab_name) });
            }
        }

        // Time display immediately to the left of the visualizer
        let time_str=Local::now().format("%H:%M").to_string();
        let clock_x=self.width-160.;
        let clock_w=38.;
        p.text_centered(&time_str,clock_x,y,clock_w,h,11.,0xffffff,0.85,true);

        // Privacy rail in Stage 2 header if active
        let rail_w = if self.settings.privacy_enabled && self.privacy_snapshot.any_active() {
            crate::shell::privacy::compute_privacy_rail_width(&self.privacy_snapshot)
        } else {
            0.0
        };
        if rail_w > 0.0 {
            self.draw_privacy_rail(p, clock_x - rail_w - 8.0, y + 4.0, rail_w, 18.0);
        }

        if self.track.playing&&self.settings.effective_presentation().visualizer{
            let levels=if self.levels.len()>=4{&self.levels[0..4]}else{&[0.4,0.8,1.0,0.6]};
            p.bars(levels,self.width-118.,16.,14.,12.,0xffffff);
        }else if self.settings.effective_presentation().visualizer{
            p.fill(self.width-118.,21.,14.,2.,1.,0x454e4d);
        }

        // Window buttons: Pin, Collapse, Settings
        self.navigation_control(p,"sliders",self.width-120.,y,Action::QuickControlsOpen,self.quick_controls_open);
        self.navigation_control(p,"pin",self.width-92.,y,Action::TogglePin,self.pinned);
        self.navigation_control(p,"up",self.width-64.,y,Action::ToStage1,false);
        self.navigation_control(p,"settings",self.width-36.,y,Action::Settings,false);
    }}
    fn draw_expanded(&mut self,p:&mut Painter,alpha:f32){unsafe{
        p.push_alpha(alpha);
        p.clip(0.,0.,self.width,self.height);
        if self.quick_controls_open {
            self.draw_quick_controls(p);
        } else if self.privacy_detail_open {
            self.draw_privacy_detail(p);
        } else if self.settings_open {
            self.draw_settings(p);
        }else if self.mod_view.is_some(){
            if self.settings.quiet_mode{self.quiet_placeholder(p);}else{self.draw_mod_view(p);}
        }else{
            if let Some((old_tab,started,_dir))=&self.tab_fade{
                let progress=(started.elapsed().as_secs_f32()/ui::TAB_FADE_DURATION.as_secs_f32()).clamp(0.,1.);
                let ease=ui::ease_out_cubic(progress);
                let old_alpha=(1.-ease).clamp(0.,1.);
                let new_alpha=ease.clamp(0.,1.);

                if old_alpha>0.02{
                    let saved=self.tab.clone();
                    self.tab=old_tab.clone();
                    let hits_before=self.hits.len();
                    p.push_alpha(old_alpha);
                    self.draw_page(p);
                    p.pop_alpha();
                    self.hits.truncate(hits_before);
                    self.tab=saved;
                }
                if new_alpha>0.02{
                    p.push_alpha(new_alpha);
                    self.draw_page(p);
                    p.pop_alpha();
                }
                if progress>=1.{self.tab_fade=None;let _=KillTimer(Some(self.hwnd),10);}
            }else{
                self.draw_page(p);
            }
            self.draw_top_tabs(p);
        }
        if self.has_activity_bar(){self.draw_activity(p,self.height-30.,true);}
        if let Some(toast)=self.toast.clone().filter(|_|!self.settings.quiet_mode){
            let y=self.height-28.;p.fill(16.,y,self.width-32.,23.,7.,self.palette.card_bg);p.stroke(16.,y,self.width-32.,23.,7.,self.palette.border,1.0);
            let water=toast.kind==ToastKind::Water&&!toast.confirmed;
            let is_update=toast.kind==ToastKind::UpdateReady;
            if is_update {
                p.text_ellipsis(&toast.message,25.,y+4.,self.width-180.,17.,10.5,render::TEXT,false);
                self.button_h(p,crate::i18n::t("update.restart_now", &self.settings.language),self.width-160.,y,90.,23.,Action::RestartNow,true);
                self.button_h(p,crate::i18n::t("update.restart_later", &self.settings.language),self.width-65.,y,48.,23.,Action::RestartLater,true);
            } else {
                p.text_ellipsis(&toast.message,25.,y+4.,self.width-if water{116.}else{50.},17.,10.5,render::TEXT,false);
                if water{self.button_h(p,crate::i18n::t("toast.drink_confirm", &self.settings.language),self.width-88.,y,64.,23.,Action::ConfirmWater,true);}
            }
        }
        if self.confirm_provider_download {
            let dw = 420.0f32.min(self.width - 32.);
            let dh = 140.0f32;
            let dx = (self.width - dw) / 2.;
            let dy = (self.height - dh) / 2.;
            p.fill(0., 0., self.width, self.height, 12., 0xdd0d0d12);
            p.fill(dx, dy, dw, dh, 10., 0x181820);
            p.stroke(dx, dy, dw, dh, 10., 0x383846, 1.5);

            let title = if self.settings.language == "en" { "Download Power Provider Companion?" } else { "Unduh Companion Power Provider?" };
            p.text(title, dx + 18., dy + 16., dw - 36., 20., 13.5, render::TEXT, true);

            let desc = if self.settings.language == "en" {
                "Download companion package (~45 MB) containing mynotch-power.exe and hardware sensor drivers directly from GitHub Releases?"
            } else {
                "Unduh paket pendamping (~45 MB) berisi mynotch-power.exe dan driver sensor hardware langsung dari GitHub Releases?"
            };
            p.text(desc, dx + 18., dy + 44., dw - 36., 44., 10.5, render::MUTED, false);

            let cancel_lbl = if self.settings.language == "en" { "Cancel" } else { "Batal" };
            self.button_h(p, cancel_lbl, dx + dw - 180., dy + dh - 36., 80., 24., Action::CancelDownloadPowerProvider, false);

            let ok_lbl = if self.settings.language == "en" { "Download" } else { "Unduh" };
            self.button_h(p, ok_lbl, dx + dw - 92., dy + dh - 36., 76., 24., Action::ConfirmDownloadPowerProvider, true);
        }
        p.unclip();
        p.pop_alpha();
    }}
    fn active_lyric(&self)->Option<String>{let position=self.media.as_ref().map(|m|m.position_seconds()).unwrap_or(self.track.position_seconds);self.lyrics.iter().rev().find(|line|line.at<=position).map(|line|line.text.clone())}
    fn tray(&self,add:bool){unsafe{use windows::Win32::UI::Shell::*;let mut data=NOTIFYICONDATAW{cbSize:std::mem::size_of::<NOTIFYICONDATAW>()as u32,hWnd:self.hwnd,uID:1,uFlags:NIF_MESSAGE|NIF_ICON|NIF_TIP,uCallbackMessage:TRAY,hIcon:LoadIconW(None,IDI_APPLICATION).unwrap_or_default(),..Default::default()};let text:Vec<u16>=app_title().encode_utf16().take(data.szTip.len()-1).collect();data.szTip[..text.len()].copy_from_slice(&text);let _=Shell_NotifyIconW(if add{NIM_ADD}else{NIM_DELETE},&data);}}
    fn menu(&self){unsafe{if let Ok(menu)=CreatePopupMenu(){let _=AppendMenuW(menu,MF_STRING,900,w!("Buka / tutup notch"));let _=AppendMenuW(menu,MF_STRING,901,w!("Pengaturan"));let _=AppendMenuW(menu,MF_STRING,902,w!("Posisi atas"));let _=AppendMenuW(menu,MF_STRING,903,w!("Posisi bawah"));let _=AppendMenuW(menu,MF_SEPARATOR,0,PCWSTR::null());let _=AppendMenuW(menu,MF_STRING,999,w!("Keluar"));let mut cursor=POINT::default();let _=GetCursorPos(&mut cursor);let _=TrackPopupMenu(menu,TPM_RIGHTBUTTON,cursor.x,cursor.y,None,self.hwnd,None);let _=DestroyMenu(menu);}}}
}
impl Drop for Shell{fn drop(&mut self){if let Ok(mut q)=self.queue.lock(){q.close();}self.flush_note_edits();crate::clipboard::unlisten_clipboard(self.hwnd);self.cancel_jobs();self.tray(false);crate::system::set_volume_osd_hidden(false);if let Some(mut m)=self.media.take(){m.stop();}if let Some(mut v)=self.visualizer.take(){v.stop();}if let Some(mut p_mon)=self.privacy_monitor.take(){p_mon.stop();}unsafe{let h_val = HOOK_HANDLE.swap(0, Ordering::Relaxed); if h_val != 0 { let _ = UnhookWindowsHookEx(HHOOK(h_val as *mut _)); } HOOK_HWND.store(0, Ordering::Relaxed);for id in 1..=16{let _=KillTimer(Some(self.hwnd),id);}if self.morph_timer!=0{let _=timeKillEvent(self.morph_timer);}let _=UnregisterHotKey(Some(self.hwnd),1);let _=DestroyWindow(self.inputs_hwnd);if let Ok(tray)=FindWindowW(w!("Shell_TrayWnd"),None){let _=ShowWindow(tray,SW_SHOW);}if let Ok(sec_tray)=FindWindowW(w!("Shell_SecondaryTrayWnd"),None){let _=ShowWindow(sec_tray,SW_SHOW);}}self.save();}}
fn time_label(seconds:f64)->String{let secs=seconds.max(0.)as u64;format!("{:02}:{:02}",secs/60,secs%60)}

/// Resets `painting` even if drawing panics: without this, one panicked frame would
/// leave the flag set and make every later `paint()` call a no-op (the early return at
/// the top of `paint()`). The guard holds a raw pointer instead of a borrow so the rest
/// of `paint()` can keep using `&mut self` freely. It must stay on the UI thread: the
/// guard lives only for the duration of `paint()`, and the Shell outlives it.
/// Note: release builds use panic=abort, so a panic there aborts the process regardless;
/// the guard protects debug/test/unwind configurations and documents the invariant.
struct PaintingGuard(*mut bool);
impl Drop for PaintingGuard {
    fn drop(&mut self) {
        unsafe { *self.0 = false; }
    }
}

fn covers_monitor(window:RECT,monitor:RECT)->bool{window.left<=monitor.left&&window.top<=monitor.top&&window.right>=monitor.right&&window.bottom>=monitor.bottom}

/// Extended styles for the composition shell, split out so unit tests can assert the
/// passive-display style without creating real windows.
fn shell_ex_styles(preview:bool)->WINDOW_EX_STYLE{
    if preview{WS_EX_NOREDIRECTIONBITMAP|WS_EX_APPWINDOW|WS_EX_TOPMOST}
    else{WS_EX_NOREDIRECTIONBITMAP|WS_EX_TOOLWINDOW|WS_EX_TOPMOST|WS_EX_NOACTIVATE}
}
/// Extended styles for the input host (mynotch.Inputs). Deliberately without
/// WS_EX_NOACTIVATE: clicking an EDIT control must activate it so typing works.
fn input_host_ex_styles()->WINDOW_EX_STYLE{
    WS_EX_TOOLWINDOW|WS_EX_TOPMOST
}

fn covers_rect_tolerance(window:RECT,target:RECT,tol:i32)->bool{
    window.left<=target.left+tol&&window.top<=target.top+tol&&window.right>=target.right-tol&&window.bottom>=target.bottom-tol
}

fn is_borderless_fullscreen_rect(
    window:RECT,
    client:Option<RECT>,
    monitor:RECT,
    work:RECT,
    style:u32,
    ex_style:u32,
    is_zoomed:bool,
)->bool{
    if covers_monitor(window,monitor){
        return true;
    }
    if (ex_style&WS_EX_TOOLWINDOW.0)!=0{
        return false;
    }
    let has_caption=(style&WS_CAPTION.0)==WS_CAPTION.0;
    let has_thickframe=(style&WS_THICKFRAME.0)!=0;
    let is_normal_maximized=is_zoomed&&has_caption;

    if !is_normal_maximized{
        if covers_rect_tolerance(window,monitor,2){
            return true;
        }
        if client.is_some_and(|c|covers_rect_tolerance(c,monitor,2)){
            return true;
        }
    }

    if !has_caption{
        if covers_rect_tolerance(window,work,2)
            ||client.is_some_and(|c|covers_rect_tolerance(c,work,2))
        {
            return true;
        }
        if !is_zoomed&&!has_thickframe{
            let w=(window.right-window.left)as i64;
            let h=(window.bottom-window.top)as i64;
            let mon_w=(monitor.right-monitor.left).max(1)as i64;
            let mon_h=(monitor.bottom-monitor.top).max(1)as i64;
            let touches_top=window.top<=monitor.top+2&&window.bottom>monitor.top+48;
            let covers_half=w*h*2>=mon_w*mon_h;
            if w>=1024&&h>=720&&(touches_top||covers_half){
                return true;
            }
        }
    }else if !is_zoomed&&window.top<monitor.top-12{
        if client.is_some_and(|c|covers_rect_tolerance(c,work,2)){
            return true;
        }
    }

    false
}

fn is_cursor_visible()->bool{
    unsafe{
        let mut ci=CURSORINFO{cbSize:std::mem::size_of::<CURSORINFO>()as u32,..Default::default()};
        if GetCursorInfo(&mut ci).is_ok(){
            (ci.flags.0&CURSOR_SHOWING.0)!=0&&!ci.hCursor.0.is_null()
        }else{
            true
        }
    }
}

fn is_window_cloaked(hwnd:HWND)->bool{
    unsafe{
        use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute,DWMWA_CLOAKED};
        let mut cloaked:u32=0;
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&mut cloaked as *mut u32).cast(),
            std::mem::size_of::<u32>()as u32,
        )
        .is_ok()
            &&cloaked!=0
    }
}

fn is_shell_or_desktop_window(hwnd:HWND)->bool{
    unsafe{
        if hwnd.0.is_null()||hwnd==GetShellWindow()||hwnd==GetDesktopWindow(){
            return true;
        }
        let mut buf=[0u16;64];
        let len=GetClassNameW(hwnd,&mut buf);
        if len>0{
            let class_name=String::from_utf16_lossy(&buf[..len as usize]);
            matches!(
                class_name.as_str(),
                "Progman"|"WorkerW"|"Shell_TrayWnd"|"Shell_SecondaryTrayWnd"
            )
        }else{
            false
        }
    }
}

fn client_screen_rect(hwnd:HWND)->Option<RECT>{
    unsafe{
        let mut client=RECT::default();
        if GetClientRect(hwnd,&mut client).is_err(){
            return None;
        }
        if client.right<=client.left||client.bottom<=client.top{
            return None;
        }
        let mut tl=POINT{x:client.left,y:client.top};
        let mut br=POINT{x:client.right,y:client.bottom};
        if !ClientToScreen(hwnd,&mut tl).as_bool()||!ClientToScreen(hwnd,&mut br).as_bool(){
            return None;
        }
        Some(RECT{left:tl.x,top:tl.y,right:br.x,bottom:br.y})
    }
}

fn is_cursor_clipped_to_window(window:RECT,monitor:RECT)->bool{
    unsafe{
        let mut clip=RECT::default();
        if GetClipCursor(&mut clip).is_err(){
            return false;
        }
        let virt_l=GetSystemMetrics(SM_XVIRTUALSCREEN);
        let virt_t=GetSystemMetrics(SM_YVIRTUALSCREEN);
        let virt_r=virt_l+GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let virt_b=virt_t+GetSystemMetrics(SM_CYVIRTUALSCREEN);
        if clip.left<=virt_l&&clip.top<=virt_t&&clip.right>=virt_r&&clip.bottom>=virt_b{
            return false;
        }
        let win_w=window.right-window.left;
        let win_h=window.bottom-window.top;
        if win_w<800||win_h<600{
            return false;
        }
        let clip_w=clip.right-clip.left;
        let clip_h=clip.bottom-clip.top;
        if clip_w<=0||clip_h<=0{
            return false;
        }
        let inside_window=clip.left>=window.left-16
            &&clip.top>=window.top-16
            &&clip.right<=window.right+16
            &&clip.bottom<=window.bottom+16;
        let matches_monitor=covers_rect_tolerance(clip,monitor,2)&&covers_rect_tolerance(monitor,clip,16);
        inside_window||matches_monitor
    }
}

fn is_d3d_fullscreen_notification_active()->bool{
    unsafe{
        use windows::Win32::UI::Shell::{SHQueryUserNotificationState,QUNS_RUNNING_D3D_FULL_SCREEN};
        SHQueryUserNotificationState().is_ok_and(|state|state==QUNS_RUNNING_D3D_FULL_SCREEN)
    }
}

fn check_hwnd_fullscreen(hwnd:HWND,my_monitor:HMONITOR,info:&MONITORINFO)->bool{
    unsafe{
        if hwnd.0.is_null()
            ||!IsWindowVisible(hwnd).as_bool()
            ||IsIconic(hwnd).as_bool()
            ||is_shell_or_desktop_window(hwnd)
            ||is_window_cloaked(hwnd)
        {
            return false;
        }
        let monitor=MonitorFromWindow(hwnd,MONITOR_DEFAULTTONEAREST);
        if monitor!=my_monitor{
            return false;
        }
        let mut window=RECT::default();
        if GetWindowRect(hwnd,&mut window).is_err(){
            return false;
        }
        let client=client_screen_rect(hwnd);
        let style=GetWindowLongW(hwnd,GWL_STYLE)as u32;
        let ex_style=GetWindowLongW(hwnd,GWL_EXSTYLE)as u32;
        let is_zoomed=IsZoomed(hwnd).as_bool();

        if is_borderless_fullscreen_rect(
            window,
            client,
            info.rcMonitor,
            info.rcWork,
            style,
            ex_style,
            is_zoomed,
        ){
            return true;
        }
        let covers_mon = covers_rect_tolerance(window,info.rcMonitor,16)
            || covers_rect_tolerance(window,info.rcWork,16)
            || client.is_some_and(|c|covers_rect_tolerance(c,info.rcWork,16));
        if is_cursor_clipped_to_window(window,info.rcMonitor)&&covers_mon{
            return true;
        }
        let win_w=window.right-window.left;
        let win_h=window.bottom-window.top;
        if win_w>=800&&win_h>=600&&covers_mon&&is_d3d_fullscreen_notification_active(){
            return true;
        }
        false
    }
}

fn top_standard_window_on_monitor(
    self_hwnd:HWND,
    inputs_hwnd:HWND,
    skip_hwnd:Option<HWND>,
    my_monitor:HMONITOR,
)->Option<HWND>{
    unsafe{
        let mut cur=GetTopWindow(None).unwrap_or_default();
        for _ in 0..256{
            if cur.0.is_null(){
                break;
            }
            if cur!=self_hwnd
                &&cur!=inputs_hwnd
                &&skip_hwnd!=Some(cur)
                &&IsWindowVisible(cur).as_bool()
                &&!IsIconic(cur).as_bool()
                &&!is_shell_or_desktop_window(cur)
                &&!is_window_cloaked(cur)
                &&MonitorFromWindow(cur,MONITOR_DEFAULTTONEAREST)==my_monitor
            {
                let ex_style=GetWindowLongW(cur,GWL_EXSTYLE)as u32;
                let mut r=RECT::default();
                if GetWindowRect(cur,&mut r).is_ok(){
                    let w=r.right-r.left;
                    let h=r.bottom-r.top;
                    if (ex_style&(WS_EX_TOOLWINDOW.0|WS_EX_TRANSPARENT.0|WS_EX_NOACTIVATE.0))==0
                        &&w>=240
                        &&h>=160
                    {
                        return Some(cur);
                    }
                }
            }
            cur=GetWindow(cur,GW_HWNDNEXT).unwrap_or_default();
        }
        None
    }
}

pub(super) fn classify_notice_kind(msg:&str)->ToastKind{
    let lower = msg.to_lowercase();
    if msg.contains("💧")||lower.contains("minum")||lower.contains("drink")||lower.contains("hydration"){
        ToastKind::Water
    }else if msg.contains("🎯")||lower.contains("fokus")||lower.contains("focus"){
        ToastKind::Focus
    }else if msg.contains("🧘")||lower.contains("regang")||lower.contains("stretch"){
        ToastKind::Stretch
    }else{
        ToastKind::Generic
    }
}

fn lyric_request_key(track: &MediaSnapshot) -> String {
    serde_json::to_string(&(&track.session_id, &track.source_app, &track.title, &track.artist, &track.album, track.duration_seconds.round() as u64)).unwrap_or_default()
}

pub(super) fn lyric_display_index(lines:&[crate::lyrics::LyricLine],position:f64,offset:usize)->usize{
    if offset>0&&!lines.is_empty(){
        (offset-1).min(lines.len()-1)
    }else{
        crate::lyrics::active_line(lines,position).unwrap_or(0)
    }
}

pub(super) fn collapse_target_stage(minimal:bool)->u8{
    if minimal{0}else{1}
}

/// Semantik stage awal lama (tanpa sekuens boot F1). Dipertahankan untuk uji
/// pembanding; runtime kini memakai boot_start_stage()/boot_completes_to_stage().
#[cfg(test)]
pub(super) fn initial_hover_stage(preview:bool,minimal:bool)->u8{
    if preview{2}else if minimal{0}else{1}
}

// --- F1 sekuens boot (mockup notifikasi-reminder-mockups.html) -----------------
/// Delay sebelum boot morph dari Stage 0 ke Stage 1 idle (mockup: 2,5 dtk).
pub const BOOT_STAGE1_DELAY_MS:u32=2500;
/// Durasi animasi "turun kecil" saat boot (mockup @keyframes bootin: 0,6 dtk).
pub const BOOT_DROP_DURATION:Duration=Duration::from_millis(600);
/// Stage awal boot: preview langsung Stage 2; runtime selalu mulai di Stage 0.
pub(super) fn boot_start_stage(preview:bool)->u8{if preview{2}else{0}}
/// Stage tujuan setelah 2,5 dtk: Stage 1 idle, kecuali mode minimal (tetap Stage 0).
pub(super) fn boot_completes_to_stage(minimal:bool)->u8{if minimal{0}else{1}}
/// Offset vertikal boot (DIP): -10 -> 0 dengan ease-out cubic (mockup bootin
/// translateY(-10px) -> 0). Nilai negatif berarti masih di atas posisi akhir.
pub(super) fn boot_drop_offset_at(elapsed:Duration)->f32{
    -10.*(1.-ui::ease_out_cubic((elapsed.as_secs_f32()/BOOT_DROP_DURATION.as_secs_f32()).min(1.)))
}

/// A duplicate instance may only surface the production notch. A second `--preview` must not
/// toggle the real window when it exits because its mutex is already held.
pub(super) fn should_focus_existing_instance(preview:bool,duplicate:bool)->bool{duplicate&&!preview}

pub(super) fn toast_target_reminder(kind:ToastKind)->Option<usize>{
    match kind{
        ToastKind::Focus=>Some(0),
        ToastKind::Water=>Some(1),
        ToastKind::Stretch=>Some(2),
        ToastKind::Prayer=>Some(3),
        ToastKind::Generic | ToastKind::UpdateReady => None,
    }
}

pub(super) fn should_revert_stage1(tab: &str, reminder_index: usize, lyric_mode: bool) -> bool {
    if lyric_mode {
        return false;
    }
    if tab == "Kalender" || tab == "Daya" || tab == "Power" {
        return false;
    }
    if tab == "Reminder" && reminder_index == 0 {
        return false;
    }
    true
}

// An isolated, timed test window. It has no modules, storage, audio or network access.
unsafe fn fullscreen_fixture()->Result<(),String>{unsafe{
    unsafe extern "system" fn procedure(hwnd:HWND,msg:u32,wp:WPARAM,lp:LPARAM)->LRESULT{unsafe{match msg{
        WM_TIMER|WM_CLOSE=>{let _=DestroyWindow(hwnd);LRESULT(0)},
        WM_DESTROY=>{PostQuitMessage(0);LRESULT(0)},
        WM_KEYDOWN if wp.0==27=>{let _=DestroyWindow(hwnd);LRESULT(0)},
        _=>DefWindowProcW(hwnd,msg,wp,lp)
    }}}
    let module=GetModuleHandleW(None).map_err(|e|e.to_string())?;
    RegisterClassW(&WNDCLASSW{lpfnWndProc:Some(procedure),hInstance:module.into(),lpszClassName:w!("mynotch.FullscreenFixture"),hbrBackground:HBRUSH(GetStockObject(BLACK_BRUSH).0),..Default::default()});
    let hwnd=CreateWindowExW(WS_EX_APPWINDOW,w!("mynotch.FullscreenFixture"),w!("Notch Fullscreen Test"),WS_POPUP,0,0,GetSystemMetrics(SM_CXSCREEN),GetSystemMetrics(SM_CYSCREEN),None,None,Some(module.into()),None).map_err(|e|e.to_string())?;
    SetTimer(Some(hwnd),1,15000,None);let _=ShowWindow(hwnd,SW_SHOW);let _=SetForegroundWindow(hwnd);
    let mut msg=MSG::default();while GetMessageW(&mut msg,None,0,0).as_bool(){let _=TranslateMessage(&msg);DispatchMessageW(&msg);}Ok(())
}}

#[cfg(test)]mod tests{
    use super::*;

    #[test]
    fn queue_flood_preserves_controls_and_bounds_streaming() {
        let mut q = EventQueue::default();
        let mut wakes = 0;
        for i in 0..100_000 {
            wakes += usize::from(q.push(Event::Levels(vec![i as f32; 24])));
            if i % 1000 == 0 { wakes += usize::from(q.push(Event::Notice(i.to_string()))); }
            assert!(q.events.len() <= 101);
            assert_eq!(q.events.iter().filter(|e| matches!(e, Event::Levels(_))).count(), 1);
        }
        assert_eq!(wakes, 1);
        let events = q.drain();
        let controls: Vec<_> = events.iter().filter_map(|e| if let Event::Notice(s) = e { Some(s.clone()) } else { None }).collect();
        assert_eq!(controls, (0..100_000).step_by(1000).map(|i| i.to_string()).collect::<Vec<_>>());
        assert!(matches!(events.last(), Some(Event::Levels(v)) if v[0] == 99_999.));
        assert!(q.events.is_empty());
        assert!(q.push(Event::Notice("next".into())));
    }

    #[test]
    fn queue_concurrent_drain_keeps_control_sequence() {
        let queue = Arc::new(Mutex::new(EventQueue::default()));
        let producer = queue.clone();
        let worker = std::thread::spawn(move || {
            for i in 0..10_000 {
                let mut q = producer.lock().unwrap();
                q.push(Event::Levels(vec![i as f32]));
                q.push(Event::Notice(i.to_string()));
            }
        });
        let mut controls = Vec::new();
        loop {
            for event in queue.lock().unwrap().drain() { if let Event::Notice(value) = event { controls.push(value); } }
            if worker.is_finished() { break; }
            std::thread::yield_now();
        }
        worker.join().unwrap();
        for event in queue.lock().unwrap().drain() { if let Event::Notice(value) = event { controls.push(value); } }
        assert_eq!(controls, (0..10_000).map(|i| i.to_string()).collect::<Vec<_>>());
    }

    #[test]
    fn queue_shutdown_does_not_wait_for_consumer() {
        use std::sync::mpsc;
        let queue = Arc::new(Mutex::new(EventQueue::default()));
        let poster = Poster { queue: queue.clone(), hwnd: usize::MAX };
        let (tx, rx) = mpsc::channel();
        let (started_tx, started_rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            for i in 0..20_000 {
                poster.send(Event::Levels(vec![i as f32; 24]));
                if i % 1000 == 0 { poster.send(Event::Notice(i.to_string())); }
                if i == 0 { started_tx.send(()).unwrap(); }
            }
            tx.send(()).unwrap();
        });
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        queue.lock().unwrap().close();
        rx.recv_timeout(Duration::from_secs(5)).expect("producer must finish without UI drain");
        worker.join().unwrap();
        let mut q = queue.lock().unwrap();
        assert!(q.closed);
        assert!(q.events.is_empty());
        assert!(!q.push(Event::Notice("late completion".into())));
        assert!(q.drain().is_empty());
    }

    #[test] fn consent_audio_scroll_keeps_existing_player_action_reachable() {
        let layout=settings_layout::SettingsLayout::compute(560.,360.,570.,1000.);
        assert!(layout.visible_hit(render::rect(418.,524.,124.,30.)).is_some());
        let top=settings_layout::SettingsLayout::compute(560.,360.,570.,0.);
        assert!(top.visible_hit(render::rect(506.,120.,42.,31.)).is_some());
        assert!(top.visible_hit(render::rect(506.,220.,42.,31.)).is_some());
    }
    #[test]fn preview_does_not_install_a_global_volume_hook(){
        assert!(!install_volume_hook(true));assert!(install_volume_hook(false));
    }
    #[test]fn quit_is_posted_even_during_reentrant_destroy(){APP.with(|cell|{let _borrow=cell.borrow_mut();unsafe{wndproc(HWND::default(),WM_DESTROY,WPARAM(0),LPARAM(0));let mut msg=MSG::default();assert!(PeekMessageW(&mut msg,None,WM_QUIT,WM_QUIT,PM_REMOVE).as_bool());assert_eq!(msg.message,WM_QUIT);}});}
    #[test]fn fullscreen_requires_whole_monitor_not_work_area(){let monitor=RECT{left:0,top:0,right:1920,bottom:1080};assert!(covers_monitor(monitor,monitor));assert!(!covers_monitor(RECT{bottom:1040,..monitor},monitor));assert!(!covers_monitor(RECT{left:1,..monitor},monitor));assert!(covers_monitor(RECT{left:-8,top:-8,right:1928,bottom:1088},monitor));}
    #[test]fn fullscreen_detection_tolerates_small_anti_directflip_margins(){
        // Borderless games with an anti-DirectFlip margin 1-2 px larger than the monitor must
        // still be detected as fullscreen (they cover the whole monitor and more).
        let monitor=RECT{left:0,top:0,right:1920,bottom:1080};
        let work=RECT{left:0,top:0,right:1920,bottom:1040};
        let popup_style=WS_POPUP.0|WS_VISIBLE.0;
        let anti_df1=RECT{left:-1,top:-1,right:1921,bottom:1081};
        let anti_df2=RECT{left:-2,top:-2,right:1922,bottom:1082};
        // 1. Helper check
        assert!(covers_monitor(anti_df1,monitor));
        assert!(covers_monitor(anti_df2,monitor));
        // 2. Production detector check (called at shell.rs:3051)
        assert!(is_borderless_fullscreen_rect(anti_df1,Some(anti_df1),monitor,work,popup_style,0,false));
        assert!(is_borderless_fullscreen_rect(anti_df2,Some(anti_df2),monitor,work,popup_style,0,false));
    }
    #[test]fn fullscreen_detection_works_on_negative_origin_monitors(){
        // A secondary monitor at (-1920,0) must detect a borderless game that exactly covers
        // its rcMonitor, including the 1-2 px anti-DirectFlip overhang.
        let monitor=RECT{left:-1920,top:0,right:0,bottom:1080};
        let work=RECT{left:-1920,top:0,right:0,bottom:1040};
        let popup_style=WS_POPUP.0|WS_VISIBLE.0;
        let overlapped_style=WS_OVERLAPPEDWINDOW.0|WS_VISIBLE.0;
        // 1. Helper checks
        assert!(covers_monitor(monitor,monitor));
        assert!(covers_monitor(RECT{left:-1921,top:-1,right:1,bottom:1081},monitor));
        assert!(!covers_monitor(RECT{left:-1920,top:0,right:-1,bottom:1080},monitor));
        // 2. Production detector checks (called at shell.rs:3051)
        assert!(is_borderless_fullscreen_rect(monitor,Some(monitor),monitor,work,popup_style,0,false));
        let anti_df=RECT{left:-1921,top:-1,right:1,bottom:1081};
        assert!(is_borderless_fullscreen_rect(anti_df,Some(anti_df),monitor,work,popup_style,0,false));
        // Maximized bordered window on negative-origin monitor is rejected by production detector:
        let max_win=RECT{left:-1928,top:-8,right:8,bottom:1048};
        assert!(!is_borderless_fullscreen_rect(max_win,Some(work),monitor,work,overlapped_style,0,true));
    }
    #[test]fn fullscreen_detection_rejects_maximized_window_with_borders(){
        // A plain maximized (bordered) window sits inside the work area, inside the monitor:
        // it must NOT be classified as fullscreen, so the notch is not hidden by mistake.
        let monitor=RECT{left:0,top:0,right:1920,bottom:1080};
        let work=RECT{left:0,top:0,right:1920,bottom:1040}; // taskbar takes 40 px
        let overlapped_style=WS_OVERLAPPEDWINDOW.0|WS_VISIBLE.0;
        // 1. Helper checks
        let maximized=RECT{left:0,top:0,right:1920,bottom:1040};
        assert!(!covers_monitor(maximized,monitor));
        let monitor2=RECT{left:-1920,top:0,right:0,bottom:1080};
        assert!(!covers_monitor(RECT{left:-1920,top:0,right:0,bottom:1040},monitor2));
        // 2. Production detector checks (called at shell.rs:3051)
        let max_win=RECT{left:-8,top:-8,right:1928,bottom:1048};
        assert!(!is_borderless_fullscreen_rect(max_win,Some(work),monitor,work,overlapped_style,0,true));
        let max_win2=RECT{left:-1928,top:-8,right:8,bottom:1048};
        let work2=RECT{left:-1920,top:0,right:0,bottom:1040};
        assert!(!is_borderless_fullscreen_rect(max_win2,Some(work2),monitor2,work2,overlapped_style,0,true));
    }
    #[test]fn fullscreen_geometry_and_production_detector_smaller_window_handling(){
        // Helper vs production detector policies on windows smaller than the monitor:
        let monitor=RECT{left:0,top:0,right:1920,bottom:1080};
        let work=RECT{left:0,top:0,right:1920,bottom:1040};
        let popup_style=WS_POPUP.0|WS_VISIBLE.0;
        let game_1400x1080=RECT{left:260,top:0,right:1660,bottom:1080};

        // 1. Helper covers_monitor strictly checks whether the window covers the full monitor:
        assert!(!covers_monitor(game_1400x1080,monitor));
        assert!(!covers_monitor(RECT{left:0,top:0,right:1920,bottom:1000},monitor));

        // 2. Production detector (is_borderless_fullscreen_rect at :3051) implements dev heuristic:
        // it accepts large borderless game windows (w >= 1024 && h >= 720 and touches_top or covers_half)
        // even when narrower than the monitor (e.g. 1400x1080 centered/pillarboxed game):
        assert!(is_borderless_fullscreen_rect(game_1400x1080,Some(game_1400x1080),monitor,work,popup_style,0,false));

        // 3. But production detector rejects small borderless utility/splash windows (< 1024x720):
        let splash=RECT{left:640,top:340,right:1280,bottom:740}; // 640x400
        assert!(!covers_monitor(splash,monitor));
        assert!(!is_borderless_fullscreen_rect(splash,Some(splash),monitor,work,popup_style,0,false));
    }
    #[test]fn fullscreen_detection_rejects_multi_monitor_windows(){
        // A borderless window spanning two monitors must not count as fullscreen for either
        // single monitor in the strict covering helper sense:
        let a=RECT{left:0,top:0,right:1920,bottom:1080};
        let b=RECT{left:1920,top:0,right:3840,bottom:1080};
        let span=RECT{left:960,top:0,right:2880,bottom:1080};
        assert!(!covers_monitor(span,a));
        assert!(!covers_monitor(span,b));
    }
    #[test]fn shell_and_input_host_styles_match_task_015_policy(){
        // Task 015 / spec 006a: the composition shell (passive display) carries WS_EX_NOACTIVATE
        // in normal production mode, so no passive show/refresh path can steal focus from a fullscreen game.
        // Preview mode follows dev policy (WS_EX_APPWINDOW without WS_EX_NOACTIVATE) so the preview window
        // can be foregrounded as a standalone tool.
        // The input host must NOT carry WS_EX_NOACTIVATE, otherwise clicking an EDIT control could not take
        // focus and Notes/Clipboard typing breaks.
        assert_eq!(shell_ex_styles(false).0&WS_EX_NOACTIVATE.0,WS_EX_NOACTIVATE.0);
        assert_eq!(shell_ex_styles(false).0&WS_EX_TOOLWINDOW.0,WS_EX_TOOLWINDOW.0);
        assert_eq!(shell_ex_styles(true).0&WS_EX_NOACTIVATE.0,0, "preview shell follows dev policy without NOACTIVATE");
        assert_eq!(shell_ex_styles(true).0&WS_EX_APPWINDOW.0,WS_EX_APPWINDOW.0);
        assert_eq!(input_host_ex_styles().0&WS_EX_NOACTIVATE.0,0, "input host must stay activatable for EDIT focus");
        assert_eq!(input_host_ex_styles().0&WS_EX_TOOLWINDOW.0,WS_EX_TOOLWINDOW.0);
        assert_eq!(input_host_ex_styles().0&WS_EX_TOPMOST.0,WS_EX_TOPMOST.0);
    }
    #[test]fn mouse_activate_response_never_activates_but_keeps_the_click(){
        // The handler returns MA_NOACTIVATE (3): the pill never takes activation while the
        // mouse-down is still delivered to the shell as WM_LBUTTONDOWN.
        assert_eq!(MA_NOACTIVATE,3u32);
        assert_ne!(MA_NOACTIVATE,MA_NOACTIVATEANDEAT);
        let response=LRESULT(MA_NOACTIVATE as isize);
        assert_eq!(response.0,MA_NOACTIVATE as isize);
    }
    #[test]fn notice_classification_prioritizes_focus_completion_over_stretch(){let msg="🎯 Sesi fokus selesai! Waktunya istirahat & peregangan.";let kind=classify_notice_kind(msg);assert_eq!(kind,ToastKind::Focus);}
    #[test]fn lyric_display_index_respects_manual_scroll_offset(){let lines=vec![crate::lyrics::LyricLine{at:1.0,text:"A".into()},crate::lyrics::LyricLine{at:5.0,text:"B".into()},crate::lyrics::LyricLine{at:10.0,text:"C".into()}];assert_eq!(lyric_display_index(&lines,6.0,0),1);assert_eq!(lyric_display_index(&lines,6.0,3),2);}
    #[test]
    fn lyric_request_identity_changes_when_the_source_session_changes() {
        let mut track=MediaSnapshot {title:"Song".into(),artist:"Artist".into(),source_app:"spotify.exe".into(),duration_seconds:200.,..Default::default()};
        let old=lyric_request_key(&track);
        track.session_id="another-session-same-app".into();
        assert_ne!(old,lyric_request_key(&track));
        track.source_app="youtube.app".into();
        assert_ne!(old,lyric_request_key(&track));
        let key=lyric_request_key(&track);
        track.playing=true;track.position_seconds=100.;
        assert_eq!(key,lyric_request_key(&track));
    }
    #[test]fn minimal_mode_collapses_directly_to_stage0(){assert_eq!(collapse_target_stage(true),0);assert_eq!(collapse_target_stage(false),1);assert_eq!(initial_hover_stage(false,true),0);assert_eq!(initial_hover_stage(false,false),1);}
    #[test]fn f1_boot_starts_at_stage0_and_completes_to_idle_or_minimal(){
        assert_eq!(boot_start_stage(false),0);
        assert_eq!(boot_start_stage(true),2);
        assert_eq!(boot_completes_to_stage(false),1);
        assert_eq!(boot_completes_to_stage(true),0);
        assert_eq!(BOOT_STAGE1_DELAY_MS,2500);
    }
    #[test]fn f1_boot_drop_slides_from_minus_ten_to_zero(){
        assert_eq!(boot_drop_offset_at(Duration::ZERO),-10.);
        assert_eq!(boot_drop_offset_at(BOOT_DROP_DURATION),0.);
        assert_eq!(boot_drop_offset_at(BOOT_DROP_DURATION*2),0.);
        // Kurva ease-out: separuh waktu sudah menempuh >80% jarak.
        let half=boot_drop_offset_at(Duration::from_millis(300));
        assert!(half>-2.&&half<0.,"offset pada 300ms = {half}");
        // Monoton naik (turun ke bawah) tanpa pernah melewati 0.
        let mut prev=-10.;
        for ms in [0u64,50,100,150,200,300,450,600,900]{
            let off=boot_drop_offset_at(Duration::from_millis(ms));
            assert!(off>=prev&&off<=0.,"offset {off} pada {ms}ms tidak monoton");
            prev=off;
        }
    }
    #[test]fn volume_media_keys_map_to_their_action(){
        assert_eq!(volume_key(0xAF),Some(VolumeKey::Up));
        assert_eq!(volume_key(0xAE),Some(VolumeKey::Down));
        assert_eq!(volume_key(0xAD),Some(VolumeKey::Mute));
        assert_eq!(volume_key(0x41),None);
    }
    #[test]fn only_a_duplicate_production_instance_surfaces_the_notch(){
        assert!(should_focus_existing_instance(false,true));
        assert!(!should_focus_existing_instance(true,true));
        assert!(!should_focus_existing_instance(false,false));
        assert!(!should_focus_existing_instance(true,false));
    }
    #[test]fn toast_click_maps_all_reminder_kinds_including_prayer(){assert_eq!(toast_target_reminder(ToastKind::Focus),Some(0));assert_eq!(toast_target_reminder(ToastKind::Water),Some(1));assert_eq!(toast_target_reminder(ToastKind::Stretch),Some(2));assert_eq!(toast_target_reminder(ToastKind::Prayer),Some(3));assert_eq!(toast_target_reminder(ToastKind::Generic),None);}
    #[test]fn prayer_summary_formats_zero_lead_as_tepat_waktu(){assert_eq!(settings::prayer_lead_summary("Jakarta",0),"Jakarta · Tepat waktu");assert_eq!(settings::prayer_lead_summary("Bandung",5),"Bandung · 5 menit sebelumnya");}
    #[test]
    fn stage1_temporary_behavior_expires_cleanly() {
        let until = Some(Instant::now() + Duration::from_millis(50));
        assert!(until.is_some_and(|u| Instant::now() < u));

        let expired = Some(Instant::now() - Duration::from_millis(10));
        assert!(!expired.is_some_and(|u| Instant::now() < u));
    }
    #[test]
    fn borderless_fullscreen_detects_game_windows_and_ignores_normal_desktop_windows() {
        let monitor = RECT { left: 0, top: 0, right: 2560, bottom: 1440 };
        let work = RECT { left: 0, top: 0, right: 2560, bottom: 1392 };
        let popup_style = WS_POPUP.0 | WS_VISIBLE.0;
        let overlapped_style = WS_OVERLAPPEDWINDOW.0 | WS_VISIBLE.0;

        // 1. Exact monitor borderless or exclusive fullscreen
        assert!(is_borderless_fullscreen_rect(monitor, Some(monitor), monitor, work, popup_style, 0, false));

        // 2. 1-2px anti-DirectFlip borderless game windows (2560x1439, 2559x1440, top=1)
        let anti_df_h = RECT { left: 0, top: 0, right: 2560, bottom: 1439 };
        let anti_df_w = RECT { left: 0, top: 0, right: 2559, bottom: 1440 };
        let anti_df_t = RECT { left: 0, top: 1, right: 2560, bottom: 1440 };
        assert!(is_borderless_fullscreen_rect(anti_df_h, Some(anti_df_h), monitor, work, popup_style, 0, false));
        assert!(is_borderless_fullscreen_rect(anti_df_w, Some(anti_df_w), monitor, work, popup_style, 0, false));
        assert!(is_borderless_fullscreen_rect(anti_df_t, Some(anti_df_t), monitor, work, overlapped_style, 0, false));

        // 3. Borderless WS_POPUP window covering work area (e.g. clamped above taskbar)
        assert!(is_borderless_fullscreen_rect(work, Some(work), monitor, work, popup_style, 0, false));

        // 4. Fixed-size borderless game window (1920x1080 on 2560x1440 monitor)
        let game_1080p = RECT { left: 320, top: 180, right: 2240, bottom: 1260 };
        assert!(is_borderless_fullscreen_rect(game_1080p, Some(game_1080p), monitor, work, popup_style, 0, false));

        // 5. Off-screen captioned borderless window
        let offscreen_win = RECT { left: -8, top: -31, right: 2568, bottom: 1400 };
        assert!(is_borderless_fullscreen_rect(offscreen_win, Some(work), monitor, work, overlapped_style, 0, false));

        // 6. Normal maximized desktop window (e.g. browser/editor) must NOT be treated as fullscreen
        let max_win = RECT { left: -8, top: -8, right: 2568, bottom: 1400 };
        assert!(!is_borderless_fullscreen_rect(max_win, Some(work), monitor, work, overlapped_style, 0, true));

        // 7. Normal un-maximized desktop window covering work area must NOT be treated as fullscreen
        assert!(!is_borderless_fullscreen_rect(work, Some(work), monitor, work, overlapped_style, 0, false));

        // 8. Small borderless splash window (< 1024x720) must NOT be treated as fullscreen
        let splash = RECT { left: 960, top: 520, right: 1600, bottom: 920 };
        assert!(!is_borderless_fullscreen_rect(splash, Some(splash), monitor, work, popup_style, 0, false));

        // 9. Toolwindow covering work area must NOT be treated as fullscreen
        assert!(!is_borderless_fullscreen_rect(work, Some(work), monitor, work, popup_style, WS_EX_TOOLWINDOW.0, false));
    }

}

fn clock_delay_ms(fullscreen:bool,ui_active:bool,power_active:bool,second:u32)->u32{if power_active||(!fullscreen&&ui_active){1000}else{(60-second).max(1)*1000}}
#[cfg(test)]mod power_clock_tests{
use super::*;
#[test]fn background_power_sampling_stays_fast_even_when_ui_is_hidden(){for fullscreen in [false,true]{assert_eq!(clock_delay_ms(fullscreen,false,true,10),1000);}assert_eq!(clock_delay_ms(false,false,false,10),50000);}
}

fn active_package(root:&std::path::Path,settings:&Settings,id:&str)->bool{crate::mods::package::PackageManager::new(root).is_active(settings,id)}
fn duplicate_toast(active:Option<&ToastNotification>,queue:&std::collections::VecDeque<(ToastKind,String,u32)>,kind:ToastKind,message:&str,now:Instant)->bool{active.is_some_and(|t|t.expires_at>now&&t.kind==kind&&t.message==message)||queue.iter().any(|(k,m,_)|*k==kind&&m==message)}
#[cfg(test)]mod package_and_notice_tests{
use super::*;
#[test]fn runtime_activation_matches_filesystem_package_and_persisted_disabled_state(){
 let root=std::env::temp_dir().join(format!("mynotch-runtime-package-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
 std::fs::create_dir_all(root.join("mods/daya")).unwrap();let manifest=crate::mods::catalog::get_catalog(&root,&Settings::default()).entries.into_iter().find(|e|e.manifest.id=="daya").unwrap().manifest;std::fs::write(root.join("mods/daya/mod.json"),serde_json::to_vec(&manifest).unwrap()).unwrap();
 let mut settings=Settings::default();assert!(active_package(&root,&settings,"daya"));
 settings.disabled_mods.push("daya".into());settings.normalize();let settings:Settings=serde_json::from_slice(&serde_json::to_vec(&settings).unwrap()).unwrap();assert!(!active_package(&root,&settings,"daya"));std::fs::remove_dir_all(root).unwrap();
}
#[test]fn repeated_mod_notice_does_not_build_a_minutes_long_queue(){
 let now=Instant::now();let toast=ToastNotification{kind:ToastKind::Generic,message:"Mod uninstalled".into(),expires_at:now+Duration::from_secs(8),confirmed:false,confirmed_at:None};
 let mut queue=std::collections::VecDeque::new();assert!(duplicate_toast(Some(&toast),&queue,ToastKind::Generic,"Mod uninstalled",now));
 queue.push_back((ToastKind::Generic,"Mod uninstalled".into(),8000));assert!(duplicate_toast(None,&queue,ToastKind::Generic,"Mod uninstalled",now));
 queue.clear();assert!(!duplicate_toast(Some(&toast),&queue,ToastKind::Generic,"Mod uninstalled",now+Duration::from_secs(9)));
 assert!(!duplicate_toast(Some(&toast),&queue,ToastKind::Generic,"Different message",now));
}
}

#[cfg(test)]
mod public_regression_tests {
use super::*;
#[test]
fn stage1_revert_rules_match_specification() {
        // Focus mode (reminder_index == 0) never reverts to calendar
        assert!(!should_revert_stage1("Reminder", 0, false));
        // Lyrics mode never reverts to calendar
        assert!(!should_revert_stage1("Musik", 0, true));
        // Calendar mode is already calendar
        assert!(!should_revert_stage1("Kalender", 0, false));

        // Sub-modes of reminder (water, stretch, sholat) revert to calendar
        assert!(should_revert_stage1("Reminder", 1, false));
        assert!(should_revert_stage1("Reminder", 2, false));
        assert!(should_revert_stage1("Reminder", 3, false));

        // Clipboard, Catatan, Musik without lyrics revert to calendar
        assert!(should_revert_stage1("Clipboard", 0, false));
        assert!(should_revert_stage1("Catatan", 0, false));
        assert!(should_revert_stage1("Cuaca", 0, false));
        assert!(should_revert_stage1("Musik", 0, false));
        assert!(!should_revert_stage1("Daya", 0, false));
        assert!(!should_revert_stage1("Power", 0, false));
        assert!(should_revert_stage1("Sistem", 0, false));
    }
}

#[cfg(test)]
mod stage1_dispatch_tests {

    #[test]
    fn test_resolve_collapsed_route_hardware_monitor_priority() {
        let mut state = State::default();
        let mut runtime = ActivityRuntime::default();
        let inputs = ActivityInputs::default();
        runtime.reconcile(&mut state, &inputs, 0, PresentationPolicy::default());
        let snapshot = runtime.snapshot.as_ref();

        // 1. Stage 1 with hw_monitor enabled, music not playing -> HardwareMonitor
        let r_hw = resolve_collapsed_route(1, false, None, false, snapshot, false, true);
        assert_eq!(r_hw, CollapsedRoute::HardwareMonitor);

        // 2. Stage 1 with hw_monitor enabled BUT music playing (lyric active) -> LyricStretch takes precedence
        let r_lyric_priority = resolve_collapsed_route(1, false, None, false, snapshot, true, true);
        assert_eq!(r_lyric_priority, CollapsedRoute::LyricStretch);

        // 3. Stage 1 with hw_monitor enabled AND Toast active -> Toast takes precedence
        let toast = ToastNotification {
            kind: ToastKind::Water,
            message: "Minum".into(),
            expires_at: Instant::now() + Duration::from_secs(5),
            confirmed: false,
            confirmed_at: None,
        };
        let r_toast = resolve_collapsed_route(1, false, Some(&toast), false, snapshot, false, true);
        assert_eq!(r_toast, CollapsedRoute::Toast);

        // 4. Quiet mode active -> Falls back to IdleClock (privacy / silence)
        let r_quiet = resolve_collapsed_route(1, false, None, true, snapshot, false, true);
        assert_eq!(r_quiet, CollapsedRoute::IdleClock);

        // 5. RecordingCollapsedSink dispatches on_hardware_monitor
        let mut sink = RecordingCollapsedSink::default();
        let r_disp = dispatch_collapsed(&mut sink, 1.0, 1, false, None, false, snapshot, false, false, true);
        assert_eq!(r_disp, CollapsedRoute::HardwareMonitor);
        assert_eq!(sink.hardware_monitor_calls, 1);
        assert_eq!(sink.alpha_depth, 0);
    }

    use super::*;
    use crate::activity::{ActivityId, ActivityInputs, ActivityRuntime, PresentationPolicy};

    #[test]
    fn test_resolve_collapsed_route_idle_clock_and_active_activities() {
        let mut state = State::default();
        let mut runtime = ActivityRuntime::default();
        let mut inputs = ActivityInputs::default();
        // Without active activities, runtime reconciles to Clock
        runtime.reconcile(&mut state, &inputs, 0, PresentationPolicy::default());
        let snapshot = runtime.snapshot.as_ref();
        assert!(matches!(snapshot.unwrap().primary.id, ActivityId::Clock));

        // P01: Stage 1, no toast, quiet false, no lyric -> IdleClock
        let route = resolve_collapsed_route(1, false, None, false, snapshot, false, false);
        assert_eq!(route, CollapsedRoute::IdleClock);

        // Task 031: Stage 1, music playing with lyric active, quiet false -> LyricStretch
        let route_lyric = resolve_collapsed_route(1, false, None, false, snapshot, true, false);
        assert_eq!(route_lyric, CollapsedRoute::LyricStretch);

        // Task 031: Stage 1, music playing with lyric active, quiet true (privacy) -> IdleClock
        let route_lyric_quiet = resolve_collapsed_route(1, false, None, true, snapshot, true, false);
        assert_eq!(route_lyric_quiet, CollapsedRoute::IdleClock);

        // P03/P04: Add HUD input -> reconciles to active HUD (priority 1 takes precedence over lyrics)
        inputs.hud(1, 0, 50, false);
        runtime.reconcile(&mut state, &inputs, 1, PresentationPolicy::default());
        let snapshot_hud = runtime.snapshot.as_ref();
        assert!(matches!(snapshot_hud.unwrap().primary.id, ActivityId::Hud(_)));
        let route_hud = resolve_collapsed_route(1, false, None, false, snapshot_hud, true, false);
        assert_eq!(route_hud, CollapsedRoute::Activity);
    }

    #[test]
    fn media_activity_does_not_hijack_stage1_idle_clock() {
        let mut state = State::default();
        let mut runtime = ActivityRuntime::default();
        let mut inputs = ActivityInputs::default();
        let track = crate::media::MediaSnapshot {
            session_id: "1".into(),
            selection_revision: 1,
            title: "Kaisarion".into(),
            playing: true,
            ..Default::default()
        };
        inputs.media(&track, 0);
        runtime.reconcile(&mut state, &inputs, 0, PresentationPolicy::default());
        let snapshot = runtime.snapshot.as_ref().unwrap();
        assert!(matches!(snapshot.primary.id, ActivityId::Media(_)));

        // No transient, no lyric mode: the media activity must fall back to the
        // idle clock instead of holding Stage 1 as a persistent now-playing bar.
        assert_eq!(resolve_collapsed_route_ex(1, false, None, false, Some(snapshot), false, false, false), CollapsedRoute::IdleClock);
        // During the short track-change transient: dedicated NowPlaying pill.
        assert_eq!(resolve_collapsed_route_ex(1, false, None, false, Some(snapshot), false, true, false), CollapsedRoute::NowPlaying);
        // Explicit lyric mode still wins.
        assert_eq!(resolve_collapsed_route_ex(1, false, None, false, Some(snapshot), true, false, false), CollapsedRoute::LyricStretch);
    }

    #[test]
    fn test_resolve_collapsed_route_toast_filtering_and_quiet_mode() {
        let mut state = State::default();
        let mut runtime = ActivityRuntime::default();
        let inputs = ActivityInputs::default();
        runtime.reconcile(&mut state, &inputs, 0, PresentationPolicy::default());
        let snapshot = runtime.snapshot.as_ref();

        let toast = ToastNotification {
            kind: ToastKind::Water,
            message: "Time to drink".into(),
            expires_at: Instant::now() + Duration::from_secs(5),
            confirmed: false,
            confirmed_at: None,
        };

        // P06: Visible toast with quiet=false -> Toast route
        let route_toast = resolve_collapsed_route(1, false, Some(&toast), false, snapshot, false, false);
        assert_eq!(route_toast, CollapsedRoute::Toast);

        // P07: Raw toast with quiet=true -> filtered out, resolves to IdleClock
        let route_quiet = resolve_collapsed_route(1, false, Some(&toast), true, snapshot, false, false);
        assert_eq!(route_quiet, CollapsedRoute::IdleClock);
    }

    #[test]
    fn test_resolve_collapsed_route_stage0_and_stage2() {
        let mut state = State::default();
        let mut runtime = ActivityRuntime::default();
        let inputs = ActivityInputs::default();
        runtime.reconcile(&mut state, &inputs, 0, PresentationPolicy::default());
        let snapshot = runtime.snapshot.as_ref();

        // P08: Stage 0 no toast -> Empty
        let route_stage0 = resolve_collapsed_route(0, false, None, false, snapshot, false, false);
        assert_eq!(route_stage0, CollapsedRoute::Empty);

        // P08: Stage 1 expanded opening to Stage 2 -> Activity
        let route_stage2 = resolve_collapsed_route(1, true, None, false, snapshot, false, false);
        assert_eq!(route_stage2, CollapsedRoute::Activity);
    }

    #[test]
    fn test_dispatch_collapsed_recording_sink_and_alpha_depth_balance() {
        let mut state = State::default();
        let mut runtime = ActivityRuntime::default();
        let mut inputs = ActivityInputs::default();
        runtime.reconcile(&mut state, &inputs, 0, PresentationPolicy::default());
        let snapshot_clock = runtime.snapshot.clone();

        // 1. IdleClock
        let mut sink1 = RecordingCollapsedSink::default();
        let r1 = dispatch_collapsed(&mut sink1, 1.0, 1, false, None, false, snapshot_clock.as_ref(), false, false, false);
        assert_eq!(r1, CollapsedRoute::IdleClock);
        assert_eq!(sink1.idle_clock_calls, 1);
        assert_eq!(sink1.alpha_depth, 0, "alpha depth must balance to 0");
        assert_eq!(sink1.max_alpha_depth, 1);

        // 2. LyricStretch (Task 031)
        let mut sink_lyric = RecordingCollapsedSink::default();
        let r_lyric = dispatch_collapsed(&mut sink_lyric, 1.0, 1, false, None, false, snapshot_clock.as_ref(), true, false, false);
        assert_eq!(r_lyric, CollapsedRoute::LyricStretch);
        assert_eq!(sink_lyric.lyric_stretch_calls, 1);
        assert_eq!(sink_lyric.alpha_depth, 0, "alpha depth must balance to 0");
        assert_eq!(sink_lyric.max_alpha_depth, 1);

        // 3. Activity
        inputs.hud(1, 0, 50, false);
        runtime.reconcile(&mut state, &inputs, 1, PresentationPolicy::default());
        let mut sink2 = RecordingCollapsedSink::default();
        let r2 = dispatch_collapsed(&mut sink2, 1.0, 1, false, None, false, runtime.snapshot.as_ref(), false, false, false);
        assert_eq!(r2, CollapsedRoute::Activity);
        assert_eq!(sink2.activity_calls, 1);
        assert_eq!(sink2.alpha_depth, 0, "alpha depth must balance to 0");
        assert_eq!(sink2.max_alpha_depth, 1);

        // 4. Toast
        let toast = ToastNotification {
            kind: ToastKind::Water,
            message: "Drink water".into(),
            expires_at: Instant::now() + Duration::from_secs(5),
            confirmed: false,
            confirmed_at: None,
        };
        let mut sink3 = RecordingCollapsedSink::default();
        let r3 = dispatch_collapsed(&mut sink3, 1.0, 1, false, Some(&toast), false, snapshot_clock.as_ref(), false, false, false);
        assert_eq!(r3, CollapsedRoute::Toast);
        assert_eq!(sink3.toast_kinds, vec![ToastKind::Water]);
        assert_eq!(sink3.alpha_depth, 0, "alpha depth must balance to 0");
        assert_eq!(sink3.max_alpha_depth, 1);

        // 5. Empty
        let mut sink4 = RecordingCollapsedSink::default();
        let r4 = dispatch_collapsed(&mut sink4, 1.0, 0, false, None, false, snapshot_clock.as_ref(), false, false, false);
        assert_eq!(r4, CollapsedRoute::Empty);
        assert_eq!(sink4.empty_calls, 1);
        assert_eq!(sink4.alpha_depth, 0, "alpha depth must balance to 0");
        assert_eq!(sink4.max_alpha_depth, 1);
    }

    #[test]
    fn test_lyric_stretch_pause_and_quiet_mode_transitions() {
        let mut state = State::default();
        let mut runtime = ActivityRuntime::default();
        let inputs = ActivityInputs::default();
        runtime.reconcile(&mut state, &inputs, 0, PresentationPolicy::default());
        let snapshot = runtime.snapshot.as_ref();

        // 1. When music is playing and lyrics active -> LyricStretch
        let route_playing = resolve_collapsed_route(1, false, None, false, snapshot, true, false);
        assert_eq!(route_playing, CollapsedRoute::LyricStretch);

        // 2. When music is paused/stopped (lyric_active = false) -> IdleClock
        let route_paused = resolve_collapsed_route(1, false, None, false, snapshot, false, false);
        assert_eq!(route_paused, CollapsedRoute::IdleClock);

        // 3. When quiet mode is active (privacy) -> IdleClock
        let route_quiet = resolve_collapsed_route(1, false, None, true, snapshot, true, false);
        assert_eq!(route_quiet, CollapsedRoute::IdleClock);
    }
}
