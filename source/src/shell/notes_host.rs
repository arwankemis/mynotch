//! Approved Notes | Tasks presenter and ID-based commands.
use super::*;
use crate::todo_notes::{self,CreateResult,NoteAccess,NoteTarget};

#[derive(Default)]
pub(super) struct NotesHost {
    pub(super) tasks:bool,pub(super) selected:Option<u64>,pub(super) page:usize,
    pub(super) revision:u64,signature:Vec<(u64,bool,Option<u64>)>,
}
impl NotesHost {
    fn refresh(&mut self,state:&State)->bool{
        let next=state.todos.iter().map(|t|(t.id,t.done,t.note_id)).collect::<Vec<_>>();
        let changed=self.signature!=next;
        if changed{self.signature=next;self.revision=self.revision.saturating_add(1);}
        if !self.selected.is_some_and(|id|todo_notes::todo_is_unambiguous(state,id)) {self.selected=state.todos.iter().find(|t|todo_notes::todo_is_unambiguous(state,t.id)).map(|t|t.id);}
        self.page=self.page.min(state.todos.len().saturating_sub(1)/5);
        changed
    }
    fn select(&mut self,state:&State,id:u64)->bool{
        if !todo_notes::todo_is_unambiguous(state,id){return false;}
        self.selected=Some(id);self.page=state.todos.iter().position(|t|t.id==id).unwrap()/5;true
    }
    fn valid(&self,state:&State,id:u64,revision:u64)->bool {self.tasks&&self.revision==revision&&todo_notes::todo_is_unambiguous(state,id)}
}
impl Shell {
    pub(super) fn refresh_notes_host(&mut self){
        if self.notes_host.refresh(&self.state){self.clear_note_hits();}
    }
    pub(super) fn clear_note_hits(&mut self){
        self.pressed=None;self.focused=None;self.note_export_open=None;self.hits.clear();
    }
    pub(super) fn notes_subview(&mut self,tasks:bool){
        if self.settings.quiet_mode||!self.flush_editors_barrier(){return;}
        self.simpan_sub=if tasks{2}else{0};
        self.notes_host.tasks=tasks;self.notes_host.revision=self.notes_host.revision.saturating_add(1);
        self.refresh_notes_host();self.clear_note_hits();self.make_inputs();self.invalidate();
    }
    pub(super) fn show_todo(&mut self,id:u64){
        if self.settings.quiet_mode||!self.settings.enabled("Catatan")||!self.notes_host.select(&self.state,id){return;}
        self.tab="Catatan".into();self.simpan_sub=2;self.notes_host.tasks=true;self.settings_open=false;self.privacy_detail_open=false;self.mod_view=None;
        self.refresh_notes_host();self.clear_note_hits();self.set_hover_stage(2);self.make_inputs();
        self.focused=Some(Action::TaskSelect(id,self.notes_host.revision));self.invalidate();
    }
    pub(super) fn convert_note(&mut self,id:u64){
        if !self.settings.enabled("Catatan")||self.settings.quiet_mode{return;}
        let now=Local::now().format("%Y-%m-%d %H:%M").to_string();
        let result={todo_notes::create_from_note(&mut self.state,id,NoteAccess::from_settings(&self.settings),&mut self.editors.registry,&mut *self.editors.source.borrow_mut(),&mut self.store,&now)};
        match result{
            CreateResult::Created(todo)|CreateResult::Existing(todo)=>{self.editors.unsaved=false;self.show_todo(todo);},
            CreateResult::SaveFailed(_)|CreateResult::SourceSaveFailed=>{self.editors.unsaved=true;self.invalidate();},
            CreateResult::Empty=>{self.editors.unsaved=false;self.activity_feedback(if self.settings.language=="en"{"The note is empty"}else{"Catatan kosong"}.into(),4000);},
            _=>{self.editors.unsaved=true;self.activity_feedback("Tindakan ditahan; catatan tidak tersedia".into(),5000);},
        }
    }
    pub(super) fn select_note_id(&mut self,id:u64){
        if self.settings.quiet_mode||self.state.sticky_notes.iter().filter(|n|n.id==id).count()!=1||!self.flush_editors_barrier(){return;}
        self.state.selected_note=self.state.sticky_notes.iter().position(|n|n.id==id).unwrap();self.simpan_sub=0;self.notes_host.tasks=false;
        self.clear_note_hits();self.make_inputs();self.save();self.invalidate();
    }
    pub(super) fn task_action(&mut self,id:u64,revision:u64,operation:u8,value:bool){
        self.refresh_notes_host();
        if self.settings.quiet_mode||!self.settings.enabled("Catatan")||!self.notes_host.valid(&self.state,id,revision){return;}
        if operation!=0&&self.notes_host.selected!=Some(id){return;}
        if !self.flush_editors_barrier(){return;}
        match operation{
            0=>{self.notes_host.select(&self.state,id);self.notes_host.revision=self.notes_host.revision.saturating_add(1);},
            1=>{if todo_notes::set_done(&mut self.state,id,value,false,&mut self.store).is_err(){self.editors.unsaved=true;}},
            2=>{
                let now=Local::now().format("%Y-%m-%d %H:%M").to_string();
                let result={todo_notes::open_note(&mut self.state,id,NoteAccess::from_settings(&self.settings),&mut self.editors.registry,&mut *self.editors.source.borrow_mut(),&mut self.store,&now)};
                match result{Ok(NoteTarget::Note{..})=>{self.simpan_sub=0;self.notes_host.tasks=false;self.make_inputs();},_=>{self.editors.unsaved=true;} }
            },
            3=>{if todo_notes::detach(&mut self.state,id,false,&mut self.store).is_err(){self.editors.unsaved=true;}},_=>{},
        }
        self.refresh_notes_host();self.reconcile_activities();self.clear_note_hits();self.invalidate();
    }
    pub(super) fn delete_todo(&mut self,id:u64,revision:u64){
        self.refresh_notes_host();
        if self.settings.quiet_mode||!self.settings.enabled("Catatan")||!self.notes_host.valid(&self.state,id,revision)||self.notes_host.selected!=Some(id){return;}
        if !self.flush_editors_barrier(){return;}
        if todo_notes::delete(&mut self.state,id,false,&mut self.store).is_err(){self.editors.unsaved=true;}
        self.refresh_notes_host();self.reconcile_activities();self.clear_note_hits();self.make_inputs();self.invalidate();
    }
    pub(super) fn note_export_menu(&mut self,p:&mut Painter,id:u64){unsafe{
        let(x,y,w,h)=(316.,70.,124.,54.);
        p.fill_alpha(x,y,w,h,8.,0x141418,0.98);
        p.stroke(x,y,w,h,8.,0x50505a,1.);
        self.button_h(p,".txt",x+4.,y+4.,w-8.,22.,Action::NoteExportPick(id,1),false);
        self.button_h(p,"Sticky Note",x+4.,y+26.,w-8.,22.,Action::NoteExportPick(id,0),false);
    }}
    pub(super) fn notes_header(&mut self,p:&mut Painter){
        let en=self.settings.language=="en";
        self.button_h(p,if en{"Notes"}else{"Catatan"},22.,44.,66.,26.,Action::SimpanSub(0),self.simpan_sub==0);
        self.button_h(p,"Clipboard",90.,44.,70.,26.,Action::SimpanSub(1),self.simpan_sub==1);
        self.button_h(p,if en{"Tasks"}else{"Tugas"},162.,44.,58.,26.,Action::SimpanSub(2),self.simpan_sub==2);
    }
    /// Host for the merged "Saved" tab: Notes / Clipboard / Tasks sub-views.
    pub(super) fn simpan_page(&mut self,p:&mut Painter){
        self.notes_host.tasks=self.simpan_sub==2;
        self.notes_header(p);
        match self.simpan_sub{
            1=>{
                // Clipboard content is normally drawn from y=41; shift it below the
                // Simpan sub-tab header (and offset the hit rects by the same amount).
                const DY:f32=34.;
                unsafe{ p.translate_relative(0.,DY); }
                let first=self.hits.len();
                self.clipboard_page(p);
                unsafe{ p.translate_relative(0.,-DY); }
                for h in &mut self.hits[first..]{ h.rect.top+=DY; h.rect.bottom+=DY; }
            }
            2=>self.tasks_page(p),
            _=>self.notes_page(p),
        }
    }
    pub(super) fn notes_link_toolbar(&mut self,p:&mut Painter,id:u64){unsafe{
        let linked=self.state.todos.iter().any(|t|t.note_id==Some(id));
        let empty=self.state.sticky_notes.iter().find(|n|n.id==id).is_none_or(|n|todo_notes::derived_task_text(n).is_none());
        let en=self.settings.language=="en";
        let label=if self.editors.unsaved{if en{"Retry save"}else{"Coba simpan"}}else if linked{if en{"View task"}else{"Lihat tugas"}}else{if en{"Create task"}else{"Jadikan tugas"}};
        // Save retry remains possible even after an intentionally emptied note.
        if linked||!empty||self.editors.unsaved{self.button_h(p,label,256.,44.,140.,22.,Action::ConvertNote(id),false);}else{p.text_ellipsis(label,262.,47.,128.,20.,11.,0x666672,false);}
        let status=if self.editors.unsaved{if en{"Not saved — retry saving"}else{"Belum tersimpan — coba simpan lagi"}}else if self.editors.registry.panel_read_only(id){if en{"Editing in separate window · read-only"}else{"Diedit di jendela terpisah · panel readonly"}}else{""};
        if empty&&!linked&&!self.editors.unsaved{p.text(if en{"The note is empty"}else{"Catatan kosong"},256.,248.,264.,20.,9.,0x92929e,false);}
        if !status.is_empty(){p.text_ellipsis(status,256.,248.,264.,20.,9.,0xe8c87f,false);}
        if self.editors.registry.panel_read_only(id)||self.note_export_open==Some(id){
            if let Some(note)=self.state.sticky_notes.iter().find(|n|n.id==id){
                p.text_ellipsis(&note.title,262.,79.,250.,22.,12.,render::TEXT,true);
                p.text_ellipsis(&note.content,262.,111.,250.,96.,11.,render::TEXT,false);
            }
        }
    }}
    pub(super) fn tasks_page(&mut self,p:&mut Painter){unsafe{
        self.refresh_notes_host();
        let en=self.settings.language=="en";let revision=self.notes_host.revision;
        let start=self.notes_host.page*5;
        let rows:Vec<_>=self.state.todos.iter().skip(start).take(5).cloned().collect();
        for (i,todo) in rows.iter().enumerate(){
            let y=78.+i as f32*32.;let selected=self.notes_host.selected==Some(todo.id);
            self.button_h(p,"",22.,y,212.,29.,Action::TaskSelect(todo.id,revision),selected);
            p.text_ellipsis_vcenter(&format!("{} {}",if todo.done{"✓"}else{"○"},todo.text),30.,y,196.,29.,11.,render::TEXT,selected);
        }
        self.button_h(p,"‹",22.,245.,28.,24.,Action::TasksPage(-1),false);
        self.button_h(p,"›",56.,245.,28.,24.,Action::TasksPage(1),false);
        p.text(&format!("{} / {}",self.notes_host.page+1,self.state.todos.len().div_ceil(5).max(1)),96.,249.,130.,20.,10.,0x92929e,false);
        p.fill(244.,44.,1.,220.,0.,0x33333a);
        let selected=self.notes_host.selected.and_then(|id|self.state.todos.iter().find(|t|t.id==id)).cloned();
        if let Some(todo)=selected{
            self.button_h(p,if todo.done{if en{"✓ Done"}else{"✓ Selesai"}}else{if en{"Mark done"}else{"Tandai selesai"}},256.,44.,140.,26.,Action::TaskDone(todo.id,!todo.done,revision),todo.done);
            self.button_h(p,if en{"Delete"}else{"Hapus"},404.,44.,76.,26.,Action::DeleteTodo(todo.id,revision),false);
            p.text_ellipsis(&todo.text,256.,84.,264.,64.,13.,render::TEXT,true);
            let due=todo.due.map(|d|d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(||if en{"No due date"}else{"Tanpa tenggat"}.into());
            p.text_ellipsis(&due,256.,153.,264.,20.,10.,0x92929e,false);
            if let Some(note_id)=todo.note_id{
                let available=self.state.sticky_notes.iter().filter(|n|n.id==note_id).count()==1;
                if available{self.button_h(p,if en{"Open note"}else{"Buka catatan"},256.,182.,125.,28.,Action::OpenTodoNote(todo.id,revision),false);}else{p.text(if en{"Note unavailable"}else{"Catatan tidak tersedia"},256.,183.,264.,22.,11.,0x92929e,false);}
                self.button_h(p,if en{"Detach link"}else{"Lepas tautan"},256.,216.,125.,26.,Action::DetachTodoNote(todo.id,revision),false);
            }else{p.text(if en{"No note link"}else{"Tanpa tautan catatan"},256.,184.,264.,22.,11.,0x92929e,false);}
            if self.editors.unsaved{self.button_h(p,if en{"Retry save"}else{"Coba simpan"},386.,216.,134.,26.,Action::RetryNoteSave,false);}
        }
    }}
}

#[cfg(test)]mod tests{
 use super::*;
 fn todo(id:u64)->Todo{Todo{id,text:format!("legacy{id}"),done:false,due:None,reminded:false,note_id:None}}
 #[test]fn activity_route_pages_to_exact_legacy_task_and_rejects_stale_row(){
  let mut state=State::default();state.todos=(0..13).map(todo).collect();let mut host=NotesHost::default();host.tasks=true;host.refresh(&state);assert!(host.select(&state,12));assert_eq!(host.page,2);let revision=host.revision;
  state.todos.remove(3);host.refresh(&state);assert!(!host.valid(&state,12,revision));assert_eq!(host.selected,Some(12));assert!(host.select(&state,12));assert_eq!(host.page,2);
 }
 #[test]fn checkbox_replay_is_invalid_after_done_revision_and_dangling_link_stays(){
  let mut state=State::default();let mut t=todo(1);t.note_id=Some(99);state.todos.push(t);let mut host=NotesHost::default();host.tasks=true;host.refresh(&state);let revision=host.revision;
  state.todos[0].done=true;host.refresh(&state);assert!(!host.valid(&state,1,revision));assert_eq!(state.todos[0].note_id,Some(99));
 }
  #[test]fn every_page_is_reachable_and_ambiguous_ids_do_not_select_neighbours(){
   let mut state=State::default();state.todos=(0..29).map(todo).collect();let mut host=NotesHost::default();host.refresh(&state);
   for id in 0..29{assert!(host.select(&state,id));assert_eq!(host.page,id as usize/5);}
   state.todos.push(todo(28));host.refresh(&state);assert!(!host.select(&state,28));assert_ne!(host.selected,Some(28));
  }
  #[test]fn saved_tab_navigation_sets_the_matching_sub_view(){
   // Source guard: every entry point that switches the Saved tab must also set the
   // sub-view, otherwise the page opens on the wrong sub-tab (Notes/Clipboard/Tasks).
   let src=include_str!("notes_host.rs");
   assert!(src.contains("self.tab=\"Catatan\".into();self.simpan_sub=2;"));
   assert!(src.contains("self.simpan_sub=0;self.notes_host.tasks=false;"));
   assert!(src.contains("self.simpan_sub=if tasks{2}else{0};"));
   assert!(src.contains("pub(super) fn simpan_page"));
   assert!(src.contains("self.clipboard_page(p)"));
   assert!(src.contains("self.tasks_page(p)"));
   assert!(src.contains("self.notes_page(p)"));
  }
}
