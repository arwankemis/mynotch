use super::*;
use crate::i18n::t;
const FG:u32=0xf3f3f5;const MUTED:u32=0xb2bac8;const ACCENT:u32=0x69d2e5;
#[derive(Clone)]pub(super) struct SettingsPopup{pub key:&'static str,pub x:f32,pub y:f32,pub width:f32,pub cursor:usize}
#[allow(dead_code)]
pub(super) fn prayer_lead_summary(city:&str,lead:u32)->String{
    prayer_lead_summary_i18n(city, lead, "id")
}
pub(super) fn prayer_lead_summary_i18n(city:&str,lead:u32,lang:&str)->String{
    if lang == "en" {
        if lead==0{format!("{city} · On time")}else{format!("{city} · {lead} min before")}
    } else {
        if lead==0{format!("{city} · Tepat waktu")}else{format!("{city} · {lead} menit sebelumnya")}
    }
}
impl Shell{
    fn settings_content_bottom(&self)->f32{match self.settings_cat{
        0=>if self.settings_subpage_privacy{400.}else{370.},
        1=>414.,2=>116.+self.settings.settings_tab_order().len()as f32*36.,
        3=>300.,4=>354.,5=>420.,_=>self.height-20.
    }}
    pub(super) fn scroll_settings(&mut self,delta:f32){
        if self.settings_cat==5{self.scroll_catalog(delta.signum()as i32);return}
        let cat=self.settings_cat.min(6);
        let l=super::settings_layout::SettingsLayout::compute(self.width,self.height.min(360.),self.settings_content_bottom(),self.settings_scroll[cat]+delta);
        self.settings_scroll[cat]=l.scroll_y;self.settings_popup=None;self.focused=None;self.invalidate();
    }

