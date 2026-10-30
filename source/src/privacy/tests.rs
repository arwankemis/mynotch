use crate::privacy::{
    mailbox, Activity, CapabilityKind, Coverage, Evidence, MailboxError, PrivacyEvent,
    PrivacyReducer, PrivacySnapshot,
};

#[test]
fn test_immediate_onset_on_activity_in_privacy_tests() {
    let mut reducer = PrivacyReducer::new();
    assert_eq!(reducer.snapshot().microphone.activity, Activity::Inactive);

    reducer.handle_event(
        PrivacyEvent::MicActive {
            app: "slack.exe".into(),
        },
        1000u64,
    );

    let snap = reducer.snapshot();
    assert_eq!(snap.microphone.activity, Activity::Active);
    assert_eq!(snap.microphone.active_since, Some(1000));
    assert_eq!(snap.microphone.applications, vec!["slack.exe"]);
    assert_eq!(snap.microphone.evidence, Some(Evidence::CoreAudio));
    assert_eq!(snap.revision, 1);
}

#[test]
fn test_grace_period_and_cancellation_on_immediate_resume() {
    let mut reducer = PrivacyReducer::new();

    // 1. Mic becomes active at t = 5000
    reducer.handle_event(
        PrivacyEvent::MicActive {
            app: "google_chrome.exe".into(),
        },
        5000u64,
    );
    assert_eq!(reducer.snapshot().microphone.activity, Activity::Active);
    assert_eq!(reducer.snapshot().microphone.active_since, Some(5000));

    // 2. Mic reports inactive at t = 6000 (grace period 500ms -> expires at 6500)
    reducer.handle_event(PrivacyEvent::MicInactive, 6000u64);
    // State MUST remain Active during grace period
    assert_eq!(
        reducer.snapshot().microphone.activity,
        Activity::Active,
        "Activity must stay Active during 500ms grace period"
    );
    assert_eq!(
        reducer.snapshot().microphone.applications,
        vec!["google_chrome.exe"]
    );

    // 3. At t = 6300 (< 6500), Mic resumes activity! (Grace cancellation)
    reducer.handle_event(
        PrivacyEvent::MicActive {
            app: "google_chrome.exe".into(),
        },
        6300u64,
    );
    assert_eq!(reducer.snapshot().microphone.activity, Activity::Active);

    // 4. Time ticks to t = 7000 (past old deadline 6500)
    reducer.handle_event(PrivacyEvent::Tick, 7000u64);
    // Should still be active because grace period was cancelled!
    assert_eq!(
        reducer.snapshot().microphone.activity,
        Activity::Active,
        "Mic should stay Active because deactivation was cancelled by resume"
    );

    // 5. Inactive again at t = 8000 (deadline = 8500)
    reducer.handle_event(PrivacyEvent::MicInactive, 8000u64);
    assert_eq!(reducer.snapshot().microphone.activity, Activity::Active);

    // 6. At t = 8500, deadline reached
    reducer.handle_event(PrivacyEvent::Tick, 8500u64);
    assert_eq!(
        reducer.snapshot().microphone.activity,
        Activity::Inactive,
        "Mic must become Inactive once grace period has elapsed"
    );
    assert!(reducer.snapshot().microphone.applications.is_empty());
    assert_eq!(reducer.snapshot().microphone.active_since, None);
}

#[test]
fn test_unknown_status_propagation_never_masquerades() {
    let mut reducer = PrivacyReducer::new();

    // Verify screen sharing is unknown by default
    assert_eq!(reducer.snapshot().screen_share.activity, Activity::Unknown);
    assert_eq!(reducer.snapshot().screen_share.coverage, Coverage::Unsupported);

    // Set microphone active
    reducer.handle_event(
        PrivacyEvent::MicActive {
            app: "audacity.exe".into(),
        },
        1000u64,
    );
    assert_eq!(reducer.snapshot().microphone.activity, Activity::Active);

    // Monitor encounters error or unsupported audio environment -> propagates Unknown
    reducer.handle_event(
        PrivacyEvent::CapabilityUpdate {
            kind: CapabilityKind::Microphone,
            activity: Activity::Unknown,
            evidence: None,
            applications: vec![],
        },
        1500u64,
    );

    let snap = reducer.snapshot();
    assert_eq!(
        snap.microphone.activity,
        Activity::Unknown,
        "Unknown activity must be reported truthfully and not masked as Inactive or Active"
    );
    assert!(snap.microphone.applications.is_empty());
    assert_eq!(snap.microphone.active_since, None);
}

#[test]
fn test_multiple_app_merging_and_deduplication_comprehensive() {
    let mut reducer = PrivacyReducer::new();

    // Add first app
    reducer.handle_event(
        PrivacyEvent::MicActive {
            app: "Discord.exe".into(),
        },
        1000u64,
    );
    assert_eq!(
        reducer.snapshot().microphone.applications,
        vec!["Discord.exe"]
    );

    // Add second app via capability update with multiple entries, duplicates and whitespace
    reducer.handle_event(
        PrivacyEvent::CapabilityUpdate {
            kind: CapabilityKind::Microphone,
            activity: Activity::Active,
            evidence: Some(Evidence::CoreAudio),
            applications: vec![
                "discord.exe".into(), // duplicate different case
                "obs64.exe".into(),
                "   ".into(),         // empty whitespace
                "obs64.exe".into(),   // duplicate
                "chrome.exe".into(),
            ],
        },
        1100u64,
    );

    let apps = &reducer.snapshot().microphone.applications;
    assert_eq!(apps.len(), 3);
    assert_eq!(apps[0], "Discord.exe");
    assert_eq!(apps[1], "obs64.exe");
    assert_eq!(apps[2], "chrome.exe");
}

