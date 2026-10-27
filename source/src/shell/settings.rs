use super::*;
const FG:u32=0xf3f3f5;const MUTED:u32=0x92929d;const ACCENT:u32=0x69d2e5;
const CATEGORIES:[(&str,&str,&str);5]=[("Umum","settings","mynotch 0.0.1 alpha · Portable Windows."),("Tampilan","monitor","Posisi dan kenyamanan tampilan."),("Tab & navigasi","layout","Atur urutan; sisakan satu tab aktif."),("Musik","music","Pemutar, lirik, dan visualisasi audio."),("Reminder","bell","Buka pengaturan pengingat harian.")];
#[derive(Clone)]pub(super) struct SettingsPopup{pub key:&'static str,pub x:f32,pub y:f32,pub width:f32,pub cursor:usize}
pub(super) fn prayer_lead_summary(city:&str,lead:u32)->String{if lead==0{format!("{city} · Tepat waktu")}else{format!("{city} · {lead} menit sebelumnya")}}
impl Shell{
    fn st_hit(&mut self,x:f32,y:f32,w:f32,h:f32,action:Action,enabled:bool){if self.hit_enabled&&enabled{self.hits.push(Hit{rect:render::rect(x,y,w,h),action});}}
    unsafe fn st_button(&mut self,p:&mut Painter,label:&str,x:f32,y:f32,w:f32,action:Action,enabled:bool){unsafe{
        let hover=enabled&&self.pointer.is_some_and(|(a,b)|a>=x&&a<=x+w&&b>=y&&b<=y+30.);
        if !enabled{p.push_alpha(0.3);}p.fill(x,y,w,30.,7.,if hover{0x292930}else{0x1c1c23});p.text_centered(label,x,y,w,30.,11.,FG,1.,false);
        if self.focused.as_ref()==Some(&action){p.stroke(x-1.,y-1.,w+2.,32.,8.,ACCENT,1.);}
        if !enabled{p.pop_alpha();}self.st_hit(x,y,w,30.,action,enabled);
    }}
    unsafe fn st_toggle(&mut self,p:&mut Painter,x:f32,y:f32,on:bool,action:Action,enabled:bool){unsafe{
        let hover=enabled&&self.pointer.is_some_and(|(a,b)|a>=x-6.&&a<=x+36.&&b>=y-7.&&b<=y+24.);
        if !enabled{p.push_alpha(0.3);}p.fill(x,y,30.,17.,8.5,if on{ACCENT}else if hover{0x454550}else{0x35353d});p.fill(x+if on{15.}else{2.},y+2.,13.,13.,6.5,if on{0x101015}else{0xaaaab3});if !enabled{p.pop_alpha();}
        if self.focused.as_ref()==Some(&action){p.stroke(x-4.,y-5.,38.,27.,10.,ACCENT,1.);}
        self.st_hit(x-6.,y-7.,42.,31.,action,enabled);
    }}
    unsafe fn st_row(&self,p:&mut Painter,y:f32,title:&str,description:&str){unsafe{
        p.text(title,196.,y+3.,210.,20.,12.,FG,false);p.text_ellipsis(description,196.,y+24.,210.,16.,10.,MUTED,false);
        p.fill(196.,y+47.,346.,1.,0.,0x222229);
    }}
    unsafe fn st_row_compact(&self,p:&mut Painter,y:f32,title:&str,description:&str){unsafe{
        p.text(title,196.,y+2.,210.,18.,12.,FG,false);p.text_ellipsis(description,196.,y+21.,210.,15.,10.,MUTED,false);
        p.fill(196.,y+41.,346.,1.,0.,0x222229);
    }}
    unsafe fn st_select(&mut self,p:&mut Painter,key:&'static str,value:&str,x:f32,y:f32,w:f32,enabled:bool){unsafe{
        let hover=enabled&&self.pointer.is_some_and(|(a,b)|a>=x&&a<=x+w&&b>=y&&b<=y+30.);
        if !enabled{p.push_alpha(0.3);}p.fill(x,y,w,30.,7.,if hover{0x22222a}else{0x18181e});p.stroke(x,y,w,30.,7.,if hover{0x454552}else{0x303039},1.);p.text_ellipsis(value,x+8.,y+7.,w-29.,18.,11.,0xdddde5,false);p.outline_icon("down",x+w-20.,y+8.,13.,0xaaaab4);
        if self.focused.as_ref()==Some(&Action::SettingsMenu(key,x,y,w)){p.stroke(x-1.,y-1.,w+2.,32.,8.,ACCENT,1.);}
        if !enabled{p.pop_alpha();}self.st_hit(x,y,w,30.,Action::SettingsMenu(key,x,y,w),enabled);
    }}
    pub(super) fn draw_settings(&mut self,p:&mut Painter){unsafe{
        p.outline_icon("settings",18.,18.,18.,0xc5c5ce);p.text("Pengaturan",44.,15.,200.,25.,17.,FG,false);p.text("Perubahan tersimpan otomatis",44.,40.,250.,14.,10.,0x777781,false);
        self.st_button(p,"Kembali",462.,15.,80.,Action::Back,true);p.outline_icon("left",447.,24.,13.,MUTED);
        p.fill(176.,65.,1.,279.,0.,0x26262c);
        for (i,(label,icon,_)) in CATEGORIES.iter().enumerate(){let y=66.+i as f32*47.;let selected=i==self.settings_cat;let hover=self.pointer.is_some_and(|(a,b)|a>=18.&&a<=160.&&b>=y&&b<=y+39.);
            if selected||hover{p.fill(18.,y,142.,39.,9.,if selected{0x1c1d22}else{0x111115});}
            p.outline_icon(icon,30.,y+11.,17.,if selected{0xefeff3}else{0x777782});p.text(label,56.,y+10.,103.,22.,12.,if selected{FG}else{MUTED},selected);
            if self.focused.as_ref()==Some(&Action::SettingsCategory(i)){p.stroke(18.,y,142.,39.,9.,ACCENT,1.);}
            self.st_hit(18.,y,142.,39.,Action::SettingsCategory(i),true);
        }
        let category=self.settings_cat.min(4);p.text(CATEGORIES[category].0,196.,66.,346.,25.,17.,FG,false);p.text(CATEGORIES[category].2,196.,92.,346.,16.,10.5,MUTED,false);
        match category{0=>self.settings_general(p),1=>self.settings_appearance(p),2=>self.settings_tabs(p),3=>self.settings_audio(p),_=>self.settings_reminders(p)}
        if let Some(popup)=self.settings_popup.clone(){self.draw_settings_popup(p,&popup);}
    }}

    fn settings_general(&mut self,p:&mut Painter){unsafe{
        self.st_row(p,116.,"Sembunyikan saat layar penuh","Saat bermain game atau menonton video.");self.st_toggle(p,512.,129.,self.settings.hide_fullscreen,Action::Toggle("fullscreen"),true);
        self.st_row(p,169.,"Mode bar kecil","Gunakan bar kecil saat notch ditutup.");self.st_toggle(p,512.,182.,self.settings.minimal,Action::Minimal,true);
        self.st_row(p,222.,"Tab saat dibuka","Halaman awal ketika aplikasi berjalan.");let current=self.settings.default_tab.clone();self.st_select(p,"default_tab",&current,418.,230.,124.,!self.settings.active_tab_order().is_empty());
    }}
    fn settings_appearance(&mut self,p:&mut Painter){unsafe{
        self.st_row(p,116.,"Posisi notch","Pilih tepi layar untuk notch.");
        for (label,x,bottom) in [("Atas",418.,false),("Bawah",482.,true)]{
            let selected=self.settings.bottom==bottom;
            let action=Action::Bottom(bottom);
            let hover=self.pointer.is_some_and(|(a,b)|a>=x&&a<=x+60.&&b>=124.&&b<=154.);
            p.fill(x,124.,60.,30.,7.,if selected{0x303039}else if hover{0x22222a}else{0x18181e});
            p.text_centered(label,x,124.,60.,30.,11.,if selected||hover{FG}else{MUTED},1.,false);
            if self.focused.as_ref()==Some(&action){p.stroke(x-1.,123.,62.,32.,8.,ACCENT,1.);}
            self.st_hit(x,124.,60.,30.,action,true);
        }
        self.st_row(p,162.,"Ukuran antarmuka","Teks dan kontrol diperbesar bersama.");self.st_select(p,"scale",&format!("{:.0}%",self.settings.scale*100.),418.,170.,124.,true);
        self.st_row(p,208.,"Jarak dari tepi","0 px membuat notch menempel layar.");self.st_select(p,"gap",&format!("{} px",self.settings.edge_gap),418.,216.,124.,true);
        self.st_row(p,254.,"Kepekatan latar","Nilai tinggi membuat latar lebih pekat.");self.st_select(p,"opacity",&format!("{}%",(self.settings.opacity as f32/255.*100.).round()as u32),418.,262.,124.,true);
    }}
    fn settings_tabs(&mut self,p:&mut Painter){unsafe{
        let mut order=self.settings.tab_order.clone();for &m in MODULES{if !order.iter().any(|v|v==m){order.push(m.into());}}order.retain(|t|MODULES.contains(&t.as_str()));
        let count=order.len();let active=self.settings.active_tab_order().len();
        for (i,module) in order.iter().enumerate(){let y=116.+i as f32*36.;let on=self.settings.enabled(module);p.outline_icon(ui::module_icon(module),196.,y+9.,17.,if on{0xd9d9e0}else{0x62626e});p.text(module,224.,y+8.,168.,22.,12.,if on{FG}else{MUTED},false);
            for (icon,x,d,ok) in [("up",412.,-1,i>0),("down",442.,1,i+1<count)]{
                let action=Action::MoveTab(module.clone(),d);
                let hover=ok&&self.pointer.is_some_and(|(a,b)|a>=x&&a<=x+25.&&b>=y+2.&&b<=y+33.);
                if hover{p.fill(x,y+4.,25.,24.,6.,0x22222a);}
                if !ok{p.push_alpha(0.25);}p.outline_icon(icon,x+5.,y+9.,14.,if hover{FG}else{MUTED});if !ok{p.pop_alpha();}
                if self.focused.as_ref()==Some(&action){p.stroke(x,y+4.,25.,24.,6.,ACCENT,1.);}
                self.st_hit(x,y+2.,25.,31.,action,ok);
            }
            self.st_toggle(p,512.,y+9.,on,Action::ToggleModule(module.clone()),!(on&&active<=1));if i+1<count{p.fill(196.,y+35.,346.,1.,0.,0x222229);}
        }
    }}

    fn settings_audio(&mut self,p:&mut Painter){unsafe{
        self.st_row_compact(p,116.,"Visualizer musik","Animasi mengikuti audio yang diputar.");self.st_toggle(p,512.,127.,self.settings.visualizer,Action::Toggle("visualizer"),true);
        self.st_row_compact(p,162.,"Efek warna sampul","Gunakan warna sampul pada pemutar.");self.st_toggle(p,512.,173.,self.settings.glow,Action::Toggle("glow"),true);
        self.st_row_compact(p,208.,"Romanisasi lirik CJK","Ubah aksara Jepang/Korea/Mandarin ke Latin.");self.st_toggle(p,512.,219.,self.settings.romanize_lyrics,Action::Toggle("romanize_lyrics"),true);
        self.st_row_compact(p,254.,"Pemutar dan lirik","Kontrol musik yang sedang diputar.");self.st_button(p,"Buka Musik",418.,260.,124.,Action::SettingsOpenTab("Musik".into(),0),self.settings.enabled("Musik"));
    }}
    fn settings_reminders(&mut self,p:&mut Painter){unsafe{
        let enabled=self.settings.enabled("Reminder");
        if !enabled{p.text("Tab Reminder sedang dimatikan.",196.,116.,210.,18.,11.,MUTED,false);self.st_button(p,"Aktifkan",448.,112.,94.,Action::ToggleModule("Reminder".into()),true);}
        let top=if enabled{116.}else{153.};
        let rows=[("Fokus","timer",format!("{} menit · istirahat {} menit",self.settings.focus_minutes,self.settings.break_minutes),self.settings.focus_enabled),("Hidrasi","droplet",format!("{} ml · setiap {} menit",self.settings.water_target,self.settings.water_interval),self.settings.water_reminder),("Peregangan","person",format!("Jeda setiap {} menit",self.settings.move_interval),self.settings.move_reminder),("Sholat","moon",prayer_lead_summary(&self.settings.prayer_city,self.settings.prayer_lead),self.settings.prayer_enabled)];
        let row_h=if enabled{53.}else{45.};
        for (i,(name,icon,summary,on)) in rows.iter().enumerate(){let y=top+i as f32*row_h;p.outline_icon(icon,196.,y+10.,18.,0xa8a8b2);p.text(name,224.,y+1.,164.,22.,12.,FG,false);p.text_ellipsis(summary,224.,y+23.,198.,17.,10.,MUTED,false);p.text(if *on{"Aktif"}else{"Mati"},423.,y+11.,42.,16.,10.,if *on{ACCENT}else{MUTED},false);self.st_button(p,"Atur",474.,y+4.,68.,Action::SettingsOpenTab("Reminder".into(),i),enabled);if i<3{p.fill(196.,y+row_h-1.,346.,1.,0.,0x222229);}}
    }}
    fn settings_options(&self,key:&str)->Vec<(u32,String)>{match key{
        "default_tab"=>self.settings.active_tab_order().iter().filter_map(|tab|MODULES.iter().position(|m|*m==tab).map(|i|(i as u32,tab.clone()))).collect(),
        "scale"=>[80,100,110,125,135].iter().map(|&v|(v,format!("{v}%"))).collect(),
        "gap"=>[0,4,8,16,24].iter().map(|&v|(v,format!("{v} px"))).collect(),
        "opacity"=>[100,98,90,80,70].iter().map(|&v|(v,format!("{v}%"))).collect(),_=>vec![]
    }}
    pub(super) fn open_settings_menu(&mut self,key:&'static str,x:f32,y:f32,width:f32){
        if self.settings_popup.as_ref().is_some_and(|p|p.key==key){self.settings_popup=None;return}
        let current=match key{"default_tab"=>MODULES.iter().position(|m|*m==self.settings.default_tab).unwrap_or(0)as u32,"scale"=>(self.settings.scale*100.).round()as u32,"gap"=>self.settings.edge_gap as u32,"opacity"=>(self.settings.opacity as f32/255.*100.).round()as u32,_=>0};let options=self.settings_options(key);if options.is_empty(){return}
        let cursor=options.iter().position(|(v,_)|*v==current).unwrap_or(0);self.settings_popup=Some(SettingsPopup{key,x,y,width,cursor});
    }
    pub(super) fn apply_settings_value(&mut self,key:&str,value:u32){
        if !self.settings_options(key).iter().any(|(v,_)|*v==value){return}
        match key{"default_tab"=>{if let Some(tab)=MODULES.get(value as usize){self.settings.default_tab=(*tab).into();}},"scale"=>{self.settings.scale=value as f32/100.;self.layout();self.make_inputs();},"gap"=>{self.settings.edge_gap=value as i32;self.layout();},"opacity"=>self.settings.opacity=((value as f32/100.)*255.).round()as u8,_=>{}}
    }
    pub(super) fn handle_settings_key(&mut self,key:u16)->bool{
        if !self.settings_open{return false}
        if let Some(popup)=self.settings_popup.as_ref(){let end=self.settings_options(popup.key).len().saturating_sub(1);let popup=self.settings_popup.as_mut().unwrap();match key{
            0x26=>popup.cursor=popup.cursor.saturating_sub(1),0x28=>popup.cursor=(popup.cursor+1).min(end),0x24=>popup.cursor=0,0x23=>popup.cursor=end,
            0x0d|0x20=>{let key=popup.key;let i=popup.cursor;if let Some((value,_))=self.settings_options(key).get(i){self.action(Action::SettingsValue(key,*value));}},0x1b=>self.settings_popup=None,0x09=>{self.settings_popup=None;self.invalidate();return false},_=>return false}
            self.invalidate();return true
        }
        match key{0x1b=>{self.action(Action::Back);true},0x20=>{if let Some(action)=self.focused.clone(){self.action(action);}true},0x26|0x28|0x24|0x23 if self.focused.as_ref().is_none_or(|a|matches!(a,Action::SettingsCategory(..)))=>{let cat=match key{0x24=>0,0x23=>4,0x26=>self.settings_cat.saturating_sub(1),_=>(self.settings_cat+1).min(4)};self.action(Action::SettingsCategory(cat));true},_=>false}
    }
    fn draw_settings_popup(&mut self,p:&mut Painter,popup:&SettingsPopup){unsafe{
        let options=self.settings_options(popup.key);let h=options.len()as f32*26.+8.;let top=if popup.y+34.+h<=self.height-12.{popup.y+34.}else{(popup.y-h-4.).max(56.)};let x=popup.x;let w=popup.width;
        p.fill(x-3.,top-3.,w+6.,h+6.,9.,0x09090c);p.fill(x,top,w,h,7.,0x1c1c23);p.stroke(x,top,w,h,7.,0x3a3a45,1.);
        for (i,(v,label)) in options.iter().enumerate(){let y=top+4.+i as f32*26.;let hover=self.pointer.is_some_and(|(a,b)|a>=x&&a<=x+w&&b>=y&&b<=y+26.);if hover||i==popup.cursor{p.fill(x+3.,y,w-6.,25.,4.,0x30303b);}p.text(label,x+8.,y+5.,w-16.,18.,11.,FG,false);self.st_hit(x+2.,y,w-4.,26.,Action::SettingsValue(popup.key,*v),true);}
    }}
}
