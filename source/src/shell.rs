use crate::{core::*, ui, media::{MediaService,MediaEvent,MediaSnapshot}, visualizer::Visualizer, render::{self,Painter}, online::{OnlineService,WeatherReport,LyricsResult}, ical::CalendarEvent};
use chrono::{Local,Datelike,Timelike,NaiveDate};
use std::{cell::RefCell,collections::BTreeMap,sync::{Arc,Mutex,mpsc},time::{Duration,Instant},thread};
use windows::core::{w,PCWSTR};
use windows::Win32::{Foundation::*,System::{LibraryLoader::GetModuleHandleW,Com::*},Graphics::Gdi::*,Media::{timeSetEvent,timeKillEvent,TIME_PERIODIC},UI::{Controls::WM_MOUSELEAVE,WindowsAndMessaging::*,Input::KeyboardAndMouse::*,HiDpi::*}};

mod pages;
mod reminder;
mod settings;
const WAKE:u32=WM_APP+1;const TRAY:u32=WM_APP+2;const MORPH_MSG:u32=WM_APP+5;
unsafe extern "system" {
    fn MessageBeep(utype: u32) -> i32;
}
thread_local!{static APP:RefCell<Option<Shell>>=const{RefCell::new(None)};}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToastKind {
    Water,
    Stretch,
    Prayer,
    Focus,
    Generic,
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
    Tab(String),Settings,Back,Close,Exit,Bottom(bool),Minimal,Gap(i32),Scale(f32),Opacity(i32),ToggleModule(String),Toggle(&'static str),
    Play,Next,Previous,Seek(f64),SeekPointer,LyricsFollow,Lyric(usize),FocusStart,FocusPause,FocusReset,FocusBreak,FocusLength(i32),BreakLength(i32),
    AddTodo,Todo(u64),DeleteTodo(u64),TodoPage(i32),SaveNote,Water,UndoWater,WaterTarget(i32),WaterServing(i32),Breath,
    Month(i32),Day(u32),JumpToDate(i32,u32,u32),CalendarSave,CalendarRefresh,WeatherSave,WeatherRefresh,AiSave,AiSend,AiClear,
    ToggleLyrics,SelectMode(usize),
    CalendarToday,CalendarLeaveToggle,
    TogglePin,ToStage0,ToStage1,
    WaterInterval(i32),MoveInterval(i32),ReminderSelect(usize),ReminderStep(i32),StretchLength(i32),StretchStart,StretchDone,PrayerLead(i32),PrayerRefresh,ReminderMenu(&'static str),ReminderValue(&'static str,u32),
    ConfirmWater,SelectClip(usize),CopyClip(usize),DeleteClip(u64),ClearClips,ClipPage(i32),
    MoveTab(String, i32),
    SettingsCategory(usize),SetDefaultTab(String),SettingsMenu(&'static str,f32,f32,f32),SettingsValue(&'static str,u32),SettingsOpenTab(String,usize),
    InstallMod(String),UninstallMod(String),NoteAdd,NoteSelect(usize),NoteDelete(u64),NoteColor(u8),NoteExport(u8),NotePopout,
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
#[derive(Clone)]struct LyricAnim{prev_text:String,started:Instant}
#[derive(Clone)]struct Poster{queue:Arc<Mutex<Vec<Event>>>,hwnd:usize}
impl Poster{fn send(&self,event:Event){if let Ok(mut q)=self.queue.lock(){if q.len()>64{q.remove(0);}q.push(event);}unsafe{let _=PostMessageW(Some(HWND(self.hwnd as *mut _)),WAKE,WPARAM(0),LPARAM(0));}}}
#[allow(dead_code)]
enum Event {
    Prayers(String,String,Result<crate::online::PrayerSchedule,String>),
    Artwork(u64,Result<crate::artwork::Artwork,String>),
    Completed(&'static str,u64,Box<Event>),
    Media(MediaEvent),Levels(Vec<f32>),Lyrics(String,Result<LyricsResult,String>),Weather(Result<WeatherReport,String>),
    Agenda(Result<Vec<CalendarEvent>,String>),Ai(Result<String,String>),Notice(String),
}

#[allow(dead_code)]
pub(super) struct Shell {
    suspended:bool,
    render_ready:bool,
    cover:Option<crate::artwork::Artwork>,cover_key:u64,preview_silent:bool,
    online:OnlineService,
    drafts:BTreeMap<(String,u16),String>,input_page:String,
    jobs:BTreeMap<&'static str,(u64,crate::net::Cancellation)>,next_job:u64,
    hwnd:HWND,inputs_hwnd:HWND,input_x:i32,input_y:i32,canvas_w:i32,canvas_h:i32,win_x:i32,win_y:i32,painter:Option<Painter>,store:Store,settings:Settings,state:State,
    expanded:bool,settings_open:bool,settings_cat:usize,settings_popup:Option<settings::SettingsPopup>,tab:String,scale:f32,width:f32,height:f32,hits:Vec<Hit>,inputs:BTreeMap<u16,HWND>,
    morph:Option<ui::Morph>,pointer:Option<(f32,f32)>,hit_enabled:bool,pressed:Option<Action>,focused:Option<Action>,tab_fade:Option<(String,Instant,i8)>,painting:bool,morph_timer:u32,
    media:Option<MediaService>,track:MediaSnapshot,visualizer:Option<Visualizer>,levels:Vec<f32>,
    lyrics:Vec<crate::lyrics::LyricLine>,plain_lyrics:String,lyrics_key:String,lyric_offset:usize,
    focus:FocusClock,breathing:Option<Instant>,last_water:Instant,last_move:Instant,last_eye:Instant,
    toast:Option<ToastNotification>,queue:Arc<Mutex<Vec<Event>>>,fullscreen:bool,toast_from_stage0:bool,
    last_clipboard_seq:u32,selected_clip:usize,clip_offset:usize,
    year:i32,month:u32,day:u32,holidays:Vec<HolidayRecord>,agenda:Vec<CalendarEvent>,weather:Option<WeatherReport>,
    ai_messages:Vec<crate::online::ChatMessage>,busy:bool,todo_offset:usize,
    prayer_schedule:Option<crate::online::PrayerSchedule>,prayer_attempt:Option<(String,Instant)>,prayer_error:Option<String>,reminder_menu:Option<&'static str>,reminder_menu_cursor:usize,
    reminder_index:usize,reminder_anim:Option<(f32,Instant)>,stretch_deadline:Option<Instant>,toast_queue:std::collections::VecDeque<(ToastKind,String,u32)>,last_schedule:String,weather_updated:Option<Instant>,agenda_updated:Option<Instant>,preview:bool,
    lyric_mode:bool,hover_stage:u8,pinned:bool,
    hover_blocked_until:Instant,
    last_lyric_line:String,lyric_anim:Option<LyricAnim>,
    tab_drag:Option<TabDragState>,
    /// Startup fade-in: ramps 0→1 over ~300 ms (20 ticks × 16 ms via timer 11).
    /// Permanently 1.0 once fully visible; timer 11 is killed at that point.
    startup_alpha:f32,
}
#[derive(Clone,serde::Deserialize)]struct HolidayRecord{date:NaiveDate,name:String,#[serde(default)]collective_leave:bool}
#[derive(serde::Deserialize)]struct HolidayFile{holidays:Vec<HolidayRecord>}

pub fn run(args:&[String])->Result<(),String>{unsafe{
    let _=SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    if args.iter().any(|a|a=="--fullscreen-fixture"){return fullscreen_fixture()}
    struct Instance(HANDLE);impl Drop for Instance{fn drop(&mut self){unsafe{let _=CloseHandle(self.0);}}}
    let preview_instance=args.iter().any(|s|matches!(s.as_str(),"--preview"|"--live-preview"));
    let handle=windows::Win32::System::Threading::CreateMutexW(None,false,if preview_instance{w!("Local\\mynotch.Preview.v0.0.1")}else{w!("Local\\mynotch.Native.v0.0.1")}).map_err(|e|e.to_string())?;
    let duplicate=GetLastError()==ERROR_ALREADY_EXISTS;let _instance=Instance(handle);
    if duplicate{
        if let Ok(existing)=FindWindowW(w!("mynotch.Native"),PCWSTR::null()){
            let _=ShowWindow(existing,SW_SHOWNOACTIVATE);
            let _=SetForegroundWindow(existing);
            let _=PostMessageW(Some(existing),WM_COMMAND,WPARAM(900),LPARAM(0));
        }
        return Ok(())
    }
    let _=CoInitializeEx(None,COINIT_APARTMENTTHREADED);
    let module=GetModuleHandleW(None).map_err(|e|e.to_string())?;
    let class=WNDCLASSW{lpfnWndProc:Some(wndproc),hInstance:module.into(),lpszClassName:w!("mynotch.Native"),hCursor:LoadCursorW(None,IDC_ARROW).unwrap_or_default(),..Default::default()};
    RegisterClassW(&class);
    // The shell presents through a DirectComposition swap chain, and a window with
    // WS_EX_NOREDIRECTIONBITMAP cannot show child HWNDs. The EDIT controls therefore live in
    // this separate standard window, which is clipped by a region to exactly the input
    // rectangles: everything outside the region is neither painted nor clickable, so the shell
    // behind it keeps receiving every other click.
    let input_class=WNDCLASSW{lpfnWndProc:Some(inputs_wndproc),hInstance:module.into(),lpszClassName:w!("mynotch.Inputs"),..Default::default()};
    RegisterClassW(&input_class);
    let inputs_hwnd=CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_TOPMOST,w!("mynotch.Inputs"),None,WS_POPUP,0,0,0,0,None,None,Some(module.into()),None).map_err(|e|e.to_string())?;
    let live_preview=args.iter().any(|s|s=="--live-preview");
    let preview=live_preview||args.iter().any(|s|s=="--preview");
    let hwnd=CreateWindowExW(if preview{WS_EX_NOREDIRECTIONBITMAP|WS_EX_APPWINDOW|WS_EX_TOPMOST}else{WS_EX_NOREDIRECTIONBITMAP|WS_EX_TOOLWINDOW|WS_EX_TOPMOST},w!("mynotch.Native"),w!("mynotch 0.0.1 alpha"),WS_POPUP,0,0,400,112,None,None,Some(module.into()),None).map_err(|e|e.to_string())?;
    SetWindowLongPtrW(inputs_hwnd,GWLP_HWNDPARENT,hwnd.0 as isize);
    let executable=std::env::current_exe().map_err(|_|"Lokasi aplikasi tidak ditemukan.")?;
    let root=crate::app::storage_root(&executable,preview)?;
    let mut store=Store::new(root)?;let mut settings:Settings=store.load("settings.json");settings.normalize();if preview&&args.iter().any(|a|a=="--edge-check"){settings.bottom=false;settings.edge_gap=0;if let Some(scale)=args.iter().find_map(|a|a.strip_prefix("--ui-scale=").and_then(|v|v.parse::<f32>().ok())){settings.scale=scale;settings.normalize();}}let state:State=store.load("state.json");
    let now=Local::now();let queue=Arc::new(Mutex::new(Vec::new()));let online=OnlineService::new(store.root.clone());
    let init_stage=initial_hover_stage(preview,settings.minimal);
    let mut app=Shell{
        suspended:false,
        render_ready:false,
        cover:None,cover_key:0,preview_silent:preview&&!live_preview,
        online,drafts:BTreeMap::new(),input_page:String::new(),jobs:BTreeMap::new(),next_job:0,
        hwnd,inputs_hwnd,input_x:0,input_y:0,canvas_w:0,canvas_h:0,win_x:0,win_y:0,painter:None,store,settings,state,expanded:preview,settings_open:false,settings_cat:0,settings_popup:None,tab:"Musik".into(),scale:1.,width:if preview{ui::PLAYER_WIDTH}else{220.},height:if preview{ui::PLAYER_HEIGHT}else{30.},hits:vec![],inputs:BTreeMap::new(),morph:None,pointer:None,hit_enabled:true,pressed:None,focused:None,tab_fade:None,painting:false,morph_timer:0,media:None,track:MediaSnapshot::default(),visualizer:None,levels:vec![0.;24],lyrics:vec![],plain_lyrics:String::new(),lyrics_key:String::new(),lyric_offset:0,focus:FocusClock::default(),breathing:None,last_water:Instant::now(),last_move:Instant::now(),last_eye:Instant::now(),toast:None,toast_from_stage0:false,last_clipboard_seq:0,selected_clip:0,clip_offset:0,queue,fullscreen:false,year:now.year(),month:now.month(),day:now.day(),holidays:vec![],agenda:vec![],weather:None,ai_messages:vec![],busy:false,todo_offset:0,prayer_schedule:None,prayer_attempt:None,prayer_error:None,reminder_menu:None,reminder_menu_cursor:0,reminder_index:1,reminder_anim:None,stretch_deadline:None,toast_queue:Default::default(),last_schedule:String::new(),weather_updated:None,agenda_updated:None,preview,lyric_mode:false,hover_stage:init_stage,pinned:false,hover_blocked_until:Instant::now(),startup_alpha:if preview{1.}else{0.},last_lyric_line:String::new(),lyric_anim:None,tab_drag:None,};
    app.tab=app.settings.default_tab.clone();
    if !app.settings.enabled(&app.tab)||!MODULES.contains(&app.tab.as_str()){app.tab=app.settings.active_tab_order().first().cloned().unwrap_or_default();}
    app.load_holidays();
    let init_pw=(ui::CANVAS_WIDTH*app.scale).max(1.)as u32;
    let init_ph=(ui::CANVAS_HEIGHT*app.scale).max(1.)as u32;
    app.painter=Painter::new(hwnd,init_pw,init_ph,96.*app.scale).ok();
    if app.painter.is_none(){return Err("Renderer tidak tersedia. Perbarui driver grafis atau jalankan dengan --software-renderer.".into())}
    app.layout();
    app.update_window_region();
    app.make_inputs();app.tray(true);
    crate::clipboard::listen_clipboard(hwnd);
    app.sync_clipboard();
    let _=RegisterHotKey(Some(hwnd),1,MOD_ALT|MOD_NOREPEAT,0x4e);
    if !app.preview_silent{app.start_media();}
    
    app.clock_timer();
    if !app.preview{SetTimer(Some(hwnd),11,16,None);}
    SetTimer(Some(hwnd),5,150,None);
    if args.iter().any(|s|s=="--suspend-fixture"){SetTimer(Some(hwnd),7,4000,None);SetTimer(Some(hwnd),8,7500,None);}
    APP.with(|cell|*cell.borrow_mut()=Some(app));
    let _=ShowWindow(hwnd,SW_SHOWNOACTIVATE);let _=InvalidateRect(Some(hwnd),None,false);
    let mut message=MSG::default();while GetMessageW(&mut message,None,0,0).as_bool(){let _=TranslateMessage(&message);DispatchMessageW(&message);}
    APP.with(|cell|{cell.borrow_mut().take();});CoUninitialize();Ok(())
}}

unsafe extern "system" fn wndproc(hwnd:HWND,msg:u32,wparam:WPARAM,lparam:LPARAM)->LRESULT{
    // DestroyWindow dispatches WM_DESTROY reentrantly while the command handler
    // already borrows APP. Quit must not depend on obtaining that borrow again.
    if msg==WM_DESTROY{unsafe{PostQuitMessage(0);}return LRESULT(0)}
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
            WM_CTLCOLOREDIT=>{let dc=HDC(wparam.0 as *mut _);SetTextColor(dc,COLORREF(0x00f5f1e9));SetBkColor(dc,COLORREF(0x002d2319));SetDCBrushColor(dc,COLORREF(0x002d2319));return LRESULT(GetStockObject(DC_BRUSH).0 as isize)},
            WM_ERASEBKGND=>LRESULT(1),
            _=>DefWindowProcW(hwnd,msg,wparam,lparam),
        }
    }
}

