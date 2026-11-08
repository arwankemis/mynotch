//! Pure effective settings and lifecycle intents. No presenter or OS policy is enabled here.
#![allow(dead_code)]
use crate::{activity::*,core::{Settings,State,Store}};

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct EffectivePresentation {
    pub quiet:bool,pub saver:bool,pub hide_sensitive:bool,pub automatic_popup:bool,pub sound:bool,
    pub visualizer:bool,pub glow:bool,pub reduced_motion:bool,pub material:String,
    pub privacy_enabled:bool,pub privacy_app_names:bool,
}
impl Settings {
    pub fn effective_presentation(&self)->EffectivePresentation {
        EffectivePresentation {quiet:self.quiet_mode,saver:self.power_saver_mode,
            hide_sensitive:self.quiet_mode,automatic_popup:!self.quiet_mode,sound:!self.quiet_mode,
            visualizer:self.visualizer&&!self.power_saver_mode,glow:self.glow&&!self.power_saver_mode,
            reduced_motion:self.motion_reduced||self.power_saver_mode,
            material:if self.power_saver_mode{"solid".into()}else{self.theme_material.clone()},
            privacy_enabled:self.privacy_enabled,privacy_app_names:self.privacy_show_app_names&&!self.quiet_mode}
    }
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SensitiveSurface { Compact,Music,Lyrics,Notes,Clipboard,Calendar,Weather,CommunityMod,PrivacyDetail,SettingsSecrets,Error }
pub enum Presented<T> { Hidden(SensitiveSurface), Visible(T) }
// Never accidentally format payloads into diagnostic logs, even outside Quiet.
impl<T> std::fmt::Debug for Presented<T>{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{match self{Self::Hidden(s)=>f.debug_tuple("Hidden").field(s).finish(),Self::Visible(_)=>f.write_str("Visible([content])")}}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum UserIntent { Settings,MediaTransport,HardwareVolume,ReadContent,EditContent,Copy,Export,Popout,ConvertNote,OpenNoteLink }
impl EffectivePresentation {
    pub fn present<T>(&self,surface:SensitiveSurface,value:T)->Presented<T>{if self.hide_sensitive{Presented::Hidden(surface)}else{Presented::Visible(value)}}
    pub fn allows(&self,intent:UserIntent)->bool{!self.quiet||matches!(intent,UserIntent::Settings|UserIntent::MediaTransport|UserIntent::HardwareVolume)}
    pub fn activity_policy(&self,visible:bool,reminders:bool,media:bool)->PresentationPolicy{
        PresentationPolicy{visible,quiet:self.quiet,reminders,media,reduced_motion:self.reduced_motion}
    }
    pub fn capture_allowed(&self,base:crate::core::WorkPolicy)->bool{self.visualizer&&base.visualize()}
    pub fn privacy_indicator(&self,mic:bool,camera:bool,names:Vec<String>)->PrivacyIndicator{
        PrivacyIndicator{enabled:self.privacy_enabled,mic:self.privacy_enabled&&mic,camera:self.privacy_enabled&&camera,
            app_names:if self.privacy_enabled&&self.privacy_app_names{names}else{Vec::new()}}
    }
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct PrivacyIndicator {pub enabled:bool,pub mic:bool,pub camera:bool,pub app_names:Vec<String>}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum PresetAction {SetQuiet(bool),SetPowerSaver(bool),Reset}
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub struct LifecyclePlan {
    pub flush_editors:bool,pub hide_editors:bool,pub restore_editors_without_activation:bool,
    pub stop_capture:bool,pub clear_levels:bool,pub settle_animations:bool,pub rebuild_geometry:bool,
    pub clear_sensitive_caches:bool,
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Transition {pub effective:EffectivePresentation,pub plan:LifecyclePlan,pub changed:bool}
pub fn apply(base:&mut Settings,action:PresetAction)->Transition {
    let old=base.effective_presentation();
    match action{PresetAction::SetQuiet(on)=>base.quiet_mode=on,PresetAction::SetPowerSaver(on)=>base.power_saver_mode=on,PresetAction::Reset=>{base.quiet_mode=false;base.power_saver_mode=false;}}
    let effective=base.effective_presentation();let changed=old!=effective;
    let material_changed=old.material!=effective.material;
    let plan=if !changed{LifecyclePlan::default()}else{LifecyclePlan{
        flush_editors:(!old.quiet&&effective.quiet)||material_changed,
        hide_editors:!old.quiet&&effective.quiet,restore_editors_without_activation:old.quiet&&!effective.quiet,
        stop_capture:old.visualizer&&!effective.visualizer,clear_levels:old.visualizer&&!effective.visualizer,
        settle_animations:effective.reduced_motion&&!old.reduced_motion,rebuild_geometry:material_changed,
        clear_sensitive_caches:old.quiet!=effective.quiet,
    }};
    Transition{effective,plan,changed}
}
/// Shared flush hook: 022 supplies a generation-checked editor implementation.
pub trait EditorFlush {fn flush_editors(&mut self)->Result<(),&'static str>;}
pub fn flush_for_transition(transition:&Transition,editors:&mut impl EditorFlush)->Result<(),&'static str>{
    if transition.plan.flush_editors{editors.flush_editors()?;}Ok(())
}
pub fn persist(base:&Settings,store:&mut Store)->Result<(),&'static str>{store.save("settings.json",base).map_err(|_|"Preset belum tersimpan")}

#[cfg(test)]
#[path="presets_tests.rs"]
mod tests;
