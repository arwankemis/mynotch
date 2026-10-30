use super::power_broker::*;

#[test]
fn observed_component_energy_persists_separately_from_legacy_data(){
    let root=std::env::temp_dir().join(format!("mynotch-observed-energy-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let mut e=ObservedEnergy::load(&root);e.add("2026-10-30",0.1);e.add("2026-10-30",0.05);e.add("2026-10-31",f32::NAN);e.flush(&root).unwrap();
    let loaded=ObservedEnergy::load(&root);assert!((loaded.days["2026-10-30"]-0.15).abs()<0.00001);
    let h=loaded.history(2026);assert_eq!(h[0].provenance,"sensor_component_subtotal");assert!(loaded.history(2027).is_empty());
    assert!(!root.join("mod-data/daya/history").exists());std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn corrupted_component_history_is_never_overwritten(){
    let root=std::env::temp_dir().join(format!("mynotch-corrupt-energy-{}",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(root.join("mod-data/daya")).unwrap();let path=root.join("mod-data/daya/observed-energy.json");std::fs::write(&path,b"corrupted original").unwrap();
    let mut e=ObservedEnergy::load(&root);e.add("2026-10-30",0.1);assert!(e.flush(&root).is_err());assert_eq!(std::fs::read(path).unwrap(),b"corrupted original");std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn migration_keeps_legacy_when_history_cannot_be_written() {
    let root = std::env::temp_dir().join(format!("mynotch-migration-failure-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("mod-data"), b"block directory creation").unwrap();
    let mut state = crate::core::State::default();
    state.power_month = "2026-09".into();
    state.monthly_kwh = 42.5;
    state.monthly_cost_idr = 61400;
    assert!(!migrate_legacy_power_data(&root, &mut state));
    assert_eq!(state.monthly_kwh, 42.5);
    assert_eq!(state.monthly_cost_idr, 61400);
    assert_eq!(state.power_month, "2026-09");
    std::fs::remove_file(root.join("mod-data")).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn test_energy_integration_100w_one_hour_equals_0_1_kwh() {
    let mut integrator = EnergyIntegrator::new();
    integrator.max_gap_seconds = 4000.0;
    integrator.add_sample(100.0, 0.0);
    integrator.add_sample(100.0, 3600.0);
    assert!((integrator.today_kwh() - 0.1).abs() < 1e-4);
}

#[test]
fn test_energy_integration_gap_handling_ignores_sleep_and_disconnects() {
    let mut integrator = EnergyIntegrator::new();
    integrator.max_gap_seconds = 10.0; // default 10s continuity threshold
    integrator.add_sample(150.0, 0.0);
    integrator.add_sample(150.0, 2.0); // 2s valid sample -> integrated
    let valid_kwh = integrator.today_kwh();
    assert!(valid_kwh > 0.0);

    // PC goes to sleep or disconnected for 8 hours (28800 seconds)
    integrator.add_sample(150.0, 28802.0);
    // Gap exceeded max_gap_seconds: should NOT extrapolate 150W for 8 hours!
    assert_eq!(integrator.today_kwh(), valid_kwh, "Sleep gap must not extrapolate watts!");

    // Next sample 2s later resumes integration normally
    integrator.add_sample(150.0, 28804.0);
    assert!(integrator.today_kwh() > valid_kwh);
}

#[test]
fn test_history_persistence_roundtrip() {
    let temp_dir = std::env::temp_dir().join(format!("mynotch_test_hist_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);

    let hist = MonthlyPowerHistory {
        year: 2026,
        month: 10,
        name: "Oktober".into(),
        total_kwh: 31.79,
        total_cost_idr: 45927,
        provenance: "estimated".into(),
        days: 31,
        daily_buckets: vec![
            DailyPowerBucket {
                date: "2026-10-01".into(),
                energy_kwh: 1.02,
                cost_idr: 1473,
                samples_count: 3600,
                observed_seconds: 7200,
            }
        ],
    };

    save_monthly_history(&temp_dir, &hist).unwrap();
    let loaded = load_monthly_history(&temp_dir, 2026);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].month, 10);
    assert_eq!(loaded[0].total_cost_idr, 45927);
    assert_eq!(loaded[0].provenance, "estimated");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_migrate_legacy_power_data_runs_once_and_is_idempotent() {
    let temp_dir = std::env::temp_dir().join(format!("mynotch_test_mig_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);

    let mut state = crate::core::State {
        todos: vec![],
        notes: String::new(),
        water: vec![],
        clipboard_history: vec![],
        prayer_delivered: vec![],
        sticky_notes: vec![],
        selected_note: 0,
        power_month: "2026-09".to_string(),
        monthly_kwh: 42.5,
        monthly_cost_idr: 61400,
        power_days_active: vec!["2026-09-01".into(), "2026-09-02".into()],
    };

    // First run: migrates data
    let migrated = migrate_legacy_power_data(&temp_dir, &mut state);
    assert!(migrated, "First migration should return true");
    assert_eq!(state.monthly_kwh, 0.0);
    assert_eq!(state.monthly_cost_idr, 0);
    assert!(state.power_month.is_empty());
    assert!(state.power_days_active.is_empty());

    // Check saved file
    let loaded = load_monthly_history(&temp_dir, 2026);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].month, 9);
    assert_eq!(loaded[0].total_kwh, 42.5);
    assert_eq!(loaded[0].total_cost_idr, 61400);
    assert_eq!(loaded[0].provenance, "legacy_estimated_monthly");
    assert_eq!(loaded[0].days, 2);

    // Second run: idempotent, returns false, does not re-write or corrupt
    let migrated_again = migrate_legacy_power_data(&temp_dir, &mut state);
    assert!(!migrated_again, "Second run must be a no-op");

    let _ = std::fs::remove_dir_all(&temp_dir);
}
