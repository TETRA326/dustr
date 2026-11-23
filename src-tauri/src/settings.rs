// Persistent settings, stored as JSON in the app config dir.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// "system" | "light" | "dark"
    pub theme: String,
    /// "ask" (ask on first move each session) | "preset"
    pub move_mode: String,
    pub move_path: String,
    /// Action per direction: "move" | "keep" | "recycle" | "delete" | "none"
    pub up: String,
    pub right: String,
    pub down: String,
    pub left: String,
    pub arrow_keys: bool,
    pub wasd_keys: bool,
    pub delete_confirm: bool,
    /// Seconds (0..=10) to press delete a second time.
    pub delete_confirm_seconds: f64,
    /// "modified" | "created" | "name" | "size" | "type"
    pub sort_by: String,
    /// "asc" | "desc"
    pub sort_dir: String,
    /// "ask" | "include" | "skip"
    pub hidden_files: String,
    pub system_files: String,
    /// "ask" | "delete" | "never" (recursive sessions only)
    pub empty_folders: String,
    /// Seconds the "next folder" splash waits before starting.
    pub splash_seconds: f64,
    pub animations: bool,
    /// Use Windows preview handlers for Office and other documents.
    pub native_previews: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            move_mode: "ask".into(),
            move_path: String::new(),
            up: "move".into(),
            right: "keep".into(),
            down: "delete".into(),
            left: "recycle".into(),
            arrow_keys: true,
            wasd_keys: true,
            delete_confirm: true,
            delete_confirm_seconds: 3.0,
            sort_by: "modified".into(),
            sort_dir: "asc".into(),
            hidden_files: "ask".into(),
            system_files: "ask".into(),
            empty_folders: "ask".into(),
            splash_seconds: 3.0,
            animations: true,
            native_previews: true,
        }
    }
}

fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("settings.json"))
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    settings_path(&app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    let path = settings_path(&app).ok_or("No config directory")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}
