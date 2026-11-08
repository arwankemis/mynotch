use super::*;
const ICONS:[&str;4]=["timer","droplet","person","moon"];
const COLORS:[u32;4]=[0xffad78,0x69d2e5,0x92cfac,0xe8c87f];
const WHITE:u32=0xf3f3f5;
pub(super) fn jakarta_today()->String{jakarta_now().format("%Y-%m-%d").to_string()}
fn jakarta_now()->chrono::DateTime<chrono::FixedOffset>{chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(7*3600).unwrap())}
fn ml_label(n:u32)->String{if n>=1000{format!("{}.{:03}",n/1000,n%1000)}else{n.to_string()}}
fn due_key(now:chrono::DateTime<chrono::FixedOffset>,time:&str,lead:u32)->Option<String>{
    let time=chrono::NaiveTime::parse_from_str(time,"%H:%M").ok()?;use chrono::TimeZone;
    for day in [now.date_naive(),now.date_naive()+chrono::Duration::days(1)]{
        let at=now.timezone().from_local_datetime(&day.and_time(time)).single()? - chrono::Duration::minutes(lead as i64);
        if (0..60).contains(&now.signed_duration_since(at).num_seconds()){return Some(format!("{day} {time}"))}
    }None
}
/// Effective period in seconds for a minutes-based setting of the given reminder key.
pub(super) fn period_secs(key:&str,minutes:u32)->u64{let _=key;minutes as u64*60}
/// Effective stretch countdown in seconds.
pub(super) fn stretch_secs(seconds:u32)->u64{seconds as u64}
fn minutes_label(minutes:u32,en:bool)->String{if en{format!("{minutes} min")}else{format!("{minutes} menit")}}
fn menu_values(key:&str, lang: &str)->Vec<(u32,String)>{
    let en = lang == "en";
    let (values,unit):(&[u32],&str)=match key{
        "focus"=>(&[15,25,50],if en { "min" } else { "menit" }),
        "break"=>(&[5,10,15],if en { "min" } else { "menit" }),
        "water_interval"|"move_interval"=>(&[30,45,60,90],if en { "min" } else { "menit" }),
        "target"=>(&[1500,2000,2500,3000],"ml"),
        "serving"=>(&[150,250,350,500],"ml"),
        "stretch_duration"=>(&[60,120,300],"durasi"),
        "lead"=>(&[0,5,10],"lead"),
        "city"=>(&[0,1,2],"kota"),
        _=>(&[],"")
    };
    values.iter().map(|&v|(v,match unit{
        "kota"=>["Jakarta","Bandung","Surabaya"][v as usize].into(),
        "lead"=>if v==0{
            if en { "On time".into() } else { "Tepat waktu".into() }
        } else {
            if en { format!("{v} min before") } else { format!("{v} menit sebelum") }
        },
        "durasi"=>if en { format!("{} min", v/60) } else { format!("{} menit", v/60) },
        "ml"=>format!("{} ml",ml_label(v)),
        _=>format!("{v} {unit}")
    })).collect()
}
impl Shell{
    pub(super) fn select_reminder(&mut self,index:usize){
        let index=index.min(3);if index==self.reminder_index{return}
        let from=self.geometry();let current=self.reminder_position();self.reminder_index=index;self.reminder_menu=None;self.reminder_menu_scroll=0;
        if self.hover_stage == 1 && self.should_revert_stage1_to_calendar() {
            self.trigger_stage1_temporary(5);
        }
        self.reminder_anim=if self.settings.effective_presentation().reduced_motion{None}else{Some((current,Instant::now()))};self.focused=None;
        if (from.1-self.target_geometry().1).abs()>0.1{self.retarget_morph();self.layout();self.update_window_region();}
        self.make_inputs();self.start_morph_timer();if index==3{self.refresh_prayers(false);}self.invalidate();
    }
    fn reminder_position(&self)->f32{match self.reminder_anim{Some((from,t))=>{let x=(t.elapsed().as_secs_f32()/0.18).min(1.);from+(self.reminder_index as f32-from)*(1.-(1.-x).powi(3))},None=>self.reminder_index as f32}}
    fn reminder_enabled(&self)->bool{match self.reminder_index{0=>self.settings.focus_enabled,1=>self.settings.water_reminder,2=>self.settings.move_reminder,_=>self.settings.prayer_enabled}}
    pub(super) fn open_reminder_menu(&mut self,key:&'static str){
        if self.reminder_menu==Some(key){self.reminder_menu=None;self.reminder_menu_scroll=0;return}
        let current=match key{"focus"=>self.settings.focus_minutes,"break"=>self.settings.break_minutes,"water_interval"=>self.settings.water_interval,"target"=>self.settings.water_target,"serving"=>self.settings.water_serving,"move_interval"=>self.settings.move_interval,"stretch_duration"=>self.settings.stretch_seconds,"lead"=>self.settings.prayer_lead,"city"=>["Jakarta","Bandung","Surabaya"].iter().position(|c|*c==self.settings.prayer_city).unwrap_or(0)as u32,_=>0};
        self.reminder_menu_cursor=menu_values(key, &self.settings.language).iter().position(|(v,_)|*v==current).unwrap_or(0);self.reminder_menu_scroll=0;self.reminder_menu=Some(key);
    }
    pub(super) fn handle_reminder_menu_key(&mut self,key:u16)->bool{
        let Some(menu)=self.reminder_menu else{return false};let values=menu_values(menu, &self.settings.language);let end=values.len().saturating_sub(1);
        match key{0x26=>self.reminder_menu_cursor=self.reminder_menu_cursor.saturating_sub(1),0x28=>self.reminder_menu_cursor=(self.reminder_menu_cursor+1).min(end),0x21=>self.reminder_menu_cursor=self.reminder_menu_cursor.saturating_sub(3),0x22=>self.reminder_menu_cursor=(self.reminder_menu_cursor+3).min(end),0x24=>self.reminder_menu_cursor=0,0x23=>self.reminder_menu_cursor=end,
            0x0d|0x20=>{if let Some((v,_))=values.get(self.reminder_menu_cursor){self.action(Action::ReminderValue(menu,*v));}},0x1b=>{self.reminder_menu=None;self.reminder_menu_scroll=0;},0x09=>{self.reminder_menu=None;self.reminder_menu_scroll=0;self.invalidate();return false},_=>return false}
        self.invalidate();true
    }
    pub(super) fn apply_reminder_value(&mut self,key:&str,value:u32){
        if !menu_values(key, &self.settings.language).iter().any(|(v,_)|*v==value){return}
        match key{
            "focus"=>self.settings.focus_minutes=value,"break"=>self.settings.break_minutes=value,
            "water_interval"=>{self.settings.water_interval=value;self.last_water=Instant::now();},
            "target"=>self.settings.water_target=value,"serving"=>self.settings.water_serving=value,
            "move_interval"=>{self.settings.move_interval=value;self.last_move=Instant::now();},
            "stretch_duration"=>self.settings.stretch_seconds=value,"lead"=>self.settings.prayer_lead=value,
            "city"=>{self.settings.prayer_city=["Jakarta","Bandung","Surabaya"][value as usize].into();self.prayer_schedule=None;self.prayer_attempt=None;self.cancel_job("Reminder");self.refresh_prayers(true);},_=>{}
        }
    }
    pub(super) fn refresh_prayers(&mut self,force:bool){
        if !self.settings.enabled("Reminder")||self.suspended||self.fullscreen||self.jobs.contains_key("Reminder"){return}
        let city=self.settings.prayer_city.clone();let date=jakarta_today();let key=format!("{city}:{date}");
        if !force{
            if self.prayer_schedule.as_ref().is_some_and(|s|s.city==city&&s.date==date){return}
            if self.prayer_attempt.as_ref().is_some_and(|(k,t)|k==&key&&t.elapsed()<Duration::from_secs(300)){return}
        }
        self.prayer_attempt=Some((key,Instant::now()));self.prayer_error=None;
        let api=self.online.clone();self.job("Reminder",move||{let result=api.prayers(&city,&date);Event::Prayers(city,date,result)});
    }
    pub(super) fn tick_reminder(&mut self){
        if self.stretch_deadline.is_some_and(|d|Instant::now()>=d){self.stretch_deadline=None;self.last_move=Instant::now();self.complete_activity_run(true);}
        if !self.settings.enabled("Reminder"){return}
        if self.settings.prayer_enabled||(self.tab=="Reminder"&&self.reminder_index==3){self.refresh_prayers(false);}
        if !self.settings.prayer_enabled{return}
        let now=jakarta_now();let Some(schedule)=self.prayer_schedule.as_ref().filter(|s|s.date==jakarta_today()&&s.city==self.settings.prayer_city)else{return};
        let mut occurrences=vec![];
        for (i,time) in schedule.times.iter().enumerate(){
            if let Some(key)=due_key(now,time,self.settings.prayer_lead){
                let latch=format!("{} {key}",schedule.city);
                if !self.state.prayer_delivered.contains(&latch){occurrences.push(crate::activity::ActivityId::Prayer{city:schedule.city.clone(),date:schedule.date.clone(),slot:i as u8,time:time.clone(),lead:self.settings.prayer_lead,latch});}
            }
        }
        for id in occurrences{self.enqueue_activity(id);}
    }
    fn rw_hit(&mut self,x:f32,y:f32,w:f32,h:f32,action:Action,enabled:bool){if self.hit_enabled&&enabled{self.hits.push(Hit{rect:render::rect(x,y,w,h),action});}}
    unsafe fn rw_button(&mut self,p:&mut Painter,label:&str,x:f32,y:f32,w:f32,action:Action,primary:bool,enabled:bool){unsafe{
        let hover=enabled&&self.pointer.is_some_and(|(a,b)|a>=x&&a<=x+w&&b>=y&&b<=y+28.);
        let bg=if primary{COLORS[self.reminder_index]}else{if hover{0x2b2b34}else{0x1e1e25}};
        if !enabled{p.push_alpha(0.3);}
        p.fill(x,y,w,28.,7.,bg);
        if primary&&hover{p.fill_alpha(x,y,w,28.,7.,0xffffff,0.12);}
        p.text_centered(label,x,y,w,28.,11.,if primary{0x101018}else if hover{WHITE}else{0x92929e},1.,false);
        if self.focused.as_ref()==Some(&action){p.stroke(x-1.,y-1.,w+2.,30.,8.,COLORS[self.reminder_index],1.2);}
        self.rw_hit(x,y,w,28.,action,enabled);if !enabled{p.pop_alpha();}
    }}
    unsafe fn rw_icon_button(&mut self,p:&mut Painter,icon:&str,x:f32,y:f32,action:Action,enabled:bool){unsafe{
        let hover=enabled&&self.pointer.is_some_and(|(a,b)|a>=x&&a<=x+26.&&b>=y&&b<=y+28.);
        if !enabled{p.push_alpha(0.3);}p.fill(x,y,26.,28.,7.,if hover{0x2b2b34}else{0x1e1e25});p.outline_icon(icon,x+6.5,y+7.,13.,if hover{WHITE}else{0x92929e});
        if self.focused.as_ref()==Some(&action){p.stroke(x-1.,y-1.,28.,30.,8.,COLORS[self.reminder_index],1.2);}
        if !enabled{p.pop_alpha();}self.rw_hit(x,y,26.,28.,action,enabled);
    }}
    unsafe fn rw_select(&mut self,p:&mut Painter,key:&'static str,label:&str,value:&str,x:f32,y:f32,w:f32){unsafe{
        let action=Action::ReminderMenu(key);
        let hover=self.pointer.is_some_and(|(a,b)|a>=x&&a<=x+w&&b>=y&&b<=y+30.);
        if !label.is_empty(){p.text(label,x,y-21.,w,15.,10.,0x93939f,false);}
        p.fill(x,y,w,30.,7.,if hover{0x22222a}else{0x18181e});p.stroke(x,y,w,30.,7.,if hover{0x454552}else{0x303039},1.);
        p.text_ellipsis(value,x+8.,y+7.,w-29.,18.,11.,0xdddde5,false);p.outline_icon("down",x+w-20.,y+8.,13.,0xa2a2ad);
        if self.focused.as_ref()==Some(&action){p.stroke(x-1.,y-1.,w+2.,32.,8.,COLORS[self.reminder_index],1.2);}
        self.rw_hit(x,y,w,30.,action,true);
    }}
    unsafe fn rw_check(&mut self,p:&mut Painter,label:&str,x:f32,y:f32,checked:bool,action:Action,enabled:bool){unsafe{
        let hover=enabled&&self.pointer.is_some_and(|(a,b)|a>=x&&a<=x+290.&&b>=y&&b<=y+20.);
        p.fill(x,y+2.,12.,12.,2.,if checked{COLORS[self.reminder_index]}else if hover{0x25252e}else{0x18181e});if checked{p.outline_icon("check",x+1.,y+3.,10.,0x101018);}else{p.stroke(x,y+2.,12.,12.,2.,if hover{0x757585}else{0x50505b},1.);}
        p.text(label,x+19.,y,280.,18.,10.,if hover{WHITE}else{0x9d9da9},false);
        if self.focused.as_ref()==Some(&action){p.stroke(x-3.,y-1.,296.,20.,5.,COLORS[self.reminder_index],1.);}
        self.rw_hit(x,y,290.,20.,action,enabled);
    }}
    pub(super) fn reminder_page(&mut self,p:&mut Painter){unsafe{
        let lang = self.settings.language.clone();
        if self.width<440.{
            // The page layout needs 440 px; a narrow notch used to render a blank page
            // with no explanation. Say so instead of silently skipping.
            p.text(crate::i18n::t("reminder.narrow", &lang),18.,150.,(self.width-36.).max(50.),40.,11.,0x8a9291,false);
            return
        }
        let accent=COLORS[self.reminder_index];let enabled=self.reminder_enabled();
        // Exact 170 / 20 / 324 column proportions from the accepted HTML mockup.
        let picker_shift=if self.reminder_index==3{8.}else{0.};
        p.fill(18.,158.+picker_shift,170.,56.,11.,0x1c1d22);
        let pos=self.reminder_position();let first_hit=self.hits.len();p.clip(18.,74.+picker_shift,170.,224.);
        let water:u32=self.state.water.iter().filter(|d|d.date==Local::now().date_naive()).map(|d|d.ml).sum();
        let summaries=[
            crate::i18n::focus_summary(self.settings.focus_minutes, &lang),
            format!("{} / {} ml",ml_label(water),ml_label(self.settings.water_target)),
            crate::i18n::move_summary(self.settings.move_interval, &lang),
            crate::i18n::t("reminder.five_times", &lang).into()
        ];
        for i in 0..4{
            let distance=(i as f32-pos).abs();let y=158.+picker_shift+(i as f32-pos)*56.;if y+56.<74.+picker_shift||y>298.+picker_shift{continue}
            let selected=i==self.reminder_index;
            let action=Action::ReminderSelect(i);
            let hover=self.pointer.is_some_and(|(a,b)|a>=18.&&a<=188.&&b>=y.max(74.+picker_shift)&&b<=(y+56.).min(298.+picker_shift));
            let fade=(1.-distance*0.5).clamp(0.04,1.);p.push_alpha(if selected{1.}else if hover{0.8}else{fade.min(0.52)});
            if !selected&&hover{p.fill(18.,y,170.,56.,11.,0x141519);}
            p.outline_icon(ICONS[i],32.,y+18.,20.,if selected{accent}else{0x8a8a95});
            p.text(crate::i18n::reminder_name(i, &lang),64.,y+10.,120.,20.,13.,if selected{0xf4f4f6}else{0x8a8a95},selected);
            p.text(&summaries[i],64.,y+32.,120.,14.,10.,if selected{0xa2a2ac}else{0x7d7d88},false);
            if self.focused.as_ref()==Some(&action){p.stroke(19.,y+1.,168.,54.,10.,accent,1.);}
            self.rw_hit(18.,y,170.,56.,action,true);p.pop_alpha();
        }
        for hit in &mut self.hits[first_hit..]{hit.rect.top=hit.rect.top.max(74.+picker_shift);hit.rect.bottom=hit.rect.bottom.min(298.+picker_shift);}
        p.unclip();
        for (icon,y,delta,ok) in [("up",52.+picker_shift,-1,self.reminder_index>0),("down",298.+picker_shift,1,self.reminder_index<3)]{
            let hover=ok&&self.pointer.is_some_and(|(a,b)|a>=18.&&a<=188.&&b>=y&&b<=y+22.);
            p.outline_icon(icon,96.,y+4.,14.,if !ok{0x282830}else if hover{WHITE}else{0x5d5d69});
            self.rw_hit(18.,y,170.,22.,Action::ReminderStep(delta),ok);
        }
        p.fill(208.,52.,1.,if self.reminder_index==3{284.}else{268.},0.,0x26262c);
        p.text(crate::i18n::reminder_name(self.reminder_index, &lang),229.,54.,240.,25.,17.,WHITE,false);
        let toggle=match self.reminder_index{0=>"focus_enabled",1=>"water_reminder",2=>"move_reminder",_=>"prayer_enabled"};
        let toggle_action=Action::Toggle(toggle);
        let toggle_hover=self.pointer.is_some_and(|(a,b)|a>=496.&&a<=536.&&b>=49.&&b<=81.);
        p.fill(504.,57.,28.,16.,8.,if enabled{accent}else if toggle_hover{0x454550}else{0x35353d});p.fill(if enabled{518.}else{506.},59.,12.,12.,6.,if enabled{0x101015}else{0xaaaab3});
        if self.focused.as_ref()==Some(&toggle_action){p.stroke(500.,53.,36.,24.,10.,accent,1.);}
        self.rw_hit(496.,49.,40.,32.,toggle_action,true);
        match self.reminder_index{
            0=>{
                if !enabled{p.push_alpha(0.35);}
                let total=if self.focus.is_break{self.settings.break_minutes}else{self.settings.focus_minutes};let remaining=self.focus.remaining().as_secs_f64();let shown=if remaining>0.{remaining}else{total as f64*60.};
                let ratio=if remaining>0.{shown/self.focus.total().as_secs_f64().max(1.)}else{1.};p.progress_ring(281.,137.,44.7,ratio as f32,accent);
                p.text_centered(&time_label(shown),235.,116.,92.,31.,25.,WHITE,1.,false);
                p.text_centered(crate::i18n::t("reminder.remaining", &lang),239.,148.,84.,15.,9.,0x8d8d99,1.,false);
                let status_text = if self.focus.running(){
                    if self.focus.is_break{crate::i18n::t("reminder.break_time", &lang)}else{crate::i18n::t("reminder.focusing", &lang)}
                }else if remaining>0.{
                    crate::i18n::t("reminder.paused", &lang)
                }else{
                    crate::i18n::t("reminder.ready", &lang)
                };
                p.text(status_text,346.,101.,186.,20.,12.,0xdddde5,false);
                p.text(crate::i18n::t("reminder.focus_tip", &lang),346.,126.,186.,16.,10.,0x83838e,false);
                let action=if remaining==0.{if self.focus.is_break{Action::FocusBreak}else{Action::FocusStart}}else{Action::FocusPause};
                let btn_lbl = if self.focus.running(){
                    crate::i18n::t("reminder.pause", &lang)
                }else if remaining>0.{
                    crate::i18n::t("reminder.resume", &lang)
                }else{
                    crate::i18n::t("reminder.start", &lang)
                };
                self.rw_button(p,btn_lbl,346.,148.,62.,action,true,enabled);
                self.rw_icon_button(p,"reset",414.,148.,Action::FocusReset,enabled);
                let focus_dur_val = minutes_label(self.settings.focus_minutes, lang == "en");
                let break_dur_val = minutes_label(self.settings.break_minutes, lang == "en");
                self.rw_select(p,"focus",crate::i18n::t("reminder.focus_duration", &lang),&focus_dur_val,229.,224.,146.);
                self.rw_select(p,"break",crate::i18n::t("reminder.break_duration", &lang),&break_dur_val,386.,224.,146.);
                self.rw_check(p,crate::i18n::t("reminder.auto_break", &lang),229.,277.,self.settings.auto_break,Action::Toggle("auto_break"),enabled);
                if !enabled{p.pop_alpha();}
            }
            1=>{
                p.progress_ring(281.,137.,44.7,water as f32/self.settings.water_target as f32,accent);
                p.text_centered(&ml_label(water),235.,116.,92.,31.,25.,WHITE,1.,false);
                p.text_centered(crate::i18n::t("reminder.ml_today", &lang),239.,148.,84.,15.,9.,0x8d8d99,1.,false);
                let percent=(water as f32/self.settings.water_target as f32*100.).round()as u32;
                let percent_txt = format!("{}% {}", percent, crate::i18n::t("reminder.of_target", &lang));
                p.text(&percent_txt,346.,101.,186.,20.,12.,0xdddde5,false);
                let target_desc = if lang == "en" {
                    format!("Target {} ml / day", ml_label(self.settings.water_target))
                } else {
                    format!("Target {} ml / hari", ml_label(self.settings.water_target))
                };
                p.text(&target_desc,346.,126.,186.,16.,10.,0x83838e,false);
                self.rw_button(p,&format!("+ {} ml",self.settings.water_serving),346.,148.,82.,Action::Water,true,enabled);self.rw_icon_button(p,"undo",434.,148.,Action::UndoWater,water>0);
                let interval_val = minutes_label(self.settings.water_interval, lang == "en");
                self.rw_select(p,"water_interval",crate::i18n::t("reminder.remind_every", &lang),&interval_val,229.,224.,146.);
                self.rw_select(p,"target",crate::i18n::t("reminder.daily_target", &lang),&format!("{} ml",ml_label(self.settings.water_target)),386.,224.,146.);
                p.fill(229.,265.,303.,1.,0.,0x24242a);
                p.text(crate::i18n::t("reminder.cup_size", &lang),229.,282.,200.,18.,10.,0x9494a0,false);
                self.rw_select(p,"serving","",&format!("{} ml",self.settings.water_serving),446.,276.,86.);
            }
            2=>{
                if let Some(deadline)=self.stretch_deadline{
                    let rem=deadline.saturating_duration_since(Instant::now()).as_secs_f32();
                    let ratio=(rem/(stretch_secs(self.settings.stretch_seconds) as f32).max(1.)).clamp(0.,1.);
                    p.progress_ring(281.,137.,44.7,ratio,accent);
                    p.text_centered(&time_label(rem as f64),235.,116.,92.,31.,24.,WHITE,1.,false);
                    p.text_centered(crate::i18n::t("reminder.stretching", &lang),239.,148.,84.,15.,9.,0x8d8d99,1.,false);
                    p.text(crate::i18n::t("reminder.stretching_now", &lang),346.,101.,186.,20.,12.,0xdddde5,false);
                    p.text(crate::i18n::t("reminder.stretch_tip", &lang),346.,126.,186.,16.,10.,0x83838e,false);
                    self.rw_button(p,crate::i18n::t("reminder.stretch_done", &lang),346.,148.,136.,Action::StretchDone,true,enabled);
                }else{
                    let ratio=1.-(self.last_move.elapsed().as_secs_f32()/(period_secs("move_interval",self.settings.move_interval) as f32)).min(1.);
                    p.progress_ring(281.,137.,44.7,ratio,accent);p.outline_icon("person",264.5,112.,33.,accent);
                    p.text_centered(crate::i18n::t("reminder.move_break", &lang),239.,151.,84.,15.,9.,0x8d8d99,1.,false);
                    p.text(crate::i18n::t("reminder.stand_up", &lang),346.,101.,186.,20.,12.,0xdddde5,false);
                    let move_desc = crate::i18n::move_desc(self.settings.stretch_seconds, self.settings.move_interval, &lang);
                    p.text(&move_desc,346.,126.,186.,16.,10.,0x83838e,false);
                    self.rw_button(p,crate::i18n::t("reminder.start_break", &lang),346.,148.,82.,Action::StretchStart,true,enabled);
                    self.rw_button(p,crate::i18n::t("reminder.done", &lang),434.,148.,64.,Action::StretchDone,false,enabled);
                }
                let move_interval_val = minutes_label(self.settings.move_interval, lang == "en");
                self.rw_select(p,"move_interval",crate::i18n::t("reminder.remind_every", &lang),&move_interval_val,229.,224.,146.);
                let stretch_val = if self.settings.stretch_seconds < 60 {
                    if lang == "en" { format!("{} sec", self.settings.stretch_seconds) } else { format!("{} detik", self.settings.stretch_seconds) }
                } else {
                    if lang == "en" { format!("{} min", self.settings.stretch_seconds / 60) } else { format!("{} menit", self.settings.stretch_seconds / 60) }
                };
                self.rw_select(p,"stretch_duration",crate::i18n::t("reminder.break_duration_label", &lang),&stretch_val,386.,224.,146.);
                self.rw_check(p,crate::i18n::t("reminder.stretch_after_focus", &lang),229.,277.,self.settings.stretch_after_focus,Action::Toggle("stretch_after_focus"),true);
            }
            _=>{
                p.fill(229.,104.,47.,47.,23.5,0x29251b);p.outline_icon("moon",240.5,115.5,24.,accent);
                let today=jakarta_today();let schedule=self.prayer_schedule.as_ref().filter(|s|s.city==self.settings.prayer_city&&s.date==today);
                let current=jakarta_now().format("%H:%M").to_string();let next=schedule.and_then(|s|s.times.iter().enumerate().find(|(_,t)|*t>&current));
                let title = next.map(|(i,_)| format!("{} · {}", crate::i18n::t("reminder.next", &lang), crate::i18n::prayer_name(i, &lang))).unwrap_or_else(|| crate::i18n::t("reminder.prayer_title", &lang).into());
                p.text(&title,287.,104.,245.,20.,12.,0xdddde5,false);

                let subtitle = if let Some((_,time))=next{
                    format!("{} WIB · {}",time,self.settings.prayer_city)
                }else if schedule.is_some(){
                    format!("{} · {}", crate::i18n::t("reminder.today_schedule", &lang), self.settings.prayer_city)
                }else if self.jobs.contains_key("Reminder"){
                    crate::i18n::t("reminder.loading_prayers", &lang).into()
                }else{
                    crate::i18n::t("reminder.no_schedule", &lang).into()
                };
                p.text(&subtitle,287.,128.,245.,17.,10.,0x90909d,false);

                let times=schedule.map(|s|s.times.clone());
                self.rw_select(p,"city",crate::i18n::t("reminder.city", &lang),&self.settings.prayer_city.clone(),229.,191.,146.);
                let lead_val = if self.settings.prayer_lead == 0 {
                    crate::i18n::t("reminder.on_time", &lang).into()
                } else {
                    format!("{} {}", self.settings.prayer_lead, crate::i18n::t("reminder.min_before", &lang))
                };
                self.rw_select(p,"lead",crate::i18n::t("reminder.prayer_alert", &lang),&lead_val,386.,191.,146.);
                p.fill(229.,232.,303.,1.,0.,0x25252c);p.fill(229.,287.,303.,1.,0.,0x25252c);
                for i in 0..5{
                    let x=229.+i as f32*60.6;
                    let p_name = crate::i18n::prayer_name(i, &lang);
                    p.text_centered(p_name,x,243.,60.6,14.,9.,0x93939f,1.,false);
                    p.text_centered(times.as_ref().map(|t|t[i].as_str()).unwrap_or("—"),x,262.,60.6,20.,13.,if times.is_some(){0xdddde5}else{0x737380},1.,false);
                }
                self.rw_check(p,crate::i18n::t("reminder.sound_alert", &lang),229.,303.,self.settings.prayer_sound,Action::Toggle("prayer_sound"),true);
                let refresh_hover=self.pointer.is_some_and(|(a,b)|a>=229.&&a<=532.&&b>=322.&&b<=341.);
                let note = if self.prayer_error.is_some(){
                    crate::i18n::t("reminder.load_failed", &lang)
                }else{
                    "myQuran · Kemenag · WIB"
                };
                p.text(note,229.,326.,303.,14.,9.,if refresh_hover{WHITE}else{0x737380},false);
                if self.focused.as_ref()==Some(&Action::PrayerRefresh){p.stroke(227.,322.,305.,19.,5.,accent,1.);}
                self.rw_hit(229.,322.,303.,19.,Action::PrayerRefresh,true);
            }
        }
        if let Some(key)=self.reminder_menu{self.reminder_dropdown(p,key);}
    }}
    fn reminder_dropdown(&mut self,p:&mut Painter,key:&'static str){unsafe{
        let lang = self.settings.language.clone();
        let (x,y,w)=match key{"break"|"target"|"stretch_duration"=>(386.,224.,146.),"serving"=>(446.,276.,86.),"lead"=>(386.,191.,146.),"city"=>(229.,191.,146.),_=>(229.,224.,146.)};
        let items=menu_values(key, &lang);
        let layout=DropdownLayout::compute(x,y,w,items.len(),self.reminder_menu_cursor,self.reminder_menu_scroll,self.height);
        self.reminder_menu_scroll=layout.scroll;
        p.fill(layout.x-3.,layout.top-3.,layout.width+6.,layout.height+6.,10.,0x09090c);
        p.fill(layout.x,layout.top,layout.width,layout.height,7.,0x1c1c23);
        p.stroke(layout.x,layout.top,layout.width,layout.height,7.,0x3a3a45,1.);
        p.clip(layout.x,layout.top+2.,layout.width,layout.height-4.);
        let has_scrollbar=items.len()>layout.visible_count;
        let text_w=layout.width-if has_scrollbar{24.}else{16.};
        for (row_idx,(v,label)) in items.iter().skip(layout.scroll).take(layout.visible_count).enumerate(){
            let item_idx=layout.scroll+row_idx;
            let ry=layout.top+4.+row_idx as f32*DropdownLayout::ITEM_HEIGHT;
            let hover=self.pointer.is_some_and(|(a,b)|a>=layout.x&&a<=layout.x+layout.width&&b>=ry&&b<=ry+29.&&b>=layout.top&&b<=layout.top+layout.height);
            if hover||item_idx==self.reminder_menu_cursor{p.fill(layout.x+3.,ry,layout.width-6.,28.,4.,0x30303b);}
            p.text(label.as_str(),layout.x+8.,ry+6.,text_w,18.,10.5,WHITE,false);
            if let Some(rect)=layout.item_rect(item_idx){
                self.rw_hit(rect.left,rect.top,rect.right-rect.left,rect.bottom-rect.top,Action::ReminderValue(key,*v),true);
            }
        }
        p.unclip();
        if has_scrollbar{
            let track_x=layout.x+layout.width-7.;
            let track_y=layout.top+4.;
            let track_h=layout.height-8.;
            let thumb_h=(track_h*layout.visible_count as f32/items.len() as f32).max(16.);
            let max_scroll=(items.len()-layout.visible_count)as f32;
            let thumb_y=track_y+(track_h-thumb_h)*(layout.scroll as f32/max_scroll.max(1.));
            p.fill(track_x,track_y,3.,track_h,1.5,0x26262c);
            p.fill(track_x,thumb_y,3.,thumb_h,1.5,0x5a5a66);
        }
    }}
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct DropdownLayout {
    pub x: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub visible_count: usize,
    pub scroll: usize,
    pub opens_upward: bool,
}

