//! Generation-checked editor ownership and a flush barrier shared by notes/presets.
//! Shell registers panel/pop-out HWNDs and flushes this barrier before lifecycle actions.
#![allow(dead_code)]
use std::collections::BTreeMap;
use crate::core::State;

#[derive(Clone,Copy,Debug,PartialEq,Eq,PartialOrd,Ord)]
pub enum EditorOwner {Panel,Popout}
#[derive(Clone,Copy,Debug,PartialEq,Eq,PartialOrd,Ord)]
pub struct Lease {pub note_id:u64,pub generation:u64,pub owner:EditorOwner}
#[derive(Clone,PartialEq,Eq)]
pub struct EditorText {pub title:Option<String>,pub body:String}
#[derive(Clone,PartialEq,Eq)]
pub struct EditorUpdate {pub lease:Lease,pub revision:u64,pub text:EditorText,pub closed:bool}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum EditorError {Unavailable,Busy,Conflict,Exhausted,CaptureFailed}
impl EditorError {pub fn message(self)->&'static str{match self{Self::Conflict=>"Konflik edit belum diselesaikan",Self::Unavailable=>"Editor tidak tersedia",Self::Busy=>"Catatan memiliki editor lain",Self::Exhausted=>"Identitas editor habis",Self::CaptureFailed=>"Edit belum dapat dibaca"}}}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Acquisition {New(Lease),Existing(Lease)}
#[derive(Clone)]
struct Entry {lease:Lease,revision:u64,text:EditorText}
#[derive(Clone,Default)]
pub struct EditorRegistry {
    next_generation:u64,active:BTreeMap<u64,Entry>,retired:BTreeMap<Lease,(u64,EditorText)>,
    pub conflict_drafts:Vec<EditorUpdate>,
    pending_updates:Vec<EditorUpdate>,
}
pub trait SnapshotSource {fn snapshots(&mut self)->Result<Vec<EditorUpdate>,EditorError>;}
impl EditorRegistry {
    pub fn acquire(&mut self,state:&State,note_id:u64,owner:EditorOwner)->Result<Acquisition,EditorError>{
        if state.sticky_notes.iter().filter(|n|n.id==note_id).count()!=1{return Err(EditorError::Unavailable);}
        if let Some(entry)=self.active.get(&note_id){return if entry.lease.owner==owner{Ok(Acquisition::Existing(entry.lease))}else{Err(EditorError::Busy)};}
        let generation=self.next_generation.max(1);let next=generation.checked_add(1).ok_or(EditorError::Exhausted)?;
        let note=state.sticky_notes.iter().find(|n|n.id==note_id).unwrap();
        let lease=Lease{note_id,generation,owner};self.next_generation=next;
        self.active.insert(note_id,Entry{lease,revision:0,text:EditorText{title:if owner==EditorOwner::Panel{Some(note.title.clone())}else{None},body:note.content.clone()}});
        Ok(Acquisition::New(lease))
    }
    pub fn panel_read_only(&self,note_id:u64)->bool{self.active.get(&note_id).is_some_and(|e|e.lease.owner==EditorOwner::Popout)}
    pub fn current(&self,note_id:u64)->Option<Lease>{self.active.get(&note_id).map(|e|e.lease)}
    /// Roll back only a newly acquired lease when no native editor could be created.
    pub fn cancel_unbound(&mut self,lease:Lease)->bool{
        if !self.active.get(&lease.note_id).is_some_and(|e|e.lease==lease&&e.revision==0){return false;}
        let e=self.active.remove(&lease.note_id).unwrap();self.retired.insert(lease,(e.revision,e.text));true
    }
    pub fn pending_count(&self)->usize{self.pending_updates.len()}
    fn apply_one(&mut self,state:&mut State,update:&EditorUpdate,now:&str)->Result<bool,EditorError>{
        if update.lease.owner==EditorOwner::Panel&&update.text.title.is_none(){return Err(EditorError::CaptureFailed);}
        let same=self.active.get(&update.lease.note_id).is_some_and(|e|e.lease==update.lease);
        if !same{
            return match self.retired.get(&update.lease){
                Some((revision,text)) if *revision==update.revision&&text==&update.text=>Ok(false),
                _=>Err(EditorError::Conflict),
            };
        }
        let entry=self.active.get_mut(&update.lease.note_id).unwrap();
        if update.revision<entry.revision||(update.revision==entry.revision&&update.text!=entry.text){
            return if update.text==entry.text{Ok(false)}else{Err(EditorError::Conflict)};
        }
        if state.sticky_notes.iter().filter(|n|n.id==update.lease.note_id).count()!=1{return Err(EditorError::Unavailable);}
        let note=state.sticky_notes.iter_mut().find(|n|n.id==update.lease.note_id).unwrap();
        let title=update.text.title.as_ref().map(|s|if s.trim().is_empty(){"Catatan Baru".to_string()}else{s.clone()});
        let changed=title.as_ref().is_some_and(|t|t!=&note.title)||note.content!=update.text.body;
        if let Some(title)=title{note.title=title;}
        note.content=update.text.body.clone();if changed{note.updated_at=now.into();}
        entry.revision=update.revision;entry.text=update.text.clone();
        if update.closed{
            self.active.remove(&update.lease.note_id);self.retired.insert(update.lease,(update.revision,update.text.clone()));
        }
        Ok(changed)
    }
    pub fn apply_batch(&mut self,state:&mut State,incoming:Vec<EditorUpdate>,now:&str)->Result<usize,EditorError>{
        let mut updates=self.pending_updates.clone();for update in incoming{if !updates.contains(&update){updates.push(update);}}
        if !self.conflict_drafts.is_empty(){self.pending_updates=updates;return Err(EditorError::Conflict);}
        updates.sort_by_key(|u|(u.lease,u.revision,u.closed));
        let mut model=state.clone();let mut registry=self.clone();let mut changes=0;
        registry.pending_updates.clear();
        for update in &updates{
            match registry.apply_one(&mut model,&update,now){Ok(changed)=>changes+=usize::from(changed),Err(error)=>{
                if error==EditorError::Conflict&&!self.conflict_drafts.contains(update){self.conflict_drafts.push(update.clone());}
                self.pending_updates=updates;
                return Err(error);
            }}
        }
        *state=model;*self=registry;Ok(changes)
    }
    pub fn flush(&mut self,state:&mut State,source:&mut impl SnapshotSource,now:&str)->Result<usize,EditorError>{
        if !self.conflict_drafts.is_empty(){return Err(EditorError::Conflict);}
        let updates=source.snapshots()?;self.apply_batch(state,updates,now)
    }
}
pub struct FlushSession<'a,P:SnapshotSource>{pub state:&'a mut State,pub registry:&'a mut EditorRegistry,pub source:&'a mut P,pub now:&'a str}
impl<P:SnapshotSource> crate::presets::EditorFlush for FlushSession<'_,P>{
    fn flush_editors(&mut self)->Result<(),&'static str>{self.registry.flush(self.state,self.source,self.now).map(|_|()).map_err(EditorError::message)}
}

