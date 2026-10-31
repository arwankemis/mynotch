#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod core;
mod ui;
mod secrets;
mod media;
mod visualizer;
mod net;
mod online;
mod weather;
mod weather_icons;
mod lyrics;
mod ical;
mod render;
mod artwork;
mod clipboard;
mod mods;
mod sticky;
pub mod i18n;
pub mod mod_script;

mod system;
pub mod update;
pub mod privacy;
pub mod motion;
pub mod glass;
mod shell;

fn main(){
    std::panic::set_hook(Box::new(|info|{let args:Vec<String>=std::env::args().collect();let preview=app::is_preview_args(&args);if let Ok(exe)=std::env::current_exe(){if let Ok(root)=app::storage_root(&exe,preview){let _=std::fs::create_dir_all(&root);let _=std::fs::write(root.join("panic.log"),format!("{info}"));}}}));
    let args:Vec<String>=std::env::args().collect();
    if args.iter().any(|a|a=="--power-probe"){
        let sensors=system::power_sensor::PowerSensors::start();let start=std::time::Instant::now();
        loop{let s=sensors.snapshot();if s.active_power_w.is_some()||start.elapsed()>std::time::Duration::from_secs(15){println!("{}",serde_json::to_string_pretty(&s).unwrap_or_default());return}std::thread::sleep(std::time::Duration::from_millis(50));}
    }


    // 1. Helper mode dispatch BEFORE UI/tray/single-instance mutex initialization!
    if let Some(tx_dir) = update::helper::is_helper_invocation(&args) {
        let _ = update::helper::run_helper(&tx_dir);
        std::process::exit(0);
    }

    // 2. Healthcheck ACK when launched after update
    for i in 0..args.len() {
        if args[i] == "--update-ack-tx" && i + 1 < args.len() {
            let _ = update::helper::write_health_ack(std::path::Path::new(&args[i + 1]));
        }
    }

    // 3. Durable pending update on normal startup
    let preview=app::is_preview_args(&args);
    if !preview {
        if let Ok(exe) = std::env::current_exe() {
            if let Ok(root) = app::storage_root(&exe, false) {
                if let Some(pending) = update::ui::PendingUpdate::load(&root) {
                    if pending.tx_dir.exists() {
                        let journal_file = update::transaction::TransactionJournal::journal_file(&pending.tx_dir);
                        if journal_file.exists() {
                            if let Ok(journal) = update::transaction::TransactionJournal::load(&pending.tx_dir) {
                                if journal.phase == update::transaction::TransactionPhase::Prepared {
                                    if let Some(install_dir) = exe.parent() {
                                        if let Ok(helper_exe) = update::ui::prepare_helper_copy(install_dir, &pending.tx_dir) {
                                            update::ui::PendingUpdate::clear(&root);
                                            let _ = update::ui::spawn_helper(&helper_exe, &pending.tx_dir);
                                            std::process::exit(0);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    update::ui::PendingUpdate::clear(&root);
                }
            }
        }
    }

    if let Err(error)=shell::run(&args){
        if let Ok(exe)=std::env::current_exe(){if let Ok(root)=app::storage_root(&exe,preview){let _=std::fs::create_dir_all(&root);let _=std::fs::write(root.join("error.log"),&error);}}
        use windows::Win32::UI::WindowsAndMessaging::*;use windows::core::PCWSTR;
        let message:Vec<u16>=format!("mynotch tidak dapat dibuka.\n\n{error}\n\nPastikan folder aplikasi dapat ditulis.").encode_utf16().chain(Some(0)).collect();
        let title:Vec<u16>=shell::app_title().encode_utf16().chain(Some(0)).collect();
        unsafe{MessageBoxW(None,PCWSTR(message.as_ptr()),PCWSTR(title.as_ptr()),MB_OK|MB_ICONERROR);}
        std::process::exit(1);
    }
}
