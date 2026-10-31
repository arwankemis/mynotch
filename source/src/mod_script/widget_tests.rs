use crate::mod_script::*;
use std::collections::BTreeMap;

#[test]
fn test_lua_declarative_gauge_primitive() {
    let script = r#"
        ui.gauge({ id = "energy", value = 0.26, max = 1.0, unit = "kWh", label = "kWh · hari ini", target_label = "Target 1,00 kWh" })
    "#;
    let output = run_lua_mod_source(script.as_bytes(), None).unwrap();
    let gauge = output.gauge.expect("Gauge widget should be captured");
    assert_eq!(gauge.id, "energy");
    assert!((gauge.value.unwrap() - 0.26).abs() < 1e-4);
    assert!((gauge.max.unwrap() - 1.0).abs() < 1e-4);
    assert_eq!(gauge.unit, "kWh");
    assert_eq!(gauge.label, "kWh · hari ini");
    assert_eq!(gauge.target_label.as_deref(), Some("Target 1,00 kWh"));
}

#[test]
fn test_lua_declarative_metrics_primitive() {
    let script = r#"
        ui.metrics({
            note = "Estimasi · hanya saat pencatatan berjalan",
            items = {
                { label = "Daya sekarang", value = "141,6", unit = "W" },
                { label = "Biaya hari ini", value = "Rp 376" },
                { label = "Arus", value = "—", unit = "A" },
                { label = "Tegangan", value = "—", unit = "V" }
            }
        })
    "#;
    let output = run_lua_mod_source(script.as_bytes(), None).unwrap();
    let metrics = output.metrics.expect("Metrics widget should be captured");
    assert_eq!(metrics.items.len(), 4);
    assert_eq!(metrics.items[0].label, "Daya sekarang");
    assert_eq!(metrics.items[0].value, "141,6");
    assert_eq!(metrics.items[0].unit.as_deref(), Some("W"));
    assert_eq!(metrics.items[2].value, "—");
    assert_eq!(metrics.note.as_deref(), Some("Estimasi · hanya saat pencatatan berjalan"));
}

#[test]
fn test_lua_declarative_history_primitive() {
    let script = r#"
        ui.history({
            title = "Riwayat bulanan",
            year = "2026",
            unit = "kWh",
            items = {
                { id = "10", label = "Oktober", value = "31,79 kWh", meta = "cakupan parsial" },
                { id = "09", label = "September", value = "37,43 kWh", meta = "contoh" }
            }
        })
    "#;
    let output = run_lua_mod_source(script.as_bytes(), None).unwrap();
    let hist = output.history.expect("History widget should be captured");
    assert_eq!(hist.title, "Riwayat bulanan");
    assert_eq!(hist.year, "2026");
    assert_eq!(hist.unit, "kWh");
    assert_eq!(hist.items.len(), 2);
    assert_eq!(hist.items[0].label, "Oktober");
    assert_eq!(hist.items[0].value, "31,79 kWh");
}

#[test]
fn test_lua_capability_broker_enforces_permissions() {
    // 1. Without power.read permission: system.power_snapshot() returns nil
    let script_no_perm = r#"
        local snap = system.power_snapshot and system.power_snapshot()
        if snap == nil then
            ui.text("denied", 0, 0)
        else
            ui.text("leaked", 0, 0)
        end
    "#;
    let output_denied = run_lua_mod_source(script_no_perm.as_bytes(), None).unwrap();
    assert_eq!(output_denied.texts[0].text, "denied");

    // 2. With power.read permission: returns honest snapshot
    let script_with_perm = r#"
        local snap = system.power_snapshot()
        if snap ~= nil then
            ui.text(snap.source_kind .. ":" .. tostring(snap.active_power_w), 0, 0)
        end
    "#;
    let mut snap = crate::system::PowerSnapshot::default();
    snap.source_kind = crate::system::SourceKind::Estimated;
    snap.quality = crate::system::Quality::Valid;
    snap.active_power_w = Some(141.6);

    let output_allowed = run_lua_mod_source_capabilities(
        script_with_perm.as_bytes(),
        None,
        None,
        false,
        &BTreeMap::new(),
        &["power.read".to_string(), "storage.self".to_string()],
        Some(&snap),
        &[],
        None,
    ).unwrap();

    assert_eq!(output_allowed.texts[0].text, "estimated:141.6");
}
#[test]
fn official_daya_package_keeps_missing_measurements_unavailable_and_shows_hardware(){
    let mut snapshot=crate::system::PowerSnapshot::default();snapshot.cpu_name=Some("Test CPU".into());snapshot.gpu_name=Some("Test GPU".into());
    let output=super::run_lua_mod_source_capabilities(include_bytes!("../../assets/mods/daya/main.lua"),None,None,false,&std::collections::BTreeMap::new(),&["power.read".into()],Some(&snapshot),&[],None).unwrap();
    assert!(output.gauge.unwrap().value.is_none());
    let metrics=output.metrics.unwrap();assert_eq!(metrics.items[0].label,"Test CPU");assert_eq!(metrics.items[1].label,"Test GPU");assert_eq!(metrics.items[0].value,"—");
}

#[test]
fn daya_dashboard_separates_legacy_estimates_and_formats_sensor_values(){
    let mut snapshot=crate::system::PowerSnapshot::default();snapshot.gpu_watts=Some(51.2345);snapshot.active_power_w=Some(51.2345);snapshot.provider_note=Some("GPU ready, CPU needs setup".into());
    let h=|provenance:&str,kwh|crate::mods::power_broker::MonthlyPowerHistory{year:2026,month:10,name:"October".into(),total_kwh:kwh,total_cost_idr:0,provenance:provenance.into(),days:1,daily_buckets:vec![]};
    let out=super::run_lua_mod_source_capabilities(include_bytes!("../../assets/mods/daya/main.lua"),None,None,false,&std::collections::BTreeMap::new(),&["power.read".into()],Some(&snapshot),&[h("legacy_estimated_monthly",0.429789),h("sensor_component_subtotal",0.123456)],None).unwrap();
    let history=out.history.unwrap();assert_eq!(history.items.len(),1);assert_eq!(history.items[0].value,"0.123 kWh");
    let m=out.metrics.unwrap();assert_eq!(m.items[1].value,"51.2 W");assert_eq!(m.note.as_deref(),Some("GPU ready, CPU needs setup"));
}
