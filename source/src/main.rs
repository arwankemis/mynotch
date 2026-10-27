#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod core;
mod ui;
mod secrets;
mod media;
mod visualizer;
mod net;
mod online;
mod lyrics;
mod ical;
mod render;
mod shadow;
mod artwork;
mod clipboard;
mod shell;

fn main(){
    std::panic::set_hook(Box::new(|info|{let args:Vec<String>=std::env::args().collect();let preview=app::is_preview_args(&args);if let Ok(exe)=std::env::current_exe(){if let Ok(root)=app::storage_root(&exe,preview){let _=std::fs::create_dir_all(&root);let _=std::fs::write(root.join("panic.log"),format!("{info}"));}}}));
    let args:Vec<String>=std::env::args().collect();
    let preview=app::is_preview_args(&args);
    if let Err(error)=shell::run(&args){
        if let Ok(exe)=std::env::current_exe(){if let Ok(root)=app::storage_root(&exe,preview){let _=std::fs::create_dir_all(&root);let _=std::fs::write(root.join("error.log"),&error);}}
        use windows::Win32::UI::WindowsAndMessaging::*;use windows::core::{w,PCWSTR};
        let message:Vec<u16>=format!("mynotch tidak dapat dibuka.\n\n{error}\n\nPastikan folder aplikasi dapat ditulis.").encode_utf16().chain(Some(0)).collect();unsafe{MessageBoxW(None,PCWSTR(message.as_ptr()),w!("mynotch 0.0.1 alpha"),MB_OK|MB_ICONERROR);}
        std::process::exit(1);
    }
}