#[allow(dead_code)]
impl DropdownLayout {
    pub const ITEM_HEIGHT: f32 = 29.0;
    pub const PADDING_V: f32 = 8.0;
    pub const TRIGGER_GAP: f32 = 4.0;
    pub const TRIGGER_HEIGHT: f32 = 30.0;
    pub const VIEWPORT_MARGIN_TOP: f32 = 8.0;
    pub const VIEWPORT_MARGIN_BOTTOM: f32 = 8.0;

    pub fn compute(
        x: f32,
        y: f32,
        width: f32,
        total_items: usize,
        cursor: usize,
        preferred_scroll: usize,
        viewport_height: f32,
    ) -> Self {
        if total_items == 0 {
            return Self {
                x,
                top: y + Self::TRIGGER_HEIGHT + Self::TRIGGER_GAP,
                width,
                height: Self::PADDING_V,
                visible_count: 0,
                scroll: 0,
                opens_upward: false,
            };
        }

        let full_height = total_items as f32 * Self::ITEM_HEIGHT + Self::PADDING_V;
        let min_y = Self::VIEWPORT_MARGIN_TOP;
        let max_y = (viewport_height - Self::VIEWPORT_MARGIN_BOTTOM).max(min_y + Self::ITEM_HEIGHT + Self::PADDING_V);

        let down_top = y + Self::TRIGGER_HEIGHT + Self::TRIGGER_GAP;
        let space_down = (max_y - down_top).max(0.0);

        let up_bottom = y - Self::TRIGGER_GAP;
        let space_up = (up_bottom - min_y).max(0.0);

        // Direction decision:
        // 1. Fits downwards completely -> open downwards.
        // 2. Fits upwards completely -> open upwards.
        // 3. Otherwise, pick direction with larger available space.
        let (opens_upward, available_space, base_top) = if space_down >= full_height {
            (false, space_down, down_top)
        } else if space_up >= full_height {
            (true, space_up, up_bottom)
        } else if space_up >= space_down {
            (true, space_up, up_bottom)
        } else {
            (false, space_down, down_top)
        };

        let (top, height, visible_count) = if available_space >= Self::ITEM_HEIGHT + Self::PADDING_V {
            let max_visible = ((available_space - Self::PADDING_V) / Self::ITEM_HEIGHT).floor() as usize;
            let visible = max_visible.clamp(1, total_items);
            let h = visible as f32 * Self::ITEM_HEIGHT + Self::PADDING_V;
            let t = if opens_upward {
                (base_top - h).max(min_y)
            } else {
                base_top
            };
            (t, h, visible)
        } else {
            let max_viewport_space = (max_y - min_y).max(Self::ITEM_HEIGHT + Self::PADDING_V);
            let max_visible = ((max_viewport_space - Self::PADDING_V) / Self::ITEM_HEIGHT).floor() as usize;
            let visible = max_visible.clamp(1, total_items);
            let h = visible as f32 * Self::ITEM_HEIGHT + Self::PADDING_V;
            let t = if opens_upward {
                (base_top - h).clamp(min_y, max_y - h)
            } else {
                base_top.clamp(min_y, max_y - h)
            };
            (t, h, visible)
        };

        let max_scroll = total_items.saturating_sub(visible_count);
        let mut scroll = preferred_scroll.min(max_scroll);
        if cursor < scroll {
            scroll = cursor;
        } else if cursor >= scroll + visible_count {
            scroll = (cursor + 1 - visible_count).min(max_scroll);
        }

        Self {
            x,
            top,
            width,
            height,
            visible_count,
            scroll,
            opens_upward,
        }
    }

