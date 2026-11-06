use super::*;
fn add(s:&mut State,id:ActivityId,due:i64){assert!(s.enqueue_local_reminder(PendingReminder::new(id,due).unwrap()).unwrap());}
fn focus(inputs:&mut ActivityInputs){inputs.countdown(ActivityId::Focus(99),0,60_000,120_000,true);}
fn snapshot_id(r:&ActivityRuntime)->ActivityId{r.snapshot.as_ref().unwrap().primary.id.clone()}

#[test]fn priority_preemption_preserves_budget_and_does_not_replay_sound(){
    let mut s=State::default();add(&mut s,ActivityId::Water(1),0);add(&mut s,ActivityId::Move(2),1);
    let mut i=ActivityInputs::default();focus(&mut i);let mut r=ActivityRuntime::default();let p=PresentationPolicy::default();
    assert!(r.reconcile(&mut s,&i,0,p).sound);assert_eq!(snapshot_id(&r),ActivityId::Water(1));
    i.hud(1,1_000,50,false);r.reconcile(&mut s,&i,1_000,p);
    assert_eq!(snapshot_id(&r),ActivityId::Hud(1));assert_eq!(s.pending_local_reminders[0].remaining_visible_ms,11_000);
    assert!(!r.reconcile(&mut s,&i,2_800,p).sound);assert_eq!(snapshot_id(&r),ActivityId::Water(1));
    assert_eq!(s.pending_local_reminders[0].remaining_visible_ms,11_000);
    r.reconcile(&mut s,&i,13_800,p);assert_eq!(snapshot_id(&r),ActivityId::Move(2));
    r.reconcile(&mut s,&i,21_800,p);assert_eq!(snapshot_id(&r),ActivityId::Focus(99));
}
#[test]fn tie_break_and_dedup_are_independent_of_input_order(){
    let ids=[ActivityId::Eye(4),ActivityId::Water(3),ActivityId::Stretch(2),ActivityId::Focus(1)];
    for reverse in [false,true]{
        let mut s=State::default();let mut order=ids.to_vec();if reverse{order.reverse();}
        for id in order{add(&mut s,id,10);}
        let mut r=ActivityRuntime::default();r.reconcile(&mut s,&ActivityInputs::default(),10,PresentationPolicy::default());
        assert_eq!(snapshot_id(&r),ActivityId::Focus(1));assert_eq!(r.snapshot.unwrap().other_count,3);
    }
}
#[test]fn no_sixteen_item_limit_and_same_occurrence_is_idempotent(){
    let mut s=State::default();for n in 1..=40{add(&mut s,ActivityId::Focus(n),n as i64);}
    assert!(!s.enqueue_local_reminder(PendingReminder::new(ActivityId::Focus(1),0).unwrap()).unwrap());
    assert_eq!(s.pending_local_reminders.len(),40);
}
#[test]fn recurring_sources_coalesce_without_recounting_old_cycles(){
    let mut s=State::default();for n in 1..=50{add(&mut s,ActivityId::Water(n),n as i64);}
    assert_eq!(s.pending_local_reminders.len(),1);assert_eq!(s.pending_local_reminders[0].coalesced_count,50);
    assert!(!s.enqueue_local_reminder(PendingReminder::new(ActivityId::Water(49),60).unwrap()).unwrap());
    s.dismiss_local_reminder(&ActivityId::Water(1));
    assert!(!s.enqueue_local_reminder(PendingReminder::new(ActivityId::Water(50),60).unwrap()).unwrap());
    add(&mut s,ActivityId::Water(51),60);
}
#[test]fn completion_under_hud_is_consumed_once_and_dismiss_does_not_cancel_source(){
    let mut s=State::default();let mut i=ActivityInputs::default();i.hud(1,0,1,false);focus(&mut i);
    let mut r=ActivityRuntime::default();let p=PresentationPolicy::default();r.reconcile(&mut s,&i,0,p);
    add(&mut s,ActivityId::Focus(1),100);
    assert!(!s.enqueue_local_reminder(PendingReminder::new(ActivityId::Focus(1),100).unwrap()).unwrap());
    r.reconcile(&mut s,&i,1_800,p);assert_eq!(snapshot_id(&r),ActivityId::Focus(1));
    assert!(s.dismiss_local_reminder(&ActivityId::Focus(1)));r.reconcile(&mut s,&i,1_801,p);
    assert_eq!(snapshot_id(&r),ActivityId::Focus(99));
    assert!(!s.enqueue_local_reminder(PendingReminder::new(ActivityId::Focus(1),200).unwrap()).unwrap());
}
#[test]fn expiry_between_paint_and_enter_cannot_route_a_stale_target(){
    let mut s=State::default();let mut i=ActivityInputs::default();i.hud(7,0,9,false);
    let mut r=ActivityRuntime::default();let p=PresentationPolicy::default();r.reconcile(&mut s,&i,0,p);
    let snapshot=r.snapshot.clone().unwrap();
    assert_eq!(r.open(&s,&i,1_799,p,&snapshot.primary.id,snapshot.revision),Some(ActivityTarget::AudioSettings));
    assert_eq!(r.open(&s,&i,1_800,p,&snapshot.primary.id,snapshot.revision),None);
}
#[test]fn hidden_and_quiet_time_does_not_spend_visual_budget(){
    let mut s=State::default();add(&mut s,ActivityId::Water(1),0);let i=ActivityInputs::default();let mut r=ActivityRuntime::default();
    let p=PresentationPolicy::default();r.reconcile(&mut s,&i,0,p);
    r.reconcile(&mut s,&i,0,PresentationPolicy{visible:false,..p});r.reconcile(&mut s,&i,100_000,PresentationPolicy{visible:false,..p});
    r.reconcile(&mut s,&i,100_000,p);assert_eq!(s.pending_local_reminders[0].remaining_visible_ms,12_000);
    r.reconcile(&mut s,&i,102_000,PresentationPolicy{quiet:true,..p});assert_eq!(s.pending_local_reminders[0].remaining_visible_ms,10_000);
    r.reconcile(&mut s,&i,300_000,PresentationPolicy{quiet:true,..p});assert_eq!(s.pending_local_reminders[0].remaining_visible_ms,10_000);
}
#[test]fn dismiss_and_double_confirm_water_are_separate_actions(){
    let mut s=State::default();add(&mut s,ActivityId::Water(1),0);add(&mut s,ActivityId::Move(2),1);
    s.dismiss_local_reminder(&ActivityId::Water(1));assert!(s.water.is_empty());
    add(&mut s,ActivityId::Water(3),2);s.dismiss_local_reminder(&ActivityId::Move(2));
    let mut r=ActivityRuntime::default();r.reconcile(&mut s,&ActivityInputs::default(),3,PresentationPolicy::default());
    let date=chrono::NaiveDate::from_ymd_opt(2026,1,1).unwrap();
    assert!(r.confirm_water(&mut s,&ActivityId::Water(3),250,date));assert!(!r.confirm_water(&mut s,&ActivityId::Water(3),250,date));
    assert_eq!(s.water.iter().map(|d|d.ml).sum::<u32>(),250);
}
fn todo()->crate::core::Todo{serde_json::from_value(serde_json::json!({"id":7,"text":"synthetic private text","done":false,"due":"1970-01-01T00:00:01Z","reminded":false})).unwrap()}
#[test]fn todo_latch_only_changes_at_delivery_and_due_edits_cancel_old_pending(){
    let mut s=State::default();s.todos.push(todo());s.collect_due_todos(2_000,true);s.collect_due_todos(2_000,true);
    assert_eq!(s.pending_local_reminders.len(),1);assert!(!s.todos[0].reminded);
    let mut r=ActivityRuntime::default();r.reconcile(&mut s,&ActivityInputs::default(),2_000,PresentationPolicy{reminders:false,..Default::default()});
    assert!(!s.todos[0].reminded);r.reconcile(&mut s,&ActivityInputs::default(),2_000,PresentationPolicy::default());assert!(s.todos[0].reminded);
    s.todos[0].done=true;s.collect_due_todos(3_000,false);assert!(s.pending_local_reminders.is_empty());
}
#[test]fn prayer_latch_and_same_text_reminders_are_separate_by_typed_id(){
    let mut s=State::default();
    for slot in 0..2{add(&mut s,ActivityId::Prayer{city:"synthetic".into(),date:"2026-01-01".into(),slot,time:"12:00".into(),lead:0,latch:format!("prayer-{slot}")},0);}
    let mut r=ActivityRuntime::default();r.reconcile(&mut s,&ActivityInputs::default(),0,PresentationPolicy::default());
    assert_eq!(s.prayer_delivered.len(),1);assert_eq!(s.pending_local_reminders.len(),2);
}
#[test]fn restart_defaults_dedup_and_high_water_do_not_reuse_ids(){
    let mut s:State=serde_json::from_str("{}").unwrap();assert_eq!(s.next_activity_sequence,1);
    add(&mut s,ActivityId::Focus(20),0);let duplicate=s.pending_local_reminders[0].clone();s.pending_local_reminders.push(duplicate);
    s.normalize_activities();assert_eq!(s.pending_local_reminders.len(),1);assert_eq!(s.allocate_activity_sequence().unwrap(),21);
    let mut r=ActivityRuntime::default();assert!(r.reconcile(&mut s,&ActivityInputs::default(),0,PresentationPolicy::default()).sound);
    let mut restored:State=serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();restored.normalize_activities();
    assert!(!ActivityRuntime::default().reconcile(&mut restored,&ActivityInputs::default(),0,PresentationPolicy::default()).sound);
    s.next_activity_sequence=u64::MAX;assert!(s.allocate_activity_sequence().is_err());
}
#[test]fn long_unicode_and_reduced_motion_do_not_change_the_click_target(){
    let mut inputs=ActivityInputs::default();inputs.media(&crate::media::MediaSnapshot{session_id:"1".into(),selection_revision:2,playing:true,title:"🎶漢字".repeat(500),..Default::default()},0);
    let mut s=State::default();let mut r=ActivityRuntime::default();let p=PresentationPolicy{reduced_motion:true,..Default::default()};r.reconcile(&mut s,&inputs,0,p);
    let snap=r.snapshot.as_ref().unwrap();assert!(!snap.animate);assert_eq!(r.open(&s,&inputs,0,p,&snap.primary.id,snap.revision),Some(ActivityTarget::Music));
    assert_eq!(inputs.candidates[0].text.chars().count(),1500);
}
#[test]fn missing_or_wrong_source_target_is_fail_closed(){
    let mut s=State::default();let mut p=PendingReminder::new(ActivityId::TodoDue{todo_id:99,due_ms:0},0).unwrap();p.target=ActivityTarget::Music;
    s.pending_local_reminders.push(p);s.normalize_activities();assert_eq!(s.pending_local_reminders[0].target,ActivityTarget::Unavailable);
    let mut r=ActivityRuntime::default();let i=ActivityInputs::default();let p=PresentationPolicy::default();r.reconcile(&mut s,&i,0,p);
    let snap=r.snapshot.as_ref().unwrap();assert_eq!(r.open(&s,&i,0,p,&snap.primary.id,snap.revision),None);
}
#[test]fn save_failure_retains_in_memory_pending_and_retry_is_not_a_second_delivery(){
    let root=std::env::temp_dir().join(format!("activity-fixture-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let mut store=crate::core::Store::new(root.clone()).unwrap();std::fs::write(root.join("state.json"),b"{").unwrap();let _:State=store.load("state.json");
    let mut s=State::default();add(&mut s,ActivityId::Focus(1),0);let mut r=ActivityRuntime::default();let i=ActivityInputs::default();let p=PresentationPolicy::default();
    assert!(r.reconcile(&mut s,&i,0,p).sound);assert!(save_pending(&s,&mut store).is_err());assert_eq!(s.pending_local_reminders.len(),1);
    assert!(!r.reconcile(&mut s,&i,1,p).sound);store.acknowledge_corrupt("state.json");save_pending(&s,&mut store).unwrap();
    let restored:State=store.load("state.json");assert!(restored.pending_local_reminders[0].presented_once);
}

#[test]fn run_generation_survives_pause_and_rejects_old_completion_after_restart(){
    let mut state=State::default();let mut runs=RunTracker::default();let first=runs.start(&mut state,ReminderKind::Focus,0).unwrap();
    let mut clock=crate::core::FocusClock::default();clock.start(1,false);clock.pause();clock.resume();
    let mut inputs=ActivityInputs::default();runs.focus_input(&mut inputs,&clock);assert_eq!(inputs.candidates[0].id,first);
    assert!(runs.complete(&mut state,&first,10).unwrap());assert!(!runs.complete(&mut state,&first,11).unwrap());
    let second=runs.start(&mut state,ReminderKind::Focus,20).unwrap();assert_ne!(first,second);
    assert!(!runs.complete(&mut state,&first,21).unwrap());assert_eq!(runs.id,Some(second));
}