/// timeSetEvent invokes this on its own worker thread. It must not touch any UI state — it only
/// nudges the message pump, which then runs the morph on the UI thread where painting is safe.
unsafe extern "system" fn morph_tick(_id:u32,_msg:u32,user:usize,_dw1:usize,_dw2:usize){
    unsafe{let _=PostMessageW(Some(HWND(user as *mut _)),MORPH_MSG,WPARAM(0),LPARAM(0));}
}

impl Shell {
    fn power_state(&mut self,suspended:bool){
        if suspended==self.suspended{return}self.suspended=suspended;
        if suspended{
            self.cancel_jobs();
            unsafe{
                for id in [1,2,3,4,5,6,10,11,12]{let _=KillTimer(Some(self.hwnd),id);}
                if self.morph_timer!=0{let _=timeKillEvent(self.morph_timer);self.morph_timer=0;}
            }
            self.morph=None;self.tab_fade=None;
            if let Some(mut media)=self.media.take(){media.stop();}
            if let Some(mut capture)=self.visualizer.take(){capture.stop();}
            self.track.playing=false;self.levels.fill(0.);self.save();
        }else{
            self.lyrics_key.clear();
            self.start_media();
            self.clock_timer();
            unsafe{SetTimer(Some(self.hwnd),5,150,None);}
            self.tick();
        }
        // A morph that was cut short by suspend must not leave the window at an in-between size.
        self.layout();
        if self.preview{eprintln!("suspended={} capture={}",self.suspended,self.visualizer.is_some());}
    }

    fn job(&mut self,module:&'static str,work:impl FnOnce()->Event+Send+'static){
        if self.fullscreen||self.suspended{return}self.cancel_job(module);self.next_job+=1;let id=self.next_job;
        let token=crate::net::Cancellation::default();self.jobs.insert(module,(id,token.clone()));self.busy=self.jobs.keys().any(|m|matches!(*m,"AI"|"Cuaca"|"Kalender"));
        let post=self.poster();thread::spawn(move||{let result=crate::net::with_cancel(token.clone(),work);if !token.is_cancelled(){post.send(Event::Completed(module,id,Box::new(result)));}});
    }
    fn cancel_job(&mut self,module:&str){if let Some((_,token))=self.jobs.remove(module){token.cancel();}self.busy=self.jobs.keys().any(|m|matches!(*m,"AI"|"Cuaca"|"Kalender"));}
    fn cancel_jobs(&mut self){for(_,(_,token))in std::mem::take(&mut self.jobs){token.cancel();}self.busy=false;}

