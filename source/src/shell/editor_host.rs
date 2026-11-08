//! One editor pipeline for panel, pop-outs, privacy, navigation and persistence.
use super::*;
use crate::note_editors::{*,native::NativeSnapshots};
use std::rc::Rc;

#[derive(Default)]
pub(super) struct Editors {
    pub(super) registry:EditorRegistry,
    pub(super) source:Rc<RefCell<NativeSnapshots>>,
    pub(super) panel:Option<Lease>,
    pub(super) floats:BTreeMap<u64,(Lease,HWND)>,
    pub(super) unsaved:bool,
}
impl Shell {
    pub(super) fn commit_editors(&mut self)->bool{
        let now=Local::now().format("%Y-%m-%d %H:%M").to_string();
        let result=self.editors.registry.flush(&mut self.state,&mut *self.editors.source.borrow_mut(),&now);
        self.editors.floats.retain(|_,(_,h)|unsafe{IsWindow(Some(*h)).as_bool()});
        match result{Ok(changes)=>{self.editors.unsaved|=changes>0;true},Err(_)=>{self.editors.unsaved=true;false}}
    }
    pub(super) fn flush_editors_barrier(&mut self)->bool{
        let lang=self.settings.language.clone();
        if !self.commit_editors(){self.activity_feedback(crate::i18n::t("notice.edit_held",&lang).into(),5000);return false;}
        let result=self.store.save("state.json",&self.state);
        self.editors.unsaved=result.is_err();
        if result.is_err(){self.activity_feedback(crate::i18n::t("notice.save_hold",&lang).into(),5000);return false;}true
    }
    pub(super) fn retire_panel(&mut self)->bool{
        if !self.commit_editors(){return false;}
        if let Some(lease)=self.editors.panel {
            if self.editors.source.borrow_mut().capture_close(lease).is_err(){self.editors.unsaved=true;return false;}
            if !self.commit_editors(){return false;}
            self.editors.panel=None;
        }
        true
    }
    pub(super) fn bind_panel(&mut self){
        if self.settings.quiet_mode{return;}
        let Some(note)=self.state.sticky_notes.get(self.state.selected_note)else{return};
        if self.editors.registry.panel_read_only(note.id){return;}
        let(Some(&title),Some(&body))=(self.inputs.get(&50),self.inputs.get(&51))else{return};
        let acquired=self.editors.registry.acquire(&self.state,note.id,EditorOwner::Panel);
        match acquired{
            Ok(Acquisition::New(lease)|Acquisition::Existing(lease))=>{
                if self.editors.source.borrow_mut().register(lease,Some(title),body).is_ok(){self.editors.panel=Some(lease);}else{self.editors.unsaved=true;}
            },Err(_)=>self.editors.unsaved=true,
        }
    }
    pub(super) fn popout_note(&mut self){
        if self.settings.quiet_mode||!self.flush_editors_barrier(){return;}
        let Some(note)=self.state.sticky_notes.get(self.state.selected_note).cloned()else{return};
        if let Some((_,hwnd))=self.editors.floats.get(&note.id){unsafe{let _=ShowWindow(*hwnd,SW_SHOWNORMAL);let _=SetForegroundWindow(*hwnd);}return;}
        if !self.retire_panel(){return;}
        let lease=match self.editors.registry.acquire(&self.state,note.id,EditorOwner::Popout){Ok(Acquisition::New(l)|Acquisition::Existing(l))=>l,Err(_)=>{self.editors.unsaved=true;return;}};
        match sticky_window::spawn_owned_note(&note,lease,self.editors.source.clone(),self.hwnd,&self.settings.language){
            Ok(h)=>{self.editors.floats.insert(note.id,(lease,h));self.make_inputs();},
            Err(_)=>{self.editors.registry.cancel_unbound(lease);self.editors.unsaved=true;self.activity_feedback(crate::i18n::t("notice.editor_unavailable",&self.settings.language).into(),5000);},
        }
    }
    pub(super) fn sync_popout_privacy(&mut self){unsafe{
        for (id,(_,hwnd)) in &self.editors.floats{
            if !IsWindow(Some(*hwnd)).as_bool(){continue;}
            if self.settings.quiet_mode{let _=ShowWindow(*hwnd,SW_HIDE);let title:Vec<_>=popout_caption(true,"").encode_utf16().chain(Some(0)).collect();let _=SetWindowTextW(*hwnd,PCWSTR(title.as_ptr()));}
            else if let Some(note)=self.state.sticky_notes.iter().find(|n|n.id==*id){
                let title:Vec<_>=popout_caption(false,&note.title).encode_utf16().chain(Some(0)).collect();let _=SetWindowTextW(*hwnd,PCWSTR(title.as_ptr()));let _=ShowWindow(*hwnd,SW_SHOWNOACTIVATE);
            }
        }
    }}
}

fn popout_caption(quiet:bool,title:&str)->&str{if quiet{"Catatan"}else{title}}
#[cfg(test)]mod caption_sink_r1_tests{
 use super::*;
 #[test]fn caption_payload_used_by_native_setwindowtext_redacts_and_restores(){
  let sentinel="PRIVATE-caption-🦀";assert_eq!(popout_caption(true,sentinel),"Catatan");assert_eq!(popout_caption(false,sentinel),sentinel);
  let source=include_str!("editor_host.rs");let sink=source.split("fn sync_popout_privacy(").nth(1).unwrap().split("fn popout_caption(").next().unwrap();
  assert!(sink.contains("popout_caption(true,"));assert!(sink.contains("popout_caption(false,&note.title)"));assert!(sink.contains("SW_HIDE"));assert!(sink.contains("SW_SHOWNOACTIVATE"));
 }
}
