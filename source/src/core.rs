use chrono::{DateTime, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const MODULES: &[&str] = &["Musik", "Kalender", "Reminder", "Clipboard"];
/// Mods that ship with a native page and therefore own a real tab slot. Community mods
/// (Lua/JSON) are opened from Settings → Mod and never enter the tab strip.

pub const BUILTIN_MOD_TABS: &[&str] = &["Catatan", "Cuaca"];
/// Background opacity floor. Below this the notch becomes effectively invisible while its
/// click region stays active, which looks like the app failed to start.
pub const MIN_OPACITY: u8 = 120;

/// Keeps a list viewport inside its scrollable range. `visible` is the number of rows that fit.
pub fn clamp_list_scroll(scroll: usize, total: usize, visible: usize) -> usize {
    let max = total.saturating_sub(visible.max(1));
    scroll.min(max)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub bottom: bool, pub edge_gap: i32, pub scale: f32, pub opacity: u8, pub minimal: bool,
    pub hide_fullscreen: bool, pub lyrics_below: bool, pub glow: bool, pub visualizer: bool, pub romanize_lyrics: bool,
    pub default_tab: String, pub modules: BTreeMap<String,bool>,
    pub tab_order: Vec<String>,
    #[serde(default)] pub installed_mods: Vec<String>,
    #[serde(default)] pub disabled_mods: Vec<String>,
    /// Community-mod tab names the user opted into the Stage 2 tab strip.
    #[serde(default)] pub mod_tabs: Vec<String>,
    /// Community-mod ids the user allowed to make outbound HTTPS requests.
    #[serde(default)] pub network_mods: Vec<String>,
    /// Hosts a community mod may reach even when they are IP literals or in a
    /// private/loopback/link-local range. Empty by default: the transport blocks those
    /// destinations unless listed here. Edited via settings.json for now (no settings UI).
    #[serde(default)] pub net_allowed_hosts: Vec<String>,
    #[serde(default)] pub permissions: BTreeMap<String, Vec<String>>,
    #[serde(default)] pub language: String,
    #[serde(default)] pub run_at_startup: bool,
    #[serde(default)]
    pub software_renderer: bool,
    pub focus_minutes: u32, pub break_minutes: u32,
    pub focus_enabled: bool, pub stretch_after_focus: bool, pub prayer_city:String, pub prayer_sound:bool, pub auto_break: bool, pub stretch_seconds: u32, pub prayer_enabled: bool, pub prayer_lead: u32, pub prayer_times: Vec<String>, pub prayer_date: String,
    pub water_target: u32, pub water_serving: u32, pub water_interval: u32, pub move_interval: u32, pub eye_interval: u32,
    pub water_reminder: bool, pub move_reminder: bool, pub eye_reminder: bool,
    pub latitude: f64, pub longitude: f64, pub weather_city: String, pub weather_configured: bool,
    pub calendar_feed: String, pub collective_leave: bool,
    pub ai_provider: String, pub ai_endpoint: String, pub ai_model: String, pub ai_key: String,
    pub message_preview: bool,
    pub focus_schedule: Vec<String>, pub hotkey_alt: bool, pub hotkey_ctrl: bool, pub hotkey_key: u32,

    #[serde(default = "default_true")]
    pub privacy_enabled: bool,
    #[serde(default)]
    pub privacy_show_fullscreen: bool,
    #[serde(default = "default_true")]
    pub privacy_show_app_names: bool,
    #[serde(default)]
    pub motion_reduced: bool,
    #[serde(default = "default_theme_material")]
    pub theme_material: String,
    /// "dark" (Gelap, default) or "light" (Terang). Shared by every material.
    #[serde(default = "default_theme_color")]
    pub theme_color: String,
}
fn default_true() -> bool {
    true
}
fn default_theme_color() -> String {
    "dark".to_string()
}
fn default_theme_material() -> String {
    "solid".to_string()
}
impl Default for Settings {
    fn default()->Self { Self {
        bottom:false,edge_gap:0,scale:1.,opacity:250,minimal:false,hide_fullscreen:true,lyrics_below:true,glow:true,visualizer:true,romanize_lyrics:true,
        default_tab:"Musik".into(), modules: MODULES.iter().map(|&m|(m.to_owned(),m!="Clipboard")).collect(),
        tab_order: MODULES.iter().map(|&m| m.to_string()).collect(),
        installed_mods: vec![],
        disabled_mods: vec![],
        mod_tabs: vec![],
        network_mods: vec![],
        net_allowed_hosts: vec![],
        permissions: BTreeMap::new(),
        language: "id".into(),
        run_at_startup: false,
        software_renderer: false,
        privacy_enabled: true,
        privacy_show_fullscreen: false,
        privacy_show_app_names: true,
        motion_reduced: false,
        theme_material: "solid".into(),
        theme_color: "dark".into(),
        focus_minutes:25,break_minutes:5,focus_enabled:true,stretch_after_focus:true,prayer_city:"Jakarta".into(),prayer_sound:false,auto_break:false,stretch_seconds:60,prayer_enabled:false,prayer_lead:5,prayer_times:vec![String::new();5],prayer_date:String::new(),water_target:2000,water_serving:250,water_interval:45,move_interval:45,eye_interval:25,
        water_reminder:true,move_reminder:true,eye_reminder:true,
        latitude:-6.2088,longitude:106.8456,weather_city:"Jakarta".into(),weather_configured:false,
        calendar_feed:String::new(),collective_leave:true,
        ai_provider:"OpenAI compatible".into(),ai_endpoint:"https://api.openai.com/v1".into(),ai_model:String::new(),ai_key:String::new(),
        message_preview:false,focus_schedule:vec![],hotkey_alt:true,hotkey_ctrl:false,hotkey_key:0x4e,

    }}
}
impl Settings {
    pub fn installed_mod_tabs(&self) -> Vec<String> {
        let mut tabs = Vec::new();
        for id in &self.installed_mods {
            if id == "sticky_notes" {
                tabs.push("Catatan".to_string());
            } else if id == "weather" {
                tabs.push("Cuaca".to_string());
            }

        }
        tabs
    }
    pub fn is_mod_installed(&self, id: &str) -> bool {
        self.installed_mods.iter().any(|m| m == id)
    }
    #[allow(dead_code)]
    pub fn is_mod_active(&self, id: &str) -> bool {
        self.is_mod_installed(id) && !self.disabled_mods.iter().any(|m| m == id)
    }
    pub fn is_known_module(&self, m: &str) -> bool {
        MODULES.contains(&m) || self.installed_mod_tabs().iter().any(|t| t == m) || self.mod_tabs.iter().any(|t| t == m)
    }
    pub fn active_tab_order(&self) -> Vec<String> {
        let mut order = self.tab_order.clone();
        for &m in MODULES {
            if !order.iter().any(|t| t == m) {
                order.push(m.to_string());
            }
        }
        for m in self.installed_mod_tabs() {
            if !order.iter().any(|t| t == &m) {
                order.push(m);
            }
        }
        for m in &self.mod_tabs {
            if !order.iter().any(|t| t == m) {
                order.push(m.clone());
            }
        }
        order.retain(|m| (MODULES.contains(&m.as_str()) || BUILTIN_MOD_TABS.contains(&m.as_str()) || self.mod_tabs.iter().any(|t| t == m)) && self.enabled(m));
        order
    }
    /// Opts a community mod's page into (or out of) the Stage 2 tab strip. The slot stays in
    /// `tab_order` when disabled, so re-enabling restores the position the user chose.
    pub fn set_mod_tab(&mut self, tab: &str, on: bool) {
        if tab.is_empty() || MODULES.contains(&tab) {
            return;
        }
        if BUILTIN_MOD_TABS.contains(&tab) {
            if !self.is_known_module(tab) { return; }
            self.set_module_enabled(tab,on);
            if on&&!self.tab_order.iter().any(|t|t==tab){self.tab_order.push(tab.into());}
            return;
        }
        if on {
            if !self.mod_tabs.iter().any(|t| t == tab) {
                self.mod_tabs.push(tab.to_string());
            }
            self.modules.insert(tab.to_string(), true);
            if !self.tab_order.iter().any(|t| t == tab) {
                self.tab_order.push(tab.to_string());
            }
        } else {
            // Keep the registration and the tab_order slot so the position survives restarts;
            // `modules[tab] = false` is what hides it from the Stage 2 strip.
            self.modules.insert(tab.to_string(), false);
            if self.default_tab == tab {
                self.default_tab = self.active_tab_order().first().cloned().unwrap_or_else(|| "Musik".to_string());
            }
        }
    }
    /// Explicit network permission for a community mod. Off by default; the mod engine refuses
    /// `system.http_get` unless its id is listed here.
    pub fn set_mod_network(&mut self, id: &str, on: bool) {
        if id.is_empty() { return }
        if on {
            if !self.network_mods.iter().any(|m| m == id) { self.network_mods.push(id.to_string()); }
        } else {
            self.network_mods.retain(|m| m != id);
        }
    }
    /// Tabs the user can order/enable on the Settings → Tab page. Community mods are not tabs,
    /// so they never appear here; installed built-in mods (`Catatan`, `Cuaca`) do.
    pub fn settings_tab_order(&self) -> Vec<String> {
        let mut order = self.tab_order.clone();
        for &m in MODULES {
            if !order.iter().any(|t| t == m) {
                order.push(m.to_string());
            }
        }
        for m in self.installed_mod_tabs() {
            if !order.iter().any(|t| t == &m) {
                order.push(m);
            }
        }
        for m in &self.mod_tabs {
            if !order.iter().any(|t| t == m) {
                order.push(m.clone());
            }
        }
        order.retain(|t| MODULES.contains(&t.as_str()) || (BUILTIN_MOD_TABS.contains(&t.as_str()) && self.is_known_module(t)) || self.mod_tabs.iter().any(|m| m == t));
        let mut seen = std::collections::BTreeSet::new();
        order.retain(|t| seen.insert(t.clone()));
        order
    }
    pub fn move_tab(&mut self, tab: &str, delta: i32) {
        // Only swap with a *visible* neighbour: tab_order may still hold the slot of an
        // uninstalled built-in mod, and swapping with it would look like a no-op.
        let visible = self.settings_tab_order();
        let Some(from) = visible.iter().position(|t| t == tab) else { return };
        let to = from as i32 + delta;
        if to < 0 || to as usize >= visible.len() {
            return;
        }
        let other = visible[to as usize].clone();
        self.swap_tabs(tab, &other);
    }
    #[allow(dead_code)] pub fn reorder_tab(&mut self, tab: &str, target_idx: usize) {
        for &m in MODULES {
            if !self.tab_order.iter().any(|t| t == m) {
                self.tab_order.push(m.to_string());
            }
        }
        if let Some(pos) = self.tab_order.iter().position(|t| t == tab) {
            let item = self.tab_order.remove(pos);
            let clamped = target_idx.min(self.tab_order.len());
            self.tab_order.insert(clamped, item);
        }
    }
    pub fn swap_tabs(&mut self, a: &str, b: &str) {
        for &m in MODULES {
            if !self.tab_order.iter().any(|t| t == m) {
                self.tab_order.push(m.to_string());
            }
        }
        let pos_a = self.tab_order.iter().position(|t| t == a);
        let pos_b = self.tab_order.iter().position(|t| t == b);
        if let (Some(pa), Some(pb)) = (pos_a, pos_b) {
            self.tab_order.swap(pa, pb);
        }
    }
    /// Saved tab preference is independent of whether the package is currently active.
    pub fn mod_tab_enabled(&self,m:&str)->bool{self.is_known_module(m)&&self.modules.get(m).copied().unwrap_or(false)}
    pub fn enabled(&self,m:&str)->bool{
        let owner=match m{"Catatan"=>Some("sticky_notes"),"Cuaca"=>Some("weather"),"Daya"=>Some("daya"),_=>None};
        self.mod_tab_enabled(m)&&owner.is_none_or(|id|!self.disabled_mods.iter().any(|m|m==id))
    }
    pub fn set_module_enabled(&mut self,module:&str,on:bool)->bool{
        if !self.is_known_module(module){return false}
        if !on&&self.enabled(module)&&self.active_tab_order().len()<=1{return false}
        self.modules.insert(module.into(),on);
        if !self.enabled(&self.default_tab){if let Some(first)=self.active_tab_order().first(){self.default_tab=first.clone();}}
        true
    }
    pub fn normalize(&mut self) {
        self.edge_gap=self.edge_gap.clamp(0,160); self.scale=if self.scale.is_finite(){self.scale.clamp(0.8,1.35)}else{1.};
        // A saved 0 would make the whole notch invisible while its click region stayed active.
        self.opacity=self.opacity.clamp(MIN_OPACITY,255);
        self.focus_minutes=self.focus_minutes.clamp(1,180);self.break_minutes=self.break_minutes.clamp(1,60);
        self.theme_material = crate::glass::MaterialMode::from_str(&self.theme_material).as_str().into();
        self.theme_color = if crate::glass::is_light_mode(&self.theme_color) { "light".into() } else { "dark".into() };
        self.water_serving=self.water_serving.clamp(50,2000);self.water_target=self.water_target.clamp(100,10000);
        self.water_interval=self.water_interval.clamp(5,360);self.move_interval=self.move_interval.clamp(5,360);self.eye_interval=self.eye_interval.clamp(5,120);
        if let Some(enabled)=self.modules.remove("Fokus"){self.modules.insert("Reminder".into(),enabled);}
        for tab in &mut self.tab_order {if tab=="Fokus"{*tab="Reminder".into();}}
        if self.default_tab=="Fokus"{self.default_tab="Reminder".into();}
        if !["Jakarta","Bandung","Surabaya"].contains(&self.prayer_city.as_str()){self.prayer_city="Jakarta".into();}
        self.stretch_seconds=self.stretch_seconds.clamp(15,300);self.prayer_lead=self.prayer_lead.min(30);
        self.prayer_times.resize(5,String::new());self.prayer_times.truncate(5);
        for time in &mut self.prayer_times {if chrono::NaiveTime::parse_from_str(time,"%H:%M").is_err(){time.clear();}}
        self.mod_tabs.retain(|t| !t.is_empty() && !MODULES.contains(&t.as_str()) && !BUILTIN_MOD_TABS.contains(&t.as_str()));
        let mut seen_tabs=std::collections::BTreeSet::new();
        self.mod_tabs.retain(|t| seen_tabs.insert(t.clone()));
        let mut seen_network=std::collections::BTreeSet::new();
        self.network_mods.retain(|m| !m.is_empty() && seen_network.insert(m.clone()));
        for h in &mut self.net_allowed_hosts { *h = h.trim().to_ascii_lowercase(); }
        let mut seen_hosts=std::collections::BTreeSet::new();
        self.net_allowed_hosts.retain(|h| !h.is_empty() && seen_hosts.insert(h.clone()));
        let mod_tabs=self.installed_mod_tabs(); for &m in MODULES {self.modules.entry(m.to_owned()).or_insert(true);} for m in &mod_tabs {self.modules.entry(m.clone()).or_insert(true);}
        self.modules.retain(|m,_|MODULES.contains(&m.as_str())||mod_tabs.iter().any(|t|t==m)||self.mod_tabs.iter().any(|t|t==m));

        for &m in MODULES {
            if !self.tab_order.iter().any(|t| t == m) {
                self.tab_order.push(m.to_string());
            }
        }
        // Built-in tab names stay in the order even while the mod is uninstalled, so installing
        // it again restores the slot the user chose instead of appending it to the end.
        for m in &mod_tabs {if !self.tab_order.iter().any(|t| t == m) {self.tab_order.push(m.clone());}}
        for m in &self.mod_tabs {if !self.tab_order.iter().any(|t| t == m) {self.tab_order.push(m.clone());}}
        let mut seen=std::collections::BTreeSet::new();self.tab_order.retain(|m| (MODULES.contains(&m.as_str())||BUILTIN_MOD_TABS.contains(&m.as_str())||self.mod_tabs.iter().any(|t|t==m))&&seen.insert(m.clone()));
        if self.language != "en" && self.language != "id" {
            self.language = "id".into();
        }

        if self.default_tab=="Sistem"||(!MODULES.contains(&self.default_tab.as_str())&&!mod_tabs.iter().any(|t|t==&self.default_tab)&&!self.mod_tabs.iter().any(|t|t==&self.default_tab)){self.default_tab="Musik".into();}
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Todo { pub id:u64,pub text:String,pub done:bool,pub due:Option<DateTime<Local>>,#[serde(default)]pub reminded:bool }
#[derive(Clone, Serialize, Deserialize)]
pub struct Drink {pub date:NaiveDate,pub ml:u32}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub todos: Vec<Todo>,
    pub notes: String,
    pub water: Vec<Drink>,
    pub clipboard_history: Vec<crate::clipboard::ClipItem>,
    #[serde(default)] pub prayer_delivered: Vec<String>,
    #[serde(default)] pub sticky_notes: Vec<crate::sticky::StickyNote>,
    #[serde(default)] pub selected_note: usize,
    #[serde(default)] pub power_month: String,
    #[serde(default)] pub monthly_kwh: f32,
    #[serde(default)] pub monthly_cost_idr: u32,
    #[serde(default)] pub power_days_active: Vec<String>,
}

impl State {
    pub fn record_power(&mut self, date: NaiveDate, kwh: f32) {
        let month_key = date.format("%Y-%m").to_string();
        let day_key = date.format("%Y-%m-%d").to_string();
        if self.power_month != month_key {
            self.power_month = month_key;
            self.monthly_kwh = 0.0;
            self.monthly_cost_idr = 0;
            self.power_days_active.clear();
        }
        if !self.power_days_active.iter().any(|d| d == &day_key) {
            self.power_days_active.push(day_key);
        }
        if kwh.is_finite() && kwh > 0.0 {
            self.monthly_kwh += kwh;
        }
        self.monthly_cost_idr = (self.monthly_kwh * 1444.70).round() as u32;
    }
}

pub fn resolve_date_rollover(
    current_year: i32,
    current_month: u32,
    current_day: u32,
    last_system_date: NaiveDate,
    new_system_date: NaiveDate,
    hover_stage: u8,
) -> (i32, u32, u32, bool) {
    if new_system_date == last_system_date {
        return (current_year, current_month, current_day, false);
    }
    use chrono::Datelike;
    let was_viewing_today = current_year == last_system_date.year()
        && current_month == last_system_date.month()
        && current_day == last_system_date.day();
    if was_viewing_today || hover_stage < 2 {
        (new_system_date.year(), new_system_date.month(), new_system_date.day(), true)
    } else {
        (current_year, current_month, current_day, false)
    }
}

pub struct Store { pub root:PathBuf, pub last_error: Option<String>, held: std::collections::HashSet<String> }
impl Store {
    pub fn new(root:PathBuf)->Result<Self,String> {std::fs::create_dir_all(&root).map_err(|_|"Folder data tidak dapat dibuat".to_owned())?;Ok(Self{root,last_error:None,held:std::collections::HashSet::new()})}
    pub fn load<T:serde::de::DeserializeOwned+Default>(&mut self,name:&str)->T {
        let path=self.root.join(name);
        if !path.exists(){return T::default()}
        match std::fs::read(&path).ok().and_then(|b|serde_json::from_slice(&b).ok()) {Some(v)=>v,None=>{
            self.last_error=Some(format!("{name} gagal dibaca; data asli dipertahankan sebagai recovery dan penyimpanan file ini ditahan sampai dipulihkan"));
            self.held.insert(name.to_string());
            let backup=self.root.join(format!("{name}.recovery-{}",Local::now().timestamp_millis()));
            let _=std::fs::copy(&path,backup);T::default()
        }}
    }
    /// Files whose last load failed and whose next save is held. A future settings action
    /// can offer restore_from_recovery or acknowledge_corrupt for them; until then, no
    /// save may silently overwrite the corrupt file with a default.
    pub fn held_files(&self)->Vec<String> { let mut files:Vec<String>=self.held.iter().cloned().collect(); files.sort(); files }
    /// Copies the newest `{name}.recovery-*` file back over `{name}` and lifts the save
    /// hold, so the next load reads the recovered data instead of a default.
    #[allow(dead_code)]
    pub fn restore_from_recovery(&mut self,name:&str)->Result<(),String> {
        if Path::new(name).file_name().and_then(|v|v.to_str())!=Some(name){return Err("Nama file data tidak valid".into())}
        let prefix=format!("{name}.recovery-");
        let mut best:Option<(i64,PathBuf)>=None;
        let entries=std::fs::read_dir(&self.root).map_err(|_|"Folder data tidak dapat dibaca".to_owned())?;
        for entry in entries.flatten() {
            let fname=entry.file_name().to_string_lossy().into_owned();
            if let Some(stamp)=fname.strip_prefix(&prefix).and_then(|s|s.parse::<i64>().ok()) {
                if best.as_ref().is_none_or(|(t,_)| stamp > *t) { best=Some((stamp,entry.path())); }
            }
        }
        let (_,recovery)=best.ok_or_else(||format!("Tidak ada file recovery untuk {name}"))?;
        std::fs::copy(&recovery,self.root.join(name)).map_err(|_|"Pemulihan dari recovery gagal".to_owned())?;
        self.held.remove(name);
        Ok(())
    }
    /// Accepts the data loss for `name`: the recovery copy stays on disk, but the next
    /// save is allowed to overwrite the corrupt file with fresh data.
    #[allow(dead_code)]
    pub fn acknowledge_corrupt(&mut self,name:&str) { self.held.remove(name); }
    pub fn save<T:Serialize>(&mut self,name:&str,value:&T)->Result<(),String> {
        if Path::new(name).file_name().and_then(|v|v.to_str())!=Some(name){return Err("Nama file data tidak valid".into())}
        if self.held.contains(name) {
            return Err(format!("{name} tidak disimpan: file ini gagal dibaca sebelumnya dan data aslinya dipertahankan sebagai recovery; pulihkan dulu sebelum menimpa"));
        }
        let path=self.root.join(name);let tmp=self.root.join(format!("{name}.tmp"));
        let bytes=serde_json::to_vec_pretty(value).map_err(|_|"Data tidak dapat disimpan".to_owned())?;
        std::fs::write(&tmp,bytes).map_err(|_|"Folder data tidak dapat ditulis".to_owned())?;
        // Windows MoveFileEx replaces an existing file atomically on the same volume.
        use windows::Win32::Storage::FileSystem::{MoveFileExW,MOVEFILE_REPLACE_EXISTING,MOVEFILE_WRITE_THROUGH};
        use windows::core::PCWSTR;
        use std::os::windows::ffi::OsStrExt;
        let src:Vec<u16>=tmp.as_os_str().encode_wide().chain(Some(0)).collect();let dst:Vec<u16>=path.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {MoveFileExW(PCWSTR(src.as_ptr()),PCWSTR(dst.as_ptr()),MOVEFILE_REPLACE_EXISTING|MOVEFILE_WRITE_THROUGH)}
            .map_err(|_|"Penyimpanan atomik gagal; data lama dipertahankan".to_owned())?;
        self.last_error=None;Ok(())
    }
}

#[derive(Default)]
pub struct FocusClock {deadline:Option<Instant>,paused:Duration,total:Duration,pub is_break:bool}
impl FocusClock {
    pub fn start(&mut self,minutes:u32,is_break:bool){self.deadline=Some(Instant::now()+Duration::from_secs(minutes.clamp(1,180) as u64*60));self.paused=Duration::ZERO;self.total=Duration::from_secs(minutes.clamp(1,180)as u64*60);self.is_break=is_break;}
    pub fn total(&self)->Duration{self.total}
    pub fn running(&self)->bool{self.deadline.is_some()}
    pub fn remaining(&self)->Duration{self.deadline.map(|d|d.saturating_duration_since(Instant::now())).unwrap_or(self.paused)}
    pub fn pause(&mut self){self.paused=self.remaining();self.deadline=None;}
    pub fn resume(&mut self){if !self.paused.is_zero(){self.deadline=Some(Instant::now()+self.paused);self.paused=Duration::ZERO;}}
    pub fn reset(&mut self){self.deadline=None;self.paused=Duration::ZERO;}
    pub fn complete(&mut self)->Option<bool>{
        if self.running()&&self.remaining().is_zero(){
            let was_break = self.is_break;
            self.reset();
            Some(was_break)
        } else {
            None
        }
    }
}

#[derive(Clone,Copy,Default)]
pub struct WorkPolicy {pub enabled:bool,pub visible:bool,pub fullscreen:bool,pub playing:bool}
impl WorkPolicy {pub fn visualize(&self)->bool{self.enabled&&self.visible&&!self.fullscreen&&self.playing}}

#[cfg(test)] mod tests {
 use super::*;

 #[test]fn settings_keep_default_visible_and_reject_disabling_last_tab(){let mut s=Settings::default();s.default_tab="Musik".into();assert!(s.set_module_enabled("Musik",false));assert!(s.enabled(&s.default_tab));let order=s.active_tab_order();for tab in &order[1..]{assert!(s.set_module_enabled(tab,false));}assert_eq!(s.active_tab_order().len(),1);assert!(!s.set_module_enabled(&order[0],false));assert!(!s.set_module_enabled("invalid",true));}
 #[test]fn prayer_location_defaults_to_jakarta_and_rejects_unknown_city(){let mut s:Settings=serde_json::from_str("{}").unwrap();assert_eq!(s.prayer_city,"Jakarta");s.prayer_city="not-a-city".into();s.normalize();assert_eq!(s.prayer_city,"Jakarta");s.prayer_city="Bandung".into();s.normalize();assert_eq!(s.prayer_city,"Bandung");}

 #[test]fn visualization_stops_for_every_inactive_condition(){for mask in 0..16{let p=WorkPolicy{enabled:mask&1!=0,visible:mask&2!=0,fullscreen:mask&4!=0,playing:mask&8!=0};assert_eq!(p.visualize(),mask==11);}}
 #[test]fn settings_sanitize_corrupt_values(){let mut s=Settings::default();s.scale=f32::NAN;s.focus_minutes=0;s.normalize();assert_eq!(s.scale,1.);assert_eq!(s.focus_minutes,1);}
 #[test]fn focus_pause_retains_time(){let mut f=FocusClock::default();f.start(25,false);f.pause();let remaining=f.remaining();assert!(!f.running());assert!(remaining.as_secs()>1490);f.resume();assert!(f.running());f.reset();assert!(f.remaining().is_zero());}
 #[test]fn normalize_does_not_inject_legacy_non_alpha_modules(){let mut s=Settings::default();s.normalize();assert_eq!(s.modules.len(),MODULES.len());assert!(!s.modules.contains_key("Tugas"));assert!(!s.modules.contains_key("Cuaca"));assert!(!s.modules.contains_key("AI"));assert!(!s.modules.contains_key("WhatsApp"));}
    #[test]
    fn mod_tab_installation_lifecycle_in_settings() {
        let mut s = Settings::default();
        assert!(!s.enabled("Catatan"));
        assert!(!s.is_known_module("Catatan"));

        s.installed_mods.push("sticky_notes".into());
        s.normalize();
        assert!(s.is_known_module("Catatan"));
        assert!(s.enabled("Catatan"));
        assert!(s.active_tab_order().contains(&"Catatan".to_string()));

        assert!(s.set_module_enabled("Catatan", false));
        assert!(!s.enabled("Catatan"));
        assert!(!s.active_tab_order().contains(&"Catatan".to_string()));

        s.installed_mods.push("weather".into());
        s.normalize();
        assert!(s.is_known_module("Cuaca"));
        assert!(s.enabled("Cuaca"));
        assert!(s.active_tab_order().contains(&"Cuaca".to_string()));

        s.installed_mods.clear();
        s.normalize();
        assert!(!s.is_known_module("Catatan"));
        assert!(!s.enabled("Catatan"));
        assert!(!s.is_known_module("Cuaca"));
        assert!(!s.enabled("Cuaca"));
    }

    #[test]
    fn custom_mod_tabs_never_appear_in_the_tab_strip() {
        let mut s = Settings::default();
        s.installed_mods.push("counter_lua".into());
        s.modules.insert("Kontrol".into(), true);
        s.tab_order.push("Kontrol".into());
        s.default_tab = "Kontrol".into();
        s.normalize();
        assert!(!s.tab_order.contains(&"Kontrol".to_string()));
        assert!(!s.active_tab_order().contains(&"Kontrol".to_string()));
        assert!(!s.modules.contains_key("Kontrol"));
        assert_eq!(s.default_tab, "Musik");
    }

    #[test]
    fn builtin_mod_tabs_keep_their_place_in_the_tab_strip() {
        let mut s = Settings::default();
        s.installed_mods = vec!["sticky_notes".into(), "weather".into()];
        s.normalize();
        let order = s.active_tab_order();
        assert!(order.contains(&"Catatan".to_string()));
        assert!(order.contains(&"Cuaca".to_string()));
        s.default_tab = "Cuaca".into();
        s.normalize();
        assert_eq!(s.default_tab, "Cuaca");
    }

    #[test]
    fn list_scroll_clamps_to_the_visible_window() {
        assert_eq!(clamp_list_scroll(0, 10, 4), 0);
        assert_eq!(clamp_list_scroll(6, 10, 4), 6);
        assert_eq!(clamp_list_scroll(9, 10, 4), 6);
        assert_eq!(clamp_list_scroll(3, 2, 4), 0);
        assert_eq!(clamp_list_scroll(5, 10, 0), 5);
        assert_eq!(clamp_list_scroll(usize::MAX, 10, 4), 6);
    }

    #[test]
    fn settings_tab_order_lists_the_tabs_the_user_can_manage() {
        let mut s = Settings::default();
        let expected: Vec<String> = MODULES.iter().map(|m| m.to_string()).collect();
        assert_eq!(s.settings_tab_order(), expected);

        s.installed_mods = vec!["sticky_notes".into()];
        s.normalize();
        let order = s.settings_tab_order();
        assert!(order.contains(&"Catatan".to_string()));

        s.tab_order.push("Kontrol".into());
        s.modules.insert("Kontrol".into(), true);
        assert!(!s.settings_tab_order().contains(&"Kontrol".to_string()));
    }

    #[test]
    fn opacity_is_normalized_into_a_visible_range() {
        let mut s = Settings::default();
        s.opacity = 0;
        s.normalize();
        assert_eq!(s.opacity, MIN_OPACITY);
        s.opacity = 255;
        s.normalize();
        assert_eq!(s.opacity, 255);
    }

    #[test]
    fn reinstalling_a_builtin_mod_restores_its_tab_order_slot() {
        let mut s = Settings::default();
        s.installed_mods = vec!["sticky_notes".into(), "weather".into()];
        s.normalize();
        s.move_tab("Cuaca", -3);
        let pos = s.tab_order.iter().position(|t| t == "Cuaca").unwrap();
        assert_eq!(pos, 2, "test setup should move Cuaca ahead of the defaults");

        s.installed_mods.retain(|m| m != "weather");
        s.modules.remove("Cuaca");
        s.normalize();
        assert!(!s.active_tab_order().contains(&"Cuaca".to_string()));
        assert!(!s.settings_tab_order().contains(&"Cuaca".to_string()));
        assert_eq!(s.tab_order.iter().position(|t| t == "Cuaca"), Some(pos));

        s.installed_mods.push("weather".into());
        s.normalize();
        assert_eq!(s.tab_order.iter().position(|t| t == "Cuaca"), Some(pos));
        assert!(s.active_tab_order().contains(&"Cuaca".to_string()));
    }

    #[test]
    fn moving_a_tab_skips_uninstalled_builtin_slots() {
        let mut s = Settings::default();
        s.tab_order = ["Musik", "Kalender", "Cuaca", "Reminder", "Clipboard"].iter().map(|t| t.to_string()).collect();
        s.move_tab("Reminder", -1);
        assert_eq!(s.settings_tab_order(), ["Musik", "Reminder", "Kalender", "Clipboard"].iter().map(|t| t.to_string()).collect::<Vec<_>>());
    }

    #[test]
    fn community_mod_tabs_are_opt_in_and_keep_their_slot() {
        let mut s = Settings::default();
        assert!(!s.is_known_module("IHSG"));

        s.set_mod_tab("IHSG", true);
        assert!(s.enabled("IHSG"));
        assert!(s.active_tab_order().contains(&"IHSG".to_string()));
        assert!(s.settings_tab_order().contains(&"IHSG".to_string()));
        let pos = s.tab_order.iter().position(|t| t == "IHSG").unwrap();

        s.set_mod_tab("IHSG", false);
        assert!(!s.enabled("IHSG"));
        assert!(!s.active_tab_order().contains(&"IHSG".to_string()));
        assert!(s.settings_tab_order().contains(&"IHSG".to_string()));
        assert_eq!(s.tab_order.iter().position(|t| t == "IHSG"), Some(pos));

        s.normalize();
        assert_eq!(s.tab_order.iter().position(|t| t == "IHSG"), Some(pos));

        s.set_mod_tab("IHSG", true);
        assert!(s.enabled("IHSG"));
        assert!(s.active_tab_order().contains(&"IHSG".to_string()));
        assert_eq!(s.tab_order.iter().position(|t| t == "IHSG"), Some(pos));
    }

    #[test]
    fn builtin_tab_names_cannot_be_registered_as_community_tabs() {
        let mut s = Settings::default();
        s.set_mod_tab("Musik", true);
        s.set_mod_tab("Cuaca", true);
        assert!(s.mod_tabs.is_empty());
    }

    #[test]
    fn network_permission_is_opt_in_and_idempotent() {
        let mut s = Settings::default();
        assert!(s.network_mods.is_empty());
        s.set_mod_network("ihsg_monitor", true);
        s.set_mod_network("ihsg_monitor", true);
        assert_eq!(s.network_mods, vec!["ihsg_monitor".to_string()]);
        s.normalize();
        assert_eq!(s.network_mods, vec!["ihsg_monitor".to_string()]);
        s.set_mod_network("ihsg_monitor", false);
        assert!(s.network_mods.is_empty());
    }

    #[test]
    fn settings_language_lifecycle() {
        let mut s = Settings::default();
        assert_eq!(s.language, "id");

        s.language = "unknown".into();
        s.normalize();
        assert_eq!(s.language, "id");

        s.language = "en".into();
        s.normalize();
        assert_eq!(s.language, "en");

        let json = serde_json::to_string(&s).unwrap();
        let loaded: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.language, "en");
    }

    #[test]
    fn monthly_power_accumulation_and_month_rollover() {
        let mut state = State::default();
        let d1 = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let d3 = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();

        state.record_power(d1, 0.5);
        assert_eq!(state.power_month, "2026-10");
        assert!((state.monthly_kwh - 0.5).abs() < 1e-5);
        assert_eq!(state.power_days_active.len(), 1);

        state.record_power(d2, 0.5);
        assert_eq!(state.power_month, "2026-10");
        assert!((state.monthly_kwh - 1.0).abs() < 1e-5);
        assert_eq!(state.monthly_cost_idr, 1445);
        assert_eq!(state.power_days_active.len(), 2);

        // Next month resets accumulation
        state.record_power(d3, 0.2);
        assert_eq!(state.power_month, "2026-11");
        assert!((state.monthly_kwh - 0.2).abs() < 1e-5);
        assert_eq!(state.power_days_active.len(), 1);
    }

    #[test]
    fn test_calendar_date_rollover_midnight() {
        let d1 = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();

        // 1. Same date, no rollover
        let (y, m, d, changed) = resolve_date_rollover(2026, 10, 29, d1, d1, 2);
        assert_eq!((y, m, d, changed), (2026, 10, 29, false));

        // 2. Rollover past midnight when user was viewing today
        let (y, m, d, changed) = resolve_date_rollover(2026, 10, 29, d1, d2, 2);
        assert_eq!((y, m, d, changed), (2026, 10, 30, true));

        // 3. Month rollover (Oct 31 -> Nov 1)
        let oct31 = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
        let nov1 = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        let (y, m, d, changed) = resolve_date_rollover(2026, 10, 31, oct31, nov1, 2);
        assert_eq!((y, m, d, changed), (2026, 11, 1, true));

        // 4. Year rollover (Dec 31 -> Jan 1)
        let dec31 = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let jan1 = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        let (y, m, d, changed) = resolve_date_rollover(2026, 12, 31, dec31, jan1, 2);
        assert_eq!((y, m, d, changed), (2027, 1, 1, true));

        // 5. Stage 1/0 collapsed always syncs even if user was viewing another day earlier
        let (y, m, d, changed) = resolve_date_rollover(2026, 10, 15, d1, d2, 1);
        assert_eq!((y, m, d, changed), (2026, 10, 30, true));

        // 6. Stage 2 expanded while actively browsing another day keeps active browse view
        let (y, m, d, changed) = resolve_date_rollover(2026, 10, 15, d1, d2, 2);
        assert_eq!((y, m, d, changed), (2026, 10, 15, false));
    }

    #[test]
    fn theme_material_defaults_to_solid_and_serializes_cleanly() {
        let s = Settings::default();
        assert_eq!(s.theme_material, "solid");
        let json = serde_json::to_string(&s).unwrap();
        let mut parsed: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.theme_material, "solid");
        parsed.theme_material = "".into();
        parsed.normalize();
        assert_eq!(parsed.theme_material, "solid");
        parsed.theme_material = "Adaptive".into();
        parsed.theme_color = "LIGHT".into();
        parsed.normalize();
        assert_eq!((parsed.theme_material.as_str(), parsed.theme_color.as_str()), ("adaptive", "light"));
        parsed.theme_material = "nonsense".into();
        parsed.theme_color = "neon".into();
        parsed.normalize();
        assert_eq!((parsed.theme_material.as_str(), parsed.theme_color.as_str()), ("solid", "dark"));
        let legacy: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!((legacy.theme_material.as_str(), legacy.theme_color.as_str()), ("solid", "dark"));
    }
}

#[cfg(test)]
mod public_regression_tests {
use super::*;
#[test]
fn alpha_only_exposes_portable_modules(){let s=Settings::default();assert_eq!(MODULES,["Musik","Kalender","Reminder","Clipboard"]);assert!(!s.enabled("Clipboard"));assert!(!s.enabled("Daya"));}
}