#[test]
fn test_stale_generation_rejection_in_mailbox() {
    let (tx, rx) = mailbox(10);
    assert_eq!(rx.current_generation(), 10);

    // Snapshot with older generation 9 must be rejected!
    let mut stale_snap = PrivacySnapshot::new(9);
    stale_snap.revision = 5;
    let err = tx.post(stale_snap).unwrap_err();
    assert_eq!(
        err,
        MailboxError::StaleGeneration {
            snapshot_generation: 9,
            mailbox_generation: 10,
        }
    );

    // No wake flagged and no snapshot stored
    assert!(!rx.has_wake());
    assert_eq!(rx.poll_latest(), None);

    // Snapshot with generation 10 is accepted
    let mut valid_snap = PrivacySnapshot::new(10);
    valid_snap.revision = 1;
    assert!(tx.post(valid_snap).is_ok());

    assert!(rx.has_wake());
    let polled = rx.poll_latest().expect("Should return latest snapshot");
    assert_eq!(polled.generation, 10);
    assert_eq!(polled.revision, 1);

    // Wake is now cleared
    assert!(!rx.has_wake());
    assert_eq!(rx.poll_latest(), None);

    // Receiver advances generation to 11
    rx.set_generation(11);
    assert_eq!(rx.current_generation(), 11);

    // Now generation 10 is stale
    let mut stale_snap2 = PrivacySnapshot::new(10);
    stale_snap2.revision = 2;
    assert!(matches!(
        tx.post(stale_snap2),
        Err(MailboxError::StaleGeneration { .. })
    ));
}

#[test]
fn test_privacy_monitor_handle_clean_start_stop() {
    let (tx, rx) = mailbox(1);
    let handle = crate::privacy::PrivacyMonitorHandle::start(tx);
    std::thread::sleep(std::time::Duration::from_millis(60));
    let initial = rx.poll_latest();
    assert!(initial.is_some(), "Initial snapshot should be posted on monitor start");
    drop(handle);
}

#[test]
fn test_stage0_geometry_override_calculation() {
    use crate::shell::privacy::compute_stage0_geometry;

    let mut snapshot = PrivacySnapshot::new(1);
    // 1. Inactive: default 50x10 capsule
    let inactive_geom = compute_stage0_geometry(&snapshot);
    assert_eq!(inactive_geom.width, 50.0);
    assert_eq!(inactive_geom.height, 10.0);
    assert_eq!(inactive_geom.radius, 5.0);

    // 2. Microphone active: expands to 88x18
    snapshot.microphone.activity = Activity::Active;
    let mic_geom = compute_stage0_geometry(&snapshot);
    assert_eq!(mic_geom.width, 88.0);
    assert_eq!(mic_geom.height, 18.0);
    assert_eq!(mic_geom.radius, 9.0);

    // 3. Multiple active (mic + camera): expands to 116x18
    snapshot.camera.activity = Activity::Active;
    let multi_geom = compute_stage0_geometry(&snapshot);
    assert_eq!(multi_geom.width, 116.0);
    assert_eq!(multi_geom.height, 18.0);
    assert_eq!(multi_geom.radius, 9.0);

    // 4. Return to inactive: restores 50x10
    snapshot.microphone.activity = Activity::Inactive;
    snapshot.camera.activity = Activity::Inactive;
    let restored_geom = compute_stage0_geometry(&snapshot);
    assert_eq!(restored_geom.width, 50.0);
    assert_eq!(restored_geom.height, 10.0);
    assert_eq!(restored_geom.radius, 5.0);
}

#[test]
fn test_privacy_i18n_translation_keys_exist_for_id_and_en() {
    use crate::i18n::t;

    let keys = [
        "privacy.title",
        "privacy.subtitle",
        "privacy.mic_in_use",
        "privacy.cam_in_use",
        "privacy.cam_detected",
        "privacy.screen_active",
        "privacy.share_active",
        "privacy.record_active",
        "privacy.no_activity",
        "privacy.status_unknown",
        "privacy.unavailable",
        "privacy.limited",
        "privacy.app_unknown",
        "privacy.since_startup",
        "privacy.open_settings",
        "privacy.settings_title",
        "privacy.settings_desc",
        "privacy.enabled",
        "privacy.enabled_desc",
        "privacy.fullscreen",
        "privacy.fullscreen_desc",
        "privacy.app_names",
        "privacy.app_names_desc",
        "privacy.experimental_note",
        "privacy.mic_note",
        "privacy.coverage_footer",
        "general.privacy",
        "general.privacy.desc",
        "general.privacy_open",
    ];

    for &key in &keys {
        let id_val = t(key, "id");
        let en_val = t(key, "en");
        assert_ne!(id_val, key, "Missing Indonesian translation for key: {key}");
        assert_ne!(en_val, key, "Missing English translation for key: {key}");
        assert_ne!(id_val, en_val, "ID and EN translations should differ for key: {key}");
    }
}


