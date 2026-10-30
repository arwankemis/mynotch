use super::*;
use crate::i18n::t;
use super::settings::wrap_lines;
const FG:u32=0xf3f6fc;const MUTED:u32=0xb9c5d5;const ACCENT:u32=0x69d2e5;
fn can_open(entry:&crate::mods::catalog::CatalogEntry)->bool{entry.is_active&&(entry.manifest.mod_type!=crate::mods::ModType::Builtin||entry.is_tab_enabled)}
pub(super) struct CatalogLayout{
    pub kinds:[render::D2D_RECT_F;2],pub filters:[render::D2D_RECT_F;2],pub refresh:render::D2D_RECT_F,
    pub list:render::D2D_RECT_F,pub detail:render::D2D_RECT_F,pub actions:[render::D2D_RECT_F;2],
}
impl CatalogLayout{
    pub fn new(width:f32,height:f32)->Self{
        let x=196.;let w=(width-x-18.).max(160.);let half=(w-4.)/2.;let filter=(w-48.)/2.;
        Self{kinds:[render::rect(x,116.,half,30.),render::rect(x+half+4.,116.,half,30.)],
            filters:[render::rect(x,154.,filter,30.),render::rect(x+filter+4.,154.,filter,30.)],refresh:render::rect(x+w-32.,154.,32.,30.),
            list:render::rect(x,194.,w,(height-214.).max(48.)),detail:render::rect(x,154.,w,(height-218.).max(60.)),
            actions:[render::rect(x,height-50.,(w-10.)/2.,30.),render::rect(x+(w+10.)/2.,height-50.,(w-10.)/2.,30.)]}
    }
    pub fn rows(&self)->usize{((self.list.bottom-self.list.top)/48.).floor().max(1.)as usize}
}
impl Shell{
    pub(super) fn catalog_snapshot(&mut self)->crate::mods::catalog::Catalog{
        if self.mods_catalog_cache.is_none(){self.mods_catalog_cache=Some(crate::mods::catalog::get_catalog(&self.store.root,&self.settings));}
        let mut result=self.mods_catalog_cache.clone().unwrap_or_default();
        for entry in &mut result.entries{entry.is_active=entry.is_installed&&!self.settings.disabled_mods.iter().any(|id|id==&entry.manifest.id);entry.is_tab_enabled=self.settings.mod_tab_enabled(&entry.manifest.tab_name);}
        result
    }

