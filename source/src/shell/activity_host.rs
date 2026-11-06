//! Host adapter: source events are collected before presentation is reconciled.
use super::*;
use crate::activity::*;

pub(super) struct Activities {
    pub(super) runtime: ActivityRuntime, pub(super) focus: RunTracker, stretch: RunTracker,
    pub(super) discard_release:bool,epoch:(Instant,i64),hud_at:Option<Instant>,hud_generation:u64,hud_origin_ms:i64,
    pub(super) feedback:Option<(Candidate,Instant)>, inputs: ActivityInputs, saved_at: Option<Instant>, auto_revealed: bool,
}
impl Default for Activities{
    fn default()->Self{Self{runtime:Default::default(),focus:Default::default(),stretch:Default::default(),discard_release:false,epoch:(Instant::now(),Local::now().timestamp_millis()),hud_at:None,hud_generation:0,hud_origin_ms:0,feedback:None,inputs:Default::default(),saved_at:None,auto_revealed:false}}
}
impl Shell {
    pub(super) fn activity_now(&self)->i64 { self.activities.epoch.1.saturating_add(self.activities.epoch.0.elapsed().as_millis().min(i64::MAX as u128) as i64) }
    pub(super) fn activity_policy(&self)->PresentationPolicy {
        self.settings.effective_presentation().activity_policy(
            !self.suspended && !(self.fullscreen && self.settings.hide_fullscreen) && self.hover_stage>0 && (self.toast.is_none()||self.settings.quiet_mode),
            self.settings.enabled("Reminder"),self.settings.enabled("Musik"))
    }
    pub(super) fn start_activity_run(&mut self,stretch:bool){
        let now=self.activity_now();
        let tracker=if stretch {&mut self.activities.stretch}else{&mut self.activities.focus};
        if let Err(e)=tracker.start(&mut self.state,if stretch{ReminderKind::Stretch}else{ReminderKind::Focus},now){self.notice(e);}
    }
    pub(super) fn complete_activity_run(&mut self,stretch:bool){
        let now=self.activity_now();
        let tracker=if stretch {&mut self.activities.stretch}else{&mut self.activities.focus};
        if let Some(id)=tracker.id.clone(){if let Err(e)=tracker.complete(&mut self.state,&id,now){self.notice(e);}}
        self.persist_activities();
    }
    pub(super) fn enqueue_activity(&mut self,id:ActivityId){
        if let Some(p)=PendingReminder::new(id,self.activity_now()){
            match self.state.enqueue_local_reminder(p){Ok(true)=>self.persist_activities(),Err(e)=>self.notice(e),_=>{}}
        }
    }
    pub(super) fn recurring_activity(&mut self,kind:ReminderKind){
        match self.state.allocate_activity_sequence(){Ok(n)=>self.enqueue_activity(match kind{ReminderKind::Water=>ActivityId::Water(n),ReminderKind::Eye=>ActivityId::Eye(n),_=>ActivityId::Move(n)}),Err(e)=>self.notice(e)}
    }
    pub(super) fn activity_feedback(&mut self,text:String,timeout:u32){
        let now=self.activity_now();
        self.activities.feedback=Some((Candidate{id:ActivityId::Feedback(now as u64),priority:1,created_ms:now,expires_ms:Some(now+timeout as i64),target:None,text,progress:None},Instant::now()+Duration::from_millis(timeout as u64)));
        self.reconcile_activities();self.clock_timer();self.invalidate();
    }
    pub(super) fn persist_activities(&mut self){
        self.activities.saved_at=Some(Instant::now());
        if !self.commit_editors(){self.editors.unsaved=true;return;}
        if crate::activity::save_pending(&self.state,&mut self.store).is_err(){self.notice("Aktivitas belum tersimpan");}
        self.activities.saved_at=Some(Instant::now());
    }
    pub(super) fn has_activity_bar(&self)->bool {
        self.activities.runtime.snapshot.as_ref().is_some_and(|s|s.primary.priority<=2) && (self.toast.is_none()||self.settings.quiet_mode)
    }
    pub(super) fn reconcile_activities(&mut self){
        let now=self.activity_now();
        let old=self.activities.runtime.snapshot.clone();
        let old_pending=self.state.pending_local_reminders.clone();
        let old_backlog_budget=self.state.quiet_backlog_remaining_ms;
        self.state.collect_due_todos(now,self.settings.enabled("Reminder"));
        // Drop queued reminders whose toggle is now off (or the whole module is off),
        // so switching reminders off immediately stops any stale pill from showing.
        let reminders_on=self.settings.enabled("Reminder");
        self.state.pending_local_reminders.retain(|p|{
            if !reminders_on{return false;}
            match p.kind{
                ReminderKind::Water=>self.settings.water_reminder,
                ReminderKind::Move=>self.settings.move_reminder,
                ReminderKind::Eye=>self.settings.move_reminder&&self.settings.eye_reminder,
                ReminderKind::Focus=>self.settings.focus_enabled,
                ReminderKind::Stretch=>self.settings.focus_enabled&&self.settings.stretch_after_focus,
                ReminderKind::Prayer=>self.settings.prayer_enabled,
                ReminderKind::Todo=>true,
            }
        });
        if self.focus.running()&&self.activities.focus.id.is_none(){self.start_activity_run(false);}
        let mut inputs=ActivityInputs::default();
        if self.settings.enabled("Reminder"){
            self.activities.focus.focus_input(&mut inputs,&self.focus);
            if let (Some(end),Some(id))=(self.stretch_deadline,self.activities.stretch.id.clone()){
                inputs.countdown(id,self.activities.stretch.started_ms,end.saturating_duration_since(Instant::now()).as_millis() as u64,self.settings.stretch_seconds as u64*1000,true);
            }
        }
        inputs.media(&self.track,now);
        if let Some((candidate,end))=&self.activities.feedback{if *end>Instant::now(){inputs.candidates.push(candidate.clone());}else{self.activities.feedback=None;}}

        if let Some((vol,mute,at))=self.volume_hud{
            if self.activities.hud_at!=Some(at){self.activities.hud_at=Some(at);self.activities.hud_generation=self.activities.hud_generation.saturating_add(1);self.activities.hud_origin_ms=now-at.elapsed().as_millis().min(i64::MAX as u128) as i64;}
            inputs.hud(self.activities.hud_generation,self.activities.hud_origin_ms,(vol*100.).round() as u8,mute);
        }
        let policy=self.activity_policy();
        self.preset_host.backlog.reconcile(&mut self.state,&mut inputs,now,policy,self.activities.runtime.snapshot.as_ref());
        if self.hover_stage==0 && self.settings.enabled("Reminder") && !self.settings.quiet_mode && !self.fullscreen && !self.suspended && (self.state.pending_local_reminders.iter().any(|p|!p.deferred_quiet&&p.remaining_visible_ms>0)||self.activities.feedback.is_some()||self.state.quiet_backlog_remaining_ms>0){
            self.activities.auto_revealed=true;self.set_hover_stage(1);
        }
        let policy=self.activity_policy();
        let effects=self.activities.runtime.reconcile(&mut self.state,&inputs,now,policy);
        self.activities.inputs=inputs;
        if effects.sound && (!matches!(effects.first_delivery,Some(ActivityId::Prayer{..}))||self.settings.prayer_sound){unsafe{let _=MessageBeep(0x40);}}
        let changed=old.as_ref().map(|s|(&s.primary.id,s.revision))!=self.activities.runtime.snapshot.as_ref().map(|s|(&s.primary.id,s.revision));
        if changed{
            self.hits.retain(|h|!is_activity_action(&h.action));
            self.activities.discard_release|=reconcile_activity_actions(&mut self.focused,&mut self.pressed,self.activities.runtime.snapshot.as_ref());
            self.retarget_morph();self.layout();
        }
        let budgets_changed=old_pending!=self.state.pending_local_reminders||old_backlog_budget!=self.state.quiet_backlog_remaining_ms;
        if budgets_changed&&(effects.first_delivery.is_some()||old_pending.len()!=self.state.pending_local_reminders.len()||old_backlog_budget>0&&self.state.quiet_backlog_remaining_ms==0||self.activities.saved_at.is_none_or(|t|t.elapsed()>=Duration::from_secs(1))){self.persist_activities();}
        if self.activities.auto_revealed&&!self.has_activity_bar()&&self.volume_hud.is_none(){self.activities.auto_revealed=false;if self.hover_stage==1{self.set_hover_stage(0);}}
    }
    pub(super) fn activity_action(&mut self,id:ActivityId,revision:u64,operation:u8){
        // Reconcile again at dispatch: an expired hit must not open its former source.
        self.reconcile_activities();
        if !self.activities.runtime.is_current(&self.state,&self.activities.inputs,self.activity_now(),self.activity_policy(),&id,revision){return;}
        let target=self.activities.runtime.open(&self.state,&self.activities.inputs,self.activity_now(),self.activity_policy(),&id,revision);
        match operation{
            1=>{if id==ActivityId::Backlog{self.preset_host.backlog.dismiss_summary(&mut self.state);}else{self.state.dismiss_local_reminder(&id);}self.persist_activities();},
            2=>{if self.activities.runtime.confirm_water(&mut self.state,&id,self.settings.water_serving,Local::now().date_naive()){self.last_water=Instant::now();self.persist_activities();}},
            _=>{if let Some(target)=target{self.open_activity_target(target);}},
        }
        self.reconcile_activities();self.invalidate();
    }
    pub(super) fn open_activity_target(&mut self,target:ActivityTarget){
        if !self.flush_editors_barrier(){return;}self.privacy_detail_open=false;self.settings_popup=None;self.settings_open=false;self.mod_view=None;self.reminder_menu=None;
        match target{
            ActivityTarget::AudioSettings=>{self.settings_open=true;self.settings_cat=3;},
            ActivityTarget::Reminder(n)=>{self.tab="Reminder".into();self.reminder_index=n.min(3) as usize;},
            ActivityTarget::PendingReminders=>{self.open_backlog();return;},
            ActivityTarget::Todo(id)=>{
                match crate::todo_notes::resolve_activity_todo(&self.state,id,crate::todo_notes::NoteAccess::from_settings(&self.settings)){
                    Ok(crate::todo_notes::NoteTarget::TasksSubview{todo_id})=>{self.show_todo(todo_id);self.activate_on_explicit_open();return;},
                    Ok(crate::todo_notes::NoteTarget::ReminderFallback)=>{self.tab="Reminder".into();self.reminder_index=0;},_=>return,
                }
            },
            ActivityTarget::Music=>self.tab="Musik".into(),ActivityTarget::Calendar=>self.tab="Kalender".into(),ActivityTarget::Unavailable=>return,
        }
        self.focused=None;self.pressed=None;self.hits.clear();self.set_hover_stage(2);self.make_inputs();self.activate_on_explicit_open();
    }
    pub(super) fn draw_activity(&mut self,p:&mut Painter,y:f32,_expanded:bool){unsafe{
        let Some(s)=self.activities.runtime.snapshot.clone() else{return};
        let (icon,mut text)=activity_label(&s,&self.state,&self.track,&self.settings.language,self.settings.quiet_mode);
        if matches!(s.primary.id,ActivityId::Media(_))&&self.lyric_mode&&!self.settings.quiet_mode{text=self.active_lyric().unwrap_or(text);}
        let h=self.height;
        // Right cluster: clock + chevron, aligned with the idle/now-playing pill.
        let chev_x=self.width-24.;
        self.button_chevron_up(p,chev_x,y+(h-20.)*0.5,20.,20.,Action::ToStage0,0x88888e);
        let time_str=Local::now().format("%H:%M").to_string();
        let time_w=p.measure_text(&time_str,11.5,true);
        let time_x=chev_x-6.-time_w;
        p.text_centered(&time_str,time_x,y,time_w,h,11.5,0xd8d8de,1.,true);
        let mut right=time_x-8.;
        // Keep the status indicators visible while a reminder/activity is shown.
        if self.settings.quiet_mode||self.settings.power_saver_mode{right-=32.;self.preset_indicator(p,right,y+3.);}
        if self.settings.privacy_enabled&&self.privacy_snapshot.any_active(){
            let rail=crate::shell::privacy::compute_privacy_rail_width(&self.privacy_snapshot);right-=rail;self.draw_privacy_rail(p,right,y,rail,30.);right-=8.;
        }
        if let Some(b)=&self.battery{if b.has_battery{right-=38.;p.text_centered(&format!("{}%",b.percentage),right,y,36.,30.,9.,0xaaaaaa,1.,false);}}
        if let Some(t)=&self.tws{right-=46.;p.outline_icon("headphones",right,y+8.,14.,0xaaaaaa);p.text_centered(&t.battery.map(|b|format!("{b}%")).unwrap_or_else(||"—".into()),right+16.,y,30.,30.,9.,0xaaaaaa,1.,false);}
        let open=Action::OpenActivity(s.primary.id.clone(),s.revision);
        if s.primary.target.is_some(){right-=22.;self.icon_button(p,"right",right,y+2.,22.,26.,open.clone(),false);}
        if s.primary.priority==2{
            right-=22.;self.icon_button(p,"close",right,y+2.,22.,26.,Action::DismissActivity(s.primary.id.clone(),s.revision),false);
            if matches!(s.primary.id,ActivityId::Water(_)){right-=22.;self.icon_button(p,"check",right,y+2.,22.,26.,Action::ConfirmActivity(s.primary.id.clone(),s.revision),false);}
        }
        if s.other_count>0{right-=28.;self.button_h(p,&if s.other_count>9{"9+".into()}else{format!("+{}",s.other_count)},right,y+3.,28.,24.,open.clone(),false);}
        let text_x=34.;let text_w=(right-8.-text_x).max(10.);
        if matches!(s.primary.id,ActivityId::Media(_))&&self.settings.effective_presentation().visualizer{
            p.bars(&self.levels[..self.levels.len().min(4)],12.,y+9.,14.,12.,0xf4f4f6);
        }else{p.outline_icon(icon,12.,y+8.,14.,0xf4f4f6);}

        if matches!(s.primary.id,ActivityId::Media(_))&&!self.settings.effective_presentation().reduced_motion&&!self.settings.quiet_mode{
            p.text_scrolling(&text,text_x,y,text_w,h,11.,0xf4f4f6,false,self.media.as_ref().map(|m|m.position_seconds()).unwrap_or(0.),false);
        }else{p.text_ellipsis_vcenter(&text,text_x,y,text_w,h,11.,0xf4f4f6,false);}

        if self.hit_enabled&&s.primary.target.is_some(){self.hits.push(Hit{rect:render::rect(8.,y,(text_w+22.).max(1.),h),action:open.clone()});}
        if self.focused.as_ref()==Some(&open){p.stroke(8.,y+2.,(text_w+22.).max(1.),h-4.,8.,0xd8d8de,1.);}
        if let Some(progress)=s.primary.progress{p.fill_alpha(12.,y+h-2.,(self.width-24.)*progress,2.,1.,0xd8d8de,0.8);}
    }}
}
fn is_activity_action(a:&Action)->bool{matches!(a,Action::ActivityExpired|Action::OpenActivity(..)|Action::DismissActivity(..)|Action::ConfirmActivity(..))}
fn reconcile_activity_actions(focused:&mut Option<Action>,pressed:&mut Option<Action>,snapshot:Option<&ActivitySnapshot>)->bool{
    let discard=pressed.as_ref().is_some_and(is_activity_action);
    if discard{*pressed=None;}
    if focused.as_ref().is_some_and(is_activity_action){*focused=snapshot.map(|s|if s.primary.target.is_some(){Action::OpenActivity(s.primary.id.clone(),s.revision)}else{Action::ActivityExpired});}
    discard
}
fn activity_label(s:&ActivitySnapshot,state:&State,track:&MediaSnapshot,lang:&str,quiet:bool)->(&'static str,String){
    if quiet&&!matches!(s.primary.id,ActivityId::Hud(_)|ActivityId::Clock){return ("lock",if lang=="en"{"Activity active"}else{"Aktivitas aktif"}.into());}
    let en=lang=="en";
    match &s.primary.id{
        ActivityId::Clock=>("calendar",Local::now().format("%H:%M").to_string()),
        ActivityId::Hud(_)=>("volume",s.primary.text.clone()),
        ActivityId::Focus(_)|ActivityId::Stretch(_)=>{let focus=matches!(s.primary.id,ActivityId::Focus(_));(if focus{"timer"}else{"person"},if s.primary.priority==3{format!("{} {}",if focus{if en{"Focus"}else{"Fokus"}}else{if en{"Stretch"}else{"Peregangan"}},s.primary.text)}else{if en{"Session complete"}else{"Sesi selesai"}.into()})},
        ActivityId::Water(_)=>("droplet",if en{"Time to drink"}else{"Waktunya minum"}.into()),
        ActivityId::Move(_)|ActivityId::Eye(_)=>("person",if en{"Time for a break"}else{"Waktunya istirahat"}.into()),
        ActivityId::Prayer{slot,time,..}=>("moon",format!("{} · {} WIB",crate::i18n::prayer_name(*slot as usize,lang),time)),
        ActivityId::TodoDue{todo_id,..}=>("check",state.todos.iter().find(|t|t.id==*todo_id).map(|t|t.text.clone()).unwrap_or_else(||"Tugas tidak tersedia".into())),
        ActivityId::Media(_)=>("music",track.title.clone()),_ =>("timer",s.primary.text.clone()),
    }
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn activity_action_buttons_render_glyphs_not_icon_names_as_text(){
  let src=include_str!("activity_host.rs");
  assert!(!src.contains("button_icon(p,\"right\""));
  assert!(!src.contains("button_icon(p,\"close\""));
  assert!(!src.contains("button_icon(p,\"check\""));
  assert!(src.contains("icon_button(p,\"close\""));
 }
 #[test]fn host_countdown_complete_during_hud_is_pending_once(){
  let mut state=State::default();let mut a=Activities::default();let id=a.focus.start(&mut state,ReminderKind::Focus,0).unwrap();
  a.inputs.hud(1,0,50,false);a.runtime.reconcile(&mut state,&a.inputs,1,PresentationPolicy::default());
  assert!(a.focus.complete(&mut state,&id,2).unwrap());assert!(!a.focus.complete(&mut state,&id,3).unwrap());
  a.runtime.reconcile(&mut state,&a.inputs,4,PresentationPolicy::default());assert_eq!(state.pending_local_reminders.len(),1);
  a.runtime.reconcile(&mut state,&a.inputs,1800,PresentationPolicy::default());assert_eq!(a.runtime.snapshot.as_ref().unwrap().primary.id,id);
 }
 #[test]fn countdown_text_changes_do_not_invalidate_keyboard_identity(){
  let mut state=State::default();let mut a=Activities::default();a.inputs.countdown(ActivityId::Focus(1),0,60000,60000,true);
  a.runtime.reconcile(&mut state,&a.inputs,0,PresentationPolicy::default());let revision=a.runtime.snapshot.as_ref().unwrap().revision;
  a.inputs=ActivityInputs::default();a.inputs.countdown(ActivityId::Focus(1),0,59000,60000,true);a.runtime.reconcile(&mut state,&a.inputs,1000,PresentationPolicy::default());
  assert_eq!(a.runtime.snapshot.as_ref().unwrap().revision,revision);
 }
 #[test]fn changing_primary_cancels_mouse_release_and_moves_keyboard_target(){
  let mut state=State::default();state.enqueue_local_reminder(PendingReminder::new(ActivityId::Water(1),0).unwrap()).unwrap();let mut a=Activities::default();
  a.runtime.reconcile(&mut state,&a.inputs,0,PresentationPolicy::default());let old=a.runtime.snapshot.as_ref().unwrap().clone();
  let mut focused=Some(Action::OpenActivity(old.primary.id.clone(),old.revision));let mut pressed=focused.clone();
  state.dismiss_local_reminder(&old.primary.id);a.runtime.reconcile(&mut state,&a.inputs,1,PresentationPolicy::default());
  assert!(reconcile_activity_actions(&mut focused,&mut pressed,a.runtime.snapshot.as_ref()));assert!(pressed.is_none());
  assert!(matches!(focused,Some(Action::OpenActivity(ActivityId::Clock,_))));
  assert!(!a.runtime.is_current(&state,&a.inputs,1,PresentationPolicy::default(),&old.primary.id,old.revision));
 }
 #[test]fn generic_feedback_never_turns_enter_into_a_media_shortcut(){
  let snapshot=ActivitySnapshot{primary:Candidate{id:ActivityId::Feedback(1),priority:1,created_ms:0,expires_ms:Some(1000),target:None,text:"Saved".into(),progress:None},other_count:0,revision:2,animate:false};
  let mut focused=Some(Action::OpenActivity(ActivityId::Water(1),1));let mut pressed=None;
  assert!(!reconcile_activity_actions(&mut focused,&mut pressed,Some(&snapshot)));assert!(matches!(focused,Some(Action::ActivityExpired)));
 }
 #[test]fn hidden_host_freezes_budget_across_suspend(){
  let mut state=State::default();state.enqueue_local_reminder(PendingReminder::new(ActivityId::Water(1),0).unwrap()).unwrap();let mut a=Activities::default();
  a.runtime.reconcile(&mut state,&a.inputs,0,PresentationPolicy::default());a.runtime.pause_presentation();a.runtime.reconcile(&mut state,&a.inputs,600000,PresentationPolicy::default());assert_eq!(state.pending_local_reminders[0].remaining_visible_ms,12000);
 }
 #[test]fn presenter_redacts_todo_and_clamps_long_text_with_stable_identity(){
  let mut state=State::default();state.todos.push(Todo{id:7,text:"私密🦀".repeat(300),done:false,due:None,reminded:false,note_id:None});
  let s=ActivitySnapshot{primary:Candidate{id:ActivityId::TodoDue{todo_id:7,due_ms:0},priority:2,created_ms:0,expires_ms:None,target:Some(ActivityTarget::Todo(7)),text:String::new(),progress:None},other_count:12,revision:1,animate:false};
  assert_eq!(activity_label(&s,&state,&MediaSnapshot::default(),"id",true).1,"Aktivitas aktif");assert_eq!(activity_label(&s,&state,&MediaSnapshot::default(),"id",false).1,state.todos[0].text);
 }
}
