use super::*;
use crate::note_editors::*;
fn note(id:u64,title:&str,body:&str)->StickyNote{StickyNote{id,title:title.into(),content:body.into(),color_idx:2,updated_at:"before".into()}}
fn state()->State{let mut s=State::default();s.sticky_notes.push(note(9,"Catatan Baru","first line\nsecond"));s}
fn task(id:u64,note_id:Option<u64>,done:bool)->Todo{Todo{id,text:format!("task-{id}"),done,due:None,reminded:false,note_id}}
fn store()->Store{Store::new(std::env::temp_dir().join(format!("todo-note-fixture-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()))).unwrap()}
fn access()->NoteAccess{NoteAccess{notes_enabled:true,quiet:false}}
#[derive(Default)]struct Source{updates:Vec<EditorUpdate>,calls:usize,fail:bool}
impl SnapshotSource for Source{fn snapshots(&mut self)->Result<Vec<EditorUpdate>,EditorError>{self.calls+=1;if self.fail{Err(EditorError::CaptureFailed)}else{Ok(self.updates.clone())}}}
fn lease(reg:&mut EditorRegistry,s:&State,id:u64,owner:EditorOwner)->Lease{match reg.acquire(s,id,owner).unwrap(){Acquisition::New(l)|Acquisition::Existing(l)=>l}}
fn update(lease:Lease,revision:u64,title:Option<&str>,body:&str,closed:bool)->EditorUpdate{EditorUpdate{lease,revision,text:EditorText{title:title.map(str::to_owned),body:body.into()},closed}}
fn convert(s:&mut State,reg:&mut EditorRegistry,source:&mut Source,store:&mut Store)->CreateResult{create_from_note(s,9,access(),reg,source,store,"now")}

#[test]fn legacy_schema_normalization_is_repeatable_and_preserves_zero_id_and_order(){
    let mut s:State=serde_json::from_value(serde_json::json!({"todos":[{"id":0,"text":"legacy","done":true,"due":null}],"notes":"legacy separate notes","sticky_notes":[{"id":0,"title":"old","content":"body","color_idx":4,"updated_at":"old-time"}]})).unwrap();
    assert_eq!(s.todos[0].note_id,None);assert!(!s.todos[0].reminded);s.normalize_note_links();
    assert_eq!(s.next_note_id,1);assert_eq!(s.next_todo_id,1);let once=serde_json::to_vec(&s).unwrap();
    let mut twice:State=serde_json::from_slice(&once).unwrap();twice.normalize_note_links();assert_eq!(once,serde_json::to_vec(&twice).unwrap());
    assert_eq!(twice.sticky_notes[0].id,0);assert_eq!(twice.notes,"legacy separate notes");assert!(twice.todos[0].done);
}
#[test]fn allocator_reserves_dangling_ids_and_never_reuses_deleted_highest_ids(){
    let mut s=state();s.todos.push(task(50,Some(99),false));s.next_note_id=1000;s.normalize_note_links();
    assert_eq!(s.allocate_note_id().unwrap(),1000);s.sticky_notes.clear();assert_eq!(s.allocate_note_id().unwrap(),1001);
    assert_eq!(s.allocate_todo_id().unwrap(),51);s.todos.clear();assert_eq!(s.allocate_todo_id().unwrap(),52);
}
#[test]fn allocator_overflow_keeps_data_and_does_not_wrap(){
    let mut s=state();s.todos.push(task(u64::MAX,Some(u64::MAX),true));let before=serde_json::to_value(&s.todos).unwrap();
    assert_eq!(s.allocate_note_id(),Err(LinkError::Exhausted));assert_eq!(s.allocate_todo_id(),Err(LinkError::Exhausted));
    assert_eq!(serde_json::to_value(&s.todos).unwrap(),before);assert_eq!(s.sticky_notes[0].id,9);
}
#[test]fn duplicate_ids_remain_present_but_link_and_mutation_are_rejected(){
    let mut s=state();s.sticky_notes.push(note(9,"duplicate","other"));let report=s.normalize_note_links();assert_eq!(report.ambiguous_notes,vec![9]);
    let mut reg=EditorRegistry::default();let mut source=Source::default();let mut db=store();assert_eq!(convert(&mut s,&mut reg,&mut source,&mut db),CreateResult::Unavailable);assert_eq!(s.sticky_notes.len(),2);
    s.sticky_notes.pop();s.todos=vec![task(7,Some(9),false),task(7,None,true)];
    assert_eq!(convert(&mut s,&mut reg,&mut source,&mut db),CreateResult::Unavailable);assert_eq!(s.todos.len(),2);
    assert_eq!(set_done(&mut s,7,true,false,&mut db),Err(LinkError::Ambiguous));assert_eq!(detach(&mut s,7,false,&mut db),Err(LinkError::Ambiguous));
}
#[test]fn task_text_uses_scalar_limit_and_never_changes_note_bytes(){
    let n=note(1,"New Note",&format!("\n\t\n{}\nignored", "漢🎶".repeat(100)));let before=n.clone();let text=derived_task_text(&n).unwrap();
    assert_eq!(text.chars().count(),160);assert!(text.ends_with('…'));assert_eq!(n,before);
    assert_eq!(derived_task_text(&note(1,"  題目\t X  ","")),Some("題目 X".into()));
    assert_eq!(derived_task_text(&note(1,"Catatan Baru"," \n \t ")),None);
}
#[test]fn double_conversion_is_one_link_and_done_task_is_not_reopened(){
    let mut s=state();let original=s.sticky_notes[0].clone();let mut reg=EditorRegistry::default();let mut source=Source::default();let mut db=store();
    assert_eq!(convert(&mut s,&mut reg,&mut source,&mut db),CreateResult::Created(1));assert_eq!(convert(&mut s,&mut reg,&mut source,&mut db),CreateResult::Existing(1));
    assert_eq!(s.todos.len(),1);assert_eq!(s.todos[0].note_id,Some(9));assert_eq!(s.todos[0].text,"first line");assert!(s.todos[0].due.is_none()&&!s.todos[0].reminded);
    set_done(&mut s,1,true,false,&mut db).unwrap();assert_eq!(convert(&mut s,&mut reg,&mut source,&mut db),CreateResult::Existing(1));assert!(s.todos[0].done);assert_eq!(s.sticky_notes[0],original);
}
#[test]fn existing_multi_links_choose_first_undone_in_vector_order_and_preserve_all_records(){
    let mut s=state();s.todos=vec![task(8,Some(9),true),task(5,Some(9),false),task(3,Some(9),false)];
    let mut reg=EditorRegistry::default();let mut source=Source::default();let mut db=store();assert_eq!(convert(&mut s,&mut reg,&mut source,&mut db),CreateResult::Existing(5));
    assert_eq!(s.todos.iter().map(|t|t.id).collect::<Vec<_>>(),vec![8,5,3]);
}
#[test]fn failed_save_retains_the_created_identity_and_retry_does_not_duplicate(){
    let mut s=state();let mut db=store();std::fs::write(db.root.join("state.json"),b"{").unwrap();let _:State=db.load("state.json");
    let mut reg=EditorRegistry::default();let mut source=Source::default();assert_eq!(convert(&mut s,&mut reg,&mut source,&mut db),CreateResult::SaveFailed(1));
    assert_eq!(convert(&mut s,&mut reg,&mut source,&mut db),CreateResult::SaveFailed(1));assert_eq!(s.todos.len(),1);
    db.acknowledge_corrupt("state.json");assert_eq!(convert(&mut s,&mut reg,&mut source,&mut db),CreateResult::Existing(1));
    let restored:State=db.load("state.json");assert_eq!(restored.todos[0].note_id,Some(9));assert_eq!(restored.next_todo_id,2);
}
#[test]fn quiet_and_disabled_notes_reject_before_reading_any_editor(){
    let mut s=state();let before=serde_json::to_vec(&s).unwrap();let mut reg=EditorRegistry::default();let mut source=Source::default();let mut db=store();
    for policy in [NoteAccess{notes_enabled:false,quiet:false},NoteAccess{notes_enabled:true,quiet:true}]{assert_eq!(create_from_note(&mut s,9,policy,&mut reg,&mut source,&mut db,"now"),CreateResult::Unavailable);}
    assert_eq!(source.calls,0);assert_eq!(serde_json::to_vec(&s).unwrap(),before);
}
#[test]fn open_resolves_note_id_after_rename_and_reorder_without_syncing_task_text(){
    let mut s=state();let mut reg=EditorRegistry::default();let mut source=Source::default();let mut db=store();convert(&mut s,&mut reg,&mut source,&mut db);
    s.sticky_notes[0].title="renamed".into();s.sticky_notes.insert(0,note(20,"neighbour","unchanged"));
    assert_eq!(open_note(&mut s,1,access(),&mut reg,&mut source,&mut db,"now"),Ok(NoteTarget::Note{id:9,index:1}));
    assert_eq!(s.todos[0].text,"first line");assert_eq!(s.sticky_notes[0].content,"unchanged");
}
#[test]fn dangling_link_detach_only_changes_reference_and_activity_route_is_not_legacy_tasks_tab(){
    let mut s=state();s.todos.push(task(1,Some(99),true));let mut reg=EditorRegistry::default();let mut source=Source::default();let mut db=store();
    assert_eq!(open_note(&mut s,1,access(),&mut reg,&mut source,&mut db,"now"),Err(LinkError::Unavailable));
    assert!(detach(&mut s,1,false,&mut db).unwrap());assert!(!detach(&mut s,1,false,&mut db).unwrap());assert!(s.todos[0].done);assert_eq!(s.todos[0].text,"task-1");
    assert_eq!(resolve_activity_todo(&s,1,NoteAccess{notes_enabled:false,quiet:false}),Ok(NoteTarget::ReminderFallback));
    assert_eq!(resolve_activity_todo(&s,1,access()),Ok(NoteTarget::TasksSubview{todo_id:1}));
}
#[test]fn float_has_one_owner_and_panel_is_read_only_without_creating_a_second_writer(){
    let s=state();let mut reg=EditorRegistry::default();let first=lease(&mut reg,&s,9,EditorOwner::Popout);
    assert_eq!(reg.acquire(&s,9,EditorOwner::Popout),Ok(Acquisition::Existing(first)));
    assert_eq!(reg.acquire(&s,9,EditorOwner::Panel),Err(EditorError::Busy));assert!(reg.panel_read_only(9));
}
#[test]fn intentional_empty_panel_edit_is_committed_by_id_before_conversion(){
    let mut s=state();s.sticky_notes.push(note(20,"other","other-body"));s.selected_note=1;
    let mut reg=EditorRegistry::default();let l=lease(&mut reg,&s,9,EditorOwner::Panel);let mut source=Source{updates:vec![update(l,1,Some(""),"",false)],..Default::default()};let mut db=store();
    assert_eq!(convert(&mut s,&mut reg,&mut source,&mut db),CreateResult::Empty);assert_eq!(s.sticky_notes[0].title,"Catatan Baru");assert!(s.sticky_notes[0].content.is_empty());
    assert_eq!(s.sticky_notes[1].content,"other-body");assert!(s.todos.is_empty());
}
#[test]fn absent_controls_do_not_erase_content_and_capture_failure_blocks_conversion(){
    let mut s=state();let original=s.sticky_notes[0].clone();let mut reg=EditorRegistry::default();let mut source=Source::default();
    assert_eq!(reg.flush(&mut s,&mut source,"now"),Ok(0));assert_eq!(s.sticky_notes[0],original);
    source.fail=true;let mut db=store();assert_eq!(convert(&mut s,&mut reg,&mut source,&mut db),CreateResult::FlushFailed(EditorError::CaptureFailed));assert!(s.todos.is_empty());assert_eq!(s.sticky_notes[0],original);
}
#[test]fn open_float_snapshot_is_used_for_conversion_and_both_export_formats_without_close(){
    let mut s=state();let mut reg=EditorRegistry::default();let l=lease(&mut reg,&s,9,EditorOwner::Popout);let mut source=Source{updates:vec![update(l,1,None,"latest 漢🎶\nsecond",false)],..Default::default()};let mut db=store();
    assert_eq!(convert(&mut s,&mut reg,&mut source,&mut db),CreateResult::Created(1));assert_eq!(s.todos[0].text,"latest 漢🎶");assert_eq!(reg.current(9),Some(l));
    let dir=db.root.join("exports");for format in [ExportFormat::Text,ExportFormat::Markdown]{
        let path=export_latest(&mut s,9,access(),&mut reg,&mut source,&mut db,&dir,format,"later").unwrap();let text=std::fs::read_to_string(path).unwrap();assert!(text.contains("latest 漢🎶\nsecond"));assert!(!text.contains("note_id"));
    }
    assert_eq!(s.sticky_notes[0].updated_at,"now");assert_eq!(reg.current(9),Some(l));
}
#[test]fn duplicate_close_and_old_generation_do_not_overwrite_a_new_panel_revision(){
    let mut s=state();let mut reg=EditorRegistry::default();let old=lease(&mut reg,&s,9,EditorOwner::Popout);let close=update(old,1,None,"old close",true);
    reg.apply_batch(&mut s,vec![close.clone()],"close").unwrap();let current=lease(&mut reg,&s,9,EditorOwner::Panel);assert_ne!(current.generation,old.generation);
    reg.apply_batch(&mut s,vec![update(current,1,Some("new title"),"new body",false)],"new").unwrap();
    assert_eq!(reg.apply_batch(&mut s,vec![close],"duplicate"),Ok(0));assert_eq!(s.sticky_notes[0].content,"new body");
    assert_eq!(reg.apply_batch(&mut s,vec![update(old,2,None,"late conflict",true)],"bad"),Err(EditorError::Conflict));
    assert_eq!(s.sticky_notes[0].content,"new body");assert_eq!(reg.conflict_drafts.len(),1);assert_eq!(reg.pending_count(),1);
}
#[test]fn failed_batch_preserves_all_close_payloads_and_does_not_partially_commit_other_notes(){
    let mut s=state();s.sticky_notes.push(note(20,"other","before-other"));let original=serde_json::to_vec(&s).unwrap();let mut reg=EditorRegistry::default();let a=lease(&mut reg,&s,9,EditorOwner::Popout);let mut b=lease(&mut reg,&s,20,EditorOwner::Panel);b.generation+=100;
    let updates=vec![update(a,1,None,"must retain this close",true),update(b,1,Some("other"),"conflict",true)];
    assert_eq!(reg.apply_batch(&mut s,updates,"now"),Err(EditorError::Conflict));assert_eq!(serde_json::to_vec(&s).unwrap(),original);assert_eq!(reg.pending_count(),2);
    assert_eq!(reg.current(9),Some(a));
}
#[test]fn panel_other_note_and_two_float_payloads_flush_together_before_restart(){
    let mut s=state();s.sticky_notes.push(note(20,"second","old second"));s.sticky_notes.push(note(30,"third","old third"));let mut reg=EditorRegistry::default();
    let a=lease(&mut reg,&s,9,EditorOwner::Popout);let b=lease(&mut reg,&s,20,EditorOwner::Popout);let c=lease(&mut reg,&s,30,EditorOwner::Panel);
    let mut source=Source{updates:vec![update(a,1,None,"new first",true),update(b,1,None,"new second",false),update(c,1,Some("third new"),"new third",false)],..Default::default()};
    reg.flush(&mut s,&mut source,"now").unwrap();let mut db=store();db.save("state.json",&s).unwrap();let restored:State=db.load("state.json");
    assert_eq!(restored.sticky_notes.iter().map(|n|n.content.as_str()).collect::<Vec<_>>(),vec!["new first","new second","new third"]);
}
#[test]fn preset_flush_hook_reuses_the_same_editor_barrier_before_hide(){
    let mut s=state();let mut reg=EditorRegistry::default();let l=lease(&mut reg,&s,9,EditorOwner::Panel);let mut source=Source{updates:vec![update(l,1,Some("new title"),"latest before quiet",false)],..Default::default()};
    let mut settings=crate::core::Settings::default();let transition=crate::presets::apply(&mut settings,crate::presets::PresetAction::SetQuiet(true));
    crate::presets::flush_for_transition(&transition,&mut FlushSession{state:&mut s,registry:&mut reg,source:&mut source,now:"quiet"}).unwrap();
    assert_eq!(s.sticky_notes[0].content,"latest before quiet");assert!(transition.plan.hide_editors);
    assert!(matches!(transition.effective.present(crate::presets::SensitiveSurface::Notes,s.sticky_notes[0].content.clone()),crate::presets::Presented::Hidden(_)));
}
#[test]fn a_failed_durable_barrier_does_not_export_and_preserves_latest_in_memory_text(){
    let mut s=state();let mut reg=EditorRegistry::default();let l=lease(&mut reg,&s,9,EditorOwner::Popout);let mut source=Source{updates:vec![update(l,1,None,"unsaved latest",false)],..Default::default()};let mut db=store();
    std::fs::write(db.root.join("state.json"),b"{").unwrap();let _:State=db.load("state.json");let exports=db.root.join("exports");
    assert_eq!(export_latest(&mut s,9,access(),&mut reg,&mut source,&mut db,&exports,ExportFormat::Text,"now"),Err(LinkError::SaveFailed));assert!(!exports.exists());assert_eq!(s.sticky_notes[0].content,"unsaved latest");
}

