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

/// One silent backlog activity. Presentation time advances only while that group
/// was actually primary; dismissing its summary never discards its occurrences.
#[derive(Default)]
pub struct QuietBacklog {last_ms:Option<i64>,was_primary_visible:bool}
impl QuietBacklog {
    pub fn reconcile(&mut self,state:&mut State,inputs:&mut ActivityInputs,now_ms:i64,policy:PresentationPolicy,previous:Option<&ActivitySnapshot>){
        if policy.quiet{
            for p in &mut state.pending_local_reminders{
                if !p.deferred_quiet||p.deferred_count<p.coalesced_count{state.quiet_backlog_remaining_ms=8_000;}
                p.deferred_quiet=true;p.silent=true;p.deferred_count=p.coalesced_count;
            }
        }
        if self.was_primary_visible&&policy.visible&&!policy.quiet&&previous.is_some_and(|s|s.primary.id==ActivityId::Backlog){
            let elapsed=now_ms.saturating_sub(self.last_ms.unwrap_or(now_ms)).max(0).min(u32::MAX as i64) as u32;
            state.quiet_backlog_remaining_ms=state.quiet_backlog_remaining_ms.saturating_sub(elapsed);
        }
        // Replace rather than accumulate a duplicate group in reused input buffers.
        inputs.candidates.retain(|c|c.id!=ActivityId::Backlog);
        let pending:Vec<_>=state.pending_local_reminders.iter().filter(|p|p.deferred_quiet).collect();
        if pending.is_empty(){state.quiet_backlog_remaining_ms=0;}
        if !policy.quiet&&policy.reminders&&state.quiet_backlog_remaining_ms>0&&!pending.is_empty(){
            inputs.candidates.push(Candidate{id:ActivityId::Backlog,priority:2,created_ms:pending.iter().map(|p|p.due_at_unix_ms).min().unwrap_or(now_ms),expires_ms:None,
                target:Some(ActivityTarget::PendingReminders),text:format!("{} pengingat tertahan",pending.len()),progress:None});
        }
        self.last_ms=Some(now_ms);self.was_primary_visible=policy.visible&&!policy.quiet;
    }
    pub fn dismiss_summary(&mut self,state:&mut State){state.quiet_backlog_remaining_ms=0;self.was_primary_visible=false;}
    pub fn open_occurrence(state:&mut State,id:&ActivityId,quiet:bool)->Option<ActivityTarget>{
        if quiet{return None;}
        let p=state.pending_local_reminders.iter_mut().find(|p|&p.id==id&&p.deferred_quiet&&p.target!=ActivityTarget::Unavailable)?;
        p.deferred_quiet=false;p.silent=true;Some(p.target.clone())
    }
}

#[cfg(test)]
#[path="presets_tests.rs"]
mod tests;
