//! Approved preset presentation and lifecycle, applied without mutating base choices.
use super::*;
use crate::{activity::*,presets::*};

#[derive(Default)]
pub(super) struct PresetHost { pub(super) settings_unsaved:bool }
impl Shell {
    pub(super) fn preset_action(&mut self,action:PresetAction){
        // Flush before hiding native HWNDs. Failure leaves the preset unchanged.
        if !self.flush_editors_barrier(){return;}
        let transition=crate::presets::apply(&mut self.settings,action);
        if !transition.changed{return;}
        self.activities.runtime.pause_presentation();
        self.lyric_anim=None;self.tab_fade=None;self.reminder_anim=None;self.media_source_open=false;
        self.focused=None;self.pressed=None;self.hits.clear();self.settings_popup=None;self.reminder_menu=None;self.weather_menu_open=false;self.storage_dropdown_open=false;self.cover_key=0;
        self.activities.feedback=None;self.toast_queue.clear();
        self.sync_popout_privacy();
        if transition.effective.reduced_motion{self.settle_preset_animations();}
        self.sync_visualizer();self.glass_settings_changed();self.make_inputs();
        self.reconcile_activities();self.save();self.invalidate();
    }
    pub(super) fn settle_preset_animations(&mut self){unsafe{
        self.lyric_anim=None;self.tab_fade=None;self.reminder_anim=None;self.morph=None;
        self.glass.hover.target(None,true);self.glass.hover_animating=false;
        for id in [10,11,16]{let _=KillTimer(Some(self.hwnd),id);}
        self.startup_alpha=1.;let (w,h,_)=self.target_geometry();self.width=w;self.height=h;
        if self.morph_timer!=0{let _=timeKillEvent(self.morph_timer);self.morph_timer=0;}
        self.layout();self.update_window_region();
    }}
    pub(super) fn preset_indicator(&mut self,p:&mut Painter,x:f32,y:f32){
        if !self.settings.quiet_mode&&!self.settings.power_saver_mode{return;}
        let text=match(self.settings.quiet_mode,self.settings.power_saver_mode){(true,true)=>"T+H",(true,false)=>"T",_=>"H"};
        self.button_h(p,text,x,y,30.,24.,Action::PresetSettings,false);
    }
    pub(super) fn quiet_placeholder(&mut self,p:&mut Painter){unsafe{
        let en=self.settings.language=="en";
        p.outline_icon("lock",24.,74.,20.,0x92929e);
        p.text(if en{"Content hidden while Quiet is on"}else{"Isi disembunyikan saat Tenang"},24.,106.,self.width-48.,32.,13.,render::TEXT,false);
        self.button_h(p,if en{"Open Settings"}else{"Buka Pengaturan"},24.,150.,150.,28.,Action::PresetSettings,true);
    }}
    pub(super) fn quiet_music(&mut self,p:&mut Painter){unsafe{
        p.text(if self.settings.language=="en"{"Content hidden while Quiet is on"}else{"Isi disembunyikan saat Tenang"},24.,48.,self.width-112.,22.,12.,render::TEXT,false);
        self.button_h(p,if self.settings.language=="en"{"Settings"}else{"Pengaturan"},24.,78.,130.,26.,Action::PresetSettings,true);
        // Capabilities and selected IDs are retained; no metadata/artwork reaches this painter.
        let y=self.height-49.;
        self.transport_btn(p,"⏮",self.width/2.-78.,y,38.,30.,Action::Previous,false,self.track.can_previous);
        self.transport_btn(p,if self.track.playing{"Ⅱ"}else{"▶"},self.width/2.-19.,y,38.,30.,Action::Play,false,self.track.can_toggle);
        self.transport_btn(p,"⏭",self.width/2.+40.,y,38.,30.,Action::Next,false,self.track.can_next);
        let ratio=if self.track.duration_seconds>0.{(self.track.position_seconds/self.track.duration_seconds).clamp(0.,1.) as f32}else{0.};
        let rect=render::rect(190.,self.height-66.,self.width-214.,8.);p.fill(190.,self.height-65.,self.width-214.,3.,1.5,0x33333a);p.fill(190.,self.height-65.,(self.width-214.)*ratio,3.,1.5,0xd8d8de);
        if self.hit_enabled&&self.track.can_seek{self.hits.push(Hit{rect,action:Action::SeekPointer});}
        self.draw_media_source(p);
    }}
    pub(super) fn quiet_calendar(&mut self,p:&mut Painter){unsafe{
        // Dates stay usable; event titles, descriptions, locations and feeds are never read here.
        let month=chrono::NaiveDate::from_ymd_opt(self.year,self.month,1);
        let Some(first)=month else{self.quiet_placeholder(p);return;};
        p.text(&format!("{} {}",crate::i18n::month_name(self.month,&self.settings.language),self.year),24.,54.,240.,26.,16.,render::TEXT,true);
        self.button_h(p,"‹",260.,54.,28.,24.,Action::Month(-1),false);
        self.button_h(p,"›",294.,54.,28.,24.,Action::Month(1),false);
        let offset=first.weekday().num_days_from_monday();
        for day in 1..=31{
            if chrono::NaiveDate::from_ymd_opt(self.year,self.month,day).is_none(){break;}
            let cell=offset+day-1;let x=24.+(cell%7) as f32*42.;let y=90.+(cell/7) as f32*27.;
            self.button_h(p,&day.to_string(),x,y,36.,24.,Action::Day(day),day==self.day);
        }
        p.text(if self.settings.language=="en"{"Event content hidden while Quiet is on"}else{"Isi agenda disembunyikan saat Tenang"},340.,92.,self.width-364.,80.,12.,render::TEXT,false);
        self.button_h(p,if self.settings.language=="en"{"Settings"}else{"Pengaturan"},340.,190.,150.,28.,Action::PresetSettings,true);
    }}
}
pub(super) fn quiet_action_allowed(action:&Action)->bool{
    !matches!(action,Action::NotesSubview(..)|Action::ConvertNote(..)|Action::NoteSelectId(..)|Action::NoteCommand(..)|Action::TaskSelect(..)|Action::TaskDone(..)|Action::OpenTodoNote(..)|Action::DetachTodoNote(..)|Action::TasksPage(..)|Action::SelectClip(..)|Action::CopyClip(..)|Action::DeleteClip(..)|Action::ClearClips|Action::ClipPage(..)|Action::TogglePinClip(..)|Action::ToggleClipSearch|Action::WeatherToggleCustom|Action::WeatherSave|Action::WeatherSetCity(..)|Action::CalendarSave|Action::AiSave|Action::AiSend|Action::AiClear|Action::ModButtonClick(..)|Action::NoteAdd|Action::NoteSelect(..)|Action::NoteDelete(..)|Action::NoteColor(..)|Action::NoteExport(..)|Action::NotePopout|Action::AddTodo|Action::Todo(..)|Action::DeleteTodo(..)|Action::SaveNote)
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn quiet_dispatch_blocks_content_but_keeps_escape_transport_and_base_controls(){
  for a in [Action::NotePopout,Action::NoteExport(2),Action::AddTodo,Action::CopyClip(0),Action::CalendarSave,Action::WeatherSave]{assert!(!quiet_action_allowed(&a));}
  for a in [Action::PresetSettings,Action::Preset(PresetAction::Reset),Action::Settings,Action::Play,Action::Seek(1.),Action::SelectMediaSource(None),Action::VolumeStep(1),Action::SettingsValue("material",1)]{assert!(quiet_action_allowed(&a));}
 }
}
