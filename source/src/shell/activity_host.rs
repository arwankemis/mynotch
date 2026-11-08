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
        if crate::activity::save_pending(&self.state,&mut self.store).is_err(){
            self.editors.unsaved=true;
        }
        self.activities.saved_at=Some(Instant::now());
    }
    /// Compact Stage 1 width for the activity pill: sized to its content instead of a fixed
    /// 360/440 DIP slab, so short countdowns ("Stretch 00:56") stay a tight capsule.
    pub(super) fn activity_target_width(&self)->f32 {
        let Some(s)=self.activities.runtime.snapshot.as_ref() else{return ACTIVITY_PILL_MIN};
        let (_,text)=activity_label(s,&self.state,&self.track,&self.settings.language,self.settings.quiet_mode);
        let actions=if s.primary.priority==2{if matches!(s.primary.id,ActivityId::Water(_)){2}else{1}}else{0};
        let mut status=0.;
        if self.settings.quiet_mode||self.settings.power_saver_mode{status+=32.;}
        if self.settings.privacy_enabled&&self.privacy_snapshot.any_active(){status+=crate::shell::privacy::compute_privacy_rail_width(&self.privacy_snapshot)+8.;}
        if self.battery.as_ref().is_some_and(|b|b.has_battery){status+=38.;}
        if self.tws.is_some(){status+=46.;}
        let chip=self.activity_accent_chip(s);
        activity_pill_width(estimate_activity_text_w(&text),actions,s.other_count>0,status,chip)
    }
    pub(super) fn has_activity_bar(&self)->bool {
        self.activities.runtime.snapshot.as_ref().is_some_and(|s|s.primary.priority<=2) && (self.toast.is_none()||self.settings.quiet_mode)
    }
    pub(super) fn reconcile_activities(&mut self){
        let now=self.activity_now();
        let old=self.activities.runtime.snapshot.clone();
        let old_pending=self.state.pending_local_reminders.clone();
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
                inputs.countdown(id,self.activities.stretch.started_ms,end.saturating_duration_since(Instant::now()).as_millis() as u64,super::reminder::stretch_secs(self.settings.stretch_seconds)*1000,true);
            }
        }
        inputs.media(&self.track,now);
        if let Some((candidate,end))=&self.activities.feedback{if *end>Instant::now(){inputs.candidates.push(candidate.clone());}else{self.activities.feedback=None;}}

        if let Some((vol,mute,at))=self.volume_hud{
            if self.activities.hud_at!=Some(at){self.activities.hud_at=Some(at);self.activities.hud_generation=self.activities.hud_generation.saturating_add(1);self.activities.hud_origin_ms=now-at.elapsed().as_millis().min(i64::MAX as u128) as i64;}
            inputs.hud(self.activities.hud_generation,self.activities.hud_origin_ms,(vol*100.).round() as u8,mute);
        }
        let policy=self.activity_policy();
        if self.hover_stage==0 && self.settings.enabled("Reminder") && !self.settings.quiet_mode && !self.fullscreen && !self.suspended && self.state.pending_local_reminders.iter().any(|p|p.remaining_visible_ms>0){
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
        let membership_changed=old_pending.len()!=self.state.pending_local_reminders.len();
        if effects.first_delivery.is_some()||membership_changed{self.persist_activities();}
        if self.activities.auto_revealed&&!self.has_activity_bar()&&self.volume_hud.is_none(){self.activities.auto_revealed=false;if self.hover_stage==1{self.set_hover_stage(0);}}
    }
    pub(super) fn activity_action(&mut self,id:ActivityId,revision:u64,operation:u8){
        // Reconcile again at dispatch: an expired hit must not open its former source.
        self.reconcile_activities();
        if !self.activities.runtime.is_current(&self.state,&self.activities.inputs,self.activity_now(),self.activity_policy(),&id,revision){return;}
        let target=self.activities.runtime.open(&self.state,&self.activities.inputs,self.activity_now(),self.activity_policy(),&id,revision);
        match operation{
            1=>{self.state.dismiss_local_reminder(&id);self.persist_activities();},
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
    pub(super) fn draw_activity(&mut self,p:&mut Painter,y:f32,expanded:bool){unsafe{
        let Some(s)=self.activities.runtime.snapshot.clone() else{return};
        let (icon,mut text)=activity_label(&s,&self.state,&self.track,&self.settings.language,self.settings.quiet_mode);
        if matches!(s.primary.id,ActivityId::Media(_))&&self.lyric_mode&&!self.settings.quiet_mode{text=self.active_lyric().unwrap_or(text);}
        let h = 30.0;
        // F2: aksen reminder Stage 1. Netral = tampilan lama tanpa warna; Halus = aksen
        // chip/ring + gradasi statis; Nyala = Halus + pita aksen yang menyapu kiri ke
        // kanan (3 dtk, easing mockup) + tint tombol aksi. Aksen hanya untuk jenis
        // reminder, bukan Media/HUD/dll, dan tidak tampil di mode tenang (mockup:
        // kartu quiet "tanpa aksen").
        let mode=if self.settings.quiet_mode{AccentMode::Netral}else{self.accent_mode()};
        let accent=if mode!=AccentMode::Netral{reminder_accent_rgb(&s).map(|a|accent_for_theme(a,self.ui_light()))}else{None};
        if let Some(acc)=accent.filter(|_|!expanded){
            // F2 sweep: perbarui anchor siklus SEBELUM menggambar supaya frame pertama
            // pill sudah memakai posisi pita berjalan. paint() menandai activity_sweep_live
            // = false sebelum draw() dan mengosongkan anchor setelahnya bila frame ini
            // tidak menggambar aksen, sehingga timer morph mati saat pill hilang.
            if self.accent_sweep_allowed(mode){
                self.activity_sweep=match self.activity_sweep.take(){
                    Some((anchor,prev)) if prev==acc=>Some((anchor,acc)),
                    _=>Some((Instant::now(),acc)),
                };
                self.activity_sweep_live=true;
            }else{
                self.activity_sweep=None;
            }
            self.draw_accent_background(p,acc,mode);
        }
        // Right cluster: clock + chevron, aligned with the idle/now-playing pill.
        let chev_x=self.width-24.;
        self.button_chevron_up(p,chev_x,y+(h-20.)*0.5,20.,20.,Action::ToStage0,0x88888e);
        let time_str=Local::now().format("%H:%M").to_string();
        let time_w=p.measure_text(&time_str,11.5,true);
        let time_x=chev_x-6.-time_w;
        p.text_centered(&time_str,time_x,y,time_w,h,11.5,0xd8d8de,1.,true);
        let mut right=time_x-6.;
        // Keep the status indicators visible while a reminder/activity is shown.
        if self.settings.quiet_mode||self.settings.power_saver_mode{right-=32.;self.preset_indicator(p,right,y+3.);}
        if self.settings.privacy_enabled&&self.privacy_snapshot.any_active(){
            let rail=crate::shell::privacy::compute_privacy_rail_width(&self.privacy_snapshot);right-=rail;self.draw_privacy_rail(p,right,y,rail,30.);right-=8.;
        }
        if let Some(b)=&self.battery{if b.has_battery{right-=38.;p.text_centered(&format!("{}%",b.percentage),right,y,36.,30.,9.,0xaaaaaa,1.,false);}}
        if let Some(t)=&self.tws{right-=46.;p.outline_icon("headphones",right,y+8.,14.,0xaaaaaa);p.text_centered(&t.battery.map(|b|format!("{b}%")).unwrap_or_else(||"--".into()),right+16.,y,30.,30.,9.,0xaaaaaa,1.,false);}

        // Vertical separator between status indicators and content/actions
        right-=6.;
        p.text_centered("|",right,y,6.,h,11.,0x50505a,0.85,false);
        right-=6.;

        // Action buttons for actionable reminders (priority == 2)
        if s.primary.priority==2{
            // Nyala: tint latar tombol aksi (mockup .abtn.tint).
            if let (Some(acc),false)=(accent.filter(|_|matches!(mode,AccentMode::Nyala)),expanded){
                p.fill_alpha(right-24.,y+3.,22.,24.,8.,acc,0.16);
                if matches!(s.primary.id,ActivityId::Water(_)){p.fill_alpha(right-48.,y+3.,22.,24.,8.,acc,0.16);}
            }
            right-=24.;
            self.icon_button(p,"close",right,y+3.,22.,24.,Action::DismissActivity(s.primary.id.clone(),s.revision),false);
            if matches!(s.primary.id,ActivityId::Water(_)){
                right-=24.;
                self.icon_button(p,"check",right,y+3.,22.,24.,Action::ConfirmActivity(s.primary.id.clone(),s.revision),false);
            }
        }
        let open=Action::OpenActivity(s.primary.id.clone(),s.revision);
        if s.other_count>0{
            right-=28.;
            self.button_h(p,&if s.other_count>9{"9+".into()}else{format!("+{}",s.other_count)},right,y+3.,26.,24.,open.clone(),false);
        }

        // Ikon: Netral memakai slot lama 14px; aksen memakai chip 26px dengan disc + ring
        // progres gaya gauge HW (Air: progres minum; countdown: sisa waktu).
        let text_x=match accent.filter(|_|!expanded){
            Some(acc)=>{
                let chip_x=10.;let chip_y=y+2.;
                p.fill_alpha(chip_x+5.,chip_y+5.,16.,16.,6.,acc,0.20);
                if let Some(ratio)=self.accent_progress(&s){
                    let track=mix_rgb(acc,self.palette.background,0.72);
                    p.ring_arc(chip_x+13.,chip_y+13.,12.,ratio,track,acc,2.);
                }
                p.outline_icon(icon,chip_x+7.,chip_y+7.,12.,acc);
                44.
            }
            None=>{
                if matches!(s.primary.id,ActivityId::Media(_))&&self.settings.effective_presentation().visualizer{
                    p.bars(&self.levels[..self.levels.len().min(4)],12.,y+9.,14.,12.,0xf4f4f6);
                }else{
                    p.outline_icon(icon,12.,y+8.,14.,0xf4f4f6);
                }
                34.
            }
        };
        let text_w=(right-6.-text_x).max(10.);

        if matches!(s.primary.id,ActivityId::Media(_))&&!self.settings.effective_presentation().reduced_motion&&!self.settings.quiet_mode{
            p.text_scrolling(&text,text_x,y,text_w,h,11.,0xf4f4f6,false,self.media.as_ref().map(|m|m.position_seconds()).unwrap_or(0.),false);
        }else{
            p.text_ellipsis_vcenter(&text,text_x,y,text_w,h,11.,0xf4f4f6,false);
        }

        if self.hit_enabled&&s.primary.target.is_some(){
            self.hits.push(Hit{rect:render::rect(8.,y,(text_w+22.).max(1.),h),action:open.clone()});
        }
        if self.focused.as_ref()==Some(&open){
            p.stroke(8.,y+2.,(text_w+22.).max(1.),h-4.,8.,0xd8d8de,1.);
        }
    }}
    pub(super) fn accent_mode(&self)->AccentMode{match self.settings.reminder_accent.as_str(){"halus"=>AccentMode::Halus,"nyala"=>AccentMode::Nyala,_=>AccentMode::Netral}}
    /// True saat pill memakai chip aksen 26px (lebar slot ikon 44 bukan 34).
    /// True saat pill memakai chip aksen 26px (lebar slot ikon 44 bukan 34).
    /// Mode tenang tidak beraksen, jadi lebar pill ikut kembali ke slot 34.
    pub(super) fn activity_accent_chip(&self,s:&ActivitySnapshot)->bool{
        !self.settings.quiet_mode&&self.accent_mode()!=AccentMode::Netral&&reminder_accent_rgb(s).is_some()
    }
    /// Rasio ring chip aksen: Air = progres minum hari ini; countdown = sisa waktu.
    fn accent_progress(&self,s:&ActivitySnapshot)->Option<f32>{
        match &s.primary.id{
            ActivityId::Water(_)=>{
                let today=Local::now().date_naive();
                let ml:u32=self.state.water.iter().filter(|d|d.date==today).map(|d|d.ml).sum();
                Some((ml as f32/(self.settings.water_target as f32).max(1.)).clamp(0.,1.))
            }
            ActivityId::Focus(_)|ActivityId::Stretch(_)if s.primary.priority==3=>{
                s.primary.progress.map(|v|(1.-v.clamp(0.,1.)).clamp(0.,1.))
            }
            _=>None,
        }
    }
    /// Gradasi latar pill aksen (mockup .int-halus/.int-nyala .notch-body::before),
    /// diisi DI DALAM kontur notch yang persis lewat fill_notch_gradient_h.
    /// Halus: satu lapisan statis 26% -> 12% -> transparan pada 60% lebar.
    /// Nyala: lapisan dasar lebih lembut + pita aksen (100deg) yang menyapu kiri ke
    /// kanan tiap 3 dtk dengan easing cubic-bezier(.55,.1,.45,.9) — persis mockup;
    /// gerak minimal memakai pita statis, Hemat daya hanya lapisan dasar.
    fn draw_accent_background(&self,p:&mut Painter,acc:u32,mode:AccentMode){unsafe{
        // frame_contour diisi paint() sebelum draw(): kontur yang sedang dirender.
        let Some(c)=self.frame_contour else{return};
        match mode{
            AccentMode::Netral=>{}
            AccentMode::Halus=>{
                p.fill_notch_gradient_h(c.width,c.height,c.ear,c.radius,acc,&[(0.,0.26),(0.30,0.12),(0.60,0.)]);
            }
            AccentMode::Nyala=>{
                p.fill_notch_gradient_h(c.width,c.height,c.ear,c.radius,acc,&[(0.,0.22),(0.30,0.10),(0.60,0.)]);
                if let Some(cx)=self.accent_sweep_center(c.width){
                    let half=ui::ACCENT_SWEEP_HALF*c.width;
                    p.fill_notch_band_gradient_h(c.width,c.height,c.ear,c.radius,acc,cx-half,cx+half,&ACCENT_BAND_STOPS);
                }else if !self.settings.power_saver_mode{
                    // Gerak minimal (mockup prefers-reduced-motion): pita tetap ada, tanpa animasi.
                    p.fill_notch_gradient_h(c.width,c.height,c.ear,c.radius,acc,&ACCENT_BAND_STATIC_STOPS);
                }
            }
        }
    }}
    /// Sweep hanya untuk mode Nyala; Hemat daya & gerak minimal memakai gradasi
    /// statis tanpa timer per frame (filosofi ui.rs: tidak ada render loop permanen).
    /// Saat shell tersembunyi (fullscreen) sweep juga berhenti agar tidak menggambar
    /// tanpa terlihat; pill kembali menyapu setelah keluar fullscreen.
    fn accent_sweep_allowed(&self,mode:AccentMode)->bool{
        matches!(mode,AccentMode::Nyala)
            &&!(self.fullscreen&&self.settings.hide_fullscreen)
            &&!self.settings.power_saver_mode
            &&!self.settings.effective_presentation().reduced_motion
            &&!self.preview
            &&self.screenshot_stage2_freeze.is_none()
            &&self.screenshot_hide_until.is_none()
    }
    /// Pusat pita sweep (DIP) untuk lebar pill `width`, atau None saat siklus tidak
    /// berjalan. Amplitudo & easing di ui.rs — turunan langsung CSS mockup.
    fn accent_sweep_center(&self,width:f32)->Option<f32>{
        let (anchor,_acc)=self.activity_sweep.as_ref()?;
        let t=(anchor.elapsed().as_millis() as f32/ui::ACCENT_SWEEP_PERIOD_MS as f32).min(1.);
        Some(ui::lerp(ui::ACCENT_SWEEP_CENTER_FROM*width,ui::ACCENT_SWEEP_CENTER_TO*width,ui::ease_accent_sweep(t)))
    }
}
/// Intensitas aksen pill Stage 1 (settings.reminder_accent: "netral"/"halus"/"nyala").
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
pub(super) enum AccentMode{Netral,Halus,Nyala}
/// Warna aksen per jenis reminder (mockup: Air #38bdf8, Sholat #a78bfa,
/// Gerak/Peregangan #34d399, Fokus #f98a47, sesi selesai #34d399).
/// None = bukan reminder (Media/HUD/Jam/dll) sehingga tampil netral.
fn reminder_accent_rgb(s:&ActivitySnapshot)->Option<u32>{match &s.primary.id{
    ActivityId::Water(_)=>Some(0x38bdf8),
    ActivityId::Prayer{..}=>Some(0xa78bfa),
    ActivityId::Move(_)|ActivityId::Eye(_)|ActivityId::Stretch(_)=>Some(0x34d399),
    ActivityId::Focus(_)=>Some(if s.primary.priority==3{0xf98a47}else{0x34d399}),
    _=>None,
}}
/// Tema terang menurunkan saturasi aksen (mockup --acc: 60% aksen + #1b1e24).
fn accent_for_theme(a:u32,light:bool)->u32{if light{mix_rgb(a,0x1b1e24,0.4)}else{a}}
/// Pita sweep Nyala — profil alpha dalam rentang pita rapat (0..1 pita), diambil
/// dari gradasi 100deg mockup yang menempati 24%..68% dari lebar gambar 2.4w:
/// 0 -> 12% -> 30% (puncak) -> 10% -> 0.
pub(super) const ACCENT_BAND_STOPS:[(f32,f32);5]=[(0.,0.),(0.273,0.12),(0.5,0.30),(0.773,0.10),(1.,0.)];
/// Pita statis untuk gerak minimal: profil 100deg mockup pada posisi awalnya
/// (24%..68% lebar pill) — sama dengan aproksimasi statis sebelum animasi ada.
pub(super) const ACCENT_BAND_STATIC_STOPS:[(f32,f32);5]=[(0.24,0.),(0.36,0.12),(0.46,0.30),(0.58,0.10),(0.68,0.)];
/// Campur dua warna RGB; t = porsi b.
fn mix_rgb(a:u32,b:u32,t:f32)->u32{
    let t=t.clamp(0.,1.);
    let ch=|x:u32,y:u32|((x as f32)*(1.-t)+(y as f32)*t).round().clamp(0.,255.)as u32;
    (ch((a>>16)&0xff,(b>>16)&0xff)<<16)|(ch((a>>8)&0xff,(b>>8)&0xff)<<8)|ch(a&0xff,b&0xff)
}
const ACTIVITY_PILL_MIN:f32=190.;
const ACTIVITY_PILL_MAX:f32=440.;
/// Conservative width estimate for 11 DIP regular text (no painter available during layout).
fn estimate_activity_text_w(text:&str)->f32{text.chars().count() as f32*6.6+4.}
/// Mirrors the `draw_activity` layout: icon slot (34 netral / 44 chip aksen), text,
/// separator, actions, badge, status indicators, clock and chevron.
fn activity_pill_width(text_w:f32,actions:usize,badge:bool,status_w:f32,chip:bool)->f32{
    let fixed=(if chip{44.}else{34.})+6.+12.+6.+36.+6.+24.;
    let w=fixed+text_w+actions as f32*24.+if badge{28.}else{0.}+status_w;
    w.clamp(ACTIVITY_PILL_MIN,ACTIVITY_PILL_MAX).round()
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
        ActivityId::Move(_)=>("person",if en{"Time for a break"}else{"Waktunya istirahat"}.into()),
        ActivityId::Eye(_)=>("eye",if en{"Rest your eyes"}else{"Istirahatkan mata"}.into()),
        ActivityId::Prayer{slot,time,..}=>("moon",format!("{} \u{00B7} {} WIB",crate::i18n::prayer_name(*slot as usize,lang),time)),
        ActivityId::TodoDue{todo_id,..}=>("check",state.todos.iter().find(|t|t.id==*todo_id).map(|t|t.text.clone()).unwrap_or_else(||if en{"Task unavailable"}else{"Tugas tidak tersedia"}.into())),
        ActivityId::Media(_)=>("music",track.title.clone()),
        ActivityId::Feedback(_) => {
            let lower = s.primary.text.to_lowercase();
            let icon = if lower.contains("cuaca") || lower.contains("weather") {
                "cloud"
            } else if lower.contains("simpan") || lower.contains("save") || lower.contains("catatan") || lower.contains("note") {
                "edit"
            } else if lower.contains("audio") || lower.contains("volume") || lower.contains("suara") {
                "volume"
            } else {
                "bell"
            };
            (icon, s.primary.text.clone())
        },
        _ => ("bell", s.primary.text.clone()),
    }
}