/// The host must bind only its own live controls to the corresponding lease.
/// There is no enumeration, window activation, write, or close operation here.
pub mod native {
    use super::*;
    use windows::Win32::{Foundation::HWND,UI::WindowsAndMessaging::{GetWindowTextLengthW,GetWindowTextW,IsWindow}};
    struct Binding {lease:Lease,title:Option<HWND>,body:HWND,revision:u64,last:Option<EditorText>}
    #[derive(Default)]pub struct NativeSnapshots {bindings:BTreeMap<Lease,Binding>,closed:Vec<EditorUpdate>}
    fn read(hwnd:HWND)->Result<String,EditorError>{unsafe{
        if !IsWindow(Some(hwnd)).as_bool(){return Err(EditorError::CaptureFailed);}
        let len=GetWindowTextLengthW(hwnd).max(0) as usize;let mut buffer=vec![0;len+1];
        let copied=GetWindowTextW(hwnd,&mut buffer).max(0) as usize;
        if len>0&&copied==0{return Err(EditorError::CaptureFailed);}
        Ok(String::from_utf16_lossy(&buffer[..copied]))
    }}
    fn capture(binding:&mut Binding,closed:bool)->Result<EditorUpdate,EditorError>{
        let text=EditorText{title:binding.title.map(read).transpose()?,body:read(binding.body)?};
        if binding.last.as_ref()!=Some(&text){binding.revision=binding.revision.checked_add(1).ok_or(EditorError::Exhausted)?;binding.last=Some(text.clone());}
        Ok(EditorUpdate{lease:binding.lease,revision:binding.revision,text,closed})
    }
    impl NativeSnapshots {
        pub fn register(&mut self,lease:Lease,title:Option<HWND>,body:HWND)->Result<(),EditorError>{
            if lease.owner==EditorOwner::Panel&&title.is_none(){return Err(EditorError::CaptureFailed);}
            if self.bindings.contains_key(&lease){return Ok(());}
            if self.bindings.keys().any(|old|old.note_id==lease.note_id){return Err(EditorError::Busy);}
            self.bindings.insert(lease,Binding{lease,title,body,revision:0,last:None});Ok(())
        }
        pub fn mark_changed(&mut self,lease:Lease)->Result<(),EditorError>{
            let binding=self.bindings.get_mut(&lease).ok_or(EditorError::Unavailable)?;
            binding.revision=binding.revision.checked_add(1).ok_or(EditorError::Exhausted)?;Ok(())
        }
        pub fn capture_close(&mut self,lease:Lease)->Result<(),EditorError>{
            let binding=self.bindings.get_mut(&lease).ok_or(EditorError::Unavailable)?;
            let update=capture(binding,true)?;self.closed.push(update);self.bindings.remove(&lease);Ok(())
        }
    }
    impl SnapshotSource for NativeSnapshots {
        fn snapshots(&mut self)->Result<Vec<EditorUpdate>,EditorError>{
            let mut updates=Vec::new();for binding in self.bindings.values_mut(){updates.push(capture(binding,false)?);}
            updates.extend(std::mem::take(&mut self.closed));Ok(updates)
        }
    }
    #[cfg(test)]mod win32_adapter_tests{
        use super::*;
        use windows::{core::{w,PCWSTR},Win32::UI::WindowsAndMessaging::*};
        struct Hidden(HWND);
        impl Hidden{fn new(text:&str)->Self{unsafe{
            let value:Vec<_>=text.encode_utf16().chain(Some(0)).collect();
            // Hidden controls owned by this test; no user window, focus or data.
            Self(CreateWindowExW(WINDOW_EX_STYLE::default(),w!("EDIT"),PCWSTR(value.as_ptr()),WS_POPUP,0,0,1,1,None,None,None,None).unwrap())
        }}fn set(&self,text:&str){unsafe{let value:Vec<_>=text.encode_utf16().chain(Some(0)).collect();SetWindowTextW(self.0,PCWSTR(value.as_ptr())).unwrap();}}}
        impl Drop for Hidden{fn drop(&mut self){unsafe{let _=DestroyWindow(self.0);}}}
        fn state()->State{let mut s=State::default();s.sticky_notes.push(crate::sticky::StickyNote{id:1,title:"original".into(),content:"old".into(),color_idx:0,updated_at:String::new()});s}
        #[test]fn native_empty_panel_snapshot_is_committed_before_release(){
            let mut state=state();let mut registry=EditorRegistry::default();
            let Acquisition::New(lease)=registry.acquire(&state,1,EditorOwner::Panel).unwrap()else{panic!()};
            let title=Hidden::new("original");let body=Hidden::new("old");let mut source=NativeSnapshots::default();source.register(lease,Some(title.0),body.0).unwrap();
            title.set("");body.set("");source.mark_changed(lease).unwrap();source.capture_close(lease).unwrap();registry.flush(&mut state,&mut source,"now").unwrap();
            assert_eq!(state.sticky_notes[0].title,"Catatan Baru");assert_eq!(state.sticky_notes[0].content,"");assert_eq!(registry.current(1),None);
        }
        #[test]fn live_float_flush_reads_latest_without_closing_or_panel_overwrite(){
            let mut state=state();let mut registry=EditorRegistry::default();let Acquisition::New(lease)=registry.acquire(&state,1,EditorOwner::Popout).unwrap()else{panic!()};
            let body=Hidden::new("old");let mut source=NativeSnapshots::default();source.register(lease,None,body.0).unwrap();body.set("最新🦀\ntext");source.mark_changed(lease).unwrap();
            registry.flush(&mut state,&mut source,"now").unwrap();assert_eq!(state.sticky_notes[0].content,"最新🦀\ntext");assert!(registry.panel_read_only(1));
            source.capture_close(lease).unwrap();registry.flush(&mut state,&mut source,"later").unwrap();assert!(!registry.panel_read_only(1));assert_eq!(state.sticky_notes[0].title,"original");
        }
        #[test]fn invalid_native_binding_blocks_barrier_without_clearing_source(){
            let mut state=state();let mut registry=EditorRegistry::default();let Acquisition::New(lease)=registry.acquire(&state,1,EditorOwner::Panel).unwrap()else{panic!()};
            let title=Hidden::new("title");let body=Hidden::new("body");let mut source=NativeSnapshots::default();source.register(lease,Some(title.0),body.0).unwrap();drop(body);
            assert_eq!(registry.flush(&mut state,&mut source,"now"),Err(EditorError::CaptureFailed));assert_eq!(state.sticky_notes[0].content,"old");
        }
        #[test]fn live_native_float_convert_export_and_restart_use_the_same_latest_body(){
            let mut state=state();state.sticky_notes[0].title="Catatan Baru".into();let mut registry=EditorRegistry::default();
            let Acquisition::New(lease)=registry.acquire(&state,1,EditorOwner::Popout).unwrap()else{panic!()};
            let body=Hidden::new("old");let mut source=NativeSnapshots::default();source.register(lease,None,body.0).unwrap();body.set("最新🦀");source.mark_changed(lease).unwrap();
            let root=std::env::temp_dir().join(format!("native-links-{}-{}",std::process::id(),chrono::Local::now().timestamp_nanos_opt().unwrap()));let mut store=crate::core::Store::new(root.clone()).unwrap();
            let access=crate::todo_notes::NoteAccess{notes_enabled:true,quiet:false};
            let created=crate::todo_notes::create_from_note(&mut state,1,access,&mut registry,&mut source,&mut store,"now");
            let crate::todo_notes::CreateResult::Created(id)=created else{panic!("conversion failed")};
            assert_eq!(state.todos[0].text,"最新🦀");assert_eq!(state.todos[0].note_id,Some(1));assert!(registry.panel_read_only(1));
            assert_eq!(crate::todo_notes::create_from_note(&mut state,1,access,&mut registry,&mut source,&mut store,"now"),crate::todo_notes::CreateResult::Existing(id));
            let path=crate::todo_notes::export_latest(&mut state,1,access,&mut registry,&mut source,&mut store,&root.join("exports"),crate::todo_notes::ExportFormat::Text,"now").unwrap();
            assert!(std::fs::read_to_string(path).unwrap().contains("最新🦀"));
            let reloaded:State=store.load("state.json");assert_eq!(reloaded.todos[0].note_id,Some(1));assert_eq!(reloaded.sticky_notes[0].content,"最新🦀");
        }
    }
}