    fn categories(&self) -> [(&'static str, &'static str, &'static str); 7] {
        let lang = self.settings.language.clone();
        [
            (t("cat.general", &lang), "settings", t("cat.general.desc", &lang)),
            (t("cat.appearance", &lang), "monitor", t("cat.appearance.desc", &lang)),
            (t("cat.tabs", &lang), "layout", t("cat.tabs.desc", &lang)),
            (t("cat.audio", &lang), "music", t("cat.audio.desc", &lang)),
            (t("cat.reminders", &lang), "bell", t("cat.reminders.desc", &lang)),
            (t("cat.mods", &lang), "package", t("cat.mods.desc", &lang)),
            (t("cat.update", &lang), "download", t("cat.update.desc", &lang)),
        ]
    }
    pub(super) fn st_hit(&mut self,x:f32,y:f32,w:f32,h:f32,action:Action,enabled:bool){if self.hit_enabled&&enabled{self.hits.push(Hit{rect:render::rect(x,y,w,h),action});}}
    pub(super) unsafe fn st_button(&mut self,p:&mut Painter,label:&str,x:f32,y:f32,w:f32,action:Action,enabled:bool){unsafe{
        let hover=enabled&&self.pointer.is_some_and(|(a,b)|a>=x&&a<=x+w&&b>=y&&b<=y+30.);
        if !enabled{p.push_alpha(0.3);}p.fill(x,y,w,30.,7.,if hover{0x292930}else{0x1c1c23});p.text_centered(label,x,y,w,30.,11.,FG,1.,false);
        if self.focused.as_ref()==Some(&action){p.stroke(x-1.,y-1.,w+2.,32.,8.,ACCENT,1.);}
        if !enabled{p.pop_alpha();}self.st_hit(x,y,w,30.,action,enabled);
    }}
    pub(super) unsafe fn st_toggle(&mut self,p:&mut Painter,x:f32,y:f32,on:bool,action:Action,enabled:bool){unsafe{
        let hover=enabled&&self.pointer.is_some_and(|(a,b)|a>=x-6.&&a<=x+36.&&b>=y-7.&&b<=y+24.);
        if !enabled{p.push_alpha(0.3);}p.fill(x,y,30.,17.,8.5,if on{ACCENT}else if hover{0x454550}else{0x35353d});p.fill(x+if on{15.}else{2.},y+2.,13.,13.,6.5,if on{0x101015}else{0xaaaab3});if !enabled{p.pop_alpha();}
        if self.focused.as_ref()==Some(&action){p.stroke(x-4.,y-5.,38.,27.,10.,ACCENT,1.);}
        self.st_hit(x-6.,y-7.,42.,31.,action,enabled);
    }}
    #[allow(dead_code)]
    unsafe fn st_row(&self,p:&mut Painter,y:f32,title:&str,description:&str){unsafe{
        p.text(title,196.,y+3.,210.,20.,12.,FG,false);p.text_ellipsis(description,196.,y+24.,210.,16.,10.,MUTED,false);
        p.fill(196.,y+47.,346.,1.,0.,0x222229);
    }}
    pub(super) unsafe fn st_row_compact(&self,p:&mut Painter,y:f32,title:&str,description:&str){unsafe{
        p.text(title,196.,y+2.,210.,18.,12.,FG,false);p.text_ellipsis(description,196.,y+21.,210.,16.,11.,MUTED,false);
        p.fill(196.,y+41.,346.,1.,0.,0x222229);
    }}
    unsafe fn st_select(&mut self,p:&mut Painter,key:&'static str,value:&str,x:f32,y:f32,w:f32,enabled:bool){unsafe{
        let hover=enabled&&self.pointer.is_some_and(|(a,b)|a>=x&&a<=x+w&&b>=y&&b<=y+30.);
        if !enabled{p.push_alpha(0.3);}p.fill(x,y,w,30.,7.,if hover{0x22222a}else{0x18181e});p.stroke(x,y,w,30.,7.,if hover{0x454552}else{0x303039},1.);p.text_ellipsis(value,x+8.,y+7.,w-29.,18.,11.,0xdddde5,false);p.outline_icon("down",x+w-20.,y+8.,13.,0xaaaab4);
        if settings_menu_has_focus(self.focused.as_ref(),key,x,y,w){p.stroke(x-1.,y-1.,w+2.,32.,8.,ACCENT,1.);}
        if !enabled{p.pop_alpha();}self.st_hit(x,y,w,30.,Action::SettingsMenu(key,x,y,w),enabled);
    }}
    pub(super) fn draw_settings(&mut self,p:&mut Painter){unsafe{
        let lang = self.settings.language.clone();
        p.outline_icon("settings",18.,18.,18.,0xc5c5ce);p.text(t("settings.title", &lang),44.,15.,200.,25.,17.,FG,false);p.text(t("settings.auto_saved", &lang),44.,40.,250.,14.,10.,0x777781,false);
        self.st_button(p,t("settings.back", &lang),462.,15.,80.,Action::Back,true);p.outline_icon("left",447.,24.,13.,MUTED);
        p.fill(176.,65.,1.,279.,0.,0x26262c);
        let cats = self.categories();
        for (i,(label,icon,_)) in cats.iter().enumerate(){
            let y = 64. + i as f32 * 39.;
            let selected = i == self.settings_cat;
            let hover = self.pointer.is_some_and(|(a,b)| a >= 18. && a <= 160. && b >= y && b <= y + 34.);
            if selected || hover {
                p.fill(18., y, 142., 34., 8., if selected { 0x1c1d22 } else { 0x111115 });
            }
            p.outline_icon(icon, 30., y + 8., 16., if selected { 0xefeff3 } else { 0x777782 });
            p.text(label, 56., y + 7., 103., 22., 11.5, if selected { FG } else { MUTED }, selected);
            if self.focused.as_ref() == Some(&Action::SettingsCategory(i)) {
                p.stroke(18., y, 142., 34., 8., ACCENT, 1.);
            }
            self.st_hit(18., y, 142., 34., Action::SettingsCategory(i), true);
        }
        let category = self.settings_cat.min(6);
        if !(category == 0 && self.settings_subpage_privacy) {
            p.text(cats[category].0, 196., 66., 346., 25., 17., FG, false);
            p.text(cats[category].2, 196., 92., 346., 16., 10.5, MUTED, false);
        }
        if category==5{self.draw_catalog(p);return;}
        let layout=super::settings_layout::SettingsLayout::compute(self.width,self.height.min(360.),self.settings_content_bottom(),self.settings_scroll[category]);
        self.settings_scroll[category]=layout.scroll_y;
        let scroll=layout.scroll_y;
        let first_hit=self.hits.len();let original_pointer=self.pointer;
        {
            let v=layout.viewport;p.clip(v.left,v.top,v.right-v.left,v.bottom-v.top);
            p.translate_relative(0.,-scroll);self.pointer=original_pointer.map(|(x,y)|(x,y+scroll));
        }
        match category {
            0 => {
                if self.settings_subpage_privacy {
                    self.draw_settings_privacy(p);
                } else {
                    self.settings_general(p);
                }
            }
            1 => self.settings_appearance(p),
            2 => self.settings_tabs(p),
            3 => self.settings_audio(p),
            4 => self.settings_reminders(p),
            5 => self.settings_mods(p),
            _ => self.settings_update(p),
        }
        {
            p.translate_relative(0.,scroll);p.unclip();self.pointer=original_pointer;
            let pending=self.hits.split_off(first_hit);
            for mut hit in pending{if let Some(r)=layout.visible_hit(hit.rect){hit.rect=r;
                if let Action::SettingsMenu(key,x,y,w)=hit.action{hit.action=Action::SettingsMenu(key,x,y-scroll,w);}
                self.hits.push(hit);
            }}
            if layout.max_scroll_y>0.{
                let v=layout.viewport;let track=v.bottom-v.top;let thumb=(track*track/(track+layout.max_scroll_y)).max(24.);
                p.fill(548.,v.top,3.,track,1.5,0x303039);
                p.fill(548.,v.top+(track-thumb)*scroll/layout.max_scroll_y,3.,thumb,1.5,0x808895);
            }
        }
        if let Some(popup)=self.settings_popup.clone(){self.draw_settings_popup(p,&popup);}
    }}