    pub fn item_rect(&self, item_index: usize) -> Option<render::D2D_RECT_F> {
        if item_index < self.scroll || item_index >= self.scroll + self.visible_count {
            return None;
        }
        let rel_idx = item_index - self.scroll;
        let ry = self.top + 4.0 + rel_idx as f32 * Self::ITEM_HEIGHT;
        let hit_top = ry.max(self.top);
        let hit_bottom = (ry + Self::ITEM_HEIGHT).min(self.top + self.height);
        if hit_bottom <= hit_top {
            return None;
        }
        Some(render::rect(self.x + 2.0, hit_top, self.width - 4.0, hit_bottom - hit_top))
    }

    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.width && py >= self.top && py <= self.top + self.height
    }

    pub fn is_within_viewport(&self, viewport_h: f32) -> bool {
        self.top >= Self::VIEWPORT_MARGIN_TOP
            && self.top + self.height <= (viewport_h - Self::VIEWPORT_MARGIN_BOTTOM) + 0.001
    }

    pub fn physical_bounds(&self, scale: f32) -> (f32, f32, f32, f32) {
        (self.x * scale, self.top * scale, self.width * scale, self.height * scale)
    }
}

#[cfg(test)]mod tests{
    #[test]
    fn test_reminder_dropdown_clamping_short_downward() {
        // city at y=191 with 3 items fits downwards in prayer tab (viewport 362)
        let layout = DropdownLayout::compute(229.0, 191.0, 146.0, 3, 0, 0, 362.0);
        assert!(!layout.opens_upward, "short list below y=191 should open downwards");
        assert_eq!(layout.top, 191.0 + 30.0 + 4.0);
        assert_eq!(layout.visible_count, 3);
        assert_eq!(layout.scroll, 0);
        assert!(layout.is_within_viewport(362.0));
        assert!(layout.item_rect(0).is_some());
        assert!(layout.item_rect(2).is_some());
        assert!(layout.item_rect(3).is_none());
    }

    #[test]
    fn test_reminder_dropdown_clamping_short_upward() {
        // serving at y=276 with 4 items does not fit downwards in viewport 346, opens upwards
        let layout = DropdownLayout::compute(446.0, 276.0, 86.0, 4, 0, 0, 346.0);
        assert!(layout.opens_upward, "serving at y=276 must open upwards");
        assert_eq!(layout.visible_count, 4);
        assert_eq!(layout.scroll, 0);
        assert_eq!(layout.top, 276.0 - 4.0 - layout.height);
        assert!(layout.is_within_viewport(346.0));
        assert!(layout.top >= DropdownLayout::VIEWPORT_MARGIN_TOP);
        assert!(layout.top + layout.height <= 346.0 - DropdownLayout::VIEWPORT_MARGIN_BOTTOM);
    }

    #[test]
    fn test_reminder_dropdown_clamping_long_list_and_viewport_boundaries() {
        // water_interval at y=224 with 20 items (height would be 20*29+8 = 588px)
        let layout = DropdownLayout::compute(229.0, 224.0, 146.0, 20, 0, 0, 346.0);
        assert!(layout.is_within_viewport(346.0), "must not exceed viewport");
        assert!(layout.top >= DropdownLayout::VIEWPORT_MARGIN_TOP);
        assert!(layout.top + layout.height <= 346.0 - DropdownLayout::VIEWPORT_MARGIN_BOTTOM);
        assert!(layout.visible_count < 20, "must be clamped to visible count");
        assert!(layout.visible_count >= 5, "must show at least available items");
        // Verify no hit rect escapes viewport
        for i in 0..20 {
            if let Some(rect) = layout.item_rect(i) {
                assert!(rect.top >= layout.top);
                assert!(rect.bottom <= layout.top + layout.height);
                assert!(rect.top >= DropdownLayout::VIEWPORT_MARGIN_TOP);
                assert!(rect.bottom <= 346.0 - DropdownLayout::VIEWPORT_MARGIN_BOTTOM);
            }
        }
    }

    #[test]
    fn test_reminder_dropdown_scroll_reaches_first_and_last_option() {
        let total = 15;
        // At cursor 0, first item must be visible
        let layout_first = DropdownLayout::compute(229.0, 224.0, 146.0, total, 0, 0, 346.0);
        assert_eq!(layout_first.scroll, 0);
        let first_rect = layout_first.item_rect(0);
        assert!(first_rect.is_some(), "first item must have hit rect when cursor=0");
        assert!(layout_first.item_rect(total - 1).is_none(), "last item not visible initially");

        // At cursor total - 1 (End key), last item must be visible
        let layout_last = DropdownLayout::compute(229.0, 224.0, 146.0, total, total - 1, 0, 346.0);
        let last_rect = layout_last.item_rect(total - 1);
        assert!(last_rect.is_some(), "last item must have hit rect when scrolled to end");
        assert!(layout_last.scroll > 0, "scroll offset must advance");
        assert_eq!(layout_last.scroll + layout_last.visible_count, total);
    }

    #[test]
    fn test_reminder_dropdown_hit_targets_strictly_contained() {
        let layout = DropdownLayout::compute(386.0, 224.0, 146.0, 12, 5, 0, 346.0);
        let mut hit_count = 0;
        for i in 0..12 {
            if let Some(rect) = layout.item_rect(i) {
                hit_count += 1;
                assert!(rect.left >= layout.x);
                assert!(rect.right <= layout.x + layout.width);
                assert!(rect.top >= layout.top);
                assert!(rect.bottom <= layout.top + layout.height);
            }
        }
        assert_eq!(hit_count, layout.visible_count);
    }

    #[test]
    fn test_reminder_dropdown_dpi_invariance() {
        let layout = DropdownLayout::compute(229.0, 191.0, 146.0, 10, 2, 0, 362.0);
        assert!(layout.is_within_viewport(362.0));

        // Test DPI 100%, 150%, 200%
        for scale in [1.0f32, 1.5f32, 2.0f32] {
            let (px, py, pw, ph) = layout.physical_bounds(scale);
            let phys_viewport_h = 362.0 * scale;
            assert!(py >= DropdownLayout::VIEWPORT_MARGIN_TOP * scale);
            assert!(py + ph <= (phys_viewport_h - DropdownLayout::VIEWPORT_MARGIN_BOTTOM * scale) + 0.01);
            assert_eq!(px, 229.0 * scale);
            assert_eq!(pw, 146.0 * scale);

            // Cursor conversion roundtrip
            let phys_cursor_y = (layout.top + 10.0) * scale;
            let logical_cursor_y = phys_cursor_y / scale;
            assert!(layout.contains_point(layout.x + 10.0, logical_cursor_y));
        }
    }

    #[test]
    fn test_reminder_dropdown_click_outside_points() {
        let layout = DropdownLayout::compute(229.0, 191.0, 146.0, 3, 0, 0, 362.0);
        // Points clearly outside
        assert!(!layout.contains_point(layout.x - 10.0, layout.top));
        assert!(!layout.contains_point(layout.x + layout.width + 10.0, layout.top));
        assert!(!layout.contains_point(layout.x + 10.0, layout.top - 5.0));
        assert!(!layout.contains_point(layout.x + 10.0, layout.top + layout.height + 5.0));
        // Point inside
        assert!(layout.contains_point(layout.x + 10.0, layout.top + 10.0));
    }

    #[test]
    fn test_reminder_dropdown_clamping_tiny_constrained_viewport() {
        // Small/tight viewport of 80px where neither full downwards nor upwards fits
        let layout = DropdownLayout::compute(229.0, 40.0, 146.0, 5, 0, 0, 80.0);
        assert!(layout.is_within_viewport(80.0), "tiny viewport must clamp within boundaries");
        assert!(layout.top >= DropdownLayout::VIEWPORT_MARGIN_TOP);
        assert!(layout.top + layout.height <= 80.0 - DropdownLayout::VIEWPORT_MARGIN_BOTTOM + 0.01);
        assert!(layout.visible_count >= 1, "must show at least 1 item");
        if let Some(rect) = layout.item_rect(0) {
            assert!(rect.top >= layout.top);
            assert!(rect.bottom <= layout.top + layout.height);
        }
    }

    use super::*;
    #[test]fn prayer_window_uses_wib_and_handles_midnight(){use chrono::TimeZone;let tz=chrono::FixedOffset::east_opt(7*3600).unwrap();let now=tz.with_ymd_and_hms(2026,10,26,23,56,0).single().unwrap();assert!(due_key(now,"00:01",5).unwrap().starts_with("2026-10-27"));assert!(due_key(now,"23:56",0).is_some());assert!(due_key(now,"23:54",0).is_none());assert!(due_key(now,"23:57",0).is_none());}
}

#[cfg(test)]mod reminder_period_tests{
 use super::*;
 #[test]fn periods_are_plain_minutes_and_menus_have_no_quick_options(){
  assert_eq!(period_secs("water_interval",45),2700);
  assert_eq!(period_secs("focus",25),1500);
  assert_eq!(stretch_secs(60),60);
  // Pilihan "10 detik"/"1 menit" dihapus dari semua dropdown interval.
  for key in ["focus","break","water_interval","move_interval","stretch_duration"]{
    assert!(menu_values(key,"id").iter().all(|(v,l)|*v!=0&&!l.contains("detik")&&!l.contains("sec")));
  }
 }
}