#[cfg(test)]mod tests{
 use super::*;
 #[test]fn activity_pill_is_compact_for_countdowns_and_grows_for_actions(){
  let stretch=activity_pill_width(estimate_activity_text_w("Stretch 00:56"),0,false,0.,false);
  assert!(stretch<=240.,"countdown pill too long: {stretch}");
  let water=activity_pill_width(estimate_activity_text_w("Waktunya minum"),2,false,0.,false);
  assert!(water>stretch);
  assert_eq!(activity_pill_width(2000.,2,true,200.,true),ACTIVITY_PILL_MAX);
  assert_eq!(activity_pill_width(0.,0,false,0.,false),ACTIVITY_PILL_MIN);
 }
 #[test]fn accent_chip_widens_the_icon_slot_by_ten(){
  let text=estimate_activity_text_w("Waktunya minum");
  assert_eq!(activity_pill_width(text,2,false,0.,true)-activity_pill_width(text,2,false,0.,false),10.);
 }
 #[test]fn reminder_accent_follows_the_mockup_palette(){
  let snap=|id:ActivityId,priority:u8|ActivitySnapshot{primary:Candidate{id,priority,created_ms:0,expires_ms:None,target:None,text:String::new(),progress:None},other_count:0,revision:1,animate:false};
  assert_eq!(reminder_accent_rgb(&snap(ActivityId::Water(1),2)),Some(0x38bdf8));
  assert_eq!(reminder_accent_rgb(&snap(ActivityId::Prayer{city:"Jakarta".into(),date:"2026-11-08".into(),slot:0,time:"04:30".into(),lead:5,latch:"l".into()},1)),Some(0xa78bfa));
  assert_eq!(reminder_accent_rgb(&snap(ActivityId::Move(1),2)),Some(0x34d399));
  assert_eq!(reminder_accent_rgb(&snap(ActivityId::Eye(1),2)),Some(0x34d399));
  assert_eq!(reminder_accent_rgb(&snap(ActivityId::Focus(1),3)),Some(0xf98a47));
  assert_eq!(reminder_accent_rgb(&snap(ActivityId::Focus(1),2)),Some(0x34d399));
  assert_eq!(reminder_accent_rgb(&snap(ActivityId::Stretch(1),3)),Some(0x34d399));
  assert_eq!(reminder_accent_rgb(&snap(ActivityId::Media(1),4)),None);
  assert_eq!(reminder_accent_rgb(&snap(ActivityId::Clock,5)),None);
 }
 #[test]fn accent_mixing_and_band_profile_stay_in_range(){
  assert_eq!(mix_rgb(0x38bdf8,0x000000,0.),0x38bdf8);
  assert_eq!(mix_rgb(0x38bdf8,0x000000,1.),0);
  let a=mix_rgb(0x38bdf8,0x1b1e24,0.4);
  assert_eq!(a,((0x38 as f32*0.6+0x1b as f32*0.4).round()as u32)<<16|((0xbd as f32*0.6+0x1e as f32*0.4).round()as u32)<<8|(0xf8 as f32*0.6+0x24 as f32*0.4).round()as u32);
  // Pita sweep (rentang rapat) & pita statis: selalu dalam 0..=0.30, naik lalu
  // turun, dan menutup di alpha 0 di kedua ujung.
  for stops in [&ACCENT_BAND_STOPS,&ACCENT_BAND_STATIC_STOPS]{
    for (i,&(pos,alpha)) in stops.iter().enumerate(){
      assert!((0. ..=1.).contains(&pos)&&alpha>=0.&&alpha<=0.30,"stop {i} = ({pos},{alpha})");
      if i>0{assert!(pos>stops[i-1].0,"posisi stop tidak monoton");}
    }
    assert_eq!(stops[0].1,0.);
    assert_eq!(stops[4].1,0.);
  }
 }
 #[test]fn weather_feedback_activity_uses_cloud_icon(){
  let s=ActivitySnapshot{primary:Candidate{id:ActivityId::Feedback(1),priority:1,created_ms:0,expires_ms:Some(1000),target:None,text:"Memperbarui data cuaca…".into(),progress:None},other_count:0,revision:1,animate:false};
  assert_eq!(activity_label(&s,&State::default(),&MediaSnapshot::default(),"id",false).0,"cloud");
 }
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