    fn settings_general(&mut self,p:&mut Painter){unsafe{
        let lang = self.settings.language.clone();
        self.st_row_compact(p,116.,t("general.language",&lang),t("general.language.desc",&lang));
        self.st_select(p,"language",crate::i18n::language_name(&lang),418.,122.,124.,true);
        self.st_row_compact(p,158.,t("general.fullscreen",&lang),t("general.fullscreen.desc",&lang));
        self.st_toggle(p,512.,169.,self.settings.hide_fullscreen,Action::Toggle("fullscreen"),true);
        self.st_row_compact(p,200.,t("general.minimal",&lang),t("general.minimal.desc",&lang));
        self.st_toggle(p,512.,211.,self.settings.minimal,Action::Minimal,true);
        self.st_row_compact(p,242.,t("general.startup",&lang),t("general.startup.desc",&lang));
        self.st_toggle(p,512.,253.,self.settings.run_at_startup,Action::Toggle("startup"),true);
        self.st_row_compact(p,284.,t("general.default_tab",&lang),t("general.default_tab.desc",&lang));
        let current=crate::i18n::tab_name(&self.settings.default_tab, &lang).to_string();
        let has_tabs = !self.settings.active_tab_order().is_empty();
        self.st_select(p,"default_tab",&current,418.,290.,124.,has_tabs);
        self.st_row_compact(p,326.,t("general.privacy",&lang),t("general.privacy.desc",&lang));
        self.st_button(p,t("general.privacy_open",&lang),448.,330.,94.,Action::OpenPrivacySettings,true);
    }}
    fn settings_appearance(&mut self,p:&mut Painter){unsafe{
        let lang = self.settings.language.clone();
        self.st_row_compact(p,116.,t("appearance.position",&lang),t("appearance.position.desc",&lang));
        for (label,x,bottom) in [(t("appearance.top",&lang),418.,false),(t("appearance.bottom",&lang),482.,true)]{
            let selected=self.settings.bottom==bottom;
            let action=Action::Bottom(bottom);
            let hover=self.pointer.is_some_and(|(a,b)|a>=x&&a<=x+60.&&b>=124.&&b<=154.);
            p.fill(x,124.,60.,30.,7.,if selected{0x303039}else if hover{0x22222a}else{0x18181e});
            p.text_centered(label,x,124.,60.,30.,11.,if selected||hover{FG}else{MUTED},1.,false);
            if self.focused.as_ref()==Some(&action){p.stroke(x-1.,123.,62.,32.,8.,ACCENT,1.);}
            self.st_hit(x,124.,60.,30.,action,true);
        }
        self.st_row_compact(p,158.,t("appearance.scale",&lang),t("appearance.scale.desc",&lang));
        self.st_select(p,"scale",&format!("{:.0}%",self.settings.scale*100.),418.,164.,124.,true);
        self.st_row_compact(p,200.,t("appearance.gap",&lang),t("appearance.gap.desc",&lang));
        self.st_select(p,"gap",&format!("{} px",self.settings.edge_gap),418.,206.,124.,true);
        self.st_row_compact(p,242.,t("appearance.opacity",&lang),t("appearance.opacity.desc",&lang));
        self.st_select(p,"opacity",&format!("{}%",(self.settings.opacity as f32/255.*100.).round()as u32),418.,248.,124.,true);
        // Material + Gelap/Terang share one row: the page is already at its 6-row limit.
        let fallback=match self.glass_fallback(){
            crate::glass::Fallback::ReducedTransparency=>Some(t("material.fallback.reduced",&lang)),
            crate::glass::Fallback::HighContrast=>Some(t("material.fallback.contrast",&lang)),
            crate::glass::Fallback::Software=>Some(t("material.fallback.software",&lang)),
            crate::glass::Fallback::NoBackdrop=>Some(t("material.fallback.unavailable",&lang)),
            _=>None,
        };
        self.st_row_compact(p,284.,t("appearance.material",&lang),fallback.unwrap_or(t("appearance.material.desc",&lang)));
        let mode=crate::glass::MaterialMode::from_str(&self.settings.theme_material);
        let material_label=match mode{crate::glass::MaterialMode::Solid=>t("material.solid",&lang),crate::glass::MaterialMode::Lite=>t("material.lite",&lang),crate::glass::MaterialMode::Adaptive=>t("material.adaptive",&lang)};
        self.st_select(p,"material",material_label,418.,290.,124.,true);
        let color_label=if crate::glass::is_light_mode(&self.settings.theme_color){t("color.light",&lang)}else{t("color.dark",&lang)};
        self.st_row_compact(p,326.,if lang=="en"{"Colour scheme"}else{"Warna tema"},if lang=="en"{"Text, icons and controls"}else{"Teks, ikon, dan kontrol"});
        self.st_select(p,"theme_color",color_label,418.,332.,124.,true);
        self.st_row_compact(p,368.,if lang=="en"{"Software renderer (WARP)"}else{"Renderer software (WARP)"},if lang=="en"{"CPU rendering. Restart required."}else{"Render melalui CPU. Perlu mulai ulang."});
        self.st_toggle(p,512.,379.,self.settings.software_renderer,Action::Toggle("software_renderer"),true);
    }}
    fn settings_tabs(&mut self,p:&mut Painter){unsafe{
        let order=self.settings.settings_tab_order();
        let mods=self.mods();
        let count=order.len();let active=self.settings.active_tab_order().len();
        for (i,module) in order.iter().enumerate(){let y=116.+i as f32*36.;let on=self.settings.enabled(module);
            let icon=mods.iter().find(|m|&m.tab_name==module).map(|m|m.icon.clone()).unwrap_or_else(||ui::module_icon(module).to_string());
            p.outline_icon(&icon,196.,y+9.,17.,if on{0xd9d9e0}else{0x62626e});
            let display_module = crate::i18n::tab_name(module, &self.settings.language);
            p.text(display_module,224.,y+8.,168.,22.,12.,if on{FG}else{MUTED},false);
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
        let lang = self.settings.language.clone();
        self.st_row_compact(p,116.,t("audio.visualizer",&lang),t("audio.visualizer.desc",&lang));
        self.st_toggle(p,512.,127.,self.settings.visualizer,Action::Toggle("visualizer"),true);
        self.st_row_compact(p,162.,t("audio.glow",&lang),t("audio.glow.desc",&lang));
        self.st_toggle(p,512.,173.,self.settings.glow,Action::Toggle("glow"),true);
        self.st_row_compact(p,208.,t("audio.romanize",&lang),t("audio.romanize.desc",&lang));
        self.st_toggle(p,512.,219.,self.settings.romanize_lyrics,Action::Toggle("romanize_lyrics"),true);
        self.st_row_compact(p,254.,t("audio.player_lyrics",&lang),t("audio.player_lyrics.desc",&lang));
        self.st_button(p,t("audio.open_music",&lang),418.,260.,124.,Action::SettingsOpenTab("Musik".into(),0),self.settings.enabled("Musik"));
    }}
    fn settings_reminders(&mut self,p:&mut Painter){unsafe{
        let lang = self.settings.language.clone();
        let enabled=self.settings.enabled("Reminder");
        if !enabled{p.text(t("reminders.disabled",&lang),196.,116.,210.,18.,11.,MUTED,false);self.st_button(p,t("reminders.enable",&lang),448.,112.,94.,Action::ToggleModule("Reminder".into()),true);}
        let top=if enabled{116.}else{153.};
        let rows=[
            (t("reminders.focus",&lang),"timer",if lang == "en" { format!("{} min · break {} min",self.settings.focus_minutes,self.settings.break_minutes) } else { format!("{} menit · istirahat {} menit",self.settings.focus_minutes,self.settings.break_minutes) },self.settings.focus_enabled),
            (t("reminders.hydrate",&lang),"droplet",if lang == "en" { format!("{} ml · every {} min",self.settings.water_target,self.settings.water_interval) } else { format!("{} ml · setiap {} menit",self.settings.water_target,self.settings.water_interval) },self.settings.water_reminder),
            (t("reminders.stretch",&lang),"person",if lang == "en" { format!("Break every {} min",self.settings.move_interval) } else { format!("Jeda setiap {} menit",self.settings.move_interval) },self.settings.move_reminder),
            (t("reminders.prayer",&lang),"moon",prayer_lead_summary_i18n(&self.settings.prayer_city,self.settings.prayer_lead,&lang),self.settings.prayer_enabled)
        ];
        let row_h=if enabled{53.}else{45.};
        for (i,(name,icon,summary,on)) in rows.iter().enumerate(){let y=top+i as f32*row_h;p.outline_icon(icon,196.,y+10.,18.,0xa8a8b2);p.text(name,224.,y+1.,164.,22.,12.,FG,false);p.text_ellipsis(summary,224.,y+23.,198.,17.,10.,MUTED,false);p.text(if *on{t("reminders.active",&lang)}else{t("reminders.inactive",&lang)},423.,y+11.,42.,16.,10.,if *on{ACCENT}else{MUTED},false);self.st_button(p,t("reminders.configure",&lang),474.,y+4.,68.,Action::SettingsOpenTab("Reminder".into(),i),enabled);if i<3{p.fill(196.,y+row_h-1.,346.,1.,0.,0x222229);}}
    }}
    fn settings_mods(&mut self,p:&mut Painter){self.draw_catalog(p);}
    #[allow(dead_code)]
    fn mod_type_label(&self, kind: &crate::mods::ModType) -> String {
        let lang = &self.settings.language;
        match kind {
            crate::mods::ModType::Builtin => t("mods.type_builtin", lang).to_string(),
            crate::mods::ModType::Declarative => t("mods.type_json", lang).to_string(),
            crate::mods::ModType::Lua => t("mods.type_lua", lang).to_string(),
        }
    }

    fn visible_mod_rows(&self) -> usize {super::catalog_ui::CatalogLayout::new(self.width,self.height.min(360.)).rows()}

    pub(super) fn scroll_mods(&mut self, delta: i32) {
        let total = crate::mods::catalog::get_catalog(&self.store.root,&self.settings).filter(self.mods_kind,self.mods_filter,"").len();
        let visible = self.visible_mod_rows();
        let next = (self.mods_scroll as i32 + delta).max(0) as usize;
        self.mods_scroll = crate::core::clamp_list_scroll(next, total, visible);
        self.invalidate();
    }
    pub(super) fn scroll_catalog(&mut self,delta:i32){
        if self.mods_detail_open{
            let l=super::catalog_ui::CatalogLayout::new(self.width,self.height.min(360.));
            let installed=self.mods_selected.as_ref().is_some_and(|id|crate::mods::package::PackageManager::new(&self.store.root).is_installed(&self.settings,id));
            let max=((if installed{400.}else{260.})-l.detail.bottom).max(0.);self.mods_detail_scroll=(self.mods_detail_scroll+delta as f32*42.).clamp(0.,max);self.focused=None;self.invalidate();
        }else{self.scroll_mods(delta);}
    }
    fn settings_options(&self,key:&str)->Vec<(u32,String)>{match key{
        "language"=>crate::i18n::LANGUAGES.iter().enumerate().map(|(i,(_,name))|(i as u32,name.to_string())).collect(),
        "default_tab"=>self.settings.active_tab_order().iter().enumerate().map(|(i,tab)|(i as u32, crate::i18n::tab_name(tab, &self.settings.language).to_string())).collect(),
        "scale"=>[80,100,110,125,135].iter().map(|&v|(v,format!("{v}%"))).collect(),
        "gap"=>[0,4,8,16,24].iter().map(|&v|(v,format!("{v} px"))).collect(),
        "opacity"=>[100,98,90,80,70].iter().map(|&v|(v,format!("{v}%"))).collect(),
        "material"=>{let l=&self.settings.language;vec![(0,t("material.solid",l).to_string()),(1,t("material.lite",l).to_string()),(2,t("material.adaptive",l).to_string())]},
        "theme_color"=>{let l=&self.settings.language;vec![(0,t("color.dark",l).to_string()),(1,t("color.light",l).to_string())]},
        _=>vec![]
    }}
    pub(super) fn open_settings_menu(&mut self,key:&'static str,x:f32,y:f32,width:f32){
        if self.settings_popup.as_ref().is_some_and(|p|p.key==key){self.settings_popup=None;return}
        let current=match key{
            "language"=>crate::i18n::LANGUAGES.iter().position(|(c,_)|*c==self.settings.language).unwrap_or(0)as u32,
            "default_tab"=>tab_option_index(&self.settings.active_tab_order(),&self.settings.default_tab),
            "scale"=>(self.settings.scale*100.).round()as u32,
            "gap"=>self.settings.edge_gap as u32,
            "opacity"=>(self.settings.opacity as f32/255.*100.).round()as u32,
            "material"=>crate::glass::MaterialMode::from_str(&self.settings.theme_material).index(),
            "theme_color"=>crate::glass::is_light_mode(&self.settings.theme_color) as u32,
            _=>0
        };
        let options=self.settings_options(key);if options.is_empty(){return}
        let cursor=options.iter().position(|(v,_)|*v==current).unwrap_or(0);self.settings_popup=Some(SettingsPopup{key,x,y,width,cursor});
    }
    pub(super) fn apply_settings_value(&mut self,key:&str,value:u32){
        if !self.settings_options(key).iter().any(|(v,_)|*v==value){return}
        match key{
            "language"=>{if let Some((code,_))=crate::i18n::LANGUAGES.get(value as usize){self.settings.language=(*code).into();self.save();}},
            "default_tab"=>{if let Some(tab)=self.settings.active_tab_order().get(value as usize){self.settings.default_tab=tab.clone();}},
            "scale"=>{self.settings.scale=value as f32/100.;self.layout();self.make_inputs();},
            "gap"=>{self.settings.edge_gap=value as i32;self.layout();},
            "opacity"=>self.settings.opacity=((value as f32/100.)*255.).round().clamp(MIN_OPACITY as f32,255.)as u8,
            "material"=>{self.settings.theme_material=crate::glass::MaterialMode::from_index(value).as_str().into();self.save();self.glass_settings_changed();},
            "theme_color"=>{self.settings.theme_color=if value==1{"light"}else{"dark"}.into();self.save();self.glass_settings_changed();},
            _=>{}
        }
    }
    pub(super) fn handle_settings_key(&mut self,key:u16)->bool{
        if !self.settings_open{return false}
        if key==0x1b&&self.settings_cat==5&&self.mods_detail_open{self.mods_detail_open=false;self.focused=None;self.invalidate();return true}
        if let Some(popup)=self.settings_popup.as_ref(){let end=self.settings_options(popup.key).len().saturating_sub(1);let popup=self.settings_popup.as_mut().unwrap();match key{
            0x26=>popup.cursor=popup.cursor.saturating_sub(1),0x28=>popup.cursor=(popup.cursor+1).min(end),0x24=>popup.cursor=0,0x23=>popup.cursor=end,
            0x0d|0x20=>{let key=popup.key;let i=popup.cursor;if let Some((value,_))=self.settings_options(key).get(i){self.action(Action::SettingsValue(key,*value));}},0x1b=>self.settings_popup=None,0x09=>{self.settings_popup=None;self.invalidate();return false},_=>return false}
            self.invalidate();return true
        }
        match key{0x21=>{self.scroll_settings(-168.);true},0x22=>{self.scroll_settings(168.);true},0x1b=>{self.action(Action::Back);true},0x20=>{if let Some(action)=self.focused.clone(){self.action(action);}true},0x26|0x28|0x24|0x23 if self.focused.as_ref().is_none_or(|a|matches!(a,Action::SettingsCategory(..)))=>{let cat=match key{0x24=>0,0x23=>6,0x26=>self.settings_cat.saturating_sub(1),_=>(self.settings_cat+1).min(6)};self.action(Action::SettingsCategory(cat));true},_=>false}
    }
    fn draw_settings_popup(&mut self,p:&mut Painter,popup:&SettingsPopup){unsafe{
        let options=self.settings_options(popup.key);let h=options.len()as f32*26.+8.;let top=if popup.y+34.+h<=self.height-12.{popup.y+34.}else{(popup.y-h-4.).max(56.)};let x=popup.x;let w=popup.width;
        p.fill(x-3.,top-3.,w+6.,h+6.,9.,0x09090c);p.fill(x,top,w,h,7.,0x1c1c23);p.stroke(x,top,w,h,7.,0x3a3a45,1.);
        for (i,(v,label)) in options.iter().enumerate(){let y=top+4.+i as f32*26.;let hover=self.pointer.is_some_and(|(a,b)|a>=x&&a<=x+w&&b>=y&&b<=y+26.);if hover||i==popup.cursor{p.fill(x+3.,y,w-6.,25.,4.,0x30303b);}p.text(label,x+8.,y+5.,w-16.,18.,11.,FG,false);self.st_hit(x+2.,y,w-4.,26.,Action::SettingsValue(popup.key,*v),true);}
    }}

    fn settings_update(&mut self, p: &mut Painter) { unsafe {
        let lang = self.settings.language.clone();
        let version_str = format!("v{}", super::app_version());

        // Row 1: Current Version
        p.text(t("update.current", &lang),196.,116.,196.,20.,12.,FG,false);
        let btn_label = match &self.update_status {
            super::UpdateStatus::Checking => t("update.checking", &lang),
            super::UpdateStatus::UpToDate(_) => t("update.check_again", &lang),
            super::UpdateStatus::Error(_) => t("update.retry", &lang),
            _ => t("update.check", &lang),
        };
        let is_checking = matches!(self.update_status, super::UpdateStatus::Checking);
        self.st_button(p, btn_label, 400., 120., 142., Action::CheckUpdate, !is_checking);
        p.text_ellipsis(&version_str,196.,142.,196.,18.,11.5,0x38bdf8,true);

        // Row 2: Status Card (168..248, h = 80)
        let card_x = 196.;
        let card_y = 168.;
        let card_w = 346.;
        let card_h = 80.;

        match &self.update_status {
            super::UpdateStatus::Idle => {
                p.fill_alpha(card_x, card_y, card_w, card_h, 8., 0xffffff, 0.03);
                p.stroke(card_x, card_y, card_w, card_h, 8., 0xffffff, 0.08);
                p.outline_icon("download", card_x + 14., card_y + 16., 20., 0x94a3b8);
                p.text(t("update.idle_hint", &lang), card_x + 44., card_y + 16., card_w - 56., 44., 11.0, 0x94a3b8, false);
            },
            super::UpdateStatus::Checking => {
                p.fill_alpha(card_x, card_y, card_w, card_h, 8., 0x0284c7, 0.08);
                p.stroke(card_x, card_y, card_w, card_h, 8., 0x38bdf8, 0.6);
                p.outline_icon("download", card_x + 14., card_y + 16., 20., 0x38bdf8);
                p.text(t("update.checking", &lang), card_x + 44., card_y + 28., card_w - 56., 24., 11.5, 0x38bdf8, true);
            },
            super::UpdateStatus::UpToDate(time) => {
                p.fill_alpha(card_x, card_y, card_w, card_h, 8., 0x10b981, 0.10);
                p.stroke(card_x, card_y, card_w, card_h, 8., 0x10b981, 0.6);
                p.outline_icon("check", card_x + 14., card_y + 16., 20., 0x34d399);
                p.text(t("update.up_to_date", &lang), card_x + 44., card_y + 16., card_w - 56., 20., 12.0, 0x34d399, true);
                let sub = format!("{} {} · {}", t("update.checked_at", &lang), time, t("update.up_to_date_sub", &lang));
                p.text(&sub, card_x + 44., card_y + 38., card_w - 56., 30., 10.0, 0xa7f3d0, false);
            },
            super::UpdateStatus::Available(update) => {
                p.fill_alpha(card_x, card_y, card_w, card_h + 12., 8., 0x0284c7, 0.16);
                p.stroke(card_x, card_y, card_w, card_h + 12., 8., 0x38bdf8, 1.2);
                let title = format!("{} ({})", t("update.available", &lang), update.version);
                p.text(&title, card_x + 14., card_y + 10., card_w - 28., 20., 12.5, 0x38bdf8, true);
                let rel_desc = &update.display_name;
                p.text_ellipsis(rel_desc, card_x + 14., card_y + 32., card_w - 28., 16., 10.5, 0xe0f2fe, false);

                self.st_button(p, t("update.perbarui", &lang), card_x + 14., card_y + 54., 150., Action::DownloadUpdate, true);
                self.st_button(p, t("update.release_notes", &lang), card_x + 172., card_y + 54., 160., Action::OpenReleaseNotes, true);
            },
            super::UpdateStatus::Downloading { percent, .. } => {
                p.fill_alpha(card_x, card_y, card_w, card_h, 8., 0x0284c7, 0.12);
                p.stroke(card_x, card_y, card_w, card_h, 8., 0x38bdf8, 0.8);
                let label = format!("{} ({}%)", t("update.downloading", &lang), percent);
                p.text(&label, card_x + 14., card_y + 16., card_w - 28., 20., 12.0, 0x38bdf8, true);

                let bar_w = card_w - 28.;
                let fill_w = bar_w * (*percent as f32 / 100.0).clamp(0.0, 1.0);
                p.fill_alpha(card_x + 14., card_y + 44., bar_w, 6., 3., 0xffffff, 0.1);
                if fill_w > 0.0 {
                    p.fill_alpha(card_x + 14., card_y + 44., fill_w, 6., 3., 0x38bdf8, 0.9);
                }
            },
            super::UpdateStatus::Verifying => {
                p.fill_alpha(card_x, card_y, card_w, card_h, 8., 0x0284c7, 0.12);
                p.stroke(card_x, card_y, card_w, card_h, 8., 0x38bdf8, 0.8);
                p.text(t("update.verifying", &lang), card_x + 14., card_y + 28., card_w - 28., 20., 12.0, 0x38bdf8, true);
            },
            super::UpdateStatus::Staging => {
                p.fill_alpha(card_x, card_y, card_w, card_h, 8., 0x0284c7, 0.12);
                p.stroke(card_x, card_y, card_w, card_h, 8., 0x38bdf8, 0.8);
                p.text(t("update.staging", &lang), card_x + 14., card_y + 28., card_w - 28., 20., 12.0, 0x38bdf8, true);
            },
            super::UpdateStatus::AwaitingRestart { version, .. } => {
                p.fill_alpha(card_x, card_y, card_w, card_h + 12., 8., 0x10b981, 0.14);
                p.stroke(card_x, card_y, card_w, card_h + 12., 8., 0x34d399, 1.2);
                let title = format!("{} ({})", t("update.ready", &lang), version);
                p.text(&title, card_x + 14., card_y + 10., card_w - 28., 20., 12.0, 0x34d399, true);
                p.text(t("update.restart_desc", &lang), card_x + 14., card_y + 32., card_w - 28., 16., 10.5, 0xa7f3d0, false);

                self.st_button(p, t("update.restart_now", &lang), card_x + 14., card_y + 54., 150., Action::RestartNow, true);
                self.st_button(p, t("update.restart_later", &lang), card_x + 172., card_y + 54., 160., Action::RestartLater, true);
            },
            super::UpdateStatus::Applying => {
                p.fill_alpha(card_x, card_y, card_w, card_h, 8., 0x0284c7, 0.12);
                p.stroke(card_x, card_y, card_w, card_h, 8., 0x38bdf8, 0.8);
                p.text(t("update.restarting", &lang), card_x + 14., card_y + 28., card_w - 28., 20., 12.0, 0x38bdf8, true);
            },
            super::UpdateStatus::SuccessToast(version) => {
                p.fill_alpha(card_x, card_y, card_w, card_h, 8., 0x10b981, 0.10);
                p.stroke(card_x, card_y, card_w, card_h, 8., 0x10b981, 0.6);
                let msg = format!("{} {}", t("update.updated_to", &lang), version);
                p.text(&msg, card_x + 14., card_y + 28., card_w - 28., 20., 12.0, 0x34d399, true);
            },
            super::UpdateStatus::Error(e) => {
                p.fill_alpha(card_x, card_y, card_w, card_h, 8., 0xef4444, 0.10);
                p.stroke(card_x, card_y, card_w, card_h, 8., 0xef4444, 0.6);
                p.text(t("update.error", &lang), card_x + 14., card_y + 16., card_w - 28., 20., 12.0, 0xf87171, true);
                p.text_ellipsis(e, card_x + 14., card_y + 38., card_w - 28., 30., 10.0, 0xfca5a5, false);
            },
        }

        // Row 3: Official GitHub Repository
        let row3_y = if matches!(self.update_status, super::UpdateStatus::Available(_) | super::UpdateStatus::AwaitingRestart { .. }) { 276. } else { 264. };
        self.st_row_compact(p, row3_y, t("update.repo", &lang), t("update.repo_desc", &lang));
        self.st_button(p, t("update.open_repo", &lang), 418., row3_y + 6., 124., Action::OpenRepoUrl, true);
    }}
}

/// Index of `tab` inside the options list rendered by the "Tab default" dropdown. The dropdown
/// uses `active_tab_order()`, which can include `Catatan`/`Cuaca`, so indexing into `MODULES`
/// would highlight the wrong row.
pub(super) fn tab_option_index(order: &[String], tab: &str) -> u32 {
    order.iter().position(|t| t == tab).unwrap_or(0) as u32
}

/// Greedy word wrap for the mod detail panel. `Painter::measure_text` needs `&mut`.
pub(super) fn wrap_lines(p: &mut Painter, text: &str, width: f32, font: f32, max_lines: usize) -> Vec<String> { unsafe {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if line.is_empty() || p.measure_text(&candidate, font, false) <= width {
            line = candidate;
        } else {
            lines.push(std::mem::take(&mut line));
            if lines.len() == max_lines { return lines }
            line = word.to_string();
        }
    }
    if !line.is_empty() && lines.len() < max_lines { lines.push(line); }
    lines
}}

