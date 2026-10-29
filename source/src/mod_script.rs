use std::path::Path;
use std::sync::{Arc, Mutex};
use piccolo::{
    Callback, CallbackReturn, Closure, Executor, Fuel, Lua,
    Table, UserData, Value,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LuaModOutput {
    pub cards: Vec<LuaCard>,
    pub buttons: Vec<LuaButton>,
    pub texts: Vec<LuaText>,
    pub toasts: Vec<String>,
    pub urls_to_open: Vec<String>,
    pub clipboard_copies: Vec<String>,
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

#[derive(Clone, Debug, Default)]
pub struct LuaModState {
    pub last_clicked_button: Option<String>,
    pub output: LuaModOutput,
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
    let script_bytes = std::fs::read(script_path).map_err(|e| format!("Gagal membaca skrip mod: {e}"))?;
    run_lua_mod_source(&script_bytes, last_clicked_button)
}

pub fn run_lua_mod_source(script_bytes: &[u8], last_clicked_button: Option<&str>) -> Result<LuaModOutput, String> {
    let mut lua = Lua::core();

    let state = Arc::new(Mutex::new(LuaModState {
        last_clicked_button: last_clicked_button.map(|s| s.to_string()),
        output: LuaModOutput::default(),
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

        ctx.set_global("system", sys_table)?;

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
