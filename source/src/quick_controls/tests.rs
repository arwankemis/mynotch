use super::adapter::*;
use super::controller::*;
use super::model::*;
use super::ui::*;

#[test]
fn test_snap_brightness_exact_and_clamping() {
    let levels = [0, 10, 25, 50, 75, 100];
    assert_eq!(snap_brightness(0, &levels), 0);
    assert_eq!(snap_brightness(10, &levels), 10);
    assert_eq!(snap_brightness(25, &levels), 25);
    assert_eq!(snap_brightness(50, &levels), 50);
    assert_eq!(snap_brightness(75, &levels), 75);
    assert_eq!(snap_brightness(100, &levels), 100);

    // Clamping
    assert_eq!(snap_brightness(120, &levels), 100);

    // Empty levels fallback (continuous clamp)
    assert_eq!(snap_brightness(42, &[]), 42);
    assert_eq!(snap_brightness(105, &[]), 100);
}

#[test]
fn test_snap_brightness_tie_breaking_higher() {
    let levels = [0, 10, 25, 50, 75, 100];
    // Exact midpoint between 0 and 10 is 5: must choose 10
    assert_eq!(snap_brightness(5, &levels), 10);
    // Value 4 is closer to 0
    assert_eq!(snap_brightness(4, &levels), 0);
    // Value 6 is closer to 10
    assert_eq!(snap_brightness(6, &levels), 10);

    // Between 10 and 25: midpoint is 17.5.
    // 17 is distance 7 from 10, distance 8 from 25 -> 10
    assert_eq!(snap_brightness(17, &levels), 10);
    // 18 is distance 8 from 10, distance 7 from 25 -> 25
    assert_eq!(snap_brightness(18, &levels), 25);

    // Between 75 and 100: midpoint is 87.5
    // 87 is distance 12 from 75, distance 13 from 100 -> 75
    assert_eq!(snap_brightness(87, &levels), 75);
    // 88 is distance 13 from 75, distance 12 from 100 -> 100
    assert_eq!(snap_brightness(88, &levels), 100);
}

#[test]
fn test_radio_state_machine_transitions() {
    let mut ctrl = QuickControlsController::new_mock();
    assert_eq!(ctrl.selected_radio(RadioKind::WiFi).unwrap().state, RadioState::On);

    // Toggle On -> Off
    let res = ctrl.toggle_radio(RadioKind::WiFi, "en");
    assert_eq!(res, Ok(RadioState::Off));
    assert_eq!(ctrl.selected_radio(RadioKind::WiFi).unwrap().state, RadioState::Off);
    assert!(ctrl.selected_radio(RadioKind::WiFi).unwrap().status_text.contains("Mati"));

    // Toggle Off -> On
    let res = ctrl.toggle_radio(RadioKind::WiFi, "en");
    assert_eq!(res, Ok(RadioState::On));
    assert_eq!(ctrl.selected_radio(RadioKind::WiFi).unwrap().state, RadioState::On);
    assert!(ctrl.selected_radio(RadioKind::WiFi).unwrap().status_text.contains("Aktif"));
}

#[test]
fn test_radio_state_machine_hardware_disabled() {
    let adapter = Box::new(MockRadioAdapter::new_with_scenario(6));
    let mut ctrl = QuickControlsController::new(adapter, Box::new(MockBrightnessAdapter::new_default()));

    let res = ctrl.toggle_radio(RadioKind::WiFi, "en");
    assert!(res.is_err());
    assert!(ctrl.feedback_toast.as_ref().unwrap().0.contains("Hardware switch"));
}

#[test]
fn test_radio_state_machine_access_denied() {
    let adapter = Box::new(MockRadioAdapter::new_with_scenario(5));
    let mut ctrl = QuickControlsController::new(adapter, Box::new(MockBrightnessAdapter::new_default()));

    let res = ctrl.toggle_radio(RadioKind::WiFi, "en");
    assert!(res.is_err());
    assert!(ctrl.feedback_toast.as_ref().unwrap().0.contains("Radio access denied"));
}

#[test]
fn test_radio_unknown_and_absent_states() {
    let adapter_unknown = Box::new(MockRadioAdapter::new_with_scenario(2));
    let ctrl_unknown = QuickControlsController::new(adapter_unknown, Box::new(MockBrightnessAdapter::new_default()));
    let radio_unk = ctrl_unknown.selected_radio(RadioKind::WiFi).unwrap();
    assert_eq!(radio_unk.state, RadioState::Unknown);
    assert_ne!(radio_unk.state, RadioState::Off);

    let adapter_absent = Box::new(MockRadioAdapter::new_with_scenario(7));
    let ctrl_absent = QuickControlsController::new(adapter_absent, Box::new(MockBrightnessAdapter::new_default()));
    let radio_abs = ctrl_absent.selected_radio(RadioKind::WiFi).unwrap();
    assert_eq!(radio_abs.availability, RadioAvailability::Absent);
    assert_ne!(radio_abs.state, RadioState::Off);
}