#[test]fn an_empty_conversion_persists_the_intentional_source_edit_even_without_a_task(){
    let mut s=state();let mut reg=EditorRegistry::default();let l=lease(&mut reg,&s,9,EditorOwner::Panel);let mut source=Source{updates:vec![update(l,1,Some(""),"",false)],..Default::default()};let mut db=store();
    assert_eq!(convert(&mut s,&mut reg,&mut source,&mut db),CreateResult::Empty);let restored:State=db.load("state.json");assert!(restored.sticky_notes[0].content.is_empty());assert!(restored.todos.is_empty());
}
#[test]fn source_only_save_failure_is_reported_without_inventing_a_task_identity(){
    let mut s=state();let mut reg=EditorRegistry::default();let l=lease(&mut reg,&s,9,EditorOwner::Panel);let mut source=Source{updates:vec![update(l,1,Some(""),"",false)],..Default::default()};let mut db=store();
    std::fs::write(db.root.join("state.json"),b"{").unwrap();let _:State=db.load("state.json");
    assert_eq!(convert(&mut s,&mut reg,&mut source,&mut db),CreateResult::SourceSaveFailed);assert!(s.todos.is_empty());assert!(s.sticky_notes[0].content.is_empty());
}
#[test]fn access_adapter_uses_effective_quiet_and_actual_mod_availability(){
    let mut settings=crate::core::Settings::default();assert!(!NoteAccess::from_settings(&settings).notes_enabled);
    settings.installed_mods.push("sticky_notes".into());settings.normalize();assert!(NoteAccess::from_settings(&settings).allowed());
    crate::presets::apply(&mut settings,crate::presets::PresetAction::SetQuiet(true));assert!(!NoteAccess::from_settings(&settings).allowed());
}
#[test]fn partial_panel_capture_is_not_mistaken_for_intentional_empty_input(){
    let mut s=state();let original=s.sticky_notes[0].clone();let mut reg=EditorRegistry::default();let l=lease(&mut reg,&s,9,EditorOwner::Panel);
    assert_eq!(reg.apply_batch(&mut s,vec![update(l,1,None,"",false)],"bad"),Err(EditorError::CaptureFailed));
    assert_eq!(s.sticky_notes[0],original);assert_eq!(reg.pending_count(),1);
}
#[test]fn delete_removes_only_the_selected_task_and_keeps_the_linked_note(){
    let mut s=state();s.todos=vec![task(1,Some(9),false),task(2,None,true)];let mut db=store();
    assert_eq!(delete(&mut s,1,false,&mut db),Ok(()));
    assert_eq!(s.todos.len(),1);assert_eq!(s.todos[0].id,2);assert_eq!(s.sticky_notes[0].id,9);
    assert_eq!(delete(&mut s,9,false,&mut db),Err(LinkError::Unavailable));
    assert_eq!(delete(&mut s,2,true,&mut db),Err(LinkError::Unavailable));
}
