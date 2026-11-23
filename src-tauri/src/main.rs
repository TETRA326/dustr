// Dustr - swipe through a folder to clean it out.
// Copyright (C) 2026 Dustr contributors. Licensed under GPL-3.0-only.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod fsops;
mod office;
mod preview;
mod settings;
mod shellmenu;

use serde::Serialize;

#[derive(Serialize, Clone, Default)]
struct Launch {
    path: Option<String>,
    recursive: bool,
}

/// Parses `dustr [--dir|--recursive] <path>`.
fn parse_args() -> Launch {
    let mut launch = Launch::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dir" => launch.path = args.next(),
            "--recursive" => {
                launch.recursive = true;
                launch.path = args.next();
            }
            other if !other.starts_with("--") => launch.path = Some(other.to_string()),
            _ => {}
        }
    }
    // Explorer passes drive roots as "D:\" which the argv parser turns into `D:"`.
    if let Some(p) = launch.path.as_mut() {
        if p.ends_with('"') {
            p.pop();
            p.push(std::path::MAIN_SEPARATOR);
        }
    }
    launch
}

#[tauri::command]
fn launch_args(state: tauri::State<'_, Launch>) -> Launch {
    state.inner().clone()
}

#[tauri::command]
fn platform() -> &'static str {
    std::env::consts::OS
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(parse_args())
        .invoke_handler(tauri::generate_handler![
            launch_args,
            platform,
            settings::get_settings,
            settings::save_settings,
            fsops::list_dir,
            fsops::recycle,
            fsops::restore,
            fsops::delete_file,
            fsops::move_file,
            fsops::move_back,
            fsops::dir_is_empty,
            fsops::remove_empty_dir,
            fsops::path_exists,
            fsops::read_text,
            fsops::open_location,
            office::office_preview,
            shellmenu::shell_menu_set,
            shellmenu::shell_menu_status,
            preview::preview_prepare,
            preview::preview_show,
            preview::preview_hide,
            preview::preview_release,
            preview::preview_release_all,
            preview::preview_snapshot,
        ])
        .on_window_event(|_, event| {
            if let tauri::WindowEvent::Destroyed = event {
                preview::shutdown();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Dustr");
}