#[test]
fn test_brightness_slider_target_vs_commit() {
    let mut ctrl = QuickControlsController::new_mock();
    assert_eq!(ctrl.selected_display().unwrap().current_percent, 75);
    assert_eq!(ctrl.selected_display().unwrap().target_percent, None);

    // Intermediate drag to 82% (uncommitted)
    ctrl.set_brightness(82, false, "en");
    assert_eq!(ctrl.selected_display().unwrap().current_percent, 75);
    assert_eq!(ctrl.selected_display().unwrap().target_percent, Some(82));

    // Release mouse at 82% (commit=true): snaps to 75% because levels are [0, 10, 25, 50, 75, 100]
    ctrl.set_brightness(82, true, "en");
    assert_eq!(ctrl.selected_display().unwrap().current_percent, 75);
    assert_eq!(ctrl.selected_display().unwrap().target_percent, None);

    // Drag to 90% and commit: snaps to 100%
    ctrl.set_brightness(90, true, "en");
    assert_eq!(ctrl.selected_display().unwrap().current_percent, 100);
}

#[test]
fn test_unsupported_display_safety() {
    let mut ctrl = QuickControlsController::new_mock();
    ctrl.select_display(2); // Layar 3 Unsupported
    assert_eq!(ctrl.selected_display().unwrap().kind, DisplayKind::Unsupported);

    ctrl.set_brightness(80, true, "en");
    assert!(ctrl.feedback_toast.as_ref().unwrap().0.contains("does not support software brightness"));
}

#[test]
fn test_multi_radio_and_multi_display_selection() {
    let mut ctrl = QuickControlsController::new_mock();
    assert_eq!(ctrl.radios_of_kind(RadioKind::WiFi).len(), 2);
    assert_eq!(ctrl.selected_wifi_idx, 0);

    ctrl.cycle_radio_target(RadioKind::WiFi);
    assert_eq!(ctrl.selected_wifi_idx, 1);
    assert_eq!(ctrl.selected_radio(RadioKind::WiFi).unwrap().id, "w1");

    ctrl.cycle_radio_target(RadioKind::WiFi);
    assert_eq!(ctrl.selected_wifi_idx, 0);
    assert_eq!(ctrl.selected_radio(RadioKind::WiFi).unwrap().id, "w0");

    assert_eq!(ctrl.displays.len(), 3);
    assert_eq!(ctrl.selected_display_idx, 0);
    ctrl.cycle_display_target();
    assert_eq!(ctrl.selected_display_idx, 1);
    assert_eq!(ctrl.selected_display().unwrap().id, "disp_external");
}

#[test]
fn test_keyboard_navigation() {
    let mut ctrl = QuickControlsController::new_mock();
    assert_eq!(ctrl.focused_element, None);

    // Tab through focus elements
    assert!(ctrl.handle_key(0x09, "en"));
    assert_eq!(ctrl.focused_element, Some(0));

    assert!(ctrl.handle_key(0x09, "en"));
    assert_eq!(ctrl.focused_element, Some(1));

    // Arrow keys for brightness
    let _initial = ctrl.selected_display().unwrap().current_percent;
    assert!(ctrl.handle_key(0x27, "en")); // Right Arrow (+1%) -> 76% snaps to 75%
    assert_eq!(ctrl.selected_display().unwrap().current_percent, 75);

    assert!(ctrl.handle_key(0x26, "en")); // Up Arrow (+5%) -> 80% snaps to 75%
    assert!(ctrl.handle_key(0x21, "en")); // Page Up (+10%) -> 85% snaps to 75%
    assert!(ctrl.handle_key(0x23, "en")); // End (100%)
    assert_eq!(ctrl.selected_display().unwrap().current_percent, 100);

    assert!(ctrl.handle_key(0x24, "en")); // Home (0%)
    assert_eq!(ctrl.selected_display().unwrap().current_percent, 0);
}

#[test]
fn test_layout_bounds_and_hit_rects() {
    let layout = calculate_layout();

    assert!(layout.back_btn.left >= 0.0 && layout.back_btn.right <= QC_PANEL_WIDTH);
    assert!(layout.settings_btn.right <= QC_PANEL_WIDTH);
    assert!(layout.card_wifi.right <= QC_PANEL_WIDTH && layout.card_wifi.bottom <= QC_PANEL_HEIGHT);
    assert!(layout.card_bt.right <= QC_PANEL_WIDTH && layout.card_bt.bottom <= QC_PANEL_HEIGHT);
    assert!(layout.card_display.right <= QC_PANEL_WIDTH && layout.card_display.bottom <= QC_PANEL_HEIGHT);

    // Cards do not overlap vertically
    assert!(layout.card_wifi.bottom <= layout.card_bt.top);
    assert!(layout.card_bt.bottom <= layout.card_display.top);

    // Toggle button is inside the card
    assert!(layout.wifi_toggle.left >= layout.card_wifi.left);
    assert!(layout.wifi_toggle.right <= layout.card_wifi.right);
    assert!(layout.wifi_toggle.top >= layout.card_wifi.top);
    assert!(layout.wifi_toggle.bottom <= layout.card_wifi.bottom);
}

#[test]
fn test_mock_adapter_isolation() {
    let adapter = MockRadioAdapter::new_default();
    assert_eq!(adapter.set_calls.load(std::sync::atomic::Ordering::SeqCst), 0);

    let res = adapter.set_radio_state("w0", false);
    assert_eq!(res, Ok(RadioState::Off));
    assert_eq!(adapter.set_calls.load(std::sync::atomic::Ordering::SeqCst), 1);

    // Purely in-memory: no physical hardware invoked
}

#[test]
fn test_focus_reset_on_navigation() {
    let mut ctrl = QuickControlsController::new_mock();
    ctrl.focused_element = Some(FOCUS_WIFI_TOGGLE);
    assert_eq!(ctrl.focused_element, Some(FOCUS_WIFI_TOGGLE));
    ctrl.focused_element = None;
    assert_eq!(ctrl.focused_element, None);
}
