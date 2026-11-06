//! Local Todo/note schema and commands, shared by the native Notes|Tasks presenter.
#![allow(dead_code)]
use std::{collections::BTreeMap,path::{Path,PathBuf}};
use crate::{core::{State,Store,Todo},note_editors::{EditorRegistry,SnapshotSource,EditorError},sticky::StickyNote};

#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub struct MigrationStatus {pub ambiguous_notes:Vec<u64>,pub ambiguous_todos:Vec<u64>,pub note_ids_exhausted:bool,pub todo_ids_exhausted:bool}
fn duplicates(ids:impl Iterator<Item=u64>)->Vec<u64>{let mut counts=BTreeMap::new();for id in ids{*counts.entry(id).or_insert(0usize)+=1;}counts.into_iter().filter_map(|(id,n)|(n>1).then_some(id)).collect()}
impl State {
    pub fn normalize_note_links(&mut self)->MigrationStatus{
        let max_note=self.sticky_notes.iter().map(|n|n.id).chain(self.todos.iter().filter_map(|t|t.note_id)).max();
        let max_todo=self.todos.iter().map(|t|t.id).max();
        self.next_note_id=self.next_note_id.max(1).max(max_note.map(|n|n.saturating_add(1)).unwrap_or(1));
        self.next_todo_id=self.next_todo_id.max(1).max(max_todo.map(|n|n.saturating_add(1)).unwrap_or(1));
        MigrationStatus{ambiguous_notes:duplicates(self.sticky_notes.iter().map(|n|n.id)),ambiguous_todos:duplicates(self.todos.iter().map(|t|t.id)),note_ids_exhausted:self.next_note_id==u64::MAX,todo_ids_exhausted:self.next_todo_id==u64::MAX}
    }
    pub fn allocate_note_id(&mut self)->Result<u64,LinkError>{self.normalize_note_links();let id=self.next_note_id;self.next_note_id=id.checked_add(1).ok_or(LinkError::Exhausted)?;Ok(id)}
    pub fn allocate_todo_id(&mut self)->Result<u64,LinkError>{self.normalize_note_links();let id=self.next_todo_id;self.next_todo_id=id.checked_add(1).ok_or(LinkError::Exhausted)?;Ok(id)}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct NoteAccess {pub notes_enabled:bool,pub quiet:bool}
impl NoteAccess {
    pub fn from_settings(base:&crate::core::Settings)->Self{Self{notes_enabled:base.enabled("Catatan"),quiet:base.effective_presentation().quiet}}
    fn allowed(self)->bool{self.notes_enabled&&!self.quiet}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum LinkError {Unavailable,Ambiguous,Exhausted,Flush(EditorError),SaveFailed}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum CreateResult {Created(u64),Existing(u64),Empty,Unavailable,SaveFailed(u64),SourceSaveFailed,FlushFailed(EditorError),Exhausted}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum NoteTarget {Note{id:u64,index:usize},TasksSubview{todo_id:u64},ReminderFallback}
fn note_index(state:&State,id:u64)->Result<usize,LinkError>{
    let mut matching=state.sticky_notes.iter().enumerate().filter(|(_,n)|n.id==id);
    let index=matching.next().map(|(i,_)|i).ok_or(LinkError::Unavailable)?;
    if matching.next().is_some(){Err(LinkError::Ambiguous)}else{Ok(index)}
}
fn todo_index(state:&State,id:u64)->Result<usize,LinkError>{
    let mut matching=state.todos.iter().enumerate().filter(|(_,t)|t.id==id);
    let index=matching.next().map(|(i,_)|i).ok_or(LinkError::Unavailable)?;
    if matching.next().is_some(){Err(LinkError::Ambiguous)}else{Ok(index)}
}
pub fn todo_is_unambiguous(state:&State,id:u64)->bool{todo_index(state,id).is_ok()}
pub fn derived_task_text(note:&StickyNote)->Option<String>{
    let title=note.title.trim();
    let input=if !title.is_empty()&&!matches!(title,"Catatan Baru"|"New Note"){Some(title)}else{note.content.lines().map(str::trim).find(|s|!s.is_empty())}?;
    let text=input.split_whitespace().collect::<Vec<_>>().join(" ");if text.is_empty(){return None;}
    Some(if text.chars().count()>160{let mut short:String=text.chars().take(159).collect();short.push('…');short}else{text})
}
fn save(state:&State,store:&mut Store)->Result<(),LinkError>{store.save("state.json",state).map_err(|_|LinkError::SaveFailed)}
pub fn create_from_note(state:&mut State,id:u64,access:NoteAccess,editors:&mut EditorRegistry,source:&mut impl SnapshotSource,store:&mut Store,now:&str)->CreateResult{
    if !access.allowed()||note_index(state,id).is_err(){return CreateResult::Unavailable;}
    if let Err(error)=editors.flush(state,source,now){return CreateResult::FlushFailed(error);}
    let index=match note_index(state,id){Ok(i)=>i,Err(_)=>return CreateResult::Unavailable};
    let existing=state.todos.iter().find(|t|t.note_id==Some(id)&&!t.done).or_else(||state.todos.iter().find(|t|t.note_id==Some(id))).map(|t|t.id);
    if let Some(todo_id)=existing{
        if todo_index(state,todo_id).is_err(){return CreateResult::Unavailable;}
        return if save(state,store).is_ok(){CreateResult::Existing(todo_id)}else{CreateResult::SaveFailed(todo_id)};
    }
    let Some(text)=derived_task_text(&state.sticky_notes[index])else{
        // Flushing an intentionally emptied editor is still a source edit.
        return if save(state,store).is_ok(){CreateResult::Empty}else{CreateResult::SourceSaveFailed};
    };
    let todo_id=match state.allocate_todo_id(){Ok(id)=>id,Err(_)=>return CreateResult::Exhausted};
    state.todos.push(Todo{id:todo_id,text,done:false,due:None,reminded:false,note_id:Some(id)});
    if save(state,store).is_ok(){CreateResult::Created(todo_id)}else{CreateResult::SaveFailed(todo_id)}
}
pub fn open_note(state:&mut State,todo_id:u64,access:NoteAccess,editors:&mut EditorRegistry,source:&mut impl SnapshotSource,store:&mut Store,now:&str)->Result<NoteTarget,LinkError>{
    if !access.allowed(){return Err(LinkError::Unavailable);}
    let id=state.todos[todo_index(state,todo_id)?].note_id.ok_or(LinkError::Unavailable)?;
    note_index(state,id)?;editors.flush(state,source,now).map_err(LinkError::Flush)?;
    let index=note_index(state,id)?;let old_selected=state.selected_note;state.selected_note=index;
    if let Err(error)=save(state,store){state.selected_note=old_selected;return Err(error);}
    Ok(NoteTarget::Note{id,index})
}
pub fn detach(state:&mut State,todo_id:u64,quiet:bool,store:&mut Store)->Result<bool,LinkError>{
    if quiet{return Err(LinkError::Unavailable);}
    let index=todo_index(state,todo_id)?;let changed=state.todos[index].note_id.take().is_some();save(state,store)?;Ok(changed)
}
pub fn set_done(state:&mut State,todo_id:u64,done:bool,quiet:bool,store:&mut Store)->Result<(),LinkError>{
    if quiet{return Err(LinkError::Unavailable);}
    let index=todo_index(state,todo_id)?;state.todos[index].done=done;state.collect_due_todos(i64::MIN,false);save(state,store)
}
pub fn delete(state:&mut State,todo_id:u64,quiet:bool,store:&mut Store)->Result<(),LinkError>{
    if quiet{return Err(LinkError::Unavailable);}
    let index=todo_index(state,todo_id)?;
    let removed=state.todos.remove(index);
    if let Err(error)=save(state,store){state.todos.insert(index,removed);return Err(error);}
    Ok(())
}
pub fn resolve_activity_todo(state:&State,todo_id:u64,access:NoteAccess)->Result<NoteTarget,LinkError>{
    todo_index(state,todo_id)?;if access.quiet{return Err(LinkError::Unavailable);}
    Ok(if access.notes_enabled{NoteTarget::TasksSubview{todo_id}}else{NoteTarget::ReminderFallback})
}
#[derive(Clone,Copy)]pub enum ExportFormat{Markdown,Text}
pub fn export_latest(state:&mut State,id:u64,access:NoteAccess,editors:&mut EditorRegistry,source:&mut impl SnapshotSource,store:&mut Store,exports:&Path,format:ExportFormat,now:&str)->Result<PathBuf,LinkError>{
    if !access.allowed(){return Err(LinkError::Unavailable);}
    note_index(state,id)?;editors.flush(state,source,now).map_err(LinkError::Flush)?;
    let index=note_index(state,id)?;save(state,store)?;
    match format{ExportFormat::Markdown=>crate::sticky::export_note_to_markdown(exports,&state.sticky_notes[index]),ExportFormat::Text=>crate::sticky::export_note_to_txt(exports,&state.sticky_notes[index])}.map_err(|_|LinkError::SaveFailed)
}

#[cfg(test)]
#[path="todo_notes_tests.rs"]
mod tests;