    unsafe fn catalog_segment(&mut self,p:&mut Painter,r:render::D2D_RECT_F,label:&str,on:bool,action:Action){unsafe{
        let hover=self.pointer.is_some_and(|(x,y)|x>=r.left&&x<=r.right&&y>=r.top&&y<=r.bottom);
        p.fill_alpha(r.left,r.top,r.right-r.left,r.bottom-r.top,8.,if on{0x69d2e5}else{0xffffff},if on{0.16}else if hover{0.12}else{0.05});
        if on{p.stroke(r.left,r.top,r.right-r.left,r.bottom-r.top,8.,ACCENT,0.7);}
        p.text_centered(label,r.left,r.top,r.right-r.left,r.bottom-r.top,11.5,if on{FG}else{MUTED},1.,on);
        if self.focused.as_ref()==Some(&action){p.stroke(r.left-1.,r.top-1.,r.right-r.left+2.,r.bottom-r.top+2.,9.,ACCENT,1.);}
        self.st_hit(r.left,r.top,r.right-r.left,r.bottom-r.top,action,true);
    }}
    pub(super) fn draw_catalog(&mut self,p:&mut Painter){unsafe{
        let lang=self.settings.language.clone();let l=CatalogLayout::new(self.width,self.height.min(360.));
        let entries=self.catalog_snapshot().filter(self.mods_kind,self.mods_filter,"");
        if self.mods_detail_open{
            if let Some(entry)=entries.iter().find(|e|self.mods_selected.as_deref()==Some(&e.manifest.id)){
                self.draw_catalog_detail(p,&l,entry);return;
            }
            self.mods_detail_open=false;
        }
        self.catalog_segment(p,l.kinds[0],t("mods.tab_mods",&lang),self.mods_kind==crate::mods::package::PackageKind::Mod,Action::SetModKind(crate::mods::package::PackageKind::Mod));
        self.catalog_segment(p,l.kinds[1],t("mods.tab_extensions",&lang),self.mods_kind==crate::mods::package::PackageKind::Extension,Action::SetModKind(crate::mods::package::PackageKind::Extension));
        self.catalog_segment(p,l.filters[0],t("mods.filter_all",&lang),self.mods_filter==crate::mods::catalog::CatalogFilter::All,Action::SetModFilter(crate::mods::catalog::CatalogFilter::All));
        self.catalog_segment(p,l.filters[1],t("mods.filter_installed",&lang),self.mods_filter==crate::mods::catalog::CatalogFilter::Installed,Action::SetModFilter(crate::mods::catalog::CatalogFilter::Installed));
        p.fill_alpha(l.refresh.left,l.refresh.top,32.,30.,8.,0xffffff,0.06);p.outline_icon("reset",l.refresh.left+8.,l.refresh.top+7.,16.,MUTED);
        self.st_hit(l.refresh.left,l.refresh.top,32.,30.,Action::RefreshMods,true);
        let v=l.list;self.mods_scroll=crate::core::clamp_list_scroll(self.mods_scroll,entries.len(),l.rows());
        if entries.is_empty(){p.text(t("mods.empty",&lang),v.left+12.,v.top+18.,v.right-v.left-24.,52.,12.,MUTED,false);}
        for (i,entry) in entries.iter().skip(self.mods_scroll).take(l.rows()).enumerate(){
            let m=&entry.manifest;let y=v.top+i as f32*48.;let action=Action::SelectMod(m.id.clone());
            let hover=self.pointer.is_some_and(|(x,py)|x>=v.left&&x<=v.right&&py>=y&&py<=y+44.);
            p.fill_alpha(v.left,y,v.right-v.left,44.,9.,0xffffff,if hover{0.11}else{0.045});
            p.outline_icon(&m.icon,v.left+12.,y+13.,18.,if entry.is_installed{ACCENT}else{MUTED});
            p.text_ellipsis(&m.name,v.left+42.,y+5.,v.right-v.left-76.,19.,12.,FG,true);
            let status=if entry.is_installed{if entry.is_active{t("mods.status_installed",&lang)}else{t("reminders.inactive",&lang)}}else if entry.available{t("mods.status_available",&lang)}else{t("mods.status_planned",&lang)};
            p.text_ellipsis(status,v.left+42.,y+25.,v.right-v.left-76.,15.,10.5,MUTED,false);
            p.outline_icon("right",v.right-24.,y+15.,12.,MUTED);
            if self.focused.as_ref()==Some(&action){p.stroke(v.left,y,v.right-v.left,44.,9.,ACCENT,1.);}
            self.st_hit(v.left,y,v.right-v.left,44.,action,true);
        }
        if entries.len()>l.rows(){let h=v.bottom-v.top;let thumb=h*l.rows()as f32/entries.len()as f32;let yy=v.top+(h-thumb)*self.mods_scroll as f32/(entries.len()-l.rows())as f32;p.fill(548.,yy,3.,thumb,1.5,0x8794a8);}
    }}
    unsafe fn draw_catalog_detail(&mut self,p:&mut Painter,l:&CatalogLayout,entry:&crate::mods::catalog::CatalogEntry){unsafe{
        let lang=self.settings.language.clone();let m=&entry.manifest;let v=l.detail;
        self.st_button(p,if lang=="en"{"‹ Package list"}else{"‹ Daftar paket"},196.,116.,126.,Action::BackModsList,true);
        let bottom=if entry.is_installed{400.}else{260.};let max=(bottom-v.bottom).max(0.);let scroll=self.mods_detail_scroll.clamp(0.,max);self.mods_detail_scroll=scroll;
        let hits=self.hits.len();let pointer=self.pointer;
        p.clip(v.left,v.top,v.right-v.left,v.bottom-v.top);p.translate_relative(0.,-scroll);self.pointer=pointer.map(|(x,y)|(x,y+scroll));
        p.outline_icon(&m.icon,198.,159.,24.,ACCENT);p.text_ellipsis(&m.name,232.,154.,304.,23.,15.,FG,true);
        p.text_ellipsis(&format!("v{} · {}",m.version,m.author),232.,181.,304.,18.,11.,MUTED,false);
        let desc_y=if entry.is_installed{312.}else{208.};
        for (i,line)in wrap_lines(p,&m.description,340.,11.5,2).iter().enumerate(){p.text(line,196.,desc_y+i as f32*17.,340.,17.,11.5,MUTED,false);}
        if entry.is_installed{
            self.st_row_compact(p,204.,if lang=="en"{"Active"}else{"Aktif"},if lang=="en"{"Enable this package"}else{"Jalankan fitur paket ini"});
            self.st_toggle(p,512.,215.,entry.is_active,Action::ToggleModActive(m.id.clone()),true);
            if !m.tab_name.is_empty(){self.st_row_compact(p,252.,if lang=="en"{"Show tab"}else{"Tampilkan tab"},&m.tab_name);self.st_toggle(p,512.,263.,entry.is_tab_enabled,Action::ToggleModTab(m.tab_name.clone()),true);}
            let permissions=if m.permissions.is_empty(){if lang=="en"{"No additional capabilities".into()}else{"Tanpa kapabilitas tambahan".into()}}else{m.permissions.join(" · ")};
            p.text_ellipsis(&permissions,196.,362.,340.,22.,11.,MUTED,false);
        }
        p.translate_relative(0.,scroll);p.unclip();self.pointer=pointer;
        let layout=super::settings_layout::SettingsLayout{viewport:v,scroll_y:scroll,max_scroll_y:max};
        for mut h in self.hits.split_off(hits){if let Some(r)=layout.visible_hit(h.rect){h.rect=r;self.hits.push(h);}}
        if max>0.{let height=v.bottom-v.top;let thumb=height*height/(height+max);p.fill(548.,v.top+(height-thumb)*scroll/max,3.,thumb,1.5,0x8794a8);}
        let a=l.actions;
        if entry.is_installed{
            let open=if m.mod_type==crate::mods::ModType::Builtin{Action::SettingsOpenTab(m.tab_name.clone(),0)}else{Action::OpenMod(m.id.clone())};
            self.st_button(p,t("mods.open_btn",&lang),a[0].left,a[0].top,a[0].right-a[0].left,open,can_open(entry));
            self.st_button(p,t("mods.uninstall_btn",&lang),a[1].left,a[1].top,a[1].right-a[1].left,Action::UninstallMod(m.id.clone()),true);
        }else{self.st_button(p,t(if entry.available{"mods.install_btn"}else{"mods.status_planned"},&lang),a[0].left,a[0].top,a[1].right-a[0].left,Action::InstallMod(m.id.clone()),entry.available);}
    }}
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn builtin_open_is_disabled_when_its_tab_is_hidden(){
        let settings=Settings::default();let root=std::env::temp_dir().join("mynotch-open-policy-empty");
        let mut entry=crate::mods::catalog::get_catalog(&root,&settings).entries.into_iter().find(|e|e.manifest.id=="sticky_notes").unwrap();
        entry.is_installed=true;entry.is_active=true;entry.is_tab_enabled=false;assert!(!can_open(&entry));
        entry.is_tab_enabled=true;assert!(can_open(&entry));entry.is_active=false;assert!(!can_open(&entry));
    }
    #[test]fn catalog_segments_align_and_actions_do_not_overlap_content(){
        for h in [360.,390.]{let l=CatalogLayout::new(560.,h);assert_eq!(l.kinds[0].left,l.filters[0].left);assert_eq!(l.kinds[1].right,l.refresh.right);assert!(l.filters[1].right+12.<=l.refresh.left);assert!(l.detail.bottom+14.<=l.actions[0].top);assert!(l.actions[0].right+10.<=l.actions[1].left);assert_eq!(l.rows(),3);}
    }
}