    fn poster(&self)->Poster{Poster{queue:self.queue.clone(),hwnd:self.hwnd.0 as usize}}
    fn relay<T:Send+'static>(&self,receiver:mpsc::Receiver<T>,map:fn(T)->Event){let post=self.poster();thread::spawn(move||while let Ok(event)=receiver.recv(){post.send(map(event));});}
    fn save(&mut self){if let Err(e)=self.store.save("settings.json",&self.settings){self.notice(e);}if let Err(e)=self.store.save("state.json",&self.state){self.notice(e);}}
    fn notice(&mut self,text:impl Into<String>){
        let msg=text.into();
        let kind = classify_notice_kind(&msg);
        self.show_toast(kind, msg, 8000);
    }
    fn show_water_toast(&mut self){
        let serving = self.settings.water_serving;
        let msg = format!("Waktunya minum air! (+{} ml)", serving);
        self.show_toast(ToastKind::Water, msg, 12000);
    }
    fn show_toast(&mut self, kind: ToastKind, text: impl Into<String>, timeout_ms: u32){
        let msg=text.into();
        if self.toast.as_ref().is_some_and(|t|t.expires_at>Instant::now()){
            if self.toast_queue.len()<16{self.toast_queue.push_back((kind,msg,timeout_ms));}return
        }
        let from=self.geometry();
        let from_stage0 = self.hover_stage == 0;
        self.toast_from_stage0 = from_stage0;
        self.toast=Some(ToastNotification {
            kind,
            message: msg,
            expires_at: Instant::now() + Duration::from_millis(timeout_ms as u64),
            confirmed: false,
            confirmed_at: None,
        });
        if kind!=ToastKind::Prayer||self.settings.prayer_sound{unsafe{let _=MessageBeep(0x00000040);}}
        if from_stage0 {
            self.set_hover_stage(1);
        }
        self.morph=Some(ui::Morph::new(from,self.target_geometry()));
        self.start_morph_timer();
        unsafe{SetTimer(Some(self.hwnd),3,timeout_ms,None);}
        self.layout();
        self.invalidate();
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
                    let chars = text.chars().count();
                    let lines = text.lines().count().max(1);
                    self.state.clipboard_history.insert(0, crate::clipboard::ClipItem {
                        id: Local::now().timestamp_millis() as u64,
                        text,
                        time: now_str,
                        chars,
                        lines,
                    });
                    if self.state.clipboard_history.len() > 50 {
                        self.state.clipboard_history.truncate(50);
                    }
                    let _ = self.store.save("state.json", &self.state);
                    self.invalidate();
                }
            }
        }
    }
    fn invalidate(&mut self){if self.suspended{return}self.paint();}
    fn sync_foreground(&mut self){unsafe{
        let foreground=GetForegroundWindow();
        let mut window=RECT::default();
        let full=if foreground==self.hwnd||foreground==self.inputs_hwnd||foreground.0.is_null()||foreground==GetShellWindow()||foreground==GetDesktopWindow()||IsIconic(foreground).as_bool(){false}else{
            let my_monitor=MonitorFromWindow(self.hwnd,MONITOR_DEFAULTTONEAREST);
            let monitor=MonitorFromWindow(foreground,MONITOR_DEFAULTTONEAREST);
            let mut info=MONITORINFO{cbSize:std::mem::size_of::<MONITORINFO>()as u32,..Default::default()};
            let covers=GetWindowRect(foreground,&mut window).is_ok()&&GetMonitorInfoW(monitor,&mut info).as_bool()&&covers_monitor(window,info.rcMonitor);
            (monitor==my_monitor)&&covers
        };
        let changed=full!=self.fullscreen;
        if changed{self.fullscreen=full;if full{self.cancel_jobs();self.morph=None;self.tab_fade=None;if self.morph_timer!=0{let _=timeKillEvent(self.morph_timer);self.morph_timer=0;}let _=KillTimer(Some(self.hwnd),2);let _=KillTimer(Some(self.hwnd),4);let _=KillTimer(Some(self.hwnd),10);}else{self.lyrics_key.clear();self.poster().send(Event::Media(MediaEvent::Changed(self.track.clone())));}self.sync_visualizer();self.clock_timer();self.layout();}
        let hidden=self.fullscreen&&self.settings.hide_fullscreen;
        if IsWindowVisible(self.hwnd).as_bool()==hidden{let _=ShowWindow(self.hwnd,if hidden{SW_HIDE}else{SW_SHOWNOACTIVATE});if !hidden{self.layout();self.invalidate();}self.sync_input_host();}
        if changed&&self.preview{eprintln!("lifecycle fullscreen={} visible={} capture={} jobs={}",self.fullscreen,IsWindowVisible(self.hwnd).as_bool(),self.visualizer.is_some(),self.jobs.len());}
    }}
    fn start_media(&mut self){
        if self.suspended||self.preview_silent||!self.settings.enabled("Musik")||self.media.is_some(){return}
        let(tx,rx)=mpsc::channel();match MediaService::start(tx){Ok(service)=>{self.media=Some(service);self.relay(rx,Event::Media);},Err(e)=>self.notice(format!("Media Windows belum tersedia: {e}"))}
    }
    fn sync_visualizer(&mut self){
        let policy=WorkPolicy{enabled:self.settings.enabled("Musik")&&self.settings.visualizer,visible:!self.fullscreen&&!(self.settings.minimal&&!self.expanded),fullscreen:self.fullscreen||self.suspended,playing:self.track.playing};
        if policy.visualize()&&!self.preview_silent{if self.visualizer.is_none(){let(tx,rx)=mpsc::channel();match Visualizer::start(tx){Ok(v)=>{self.visualizer=Some(v);self.relay(rx,Event::Levels);if self.preview{eprintln!("capture started playing={} fullscreen={}",self.track.playing,self.fullscreen);}},Err(e)=>{self.levels.fill(0.);self.notice(format!("Visualizer belum tersedia: {e}"));}}}}
        else if let Some(mut v)=self.visualizer.take(){v.stop();self.levels.fill(0.);if self.preview{eprintln!("capture stopped playing={} fullscreen={}",self.track.playing,self.fullscreen);}}
    }
    /// Stable geometry the shell is heading towards. The pill radius is half its height,
    /// which is what makes the compact state a capsule instead of a rounded rectangle.
    fn target_geometry(&self)->(f32,f32,f32){let(w,h,r)=self.base_geometry();(w,if self.expanded&&self.toast.is_some(){h+30.}else{h},r)}
    fn base_geometry(&self)->(f32,f32,f32){
        if self.hover_stage>=2||self.expanded{
            if self.settings_open{
                (560.,360.,27.)
            }else if self.tab=="Musik"{
                if self.lyric_mode{
                    (530.,186.,28.)
                }else{
                    (460.,180.,28.)
                }
            }else if self.tab=="Kalender"{
                (540.,280.,24.)
            }else if self.tab=="Reminder"{
                (550.,if self.reminder_index==3{362.}else{346.},27.)
            }else if self.tab=="Clipboard"{
                (530.,236.,24.)
            }else{
                (ui::EXPANDED_WIDTH,ui::EXPANDED_HEIGHT,24.)
            }
        }else if let Some(toast)=&self.toast{
            if toast.kind == ToastKind::Water {
                if toast.confirmed {
                    (280., 30., 15.)
                } else {
                    (320., 30., 15.)
                }
            } else {
                let tw = (toast.message.chars().count() as f32 * 7.5 + 48.).clamp(160., 440.);
                (tw, 30., 15.)
            }
        }else if self.hover_stage==1{
            if self.lyric_mode{
                (300.,30.,15.)
            }else if self.tab == "Kalender" {
                (158.,30.,15.)
            }else if self.tab == "Reminder" {
                (158.,30.,15.)
            }else if self.tab == "Clipboard" {
                (195.,30.,15.)
            }else if self.tab == "Musik" {
                if !self.track.playing && self.track.title.is_empty() {
                    (158.,30.,15.)
                } else {
                    let len = if !self.track.artist.is_empty() {
                        self.track.title.chars().count() + self.track.artist.chars().count() + 3
                    } else {
                        self.track.title.chars().count()
                    };
                    let w = (len as f32 * 7.2 + 110.).clamp(180., 340.);
                    (w, 30., 15.)
                }
            }else{
                (180.,30.,15.)
            }
        }else{
            (50.,10.,5.)
        }
    }
    fn set_hover_stage(&mut self,stage:u8){
        if self.fullscreen{return}
        if self.hover_stage==stage{return}
        if stage==0{
            self.hover_blocked_until=Instant::now()+Duration::from_millis(600);
        }
        let from=self.geometry();
        self.hover_stage=stage;
        self.expanded=stage>=2;
        if stage<2{
            self.settings_open=false;
            self.settings_popup=None;
            self.reminder_menu=None;
            self.focused=None;
        }
        self.morph=Some(ui::Morph::new(from,self.target_geometry()));
        self.update_window_region();
        if self.hover_stage<2{self.make_inputs();}
        self.sync_visualizer();self.clock_timer();self.start_morph_timer();self.invalidate();
    }
    fn geometry(&self)->(f32,f32,f32){match &self.morph{Some(morph)=>morph.sample(),None=>self.target_geometry()}}
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
    fn layout(&mut self){unsafe{
        let mut area=RECT::default();let _=SystemParametersInfoW(SPI_GETWORKAREA,0,Some((&mut area as *mut RECT).cast()),SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
        let dpi=GetDpiForWindow(self.hwnd).max(96)as f32/96.;self.scale=(dpi*self.settings.scale).min(((area.bottom-area.top-24)as f32/ui::EXPANDED_HEIGHT).max(0.6));
        let(width,height,_radius)=self.geometry();
        self.width=width;self.height=height;
        let monitor=MonitorFromWindow(self.hwnd,MONITOR_DEFAULTTONEAREST);
        let mut monitor_info=MONITORINFO{cbSize:std::mem::size_of::<MONITORINFO>()as u32,..Default::default()};
        let monitor_top=if GetMonitorInfoW(monitor,&mut monitor_info).as_bool(){monitor_info.rcMonitor.top}else{area.top};
        let pixel_width=((ui::CANVAS_WIDTH)*self.scale).max(1.)as i32;
        let pixel_height=((ui::CANVAS_HEIGHT)*self.scale).max(1.)as i32;
        let x=area.left+(area.right-area.left-pixel_width)/2;let y=ui::window_top(self.settings.bottom,self.settings.edge_gap,self.scale,area.top,area.bottom,monitor_top);
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
        let (width, height) = match &self.morph {
            Some(morph) => {
                let from = morph.from();
                let to = morph.to();
                (from.0.max(to.0), from.1.max(to.1))
            }
            None => {
                let g = self.geometry();
                (g.0, g.1)
            }
        };
        let ear = if !self.settings.bottom && self.settings.edge_gap == 0 {
            if self.hover_stage == 0 && !self.expanded && !self.lyric_mode {
                6.
            } else if height >= 24. {
                12.
            } else {
                (height * 0.5).max(4.)
            }
        } else {
            0.
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
    fn start_morph_timer(&mut self){unsafe{
        let needs_timer=self.morph.is_some()||self.lyric_anim.is_some()||self.tab_fade.is_some()||self.reminder_anim.is_some();
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
        }
        if self.tab_fade.as_ref().is_some_and(|(_,t,_)|t.elapsed()>=ui::TAB_FADE_DURATION){
            self.tab_fade=None;
        }
        if self.lyric_anim.as_ref().is_some_and(|a|a.started.elapsed()>=Duration::from_millis(280)){
            self.lyric_anim=None;
        }
        self.start_morph_timer();
        let (width,height,_)=self.geometry();
        self.width=width;self.height=height;
        self.invalidate();
    }
    fn sync_active_lyric(&mut self){
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
            if self.lyric_mode&&self.hover_stage==1{
                let from=self.geometry();
                self.morph=Some(ui::Morph::new(from,self.target_geometry()));
                self.update_window_region();
                self.start_morph_timer();
            }
            self.invalidate();
        }
    }
    fn set_expanded(&mut self,value:bool){
        self.set_hover_stage(if value{2}else{collapse_target_stage(self.settings.minimal)});
    }
    fn clock_timer(&self){unsafe{let now=Local::now();let delay=if !self.fullscreen&&(self.focus.running()||self.breathing.is_some()||self.expanded||self.lyric_mode||self.track.playing||self.settings.water_reminder||self.settings.move_reminder||self.settings.prayer_enabled||self.stretch_deadline.is_some()){1000}else{(60-now.second()).max(1)*1000};SetTimer(Some(self.hwnd),1,delay,None);}}
    fn make_inputs(&mut self){unsafe{
        for &id in self.inputs.keys(){self.drafts.insert((self.input_page.clone(),id),self.input(id));}
        if self.input_page=="Tugas"{if let Some(note)=self.drafts.get(&("Tugas".into(),12)){self.state.notes=note.clone();}}
        for(_,hwnd)in std::mem::take(&mut self.inputs){let _=DestroyWindow(hwnd);}if !self.expanded{self.rebuild_input_region();self.sync_input_host();return}
        self.input_page=if self.settings_open{"Pengaturan".into()}else{self.tab.clone()};
        if self.settings_open{
            self.rebuild_input_region();self.sync_input_host();return
        }
        match self.tab.as_str(){

            "Tugas"=>{self.edit(10,"",16.,46.,220.,28.,false,false);self.edit(11,"",244.,46.,100.,28.,false,false);let notes=self.state.notes.clone();self.edit(12,&notes,16.,184.,330.,68.,false,true);},
            "Cuaca"=>{let city=self.settings.weather_city.clone();self.edit(30,&city,28.,172.,140.,28.,false,false);self.edit(31,&self.settings.latitude.to_string(),176.,172.,86.,28.,false,false);self.edit(32,&self.settings.longitude.to_string(),270.,172.,86.,28.,false,false);},
            "AI"=>{let w=self.width;self.edit(40,"",28.,124.,w-56.,50.,false,true);self.edit(41,&self.settings.ai_endpoint.clone(),28.,72.,240.,26.,false,false);self.edit(42,&self.settings.ai_model.clone(),276.,72.,140.,26.,false,false);self.edit(43,"",28.,104.,240.,26.,true,false);},
            _=>{}
        }
        self.rebuild_input_region();self.sync_input_host();
    }}
    unsafe fn edit(&mut self,id:u16,value:&str,x:f32,y:f32,width:f32,height:f32,password:bool,multiline:bool){unsafe{
        let value=self.drafts.get(&(self.input_page.clone(),id)).map(|v|v.as_str()).unwrap_or(value);
        let value:Vec<u16>=value.encode_utf16().chain(Some(0)).collect();
        let style=WS_CHILD|WS_VISIBLE|WS_TABSTOP|WS_BORDER|WINDOW_STYLE(if multiline{(ES_MULTILINE|ES_AUTOVSCROLL|ES_WANTRETURN)as u32|WS_VSCROLL.0}else{ES_AUTOHSCROLL as u32})|WINDOW_STYLE(if password{ES_PASSWORD as u32}else{0});
        // Parent is the input host, not the composition window: child HWNDs do not render on a
        // WS_EX_NOREDIRECTIONBITMAP parent. The host sits exactly on top of the shell and is
        // clipped to these rectangles, so the shell keeps every click outside them.
        // Coordinates are shifted by the shadow margin because the shell window now starts
        // SHADOW_PAD above and left of the capsule.
        let (capsule_x,capsule_y)=self.capsule_origin();
        if let Ok(edit)=CreateWindowExW(WINDOW_EX_STYLE(0),w!("EDIT"),PCWSTR(value.as_ptr()),style,((x+capsule_x)*self.scale)as i32,((y+capsule_y)*self.scale)as i32,(width*self.scale)as i32,(height*self.scale)as i32,Some(self.inputs_hwnd),Some(HMENU(id as usize as *mut _)),None,None){
            SendMessageW(edit,WM_SETFONT,Some(WPARAM(GetStockObject(DEFAULT_GUI_FONT).0 as usize)),Some(LPARAM(1)));self.inputs.insert(id,edit);
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
            0x031D=>{self.sync_clipboard();return LRESULT(0)},
            WM_PAINT=>{let mut ps=PAINTSTRUCT::default();BeginPaint(self.hwnd,&mut ps);self.paint();let _=EndPaint(self.hwnd,&ps);return LRESULT(0)},
            WM_ERASEBKGND=>return LRESULT(1),
            WM_NCHITTEST=>{
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
                let mut tracking=TRACKMOUSEEVENT{cbSize:std::mem::size_of::<TRACKMOUSEEVENT>()as u32,dwFlags:TME_LEAVE,hwndTrack:self.hwnd,dwHoverTime:0};
                let _=TrackMouseEvent(&mut tracking);
                let _=KillTimer(Some(self.hwnd),2);
                let (x,y)=self.client_to_capsule(lparam);
                let moved=self.pointer.is_none_or(|(px,py)|(px-x).abs()>0.5||(py-y).abs()>0.5);
                if moved{self.pointer=Some((x,y));self.invalidate();}

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
                    SetTimer(Some(self.hwnd),2,800,None);
                }
                return LRESULT(0)
            },
            WM_LBUTTONDOWN=>{let (x,y)=self.client_to_capsule(lparam);
                if self.settings_popup.is_some(){let inside=self.hits.iter().rev().find(|h|x>=h.rect.left&&x<=h.rect.right&&y>=h.rect.top&&y<=h.rect.bottom).is_some_and(|h|matches!(h.action,Action::SettingsMenu(..)|Action::SettingsValue(..)|Action::SettingsCategory(..)|Action::Back));if !inside{self.settings_popup=None;self.pressed=None;self.invalidate();return LRESULT(0)}}
                if self.reminder_menu.is_some(){let inside=self.hits.iter().rev().find(|h|x>=h.rect.left&&x<=h.rect.right&&y>=h.rect.top&&y<=h.rect.bottom).is_some_and(|h|matches!(h.action,Action::ReminderValue(..)|Action::ReminderMenu(..)));if !inside{self.reminder_menu=None;self.pressed=None;self.invalidate();return LRESULT(0)}}
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
                    return LRESULT(0);
                }
                if self.hover_stage==0{
                    self.set_hover_stage(1);
                    return LRESULT(0);
                }else if self.hover_stage==1{
                    self.set_hover_stage(2);
                    return LRESULT(0);
                }
                return LRESULT(0)},
            WM_MOUSEWHEEL if self.expanded&&self.settings_open=>{let delta=(wparam.0>>16)as i16;if self.settings_popup.is_some(){self.handle_settings_key(if delta>0{0x26}else{0x28});}return LRESULT(0)},
            WM_MOUSEWHEEL if (self.expanded || self.hover_stage == 1)&&!self.settings_open=>{
                let mut point=POINT{x:(lparam.0 as i16)as i32,y:((lparam.0>>16)as i16)as i32};let _=ScreenToClient(self.hwnd,&mut point);
                let (ox,oy)=self.capsule_origin();
                let rel_x=point.x as f32/self.scale-ox;
                let rel_y=point.y as f32/self.scale-oy;
                let wheel_delta=(wparam.0>>16)as i16;
                if self.hover_stage == 1 || rel_y<=42.0{
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
                if self.tab=="Reminder"&&self.reminder_menu.is_some()&&rel_x>=229.{self.handle_reminder_menu_key(if wheel_delta>0{0x26}else{0x28});return LRESULT(0);}
                if self.tab=="Reminder"&&rel_x<208.&&rel_y>48.{self.action(Action::ReminderStep(if wheel_delta>0{-1}else{1}));return LRESULT(0);}
                if self.tab=="Kalender"&&rel_x<308.&&rel_y>42.{self.action(Action::Month(if wheel_delta>0{-1}else{1}));return LRESULT(0);}
                if self.tab=="Clipboard"&&rel_y>42.{self.action(Action::ClipPage(if wheel_delta>0{-1}else{1}));return LRESULT(0);}
                if self.tab=="Musik"&&self.lyric_mode&&rel_x>=250.{
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
                1=>self.tick(),
                2=>{
                    let _=KillTimer(Some(self.hwnd),2);
                    let focused=GetFocus();
                    if !self.inputs.values().any(|&h|h==focused) && !self.pinned && self.hover_stage==2 && self.settings_popup.is_none() && self.reminder_menu.is_none() {
                        self.set_hover_stage(collapse_target_stage(self.settings.minimal));
                    }
                },
                3=>{
                    let from=self.geometry();
                    let _=KillTimer(Some(self.hwnd),3);
                    if let Some((kind,msg,timeout))=self.toast_queue.pop_front(){let from0=self.toast_from_stage0;self.toast=None;self.show_toast(kind,msg,timeout);self.toast_from_stage0=from0;return LRESULT(0);}
                    self.toast=None;
                    if self.toast_from_stage0 && self.hover_stage == 1 {
                        self.toast_from_stage0 = false;
                        self.set_hover_stage(0);
                    } else {
                        self.toast_from_stage0 = false;
                        self.morph=Some(ui::Morph::new(from,self.target_geometry()));
                        self.start_morph_timer();
                        self.layout();
                        self.invalidate();
                    }
                },
                4=>{
                    let _=KillTimer(Some(self.hwnd),4);
                    if self.hover_stage==0 && self.pointer.is_some() && Instant::now()>self.hover_blocked_until {
                        self.set_hover_stage(1);
                    }
                },
                5=>{if !self.suspended{self.sync_foreground();}},
                6=>{
                    let _=KillTimer(Some(self.hwnd),6);

                    if self.hover_stage<2{
                        let from=self.geometry();
                        self.morph=Some(ui::Morph::new(from,self.target_geometry()));
                        self.start_morph_timer();
                    }
                    self.layout();
                    self.invalidate();
                },
                7=>{let _=KillTimer(Some(self.hwnd),7);self.power_state(true);},
                8=>{let _=KillTimer(Some(self.hwnd),8);self.power_state(false);},
                10=>{if self.tab_fade.as_ref().is_some_and(|(_,t,_)|t.elapsed()>=ui::TAB_FADE_DURATION){self.tab_fade=None;let _=KillTimer(Some(self.hwnd),10);}self.invalidate();},
                11=>{self.startup_alpha=(self.startup_alpha+0.05).min(1.);if self.startup_alpha>=1.{let _=KillTimer(Some(self.hwnd),11);}self.invalidate();},
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
            WM_KEYDOWN=>{if self.handle_settings_key(wparam.0 as u16){return LRESULT(0)}if self.reminder_menu.is_some()&&self.handle_reminder_menu_key(wparam.0 as u16){return LRESULT(0)}match wparam.0 as u16{
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
                0x1B if self.reminder_menu.is_some()=>{self.reminder_menu=None;self.invalidate();return LRESULT(0)},
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
            WM_SETFOCUS|WM_KILLFOCUS=>{self.focused=None;self.invalidate();return LRESULT(0)},
            WM_HOTKEY=>{if !self.fullscreen{self.set_expanded(!self.expanded);}return LRESULT(0)},
            WM_RBUTTONUP|TRAY=>{if msg==WM_RBUTTONUP||lparam.0 as u32==WM_RBUTTONUP{self.menu();}else if lparam.0 as u32==WM_LBUTTONUP{self.set_expanded(!self.expanded);}return LRESULT(0)},
            WAKE=>{self.events();return LRESULT(0)},
            WM_DPICHANGED|WM_DISPLAYCHANGE=>{self.layout();self.make_inputs();self.invalidate();return LRESULT(0)},
            WM_CLOSE=>{self.set_expanded(false);return LRESULT(0)},
            WM_DESTROY=>{PostQuitMessage(0);return LRESULT(0)},
            WM_CTLCOLOREDIT=>{let dc=HDC(wparam.0 as *mut _);SetTextColor(dc,COLORREF(0x00f5f1e9));SetBkColor(dc,COLORREF(0x002d2319));SetDCBrushColor(dc,COLORREF(0x002d2319));return LRESULT(GetStockObject(DC_BRUSH).0 as isize)},
            WM_COMMAND=>{match wparam.0&0xffff{900=>self.set_expanded(!self.expanded),901=>self.action(Action::Settings),902=>self.action(Action::Bottom(false)),903=>self.action(Action::Bottom(true)),999=>{let _=DestroyWindow(self.hwnd);},_=>{}}return LRESULT(0)},
            _=>{}
        }DefWindowProcW(self.hwnd,msg,wparam,lparam)
    }}
    fn tick(&mut self){
        if self.suspended{return}
        self.tick_reminder();
        let clip_seq = crate::clipboard::get_sequence_number();
        if clip_seq != self.last_clipboard_seq {
            self.sync_clipboard();
        }
        if let Some(was_break) = self.focus.complete() {
            if was_break {
                self.focus.is_break = false;
                self.notice("☕ Istirahat selesai! Siap untuk sesi fokus berikutnya.");
            } else {
                self.focus.is_break = true;
                if self.settings.auto_break{self.focus.start(self.settings.break_minutes,true);}
                if self.settings.stretch_after_focus{
                    self.last_move=Instant::now();
                    self.show_toast(ToastKind::Focus,"🎯 Sesi fokus selesai! Waktunya istirahat & peregangan.",8000);
                }else{
                    self.show_toast(ToastKind::Focus,"🎯 Sesi fokus selesai! Waktunya istirahat.",8000);
                }
            }
        }
        if self.settings.enabled("Tugas"){let now=Local::now();let mut reminders=vec![];for task in &mut self.state.todos{if !task.done&&!task.reminded&&task.due.is_some_and(|due|due<=now){task.reminded=true;reminders.push(task.text.clone());}}for text in reminders{self.notice(format!("Pengingat: {text}"));}}
        if self.settings.enabled("Reminder") {
            if self.settings.water_reminder&&self.state.water.iter().filter(|d|d.date==Local::now().date_naive()).map(|d|d.ml).sum::<u32>()<self.settings.water_target&&self.last_water.elapsed().as_secs()>=self.settings.water_interval as u64*60{
                self.last_water=Instant::now();
                self.show_water_toast();
            }
            if self.settings.move_reminder&&self.last_move.elapsed().as_secs()>=self.settings.move_interval as u64*60{
                self.last_move=Instant::now();
                self.notice("🧘 Waktunya peregangan otot! Berdiri sejenak & regangkan tubuh.");
            }
            if self.settings.enabled("Sehat")&&self.settings.eye_reminder&&self.last_eye.elapsed().as_secs()>=self.settings.eye_interval as u64*60{
                self.last_eye=Instant::now();
                self.notice("👀 Istirahat mata 20-20-20: pandang jauh 20 detik.");
            }
        }
        if self.breathing.is_some_and(|start|start.elapsed().as_secs()>=60){self.breathing=None;self.notice("Latihan napas selesai.");}
        let schedule=Local::now().format("%Y-%m-%d %H:%M").to_string();if self.settings.enabled("Reminder")&&self.settings.focus_enabled&&self.last_schedule!=schedule{let time=Local::now().format("%H:%M").to_string();if self.settings.focus_schedule.contains(&time){self.focus.start(self.settings.focus_minutes,false);self.notice("Jadwal fokus dimulai.");}self.last_schedule=schedule;}
        
        self.sync_active_lyric();
        self.clock_timer();self.invalidate();
    }
    fn events(&mut self){let events=if let Ok(mut q)=self.queue.lock(){std::mem::take(&mut*q)}else{vec![]};let mut repaint=false;for event in events{let event=match event{Event::Completed(module,id,result)=>{if self.jobs.get(module).map(|j|j.0)!=Some(id){continue}self.jobs.remove(module);self.busy=self.jobs.keys().any(|m|matches!(*m,"AI"|"Cuaca"|"Kalender"));*result},other=>other};
        
        repaint=true;match event{
        Event::Completed(..)=>{},
        
        Event::Media(MediaEvent::Changed(track))=>{if !self.settings.enabled("Musik"){continue}
            let key=serde_json::to_string(&(&track.title,&track.artist,&track.album,track.duration_seconds.round()as u64)).unwrap_or_default();let changed=key!=self.lyrics_key;
            let cover_key=crate::artwork::fingerprint(&track.artwork);
            if cover_key!=self.cover_key||(self.cover.is_none()&&!track.artwork.is_empty()&&!self.jobs.contains_key("Artwork")&&changed){
                self.cancel_job("Artwork");self.cover_key=cover_key;self.cover=None;
                if cover_key!=0{let bytes=track.artwork.clone();self.job("Artwork",move||Event::Artwork(cover_key,crate::artwork::decode(&bytes)));}
            }
            if self.preview&&self.track.playing!=track.playing{eprintln!("media playing={} can_seek={}",track.playing,track.can_seek);}
            self.track=track;if changed{self.cancel_job("Musik");self.lyrics_key=key.clone();self.lyrics.clear();self.plain_lyrics.clear();self.lyric_offset=0;if !self.track.title.is_empty(){let api=self.online.clone();let track=self.track.clone();self.job("Musik",move||Event::Lyrics(key,api.lyrics(&track.title,&track.artist,&track.album,track.duration_seconds)));}}
            if self.hover_stage==1 && !self.lyric_mode {
                let target=self.target_geometry();
                if (self.width-target.0).abs()>2.0||(self.height-target.1).abs()>2.0{
                    let from=self.geometry();
                    self.morph=Some(ui::Morph::new(from,target));
                    self.start_morph_timer();
                }
            }
            self.sync_visualizer();self.layout();self.clock_timer();},
        Event::Prayers(city,date,result)=>{if city==self.settings.prayer_city&&date==reminder::jakarta_today(){match result{Ok(s)=>{self.prayer_schedule=Some(s);self.prayer_error=None;},Err(e)=>self.prayer_error=Some(e)}}},
        Event::Artwork(key,result)=>{if key==self.cover_key&&self.settings.enabled("Musik"){self.cover=result.ok();if self.preview{eprintln!("artwork ready={}",self.cover.is_some());}}},
        Event::Levels(levels)=>{if self.visualizer.is_some(){self.levels=levels;self.sync_active_lyric();}},
        Event::Lyrics(key,result)=>{if key==self.lyrics_key&&self.settings.enabled("Musik"){match result{Ok(value)=>{self.lyrics=crate::lyrics::parse_lrc_with_options(&value.synced_lyrics,self.settings.romanize_lyrics);self.plain_lyrics=if value.instrumental{"Instrumental · nikmati musiknya.".into()}else if value.plain_lyrics.is_empty()&&self.lyrics.is_empty(){"Lirik belum tersedia untuk lagu ini.".into()}else if self.settings.romanize_lyrics{crate::lyrics::romanize_text(&value.plain_lyrics)}else{value.plain_lyrics};self.layout();},Err(_)=>{self.plain_lyrics="Lirik belum tersedia untuk lagu ini.".into();}}}},
        Event::Weather(result)=>{if self.settings.enabled("Cuaca"){match result{Ok(value)=>{self.weather=Some(value);self.weather_updated=Some(Instant::now());},Err(e)=>self.notice(e)}}},
        Event::Agenda(result)=>{if self.settings.enabled("Kalender"){match result{Ok(value)=>{self.agenda=value;self.agenda_updated=Some(Instant::now());},Err(e)=>self.notice(e)}}},
        Event::Ai(result)=>{if self.settings.enabled("AI"){match result{Ok(text)=>self.ai_messages.push(crate::online::ChatMessage{role:"assistant".into(),content:text}),Err(e)=>self.notice(e)}}},Event::Notice(text)=>self.notice(text),
    }}if repaint{self.invalidate();}}
    fn button_h(&mut self,p:&mut Painter,label:&str,x:f32,y:f32,width:f32,height:f32,action:Action,primary:bool){
        let hovered=self.pointer.is_some_and(|(px,py)|px>=x&&px<=x+width&&py>=y&&py<=y+height);
        let is_pressed=self.pressed.as_ref()==Some(&action);
        let is_focused=self.focused.as_ref()==Some(&action);
        let surface=if is_pressed{if primary{0x254a40}else{render::PRESSED}}else if primary{if hovered{0x2f7364}else{0x285f53}}else if hovered{render::HOVER}else{render::DIM};
        let (bx,by,bw,bh)=if is_pressed{
            let dw=width*0.02;let dh=height*0.02;
            (x+dw/2.,y+dh/2.,width*0.98,height*0.98)
        }else{(x,y,width,height)};
        let radius=ui::RADIUS_CONTROL.min(height*0.35);
        unsafe{
            p.fill(bx,by,bw,bh,radius,surface);
            p.text_centered(label,bx,by,bw,bh,if height<26.{10.5}else{11.5},if primary{0xb1ffe9}else{render::TEXT},1.0,false);
        }
        if is_focused{unsafe{p.stroke(x-2.,y-2.,width+4.,height+4.,radius+2.,render::ACCENT,1.5);}}
        if self.hit_enabled{self.hits.push(Hit{rect:render::rect(x,y,width,height),action});}
    }
    fn transport_btn(&mut self,p:&mut Painter,label:&str,x:f32,y:f32,w:f32,h:f32,action:Action,active:bool,enabled:bool){
        let hovered=enabled&&self.pointer.is_some_and(|(px,py)|px>=x&&px<=x+w&&py>=y&&py<=y+h);
        let is_pressed=enabled&&self.pressed.as_ref()==Some(&action);
        let color=if !enabled{0x555a5e}else if active{0xffffff}else if is_pressed{0x888888}else if hovered{0xffffff}else{0xdddddd};
        unsafe{
            if hovered||active{
                p.fill_alpha(x,y,w,h,6.,0xffffff,if active{0.22}else{0.10});
            }
            if self.focused.as_ref()==Some(&action){
                p.stroke(x-1.,y-1.,w+2.,h+2.,7.,0xd5d5dc,1.);
            }
            let cx=x+w*0.5;let cy=y+h*0.5;
            match label{
                "LYRICS"|"❝" => p.icon_lyrics_bubble(cx,cy,color,active),
                "⏮" => p.icon_prev(cx,cy,14.,color),
                "⏭" => p.icon_next(cx,cy,14.,color),
                "▶" => p.icon_play(cx,cy,14.,color),
                "⏸" => p.icon_pause(cx,cy,14.,color),
                _ => p.text(label,x,y+2.,w,h-4.,15.,color,true),
            }
        }
        if enabled&&self.hit_enabled{self.hits.push(Hit{rect:render::rect(x-2.,y-2.,w+4.,h+4.),action});}
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
            self.painter=unsafe{Painter::new(self.hwnd,pw,ph,96.*self.scale)}.ok();
        }
        if let Some(mut p)=self.painter.take(){
            self.painting=true;
            let paint_started=std::time::Instant::now();
            let ok=unsafe{
                let mut ok=p.begin().is_ok();
                if ok{
                    self.hits.clear();
                    let(width,height,radius)=self.geometry();
                    self.width=width;
                    self.height=height;
                    let(ox,oy)=self.capsule_origin();
                    let body_bg=if self.settings_open||self.tab=="Reminder"{0x060608}else{render::BG};
                    // Drop shadow blooms when hovered in Stage 1 or expanded in Stage 2
                    if self.hover_stage >= 1 && (self.pointer.is_some() || self.expanded) {
                        let opacity=(self.settings.opacity as f32/255.)*self.startup_alpha;
                        let shadow_accent=if !self.settings_open&&self.settings.glow&&(self.tab=="Musik"||self.lyric_mode)&&self.track.playing{
                            self.cover.as_ref().map(|c|c.accent).unwrap_or(0x000000)
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
                    p.translate(ox,oy);
                    let is_attached_top=!self.settings.bottom&&self.settings.edge_gap==0;
                    if is_attached_top{
                        let ear=if self.hover_stage==0&&!self.expanded{
                            4.
                        }else if height>=24.{
                            12.
                        }else{
                            (height*0.5).max(4.)
                        };
                        // Cover the physical screen-edge row as well as the antialiased contour.
                        // This avoids a transparent hairline at the clipped HWND/DComp boundary.
                        p.fill(-ear,-1./self.scale,width+2.*ear,2./self.scale,0.,body_bg);
                        if !self.settings_open&&self.settings.glow&&self.hover_stage>=2 && self.tab=="Musik"{
                            let accent=self.cover.as_ref().map(|c|c.accent).unwrap_or(0x1a2e3b);
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
                        if !self.settings_open&&self.settings.glow&&self.expanded&&self.tab=="Musik"{let accent=self.cover.as_ref().map(|c|c.accent).unwrap_or(0x1a2e3b);p.fill_alpha(0.,0.,width,height,radius,accent,0.12);}
                    }
                    self.draw(&mut p);
                    p.reset_transform();
                    p.pop_alpha();
                    ok=p.end().is_ok();
                }
                ok
            };
            self.painting=false;
            if self.preview{let elapsed=paint_started.elapsed();if elapsed.as_millis()>10{eprintln!("[paint] {:.1}ms",elapsed.as_secs_f64()*1000.);}}
            if !ok{self.render_ready=false;self.painter=None;if self.preview{eprintln!("renderer dropped (device lost?) — rebuilding next frame");}}else{self.render_ready=true;self.painter=Some(p);}
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
                } else if morph.to().1 < 40.0 && morph.from().1 < 40.0 {
                    let c_alpha = if morph.to().1 > morph.from().1 {
                        ((progress - 0.15) / 0.85).clamp(0.0, 1.0)
                    } else {
                        (1.0 - progress / 0.45).clamp(0.0, 1.0)
                    };
                    (c_alpha, 0.0)
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
        if collapsed_alpha > 0.002 { self.draw_collapsed(p, collapsed_alpha); }
        if expanded_alpha > 0.002 { self.draw_expanded(p, expanded_alpha); }
    }
    fn draw_collapsed(&mut self,p:&mut Painter,alpha:f32){unsafe{
        p.push_alpha(alpha);
        if let Some(toast)=&self.toast{
            p.clip(0.,0.,self.width,self.height);
            let cy=self.height*0.5;
            if toast.kind == ToastKind::Water {
                // Vibrant blue water icon: 0x38bdf8
                p.text_centered("💧",10.,0.,18.,self.height,13.,0x38bdf8,1.0,false);
                if toast.confirmed {
                    let text_w = self.width - 56.;
                    p.text_centered("Hebat! Hidrasi tercatat",28.,0.,text_w,self.height,11.5,0x6ee7b7,1.0,true);
                    p.text_centered("🎉",self.width-28.,0.,20.,self.height,13.,0xffffff,1.0,false);
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
                    p.text_centered(label,text_x,0.,text_w,self.height,11.5,0xffffff,1.0,true);

                    // Checklist confirmation button: [ ✓ ]
                    let hovered = self.pointer.is_some_and(|(px,py)| px>=btn_x && px<=btn_x+btn_w && py>=btn_y && py<=btn_y+btn_h);
                    let btn_bg = if hovered { 0x0284c7 } else { 0x0369a1 };
                    p.fill_alpha(btn_x, btn_y, btn_w, btn_h, btn_h * 0.5, btn_bg, 0.85);
                    p.stroke(btn_x, btn_y, btn_w, btn_h, btn_h * 0.5, 0x38bdf8, 1.2);
                    p.text_centered("✓", btn_x, btn_y, btn_w, btn_h, 12., 0xffffff, 1.0, true);

                    if self.hit_enabled {
                        self.hits.push(Hit {
                            rect: render::rect(btn_x - 4., btn_y - 2., btn_w + 8., btn_h + 4.),
                            action: Action::ConfirmWater,
                        });
                    }
                }
            } else {
                p.text_centered(&toast.message,0.,0.,self.width,self.height,11.5,0xffffff,1.0,true);
            }
            p.unclip();
        }else if self.hover_stage==1{
            let cy=self.height*0.5;
            let btn_up_w=20.;
            let btn_up_h=20.;
            let btn_up_x=self.width-24.;
            let btn_up_y=(self.height-btn_up_h)*0.5;

            if self.lyric_mode{
                p.clip(0.,0.,self.width,self.height);

                // 1. Icon lirik speech bubble di kiri
                p.icon_lyrics_bubble(18.,cy,0xffa46b,true);

                // 2. Synchronized live lyric line di tengah
                let curr_text=self.active_lyric().unwrap_or_else(||{
                    if self.track.title.is_empty(){
                        "Belum ada musik".into()
                    }else if self.track.artist.is_empty(){
                        self.track.title.clone()
                    }else{
                        format!("{} · {}",self.track.title,self.track.artist)
                    }
                });

                let text_x=34.;
                let text_w=(btn_up_x-6.-text_x).max(20.);

                if let Some(anim)=&self.lyric_anim{
                    let elapsed=anim.started.elapsed().as_secs_f32();
                    let progress=(elapsed/0.28).clamp(0.0,1.0);
                    let ease=ui::ease_out_cubic(progress);

                    let out_y=-(ease*12.);
                    let out_alpha=(1.0-ease).clamp(0.0,1.0);
                    p.text_centered(&anim.prev_text,text_x,out_y,text_w,self.height,11.5,render::TEXT,out_alpha,true);

                    let in_y=(1.0-ease)*12.;
                    let in_alpha=ease.clamp(0.0,1.0);
                    p.text_centered(&curr_text,text_x,in_y,text_w,self.height,11.5,render::TEXT,in_alpha,true);
                }else{
                    p.text_centered(&curr_text,text_x,0.,text_w,self.height,11.5,render::TEXT,1.0,true);
                }

                // 3. Tombol chevron up untuk mengecilkan ke Stage 0
                self.button_chevron_up(p,btn_up_x,btn_up_y,btn_up_w,btn_up_h,Action::ToStage0,0x88888e);

                p.unclip();
            }else{
                p.clip(0.,0.,self.width,self.height);

                let (_symbol, _sym_color, info_text) = match self.tab.as_str() {
                    
                    "Kalender" => {
                        let month_str = match self.month { 1=>"Jan", 2=>"Feb", 3=>"Mar", 4=>"Apr", 5=>"Mei", 6=>"Jun", 7=>"Jul", 8=>"Ags", 9=>"Sep", 10=>"Okt", 11=>"Nov", _=>"Des" };
                        let cal = format!("{} {}", self.day, month_str);
                        ("📅", 0xffffff, cal)
                    }
                    "Reminder" => {
                        let rem = if self.focus.remaining().is_zero() {
                            self.settings.focus_minutes as f64 * 60.
                        } else {
                            self.focus.remaining().as_secs_f64()
                        };
                        let foc = time_label(rem);
                        match self.reminder_index{
                            1=>("💧",0x65cfe3,format!("{} ml",self.state.water.iter().filter(|d|d.date==Local::now().date_naive()).map(|d|d.ml).sum::<u32>())),
                            2=>("◷",0xb8a1ee,self.stretch_deadline.map(|d|format!("{}s",d.saturating_duration_since(Instant::now()).as_secs())).unwrap_or("Peregangan".into())),
                            3=>("◷",0xdac68d,"Sholat".into()),
                            _=>("🎯",0xffa46b,foc)
                        }
                    }
                    
                    "Musik" => {
                        let text = if self.track.title.is_empty() {
                            "Musik".into()
                        } else if self.track.artist.is_empty() {
                            self.track.title.clone()
                        } else {
                            format!("{} · {}", self.track.title, self.track.artist)
                        };
                        ("♫", 0xffffff, text)
                    }
                    "Clipboard" => {
                        let count = self.state.clipboard_history.len();
                        let text = if let Some(first) = self.state.clipboard_history.first() {
                            let snip: String = first.text.lines().next().unwrap_or("").trim().chars().take(14).collect();
                            if snip.is_empty() {
                                format!("{} klip", count)
                            } else {
                                format!("{snip}…")
                            }
                        } else {
                            "0 klip".to_string()
                        };
                        ("📋", 0x38bdf8, text)
                    }
                    _ => {
                        ("•", 0xffffff, self.tab.clone())
                    }
                };

                // 1. Symbol / icon di kiri (x = 10, w = 16)
                let icon=if self.tab=="Reminder"{["timer","droplet","person","moon"][self.reminder_index]}else{ui::module_icon(&self.tab)};
                p.outline_icon(icon,10.,(self.height-14.)/2.,14.,0xd5d5dc);

                // 2. Tombol chevron up di sisi paling kanan
                self.button_chevron_up(p, btn_up_x, btn_up_y, btn_up_w, btn_up_h, Action::ToStage0, 0x88888e);

                // 3. Status tengah seragam: <info> | <jam> dengan pemisah "|" light & subtle
                let time_str = Local::now().format("%H:%M").to_string();
                self.draw_stage1_status(p, &info_text, &time_str, btn_up_x);

                p.unclip();
            }
        }
        p.pop_alpha();
    }}
    fn tab_pill(&mut self,p:&mut Painter,icon:&str,label:&str,x:f32,y:f32,w:f32,h:f32,action:Action,selected:bool){
        let hovered=self.pointer.is_some_and(|(px,py)|px>=x&&px<=x+w&&py>=y&&py<=y+h);
        unsafe{
            if selected||hovered{p.fill(x,y,w,h,h*0.5,if selected{0x252528}else{0x1c1d22});}
            let rgb=if selected||hovered{0xefeff3}else{0x71717a};
            if selected&&w>40.&&!label.is_empty(){
                let text_w=p.measure_text(label,11.,false);let start=x+(w-15.-6.-text_w)/2.;
                p.outline_icon(icon,start,y+(h-15.)/2.,15.,rgb);
                p.text_centered(label,start+21.,y,text_w,h,11.,rgb,1.,false);
            }else{p.outline_icon(icon,x+(w-15.)/2.,y+(h-15.)/2.,15.,rgb);}
            if self.focused.as_ref()==Some(&action){p.stroke(x-1.,y-1.,w+2.,h+2.,h*0.5,0xd5d5dc,1.);}
        }
        if self.hit_enabled{self.hits.push(Hit{rect:render::rect(x,y,w,h),action});}
    }
    fn navigation_control(&mut self,p:&mut Painter,icon:&str,x:f32,y:f32,action:Action,active:bool){unsafe{
        let hovered=self.pointer.is_some_and(|(a,b)|a>=x&&a<=x+24.&&b>=y&&b<=y+26.);
        if hovered||active{p.fill(x,y,24.,26.,12.,0x252528);}
        p.outline_icon(icon,x+5.,y+6.,14.,if active||hovered{0xefeff3}else{0x77777c});
        if self.focused.as_ref()==Some(&action){p.stroke(x,y,24.,26.,12.,0xd5d5dc,1.);}
        if self.hit_enabled{self.hits.push(Hit{rect:render::rect(x,y,24.,26.),action});}
    }}
    fn button_icon(&mut self,p:&mut Painter,icon:&str,x:f32,y:f32,w:f32,h:f32,action:Action){
        self.button_icon_colored(p,icon,x,y,w,h,action,0x8a9291);
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
    fn draw_stage1_status(&mut self,p:&mut Painter,info:&str,time_str:&str,btn_up_x:f32){unsafe{
        let left_x=28.0;
        let right_x=btn_up_x-4.0;
        let available_w=(right_x-left_x).max(10.0);

        let info_w=p.measure_text(info,11.5,true);
        let sep_w=6.0;
        let time_w=p.measure_text(time_str,11.5,true);
        let gap=7.0;

        let cluster_w=info_w+gap+sep_w+gap+time_w;

        if cluster_w<=available_w{
            let start_x=left_x+(available_w-cluster_w)*0.5;

            // Info teks (putih semi-bold)
            p.text_centered(info,start_x,0.,info_w,self.height,11.5,0xffffff,1.0,true);

            // Pemisah "|": light/normal font (bold: false), warna soft slate (tidak pure white)
            let sep_x=start_x+info_w+gap;
            p.text_centered("|",sep_x,0.,sep_w,self.height,11.0,0x78808c,0.85,false);

            // Jam (putih semi-bold)
            let time_x=sep_x+sep_w+gap;
            p.text_centered(time_str,time_x,0.,time_w,self.height,11.5,0xffffff,1.0,true);
        }else{
            let time_x=right_x-time_w;
            p.text_centered(time_str,time_x,0.,time_w,self.height,11.5,0xffffff,1.0,true);

            let sep_x=time_x-gap-sep_w;
            p.text_centered("|",sep_x,0.,sep_w,self.height,11.0,0x78808c,0.85,false);

            let info_max_w=(sep_x-gap-left_x).max(10.0);
            p.text_ellipsis_vcenter(info,left_x,0.,info_max_w,self.height,11.5,0xffffff,true);
        }
    }}
    fn draw_top_tabs(&mut self,p:&mut Painter){unsafe{
        let is_rearranging = self.tab_drag.as_ref().map(|d| d.is_dragging).unwrap_or(false);
        let tabs = self.settings.active_tab_order();
        let y = 12.0;
        let h = 26.0;

        struct TabSlot {
            tab: String,
            icon: &'static str,
            label: String,
            x: f32,
            w: f32,
            selected: bool,
        }

        let mut slots: Vec<TabSlot> = Vec::new();
        let mut cur_x = 18.0;
        for tab in &tabs {
            let icon=ui::module_icon(tab);let label=tab.clone();
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

                dragged_pill = Some((slots[cur_idx].tab.clone(), slots[cur_idx].icon, dragged_x, pill_w));
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
            self.tab_pill(p, slot.icon, &slot.label, slot.x, y, slot.w, h, Action::Tab(slot.tab.clone()), slot.selected);
        }

        if let Some((tab_name, icon, dx, dw)) = dragged_pill {
            let radius = h * 0.5;
            p.fill_alpha(dx - 2.0, y - 2.0, dw + 4.0, h + 4.0, radius + 2.0, 0xb8b8c2, 0.35);
            p.fill_alpha(dx, y, dw, h, radius, 0x18181b, 0.95);
            p.stroke(dx, y, dw, h, radius, 0xb8b8c2, 1.8);
            p.outline_icon(icon,dx+(dw-15.)/2.,y+(h-15.)/2.,15.,0xefeff3);

            if self.hit_enabled {
                self.hits.push(Hit { rect: render::rect(dx, y, dw, h), action: Action::Tab(tab_name) });
            }
        }

        // Time display immediately to the left of the visualizer
        let time_str=Local::now().format("%H:%M").to_string();
        let clock_x=self.width-160.;
        let clock_w=38.;
        p.text_centered(&time_str,clock_x,y,clock_w,h,11.,0xffffff,0.85,true);

        if self.track.playing&&self.settings.visualizer{
            let levels=if self.levels.len()>=4{&self.levels[0..4]}else{&[0.4,0.8,1.0,0.6]};
            p.bars(levels,self.width-118.,16.,14.,12.,0xffffff);
        }else if self.settings.visualizer{
            p.fill(self.width-118.,21.,14.,2.,1.,0x454e4d);
        }

        // Window buttons: Pin, Collapse, Settings
        self.navigation_control(p,"pin",self.width-92.,y,Action::TogglePin,self.pinned);
        self.navigation_control(p,"up",self.width-64.,y,Action::ToStage1,false);
        self.navigation_control(p,"settings",self.width-36.,y,Action::Settings,false);
    }}
    fn draw_expanded(&mut self,p:&mut Painter,alpha:f32){unsafe{
        p.push_alpha(alpha);
        p.clip(0.,0.,self.width,self.height);
        if self.settings_open{
            self.draw_settings(p);
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
        if let Some(toast)=self.toast.clone(){
            let y=self.height-28.;p.fill(16.,y,self.width-32.,23.,7.,0x1c2927);
            let water=toast.kind==ToastKind::Water&&!toast.confirmed;
            p.text_ellipsis(&toast.message,25.,y+4.,self.width-if water{116.}else{50.},17.,10.5,render::TEXT,false);
            if water{self.button_h(p,"+ Minum",self.width-88.,y,64.,23.,Action::ConfirmWater,true);}
        }
        p.unclip();
        p.pop_alpha();
    }}
    fn active_lyric(&self)->Option<String>{let position=self.media.as_ref().map(|m|m.position_seconds()).unwrap_or(self.track.position_seconds);self.lyrics.iter().rev().find(|line|line.at<=position).map(|line|line.text.clone())}
    fn tray(&self,add:bool){unsafe{use windows::Win32::UI::Shell::*;let mut data=NOTIFYICONDATAW{cbSize:std::mem::size_of::<NOTIFYICONDATAW>()as u32,hWnd:self.hwnd,uID:1,uFlags:NIF_MESSAGE|NIF_ICON|NIF_TIP,uCallbackMessage:TRAY,hIcon:LoadIconW(None,IDI_APPLICATION).unwrap_or_default(),..Default::default()};let text:Vec<u16>="mynotch 0.0.1 alpha".encode_utf16().collect();data.szTip[..text.len()].copy_from_slice(&text);let _=Shell_NotifyIconW(if add{NIM_ADD}else{NIM_DELETE},&data);}}
    fn menu(&self){unsafe{if let Ok(menu)=CreatePopupMenu(){let _=AppendMenuW(menu,MF_STRING,900,w!("Buka / tutup notch"));let _=AppendMenuW(menu,MF_STRING,901,w!("Pengaturan"));let _=AppendMenuW(menu,MF_STRING,902,w!("Posisi atas"));let _=AppendMenuW(menu,MF_STRING,903,w!("Posisi bawah"));let _=AppendMenuW(menu,MF_SEPARATOR,0,PCWSTR::null());let _=AppendMenuW(menu,MF_STRING,999,w!("Keluar"));let mut cursor=POINT::default();let _=GetCursorPos(&mut cursor);let _=SetForegroundWindow(self.hwnd);let _=TrackPopupMenu(menu,TPM_RIGHTBUTTON,cursor.x,cursor.y,None,self.hwnd,None);let _=DestroyMenu(menu);}}}
}
impl Drop for Shell{fn drop(&mut self){crate::clipboard::unlisten_clipboard(self.hwnd);self.cancel_jobs();self.tray(false);if let Some(mut m)=self.media.take(){m.stop();}if let Some(mut v)=self.visualizer.take(){v.stop();}unsafe{for id in 1..=12{let _=KillTimer(Some(self.hwnd),id);}if self.morph_timer!=0{let _=timeKillEvent(self.morph_timer);}let _=UnregisterHotKey(Some(self.hwnd),1);let _=DestroyWindow(self.inputs_hwnd);}self.save();}}
fn time_label(seconds:f64)->String{let secs=seconds.max(0.)as u64;format!("{:02}:{:02}",secs/60,secs%60)}

fn covers_monitor(window:RECT,monitor:RECT)->bool{window.left<=monitor.left&&window.top<=monitor.top&&window.right>=monitor.right&&window.bottom>=monitor.bottom}

pub(super) fn classify_notice_kind(msg:&str)->ToastKind{
    if msg.contains("💧")||msg.contains("minum"){
        ToastKind::Water
    }else if msg.contains("🎯")||msg.contains("fokus"){
        ToastKind::Focus
    }else if msg.contains("🧘")||msg.contains("regang"){
        ToastKind::Stretch
    }else{
        ToastKind::Generic
    }
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

pub(super) fn initial_hover_stage(preview:bool,minimal:bool)->u8{
    if preview{2}else if minimal{0}else{1}
}

pub(super) fn toast_target_reminder(kind:ToastKind)->Option<usize>{
    match kind{
        ToastKind::Focus=>Some(0),
        ToastKind::Water=>Some(1),
        ToastKind::Stretch=>Some(2),
        ToastKind::Prayer=>Some(3),
        ToastKind::Generic=>None,
    }
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
    #[test]fn quit_is_posted_even_during_reentrant_destroy(){APP.with(|cell|{let _borrow=cell.borrow_mut();unsafe{wndproc(HWND::default(),WM_DESTROY,WPARAM(0),LPARAM(0));let mut msg=MSG::default();assert!(PeekMessageW(&mut msg,None,WM_QUIT,WM_QUIT,PM_REMOVE).as_bool());assert_eq!(msg.message,WM_QUIT);}});}
    #[test]fn fullscreen_requires_whole_monitor_not_work_area(){let monitor=RECT{left:0,top:0,right:1920,bottom:1080};assert!(covers_monitor(monitor,monitor));assert!(!covers_monitor(RECT{bottom:1040,..monitor},monitor));assert!(!covers_monitor(RECT{left:1,..monitor},monitor));assert!(covers_monitor(RECT{left:-8,top:-8,right:1928,bottom:1088},monitor));}
    #[test]fn notice_classification_prioritizes_focus_completion_over_stretch(){let msg="🎯 Sesi fokus selesai! Waktunya istirahat & peregangan.";let kind=classify_notice_kind(msg);assert_eq!(kind,ToastKind::Focus);}
    #[test]fn lyric_display_index_respects_manual_scroll_offset(){let lines=vec![crate::lyrics::LyricLine{at:1.0,text:"A".into()},crate::lyrics::LyricLine{at:5.0,text:"B".into()},crate::lyrics::LyricLine{at:10.0,text:"C".into()}];assert_eq!(lyric_display_index(&lines,6.0,0),1);assert_eq!(lyric_display_index(&lines,6.0,3),2);}
    #[test]fn minimal_mode_collapses_directly_to_stage0(){assert_eq!(collapse_target_stage(true),0);assert_eq!(collapse_target_stage(false),1);assert_eq!(initial_hover_stage(false,true),0);assert_eq!(initial_hover_stage(false,false),1);}
    #[test]fn toast_click_maps_all_reminder_kinds_including_prayer(){assert_eq!(toast_target_reminder(ToastKind::Focus),Some(0));assert_eq!(toast_target_reminder(ToastKind::Water),Some(1));assert_eq!(toast_target_reminder(ToastKind::Stretch),Some(2));assert_eq!(toast_target_reminder(ToastKind::Prayer),Some(3));assert_eq!(toast_target_reminder(ToastKind::Generic),None);}
    #[test]fn prayer_summary_formats_zero_lead_as_tepat_waktu(){assert_eq!(settings::prayer_lead_summary("Jakarta",0),"Jakarta · Tepat waktu");assert_eq!(settings::prayer_lead_summary("Bandung",5),"Bandung · 5 menit sebelumnya");}
}
