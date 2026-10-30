use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use piccolo::{
    Callback, CallbackReturn, Closure, Executor, Fuel, Lua,
    Table, UserData, Value,
};

#[cfg(test)]
mod widget_tests;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LuaModOutput {
    pub cards: Vec<LuaCard>,
    pub buttons: Vec<LuaButton>,
    pub texts: Vec<LuaText>,
    pub headline: Option<LuaHeadline>,
    pub ranked_left: Option<LuaRanked>,
    pub ranked_right: Option<LuaRanked>,
    pub gauge: Option<LuaGauge>,
    pub metrics: Option<LuaMetrics>,
    pub history: Option<LuaHistory>,
    /// Every URL the script asked about with `system.http_get`.
    pub http_urls: Vec<String>,
    /// URLs that have no fresh cache entry yet; the shell turns these into background fetches.
    pub http_requests: Vec<String>,
    pub toasts: Vec<String>,
    pub urls_to_open: Vec<String>,
    pub clipboard_copies: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LuaGauge {
    pub id: String,
    pub value: Option<f32>,
    pub max: Option<f32>,
    pub unit: String,
    pub label: String,
    pub target_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LuaMetricItem {
    pub label: String,
    pub value: String,
    pub unit: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LuaMetrics {
    pub items: Vec<LuaMetricItem>,
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LuaHistoryItem {
    pub id: String,
    pub label: String,
    pub value: String,
    pub meta: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LuaHistory {
    pub title: String,
    pub year: String,
    pub unit: String,
    pub items: Vec<LuaHistoryItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LuaHeadline {
    pub label: String,
    pub value: String,
    pub delta: String,
    pub meta: String,
    pub badge: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LuaRankRow {
    pub ticker: String,
    pub name: String,
    pub price: String,
    pub change: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LuaRanked {
    pub title: String,
    pub meta: String,
    pub rows: Vec<LuaRankRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LuaCard {
    pub title: String,
    pub value: String,
    pub subtext: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LuaButton {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LuaText {
    pub text: String,
    pub x: f32,
    pub y: f32,
}

use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct LuaModState {
    pub last_clicked_button: Option<String>,
    pub output: LuaModOutput,
    pub http_allowed: bool,
    /// URL -> response body or error. The shell only passes entries that are still fresh.
    pub http_cache: BTreeMap<String, Result<String, String>>,
    pub permissions: Vec<String>,
    pub power_snapshot: Option<crate::system::PowerSnapshot>,
    pub power_history: Vec<crate::mods::power_broker::MonthlyPowerHistory>,
    pub mod_data_dir: Option<PathBuf>,
}

fn json_to_value<'gc>(ctx: piccolo::Context<'gc>, value: &serde_json::Value) -> Value<'gc> {
    match value {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(flag) => Value::Boolean(*flag),
        serde_json::Value::Number(number) => match number.as_i64() {
            Some(integer) => Value::Integer(integer),
            None => Value::Number(number.as_f64().unwrap_or(0.0)),
        },
        serde_json::Value::String(text) => Value::String(ctx.intern(text.as_bytes())),
        serde_json::Value::Array(items) => {
            let table = Table::new(&ctx);
            for (index, item) in items.iter().enumerate() {
                let _ = table.set_value(&ctx, Value::Integer((index + 1) as i64), json_to_value(ctx, item));
            }
            Value::Table(table)
        }
        serde_json::Value::Object(map) => {
            let table = Table::new(&ctx);
            for (key, item) in map {
                let _ = table.set_value(&ctx, Value::String(ctx.intern(key.as_bytes())), json_to_value(ctx, item));
            }
            Value::Table(table)
        }
    }
}

fn value_to_string(val: Value) -> String {
    match val {
        Value::String(s) => String::from_utf8_lossy(s.as_bytes()).to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Number(f) => {
            if f.fract() == 0.0 {
                (f as i64).to_string()
            } else {
                format!("{f:.2}")
            }
        }
        Value::Boolean(b) => b.to_string(),
        Value::Nil => String::new(),
        _ => String::new(),
    }
}

pub fn run_lua_mod(script_path: &Path, last_clicked_button: Option<&str>) -> Result<LuaModOutput, String> {
    run_lua_mod_with_config(script_path, last_clicked_button, None, false, &BTreeMap::new())
}

pub fn run_lua_mod_with_config(
    script_path: &Path,
    last_clicked_button: Option<&str>,
    config: Option<&serde_json::Value>,
    http_allowed: bool,
    http_cache: &BTreeMap<String, Result<String, String>>,
) -> Result<LuaModOutput, String> {
    let script_bytes = std::fs::read(script_path).map_err(|e| format!("Gagal membaca skrip mod: {e}"))?;
    run_lua_mod_source_ext(&script_bytes, last_clicked_button, config, http_allowed, http_cache)
}

pub fn run_lua_mod_source(script_bytes: &[u8], last_clicked_button: Option<&str>) -> Result<LuaModOutput, String> {
    run_lua_mod_source_ext(script_bytes, last_clicked_button, None, false, &BTreeMap::new())
}

pub fn run_lua_mod_source_ext(
    script_bytes: &[u8],
    last_clicked_button: Option<&str>,
    config: Option<&serde_json::Value>,
    http_allowed: bool,
    http_cache: &BTreeMap<String, Result<String, String>>,
) -> Result<LuaModOutput, String> {
    run_lua_mod_source_capabilities(
        script_bytes,
        last_clicked_button,
        config,
        http_allowed,
        http_cache,
        &[],
        None,
        &[],
        None,
    )
}

pub fn run_lua_mod_source_capabilities(
    script_bytes: &[u8],
    last_clicked_button: Option<&str>,
    config: Option<&serde_json::Value>,
    http_allowed: bool,
    http_cache: &BTreeMap<String, Result<String, String>>,
    permissions: &[String],
    power_snapshot: Option<&crate::system::PowerSnapshot>,
    power_history: &[crate::mods::power_broker::MonthlyPowerHistory],
    mod_data_dir: Option<&Path>,
) -> Result<LuaModOutput, String> {
    let mut lua = Lua::core();

    let state = Arc::new(Mutex::new(LuaModState {
        last_clicked_button: last_clicked_button.map(|s| s.to_string()),
        output: LuaModOutput::default(),
        http_allowed,
        http_cache: http_cache.clone(),
        permissions: permissions.to_vec(),
        power_snapshot: power_snapshot.cloned(),
        power_history: power_history.to_vec(),
        mod_data_dir: mod_data_dir.map(|p| p.to_path_buf()),
    }));

    // Register UI & System sandbox APIs
    lua.try_enter(|ctx| {
        // Register state in global userdata
        let state_ud = UserData::new_static(&ctx, state.clone());
        ctx.set_global("__mod_state", state_ud)?;

        // ui table
        let ui_table = Table::new(&ctx);

        // ui.card(title, value, subtext)
        let cb_card = Callback::from_fn(&ctx, |ctx, _, stack| {
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    let title = value_to_string(stack.get(0));
                    let value = value_to_string(stack.get(1));
                    let subtext = if stack.len() > 2 && !stack.get(2).is_nil() {
                        Some(value_to_string(stack.get(2)))
                    } else {
                        None
                    };
                    if let Ok(mut st) = arc_state.lock() {
                        st.output.cards.push(LuaCard { title, value, subtext });
                    }
                }
            }
            Ok(CallbackReturn::Return)
        });
        ui_table.set(ctx, "card", cb_card)?;

        // ui.button(label, id) -> bool (true if clicked)
        let cb_button = Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let mut clicked = false;
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    let label = value_to_string(stack.get(0));
                    let id = if stack.len() > 1 && !stack.get(1).is_nil() {
                        value_to_string(stack.get(1))
                    } else {
                        label.clone()
                    };
                    if let Ok(mut st) = arc_state.lock() {
                        clicked = st.last_clicked_button.as_deref() == Some(&id);
                        if !st.output.buttons.iter().any(|b| b.id == id) {
                            st.output.buttons.push(LuaButton { id, label });
                        }
                    }
                }
            }
            stack.clear();
            stack.push_back(Value::Boolean(clicked));
            Ok(CallbackReturn::Return)
        });
        ui_table.set(ctx, "button", cb_button)?;

        // ui.text(text, x, y)
        let cb_text = Callback::from_fn(&ctx, |ctx, _, stack| {
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    let text = value_to_string(stack.get(0));
                    let x = stack.get(1).to_number().unwrap_or(0.0) as f32;
                    let y = stack.get(2).to_number().unwrap_or(0.0) as f32;
                    if let Ok(mut st) = arc_state.lock() {
                        st.output.texts.push(LuaText { text, x, y });
                    }
                }
            }
            Ok(CallbackReturn::Return)
        });
        ui_table.set(ctx, "text", cb_text)?;

        // ui.headline(label, value, delta, meta, badge) -> summary band above ranked lists
        let cb_headline = Callback::from_fn(&ctx, |ctx, _, stack| {
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    let headline = LuaHeadline {
                        label: value_to_string(stack.get(0)),
                        value: value_to_string(stack.get(1)),
                        delta: if stack.len() > 2 { value_to_string(stack.get(2)) } else { String::new() },
                        meta: if stack.len() > 3 { value_to_string(stack.get(3)) } else { String::new() },
                        badge: if stack.len() > 4 { value_to_string(stack.get(4)) } else { String::new() },
                    };
                    if let Ok(mut st) = arc_state.lock() {
                        st.output.headline = Some(headline);
                    }
                }
            }
            Ok(CallbackReturn::Return)
        });
        ui_table.set(ctx, "headline", cb_headline)?;

        // ui.ranked(side, title, meta, rows) -> one of the two ranked lanes
        // rows: array of { ticker = "BREN", name = "...", price = "6.475", change = 12.55 }
        let cb_ranked = Callback::from_fn(&ctx, |ctx, _, stack| {
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    let side = value_to_string(stack.get(0)).to_lowercase();
                    let mut rows = Vec::new();
                    if let Value::Table(table) = stack.get(3) {
                        let len = table.length().clamp(0, 5);
                        for index in 1..=len {
                            if let Value::Table(row) = table.get(ctx, index) {
                                rows.push(LuaRankRow {
                                    ticker: value_to_string(row.get(ctx, "ticker")),
                                    name: value_to_string(row.get(ctx, "name")),
                                    price: value_to_string(row.get(ctx, "price")),
                                    change: row.get(ctx, "change").to_number().unwrap_or(0.0),
                                });
                            }
                        }
                    }
                    let ranked = LuaRanked {
                        title: value_to_string(stack.get(1)),
                        meta: if stack.len() > 2 { value_to_string(stack.get(2)) } else { String::new() },
                        rows,
                    };
                    if let Ok(mut st) = arc_state.lock() {
                        if matches!(side.as_str(), "right" | "loser" | "losers") {
                            st.output.ranked_right = Some(ranked);
                        } else {
                            st.output.ranked_left = Some(ranked);
                        }
                    }
                }
            }
            Ok(CallbackReturn::Return)
        });
        ui_table.set(ctx, "ranked", cb_ranked)?;

        // ui.gauge(table)
        let cb_gauge = Callback::from_fn(&ctx, |ctx, _, stack| {
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    let arg = stack.get(0);
                    let mut id = "gauge".to_string();
                    let mut value = None;
                    let mut max = None;
                    let mut unit = "kWh".to_string();
                    let mut label = "kWh · hari ini".to_string();
                    let mut target_label = None;

                    if let Value::Table(tbl) = arg {
                        let id_val = tbl.get(ctx, "id");
                        if !id_val.is_nil() { id = value_to_string(id_val); }
                        let val_v = tbl.get(ctx, "value");
                        if let Some(num) = val_v.to_number() { value = Some(num as f32); }
                        let max_v = tbl.get(ctx, "max");
                        if let Some(num) = max_v.to_number() { max = Some(num as f32); }
                        let unit_v = tbl.get(ctx, "unit");
                        if !unit_v.is_nil() { unit = value_to_string(unit_v); }
                        let lbl_v = tbl.get(ctx, "label");
                        if !lbl_v.is_nil() { label = value_to_string(lbl_v); }
                        let tgt_v = tbl.get(ctx, "target_label");
                        if !tgt_v.is_nil() { target_label = Some(value_to_string(tgt_v)); }
                    }
                    if let Ok(mut st) = arc_state.lock() {
                        st.output.gauge = Some(LuaGauge { id, value, max, unit, label, target_label });
                    }
                }
            }
            Ok(CallbackReturn::Return)
        });
        ui_table.set(ctx, "gauge", cb_gauge)?;

        // ui.metrics(table)
        let cb_metrics = Callback::from_fn(&ctx, |ctx, _, stack| {
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    let arg = stack.get(0);
                    let mut note = None;
                    let mut items = Vec::new();

                    if let Value::Table(tbl) = arg {
                        let note_val = tbl.get(ctx, "note");
                        if !note_val.is_nil() {
                            note = Some(value_to_string(note_val));
                        }
                        let items_table = if let Value::Table(t) = tbl.get(ctx, "items") {
                            Some(t)
                        } else if tbl.length() > 0 {
                            Some(tbl)
                        } else {
                            None
                        };

                        if let Some(items_tbl) = items_table {
                            let len = items_tbl.length().clamp(0, 8);
                            for i in 1..=len {
                                if let Value::Table(item_tbl) = items_tbl.get(ctx, i) {
                                    let label = value_to_string(item_tbl.get(ctx, "label"));
                                    let value = value_to_string(item_tbl.get(ctx, "value"));
                                    let unit_val = item_tbl.get(ctx, "unit");
                                    let unit = if !unit_val.is_nil() { Some(value_to_string(unit_val)) } else { None };
                                    items.push(LuaMetricItem { label, value, unit });
                                }
                            }
                        }
                    }
                    if let Ok(mut st) = arc_state.lock() {
                        st.output.metrics = Some(LuaMetrics { items, note });
                    }
                }
            }
            Ok(CallbackReturn::Return)
        });
        ui_table.set(ctx, "metrics", cb_metrics)?;

        // ui.history(table)
        let cb_history = Callback::from_fn(&ctx, |ctx, _, stack| {
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    let arg = stack.get(0);
                    let mut title = "Riwayat bulanan".to_string();
                    let mut year = "2026".to_string();
                    let mut unit = "kWh".to_string();
                    let mut items = Vec::new();

                    if let Value::Table(tbl) = arg {
                        let t_val = tbl.get(ctx, "title");
                        if !t_val.is_nil() { title = value_to_string(t_val); }
                        let y_val = tbl.get(ctx, "year");
                        if !y_val.is_nil() { year = value_to_string(y_val); }
                        let u_val = tbl.get(ctx, "unit");
                        if !u_val.is_nil() { unit = value_to_string(u_val); }

                        let items_table = if let Value::Table(t) = tbl.get(ctx, "items") {
                            Some(t)
                        } else if tbl.length() > 0 {
                            Some(tbl)
                        } else {
                            None
                        };

                        if let Some(items_tbl) = items_table {
                            let len = items_tbl.length().clamp(0, 24);
                            for i in 1..=len {
                                if let Value::Table(item_tbl) = items_tbl.get(ctx, i) {
                                    let id = value_to_string(item_tbl.get(ctx, "id"));
                                    let label = value_to_string(item_tbl.get(ctx, "label"));
                                    let value = value_to_string(item_tbl.get(ctx, "value"));
                                    let meta_val = item_tbl.get(ctx, "meta");
                                    let meta = if !meta_val.is_nil() { Some(value_to_string(meta_val)) } else { None };
                                    items.push(LuaHistoryItem { id, label, value, meta });
                                }
                            }
                        }
                    }
                    if let Ok(mut st) = arc_state.lock() {
                        st.output.history = Some(LuaHistory { title, year, unit, items });
                    }
                }
            }
            Ok(CallbackReturn::Return)
        });
        ui_table.set(ctx, "history", cb_history)?;

        ctx.set_global("ui", ui_table)?;

        // system table
        let sys_table = Table::new(&ctx);

        // system.toast(msg)
        let cb_toast = Callback::from_fn(&ctx, |ctx, _, stack| {
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    let msg = value_to_string(stack.get(0));
                    if let Ok(mut st) = arc_state.lock() {
                        st.output.toasts.push(msg);
                    }
                }
            }
            Ok(CallbackReturn::Return)
        });
        sys_table.set(ctx, "toast", cb_toast)?;

        // system.open_url(url)
        let cb_open_url = Callback::from_fn(&ctx, |ctx, _, stack| {
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    let url = value_to_string(stack.get(0));
                    if let Ok(mut st) = arc_state.lock() {
                        st.output.urls_to_open.push(url);
                    }
                }
            }
            Ok(CallbackReturn::Return)
        });
        sys_table.set(ctx, "open_url", cb_open_url)?;

        // system.copy(text)
        let cb_copy = Callback::from_fn(&ctx, |ctx, _, stack| {
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    let text = value_to_string(stack.get(0));
                    if let Ok(mut st) = arc_state.lock() {
                        st.output.clipboard_copies.push(text);
                    }
                }
            }
            Ok(CallbackReturn::Return)
        });
        sys_table.set(ctx, "copy", cb_copy)?;

        // system.config: parsed config.json from the mod folder (empty table when absent)
        let config_value = match config {
            Some(value) => json_to_value(ctx, value),
            None => Value::Table(Table::new(&ctx)),
        };
        sys_table.set_value(&ctx, Value::String(ctx.intern(b"config")), config_value)?;

        // system.json_decode(text) -> Lua table or nil
        let cb_json_decode = Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let text = value_to_string(stack.get(0));
            let value = serde_json::from_str::<serde_json::Value>(&text)
                .map(|decoded| json_to_value(ctx, &decoded))
                .unwrap_or(Value::Nil);
            stack.clear();
            stack.push_back(value);
            Ok(CallbackReturn::Return)
        });
        sys_table.set(ctx, "json_decode", cb_json_decode)?;

        // system.http_get(url) -> body|nil, status ("ok" | "pending" | "error" | "blocked")
        let cb_http_get = Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let url = value_to_string(stack.get(0));
            let mut body = Value::Nil;
            let mut status = "pending";
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    if let Ok(mut st) = arc_state.lock() {
                        if !st.output.http_urls.iter().any(|u| u == &url) {
                            st.output.http_urls.push(url.clone());
                        }
                        if !st.http_allowed {
                            status = "blocked";
                        } else if let Some(result) = st.http_cache.get(&url) {
                            match result {
                                Ok(text) => {
                                    body = Value::String(ctx.intern(text.as_bytes()));
                                    status = "ok";
                                }
                                Err(_) => status = "error",
                            }
                        } else if !st.output.http_requests.iter().any(|u| u == &url) {
                            st.output.http_requests.push(url.clone());
                        }
                    }
                }
            }
            stack.clear();
            stack.push_back(body);
            stack.push_back(Value::String(ctx.intern(status.as_bytes())));
            Ok(CallbackReturn::Return)
        });
        sys_table.set(ctx, "http_get", cb_http_get)?;

        // system.power_snapshot() -> table or nil (requires power.read)
        let cb_power_snapshot = Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let mut res = Value::Nil;
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    if let Ok(st) = arc_state.lock() {
                        if st.permissions.iter().any(|p| p == "power.read") {
                            if let Some(ref snap) = st.power_snapshot {
                                let tbl = Table::new(&ctx);
                                let kind_str = match snap.source_kind {
                                    crate::system::SourceKind::Measured => "measured",
                                    crate::system::SourceKind::Estimated => "estimated",
                                    crate::system::SourceKind::Derived => "derived",
                                    crate::system::SourceKind::Unavailable => "unavailable",
                                };
                                let qual_str = match snap.quality {
                                    crate::system::Quality::Valid => "valid",
                                    crate::system::Quality::Stale => "stale",
                                    crate::system::Quality::Partial => "partial",
                                    crate::system::Quality::Disconnected => "disconnected",
                                    crate::system::Quality::Error => "error",
                                };
                                let _ = tbl.set(ctx, "source_kind", Value::String(ctx.intern(kind_str.as_bytes())));
                                let _ = tbl.set(ctx, "quality", Value::String(ctx.intern(qual_str.as_bytes())));
                                if let Some(w) = snap.active_power_w {
                                    let clean = (w as f64 * 1000.0).round() / 1000.0;
                                    let _ = tbl.set(ctx, "active_power_w", Value::Number(clean));
                                }
                                if let Some(a) = snap.current_a {
                                    let clean = (a as f64 * 1000.0).round() / 1000.0;
                                    let _ = tbl.set(ctx, "current_a", Value::Number(clean));
                                }
                                if let Some(v) = snap.voltage_v {
                                    let clean = (v as f64 * 1000.0).round() / 1000.0;
                                    let _ = tbl.set(ctx, "voltage_v", Value::Number(clean));
                                }
                                if let Some(h) = snap.hourly_cost_idr {
                                    let clean = (h as f64 * 1000.0).round() / 1000.0;
                                    let _ = tbl.set(ctx, "hourly_cost_idr", Value::Number(clean));
                                }
                                if let Some(d) = snap.daily_cost_idr {
                                    let clean = (d as f64 * 1000.0).round() / 1000.0;
                                    let _ = tbl.set(ctx, "daily_cost_idr", Value::Number(clean));
                                }
                                if let Some(m) = snap.monthly_cost_idr {
                                    let clean = (m as f64 * 1000.0).round() / 1000.0;
                                    let _ = tbl.set(ctx, "monthly_cost_idr", Value::Number(clean));
                                }
                                for (key,value) in [("cpu_name",snap.cpu_name.as_deref()),("gpu_name",snap.gpu_name.as_deref()),("provider_note",snap.provider_note.as_deref())]{if let Some(v)=value{let _=tbl.set(ctx,key,Value::String(ctx.intern(v.as_bytes())));}}
                                for (key,value) in [("cpu_watts",snap.cpu_watts),("gpu_watts",snap.gpu_watts),("today_kwh",snap.today_kwh)]{if let Some(v)=value{let _=tbl.set(ctx,key,Value::Number(v as f64));}}
                                let _=tbl.set(ctx,"current_year",Value::Integer(chrono::Datelike::year(&chrono::Local::now())as i64));
                                let _ = tbl.set(ctx, "is_active", Value::Boolean(true));
                                res = Value::Table(tbl);
                            }
                        }
                    }
                }
            }
            stack.clear();
            stack.push_back(res);
            Ok(CallbackReturn::Return)
        });
        sys_table.set(ctx, "power_snapshot", cb_power_snapshot)?;

        // system.power_history(year) -> array table or nil (requires power.read)
        let cb_power_history = Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let mut res = Value::Nil;
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    if let Ok(st) = arc_state.lock() {
                        if st.permissions.iter().any(|p| p == "power.read") {
                            let tbl = Table::new(&ctx);
                            for (idx, hist) in st.power_history.iter().enumerate() {
                                let item = Table::new(&ctx);
                                let _ = item.set(ctx, "year", Value::Integer(hist.year as i64));
                                let _ = item.set(ctx, "month", Value::Integer(hist.month as i64));
                                let _ = item.set(ctx, "name", Value::String(ctx.intern(hist.name.as_bytes())));
                                let _ = item.set(ctx, "total_kwh", Value::Number(hist.total_kwh as f64));
                                let _ = item.set(ctx, "total_cost_idr", Value::Integer(hist.total_cost_idr as i64));
                                let _ = item.set(ctx, "provenance", Value::String(ctx.intern(hist.provenance.as_bytes())));
                                let _ = item.set(ctx, "days", Value::Integer(hist.days as i64));
                                let _ = tbl.set(ctx, (idx + 1) as i64, Value::Table(item));
                            }
                            res = Value::Table(tbl);
                        }
                    }
                }
            }
            stack.clear();
            stack.push_back(res);
            Ok(CallbackReturn::Return)
        });
        sys_table.set(ctx, "power_history", cb_power_history)?;

        ctx.set_global("system", sys_table)?;

        // storage table
        let storage_table = Table::new(&ctx);
        let self_table = Table::new(&ctx);

        let cb_storage_get = Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let key = value_to_string(stack.get(0));
            let mut val = Value::Nil;
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    if let Ok(st) = arc_state.lock() {
                        if st.permissions.iter().any(|p| p == "storage.self") {
                            if let Some(ref dir) = st.mod_data_dir {
                                let safe = if crate::mods::safe_mod_id(&key) { key } else { "val".to_string() };
                                let file = dir.join(format!("{safe}.txt"));
                                if let Ok(txt) = std::fs::read_to_string(&file) {
                                    val = Value::String(ctx.intern(txt.as_bytes()));
                                }
                            }
                        }
                    }
                }
            }
            stack.clear();
            stack.push_back(val);
            Ok(CallbackReturn::Return)
        });
        self_table.set(ctx, "get", cb_storage_get)?;

        let cb_storage_set = Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let key = value_to_string(stack.get(0));
            let text = value_to_string(stack.get(1));
            let mut ok = false;
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    if let Ok(st) = arc_state.lock() {
                        if st.permissions.iter().any(|p| p == "storage.self") {
                            if let Some(ref dir) = st.mod_data_dir {
                                if std::fs::create_dir_all(dir).is_ok() {
                                    let safe = if crate::mods::safe_mod_id(&key) { key } else { "val".to_string() };
                                    let file = dir.join(format!("{safe}.txt"));
                                    ok = std::fs::write(&file, text.as_bytes()).is_ok();
                                }
                            }
                        }
                    }
                }
            }
            stack.clear();
            stack.push_back(Value::Boolean(ok));
            Ok(CallbackReturn::Return)
        });
        self_table.set(ctx, "set", cb_storage_set)?;

        let cb_storage_get_json = Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let key = value_to_string(stack.get(0));
            let mut val = Value::Nil;
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    if let Ok(st) = arc_state.lock() {
                        if st.permissions.iter().any(|p| p == "storage.self") {
                            if let Some(ref dir) = st.mod_data_dir {
                                let safe = if crate::mods::safe_mod_id(&key) { key } else { "val".to_string() };
                                let file = dir.join(format!("{safe}.json"));
                                if let Ok(txt) = std::fs::read_to_string(&file) {
                                    if let Ok(decoded) = serde_json::from_str::<serde_json::Value>(&txt) {
                                        val = json_to_value(ctx, &decoded);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            stack.clear();
            stack.push_back(val);
            Ok(CallbackReturn::Return)
        });
        self_table.set(ctx, "get_json", cb_storage_get_json)?;

        let cb_storage_set_json = Callback::from_fn(&ctx, |ctx, _, mut stack| {
            let key = value_to_string(stack.get(0));
            let text = value_to_string(stack.get(1));
            let mut ok = false;
            let state_val = ctx.get_global("__mod_state");
            if let Value::UserData(ud) = state_val {
                if let Ok(arc_state) = ud.downcast_static::<Arc<Mutex<LuaModState>>>() {
                    if let Ok(st) = arc_state.lock() {
                        if st.permissions.iter().any(|p| p == "storage.self") {
                            if let Some(ref dir) = st.mod_data_dir {
                                if std::fs::create_dir_all(dir).is_ok() {
                                    let safe = if crate::mods::safe_mod_id(&key) { key } else { "val".to_string() };
                                    let file = dir.join(format!("{safe}.json"));
                                    ok = std::fs::write(&file, text.as_bytes()).is_ok();
                                }
                            }
                        }
                    }
                }
            }
            stack.clear();
            stack.push_back(Value::Boolean(ok));
            Ok(CallbackReturn::Return)
        });
        self_table.set(ctx, "set_json", cb_storage_set_json)?;

        storage_table.set(ctx, "self", self_table)?;
        ctx.set_global("storage", storage_table)?;

        Ok(())
    }).map_err(|e| format!("Gagal menginisialisasi sandbox Lua: {e:?}"))?;

    // Load chunk
    let executor = lua.try_enter(|ctx| {
        let closure = Closure::load(ctx, Some("mod"), script_bytes)?;
        Ok(ctx.stash(Executor::start(ctx, closure.into(), ())))
    }).map_err(|e| format!("Gagal memuat sintaks skrip Lua: {e:?}"))?;

    // Bounded execution with fuel
    const MAX_STEPS: usize = 200; // 200 * 4096 = ~819k instructions
    const FUEL_PER_STEP: i32 = 4096;
    let mut finished = false;

    for _ in 0..MAX_STEPS {
        let mut fuel = Fuel::with(FUEL_PER_STEP);
        let done = lua.enter(|ctx| ctx.fetch(&executor).step(ctx, &mut fuel));
        if done {
            finished = true;
            break;
        }
    }

    if !finished {
        return Err("Skrip melebihi batas waktu eksekusi (loop tak terbatas dicegah).".into());
    }

    // Check for runtime errors
    let result: Result<(), String> = lua.enter(|ctx| {
        match ctx.fetch(&executor).take_result::<()>(ctx) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(format!("{e:?}")),
            Err(mode) => Err(format!("{mode:?}")),
        }
    });

    if let Err(e) = result {
        return Err(format!("Kesalahan runtime skrip Lua: {e}"));
    }

    let final_state = state.lock().map_err(|_| "Gagal mengunci state Lua".to_string())?;
    Ok(final_state.output.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_lua_mod_declarative_elements() {
        let script = br#"
            ui.card("Kurs USD", "Rp 16.250", "Update 10:00")
            ui.card("Status", "Aktif")
            ui.text("Catatan Kecil", 10.5, 20.0)

            local clicked = ui.button("Buka Web", "btn_open")
            assert(not clicked)
        "#;

        let output = run_lua_mod_source(script, None).expect("Skrip harus berhasil dieksekusi");
        assert_eq!(output.cards.len(), 2);
        assert_eq!(output.cards[0].title, "Kurs USD");
        assert_eq!(output.cards[0].value, "Rp 16.250");
        assert_eq!(output.cards[0].subtext.as_deref(), Some("Update 10:00"));
        assert_eq!(output.cards[1].subtext, None);

        assert_eq!(output.texts.len(), 1);
        assert_eq!(output.texts[0].text, "Catatan Kecil");
        assert_eq!(output.texts[0].x, 10.5);

        assert_eq!(output.buttons.len(), 1);
        assert_eq!(output.buttons[0].id, "btn_open");
        assert_eq!(output.buttons[0].label, "Buka Web");
    }

    #[test]
    fn test_run_lua_mod_button_click_and_system_actions() {
        let script = br#"
            if ui.button("Kirim Notifikasi", "btn_notify") then
                system.toast("Halo dari mod!")
                system.open_url("https://example.com")
                system.copy("Teks disalin!")
            end
        "#;

        // When not clicked
        let output1 = run_lua_mod_source(script, None).expect("Skrip harus berhasil");
        assert!(output1.toasts.is_empty());
        assert!(output1.urls_to_open.is_empty());
        assert!(output1.clipboard_copies.is_empty());

        // When button "btn_notify" is clicked
        let output2 = run_lua_mod_source(script, Some("btn_notify")).expect("Skrip harus berhasil");
        assert_eq!(output2.toasts, vec!["Halo dari mod!"]);
        assert_eq!(output2.urls_to_open, vec!["https://example.com"]);
        assert_eq!(output2.clipboard_copies, vec!["Teks disalin!"]);
    }

    #[test]
    fn test_run_lua_mod_headline_and_ranked() {
        let script = r#"
            ui.headline("IHSG · IDX Composite", "7.648,32", "+32,14 (+0,42%)", "Sesi 2 · 13:30–15:49 WIB", "CONTOH DATA")
            ui.ranked("left", "Top 5 Gainer", "5 emiten · +8,32%", {
                { ticker = "BREN", name = "Barito Renewables", price = "6.475", change = 12.55 },
                { ticker = "CUAN", name = "Petrindo Jaya Kreasi", price = "3.120", change = 9.86 }
            })
            ui.ranked("right", "Top 5 Loser", "5 emiten · -4,90%", {
                { ticker = "GOTO", name = "GoTo Gojek Tokopedia", price = "68", change = -6.85 }
            })
        "#;

        let output = run_lua_mod_source(script.as_bytes(), None).expect("Skrip headline/ranked harus berhasil");
        let headline = output.headline.expect("headline harus terbaca");
        assert_eq!(headline.value, "7.648,32");
        assert_eq!(headline.badge, "CONTOH DATA");

        let left = output.ranked_left.expect("lane kiri harus terbaca");
        assert_eq!(left.title, "Top 5 Gainer");
        assert_eq!(left.rows.len(), 2);
        assert_eq!(left.rows[0].ticker, "BREN");
        assert!((left.rows[0].change - 12.55).abs() < 1e-9);

        let right = output.ranked_right.expect("lane kanan harus terbaca");
        assert_eq!(right.rows.len(), 1);
        assert_eq!(right.rows[0].ticker, "GOTO");
        assert!((right.rows[0].change + 6.85).abs() < 1e-9);
    }

    #[test]
    fn test_run_lua_mod_config_json_and_http_cache() {
        let config = serde_json::json!({ "endpoint": "https://example.test/data", "refresh_seconds": 60 });
        let mut cache = BTreeMap::new();
        cache.insert("https://example.test/data".to_string(), Ok(r#"{"a":[1,2],"b":"x"}"#.to_string()));
        let script = r#"
            assert(system.config.endpoint == "https://example.test/data")
            local body, status = system.http_get(system.config.endpoint)
            assert(status == "ok", status)
            local data = system.json_decode(body)
            assert(data.a[1] == 1 and data.a[2] == 2)
            assert(data.b == "x")
        "#;
        let output = run_lua_mod_source_ext(script.as_bytes(), None, Some(&config), true, &cache).unwrap();
        assert_eq!(output.http_urls, vec!["https://example.test/data"]);
        assert!(output.http_requests.is_empty());
    }

    #[test]
    fn test_run_lua_mod_http_pending_and_blocked() {
        let config = serde_json::json!({ "endpoint": "https://example.test/data" });
        let script = r#"
            local body, status = system.http_get(system.config.endpoint)
            assert(body == nil)
            assert(status == "pending", status)
        "#;
        let output = run_lua_mod_source_ext(script.as_bytes(), None, Some(&config), true, &BTreeMap::new()).unwrap();
        assert_eq!(output.http_requests, vec!["https://example.test/data"]);

        let script = r#"
            local body, status = system.http_get("https://example.test/data")
            assert(body == nil)
            assert(status == "blocked", status)
        "#;
        let output = run_lua_mod_source_ext(script.as_bytes(), None, None, false, &BTreeMap::new()).unwrap();
        assert_eq!(output.http_urls, vec!["https://example.test/data"]);
    }

    #[test]
    fn actual_ihsg_mod_renders_with_a_cached_yahoo_body() {
        let script = include_bytes!("../../examples/mods/ihsg-monitor/main.lua");
        let config = serde_json::json!({
            "refresh_seconds": 60,
            "endpoints": ["https://example.test/data"]
        });

        let pending = run_lua_mod_source_ext(script, None, Some(&config), true, &BTreeMap::new()).expect("pending state must render");
        assert!(pending.headline.is_some());
        assert_eq!(pending.http_requests, vec!["https://example.test/data"]);

        let body = r#"{"spark":{"result":[
          {"symbol":"^JKSE","response":[{"meta":{"regularMarketPrice":7648.32,"regularMarketChangePercent":0.42,"chartPreviousClose":7616.18}}]},
          {"symbol":"BREN.JK","response":[{"meta":{"regularMarketPrice":6475,"regularMarketChangePercent":12.55,"shortName":"Barito Renewables"}}]},
          {"symbol":"GOTO.JK","response":[{"meta":{"regularMarketPrice":68,"regularMarketChangePercent":-6.85,"shortName":"GoTo"}}]}
        ]}}"#;
        let mut cache = BTreeMap::new();
        cache.insert("https://example.test/data".to_string(), Ok(body.to_string()));
        let output = run_lua_mod_source_ext(script, None, Some(&config), true, &cache).expect("cached Yahoo body must render");
        assert!(output.headline.is_some());
        assert_eq!(output.ranked_left.as_ref().unwrap().rows[0].ticker, "BREN");
        assert_eq!(output.ranked_right.as_ref().unwrap().rows[0].ticker, "GOTO");
    }

    #[test]
    fn actual_ihsg_mod_fills_the_gainer_lane_when_positives_are_scarce() {
        let script = include_bytes!("../../examples/mods/ihsg-monitor/main.lua");
        let config = serde_json::json!({
            "refresh_seconds": 60,
            "endpoints": ["https://example.test/data"]
        });
        let body = r#"{"spark":{"result":[
          {"symbol":"^JKSE","response":[{"meta":{"regularMarketPrice":5110.71,"regularMarketChangePercent":-3.25,"chartPreviousClose":5266.13}}]},
          {"symbol":"WIIM.JK","response":[{"meta":{"regularMarketPrice":740,"regularMarketChangePercent":2.778,"shortName":"Wismilak"}}]},
          {"symbol":"BMAS.JK","response":[{"meta":{"regularMarketPrice":1045,"regularMarketChangePercent":0.966,"shortName":"Bank Maspion"}}]},
          {"symbol":"BBCA.JK","response":[{"meta":{"regularMarketPrice":4970,"regularMarketChangePercent":-5.79,"shortName":"Bank Central Asia"}}]},
          {"symbol":"BREN.JK","response":[{"meta":{"regularMarketPrice":1930,"regularMarketChangePercent":-6.763,"shortName":"Barito Renewables"}}]}
        ]}}"#;
        let mut cache = BTreeMap::new();
        cache.insert("https://example.test/data".to_string(), Ok(body.to_string()));
        let output = run_lua_mod_source_ext(script, None, Some(&config), true, &cache).unwrap();
        let left = output.ranked_left.expect("lane kiri harus terbaca");
        assert_eq!(left.title, "Top 5 Teratas");
        assert_eq!(left.meta, "2 naik · sisanya terbaik");
        assert_eq!(left.rows.len(), 4);
        assert_eq!(left.rows[0].ticker, "WIIM");
        assert_eq!(left.rows[3].ticker, "BREN");
    }

    #[test]
    fn actual_ihsg_mod_keeps_the_gainer_lane_filled_when_everything_falls() {
        let script = include_bytes!("../../examples/mods/ihsg-monitor/main.lua");
        let config = serde_json::json!({
            "refresh_seconds": 60,
            "endpoints": ["https://example.test/data"]
        });
        let body = r#"{"spark":{"result":[
          {"symbol":"^JKSE","response":[{"meta":{"regularMarketPrice":5110.71,"regularMarketChangePercent":-3.25,"chartPreviousClose":5266.13}}]},
          {"symbol":"BBCA.JK","response":[{"meta":{"regularMarketPrice":4970,"regularMarketChangePercent":-5.79,"shortName":"Bank Central Asia"}}]},
          {"symbol":"GOTO.JK","response":[{"meta":{"regularMarketPrice":24,"regularMarketChangePercent":0,"shortName":"GoTo"}}]}
        ]}}"#;
        let mut cache = BTreeMap::new();
        cache.insert("https://example.test/data".to_string(), Ok(body.to_string()));
        let output = run_lua_mod_source_ext(script, None, Some(&config), true, &cache).unwrap();
        let left = output.ranked_left.expect("lane kiri harus tetap terisi");
        assert_eq!(left.title, "Paling Tahan");
        assert_eq!(left.meta, "Tidak ada emiten naik");
        assert_eq!(left.rows.len(), 2);
        assert_eq!(left.rows[0].ticker, "GOTO");
    }

    #[test]
    fn test_infinite_loop_protection() {
        let script = br#"
            while true do
                -- infinite loop
            end
        "#;

        let err = run_lua_mod_source(script, None).unwrap_err();
        assert!(err.contains("loop tak terbatas"));
    }
}