#[cfg(test)]
mod tests {
    use super::tab_option_index;

    #[test]
    fn default_tab_cursor_matches_the_visible_tab_order() {
        let order: Vec<String> = ["Musik", "Kalender", "Reminder", "Clipboard", "Catatan", "Cuaca"]
            .iter().map(|t| t.to_string()).collect();
        assert_eq!(tab_option_index(&order, "Musik"), 0);
        assert_eq!(tab_option_index(&order, "Catatan"), 4);
        assert_eq!(tab_option_index(&order, "Cuaca"), 5);
        assert_eq!(tab_option_index(&order, "Hilang"), 0);
    }
}

fn settings_menu_has_focus(focused:Option<&Action>,key:&'static str,_x:f32,_y:f32,_w:f32)->bool{
    matches!(focused,Some(Action::SettingsMenu(focused_key,..)) if *focused_key==key)
}
#[cfg(test)] mod recovery_tests{
    use super::*;
    #[test] fn settings_dropdown_focus_survives_scroll(){
        let focused=Action::SettingsMenu("material",418.,216.,124.);
        assert!(settings_menu_has_focus(Some(&focused),"material",418.,290.,124.));
        assert!(!settings_menu_has_focus(Some(&focused),"theme_color",418.,332.,124.));
    }

}
