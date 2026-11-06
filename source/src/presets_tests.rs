use super::*;
fn stripped(s:&Settings)->serde_json::Value{let mut v=serde_json::to_value(s).unwrap();v.as_object_mut().unwrap().remove("quiet_mode");v.as_object_mut().unwrap().remove("power_saver_mode");v}
#[test]fn four_modes_preserve_base_and_produce_consistent_effective_values(){
    for bits in 0..64{
        let mut s=Settings::default();s.visualizer=bits&1!=0;s.glow=bits&2!=0;s.motion_reduced=bits&4!=0;s.theme_material=if bits&8!=0{"glass"}else{"solid"}.into();
        let base=stripped(&s);let quiet=bits&16!=0;let saver=bits&32!=0;
        apply(&mut s,PresetAction::SetQuiet(quiet));apply(&mut s,PresetAction::SetPowerSaver(saver));let e=s.effective_presentation();
        assert_eq!(stripped(&s),base);assert_eq!(e.visualizer,s.visualizer&&!saver);assert_eq!(e.glow,s.glow&&!saver);
        assert_eq!(e.reduced_motion,s.motion_reduced||saver);assert_eq!(e.material,if saver{"solid"}else{&s.theme_material});
        assert_eq!(e.hide_sensitive,quiet);assert_eq!(e.sound,!quiet);assert_eq!(e.automatic_popup,!quiet);
        assert!(!apply(&mut s,PresetAction::SetQuiet(quiet)).changed);
        apply(&mut s,PresetAction::Reset);assert_eq!(stripped(&s),base);
    }
}
#[test]fn editing_base_during_saver_restores_latest_choice_not_a_stale_clone(){
    let mut s=Settings::default();s.theme_material="glass".into();s.visualizer=true;apply(&mut s,PresetAction::SetPowerSaver(true));
    s.theme_material="adaptive".into();s.glow=false;s.visualizer=false;
    assert_eq!(s.effective_presentation().material,"solid");apply(&mut s,PresetAction::SetPowerSaver(false));
    assert_eq!(s.effective_presentation().material,"adaptive");assert!(!s.effective_presentation().visualizer);assert!(!s.glow);
}
#[test]fn legacy_settings_and_restart_flags_are_backward_compatible(){
    let mut s:Settings=serde_json::from_str("{}").unwrap();assert!(!s.quiet_mode&&!s.power_saver_mode);
    apply(&mut s,PresetAction::SetQuiet(true));apply(&mut s,PresetAction::SetPowerSaver(true));
    let restored:Settings=serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
    assert!(restored.quiet_mode&&restored.power_saver_mode);assert_eq!(restored.theme_material,s.theme_material);
}
#[test]fn every_sensitive_surface_drops_synthetic_payload_and_diagnostics_do_not_echo_it(){
    let mut s=Settings::default();s.quiet_mode=true;let e=s.effective_presentation();let private="SYNTHETIC_SECRET_TITLE_BODY_PATH".repeat(50);
    for surface in [SensitiveSurface::Compact,SensitiveSurface::Music,SensitiveSurface::Lyrics,SensitiveSurface::Notes,SensitiveSurface::Clipboard,SensitiveSurface::Calendar,SensitiveSurface::Weather,SensitiveSurface::CommunityMod,SensitiveSurface::PrivacyDetail,SensitiveSurface::SettingsSecrets,SensitiveSurface::Error]{
        let presented=e.present(surface,(private.clone(),vec![7u8;500]));
        assert!(matches!(presented,Presented::Hidden(_)));assert!(!format!("{presented:?}").contains("SYNTHETIC_SECRET"));
    }
    assert_eq!(private.len(),"SYNTHETIC_SECRET_TITLE_BODY_PATH".len()*50);
}
#[test]fn action_guard_keeps_controls_and_settings_but_rejects_hidden_content_actions(){
    let mut s=Settings::default();s.quiet_mode=true;let e=s.effective_presentation();
    for intent in [UserIntent::ReadContent,UserIntent::EditContent,UserIntent::Copy,UserIntent::Export,UserIntent::Popout,UserIntent::ConvertNote,UserIntent::OpenNoteLink]{assert!(!e.allows(intent));}
    for intent in [UserIntent::Settings,UserIntent::MediaTransport,UserIntent::HardwareVolume]{assert!(e.allows(intent));}
}
#[test]fn privacy_indicator_survives_quiet_while_app_names_are_redacted(){
    let mut s=Settings::default();s.quiet_mode=true;let p=s.effective_presentation().privacy_indicator(true,true,vec!["SENTINEL.exe".into()]);
    assert!(p.enabled&&p.mic&&p.camera);assert!(p.app_names.is_empty());
    s.privacy_enabled=false;assert!(!s.effective_presentation().privacy_indicator(true,true,vec![]).enabled);
}
struct Flush {calls:usize,fail:bool}
impl EditorFlush for Flush{fn flush_editors(&mut self)->Result<(),&'static str>{self.calls+=1;if self.fail{Err("Edit belum tersimpan")}else{Ok(())}}}
#[test]fn transition_flushes_once_before_hide_and_failure_does_not_reveal_content(){
    let mut s=Settings::default();let t=apply(&mut s,PresetAction::SetQuiet(true));let mut f=Flush{calls:0,fail:true};
    assert!(t.plan.flush_editors&&t.plan.hide_editors&&t.plan.clear_sensitive_caches);
    assert!(flush_for_transition(&t,&mut f).is_err());assert!(s.effective_presentation().hide_sensitive);
    let repeat=apply(&mut s,PresetAction::SetQuiet(true));flush_for_transition(&repeat,&mut f).unwrap();assert_eq!(f.calls,1);
    assert!(apply(&mut s,PresetAction::SetQuiet(false)).plan.restore_editors_without_activation);
}
#[test]fn saver_produces_capture_stop_and_settle_intents_without_modifying_clocks(){
    let mut s=Settings::default();s.theme_material="glass".into();let focus_minutes=s.focus_minutes;
    let mut clock=crate::core::FocusClock::default();clock.start(1,false);
    let t=apply(&mut s,PresetAction::SetPowerSaver(true));assert!(t.plan.stop_capture&&t.plan.clear_levels&&t.plan.settle_animations&&t.plan.rebuild_geometry);
    assert!(clock.running());assert_eq!(s.focus_minutes,focus_minutes);
    assert!(!t.effective.capture_allowed(crate::core::WorkPolicy{enabled:true,visible:true,fullscreen:false,playing:true}));
}
fn queue(s:&mut State,n:u64){s.enqueue_local_reminder(PendingReminder::new(ActivityId::Focus(n),n as i64).unwrap()).unwrap();}
fn step(q:&mut QuietBacklog,r:&mut ActivityRuntime,s:&mut State,i:&mut ActivityInputs,now:i64,p:PresentationPolicy)->PresentationEffects{
    q.reconcile(s,i,now,p,r.snapshot.as_ref());r.reconcile(s,i,now,p)
}
#[test]fn quiet_holds_pending_then_produces_one_silent_group_and_keeps_hardware_hud(){
    let mut s=State::default();for n in 1..=25{queue(&mut s,n);}
    let mut q=QuietBacklog::default();let mut r=ActivityRuntime::default();let mut i=ActivityInputs::default();
    let quiet=PresentationPolicy{quiet:true,..Default::default()};assert!(!step(&mut q,&mut r,&mut s,&mut i,0,quiet).sound);
    assert!(s.local_reminder_latches.is_empty());assert!(s.pending_local_reminders.iter().all(|p|p.deferred_quiet&&!p.presented_once));
    i.hud(1,100,40,false);step(&mut q,&mut r,&mut s,&mut i,100,quiet);assert_eq!(r.snapshot.as_ref().unwrap().primary.text,"40%");
    let mut restored:State=serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();let normal=PresentationPolicy::default();
    assert!(!step(&mut q,&mut r,&mut restored,&mut i,2_000,normal).sound);assert_eq!(r.snapshot.as_ref().unwrap().primary.id,ActivityId::Backlog);
    assert_eq!(i.candidates.iter().filter(|c|c.id==ActivityId::Backlog).count(),1);assert_eq!(restored.pending_local_reminders.len(),25);
}
#[test]fn backlog_visible_budget_pauses_under_hud_and_dismissal_does_not_discard_pending(){
    let mut s=State::default();queue(&mut s,1);let mut q=QuietBacklog::default();let mut r=ActivityRuntime::default();let mut i=ActivityInputs::default();let p=PresentationPolicy::default();
    step(&mut q,&mut r,&mut s,&mut i,0,PresentationPolicy{quiet:true,..p});step(&mut q,&mut r,&mut s,&mut i,0,p);
    i.hud(1,1_000,40,false);step(&mut q,&mut r,&mut s,&mut i,1_000,p);assert_eq!(s.quiet_backlog_remaining_ms,7_000);
    step(&mut q,&mut r,&mut s,&mut i,2_800,p);assert_eq!(s.quiet_backlog_remaining_ms,7_000);
    q.dismiss_summary(&mut s);step(&mut q,&mut r,&mut s,&mut i,2_801,p);assert_eq!(r.snapshot.as_ref().unwrap().primary.id,ActivityId::Clock);
    assert_eq!(s.pending_local_reminders.len(),1);assert!(s.local_reminder_latches.is_empty());
    let mut restored:State=serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();step(&mut q,&mut r,&mut restored,&mut i,3_000,p);
    assert_eq!(r.snapshot.as_ref().unwrap().primary.id,ActivityId::Clock);
}
#[test]fn opening_one_deferred_occurrence_is_silent_and_repeated_toggles_do_not_replay_the_group(){
    let mut s=State::default();queue(&mut s,1);queue(&mut s,2);let mut q=QuietBacklog::default();let mut r=ActivityRuntime::default();let mut i=ActivityInputs::default();let p=PresentationPolicy::default();
    step(&mut q,&mut r,&mut s,&mut i,0,PresentationPolicy{quiet:true,..p});step(&mut q,&mut r,&mut s,&mut i,0,p);
    step(&mut q,&mut r,&mut s,&mut i,2_000,p);assert_eq!(s.quiet_backlog_remaining_ms,6_000);
    step(&mut q,&mut r,&mut s,&mut i,2_000,PresentationPolicy{quiet:true,..p});step(&mut q,&mut r,&mut s,&mut i,5_000,p);assert_eq!(s.quiet_backlog_remaining_ms,6_000);
    assert!(QuietBacklog::open_occurrence(&mut s,&ActivityId::Focus(1),true).is_none());
    assert_eq!(QuietBacklog::open_occurrence(&mut s,&ActivityId::Focus(1),false),Some(ActivityTarget::Reminder(0)));
    assert!(QuietBacklog::open_occurrence(&mut s,&ActivityId::Focus(1),false).is_none());
    assert!(!step(&mut q,&mut r,&mut s,&mut i,5_001,p).sound);assert!(s.local_reminder_latches.contains(&ActivityId::Focus(1)));
    assert!(s.pending_local_reminders.iter().any(|p|p.id==ActivityId::Focus(2)&&p.deferred_quiet));
}
#[test]fn failed_settings_save_keeps_effective_runtime_and_retry_persists_only_flags(){
    let root=std::env::temp_dir().join(format!("preset-fixture-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let mut store=Store::new(root.clone()).unwrap();std::fs::write(root.join("settings.json"),b"{").unwrap();let _:Settings=store.load("settings.json");
    let mut s=Settings::default();let original=stripped(&s);apply(&mut s,PresetAction::SetQuiet(true));assert!(persist(&s,&mut store).is_err());
    assert!(s.effective_presentation().hide_sensitive);assert_eq!(stripped(&s),original);store.acknowledge_corrupt("settings.json");persist(&s,&mut store).unwrap();
    let restored:Settings=store.load("settings.json");assert!(restored.quiet_mode);assert_eq!(stripped(&restored),original);
}
#[test]fn todo_and_prayer_latches_wait_through_quiet_restart_and_silent_backlog(){
    let mut s=State::default();s.todos.push(serde_json::from_value(serde_json::json!({"id":7,"text":"SYNTHETIC_PRIVATE_TODO","done":false,"due":"1970-01-01T00:00:01Z","reminded":false})).unwrap());
    s.collect_due_todos(3_000,true);
    let prayer=ActivityId::Prayer{city:"fixture".into(),date:"2026-01-01".into(),slot:0,time:"12:00".into(),lead:0,latch:"fixture-prayer".into()};
    s.enqueue_local_reminder(PendingReminder::new(prayer.clone(),2_000).unwrap()).unwrap();
    let mut q=QuietBacklog::default();let mut r=ActivityRuntime::default();let mut i=ActivityInputs::default();let p=PresentationPolicy::default();
    step(&mut q,&mut r,&mut s,&mut i,3_000,PresentationPolicy{quiet:true,..p});assert!(!s.todos[0].reminded);assert!(s.prayer_delivered.is_empty());
    let mut restored:State=serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
    assert!(!step(&mut q,&mut r,&mut restored,&mut i,4_000,p).sound);assert!(!restored.todos[0].reminded);assert!(restored.prayer_delivered.is_empty());
    let todo=ActivityId::TodoDue{todo_id:7,due_ms:1_000};QuietBacklog::open_occurrence(&mut restored,&todo,false).unwrap();
    assert!(!step(&mut q,&mut r,&mut restored,&mut i,4_001,p).sound);assert!(restored.todos[0].reminded);assert!(!restored.todos[0].done);
    restored.dismiss_local_reminder(&todo);QuietBacklog::open_occurrence(&mut restored,&prayer,false).unwrap();
    assert!(!step(&mut q,&mut r,&mut restored,&mut i,4_002,p).sound);assert_eq!(restored.prayer_delivered,vec!["fixture-prayer"]);
}
