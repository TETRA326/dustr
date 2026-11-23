// File system operations used by a dusting session.
use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub ext: String,
    pub size: u64,
    pub modified: f64,
    pub created: f64,
    pub hidden: bool,
    pub system: bool,
}

#[derive(Serialize)]
pub struct DirListing {
    pub files: Vec<FileEntry>,
    /// Immediate subdirectories (absolute paths), sorted by name. Links/junctions excluded.
    pub dirs: Vec<String>,
}

fn ms(t: std::io::Result<std::time::SystemTime>) -> f64 {
    t.ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

#[cfg(windows)]
fn attrs(meta: &fs::Metadata, _name: &str) -> (bool, bool) {
    use std::os::windows::fs::MetadataExt;
    let a = meta.file_attributes();
    (a & 0x2 != 0, a & 0x4 != 0)
}

#[cfg(not(windows))]
fn attrs(_meta: &fs::Metadata, name: &str) -> (bool, bool) {
    (name.starts_with('.'), false)
}

#[tauri::command]
pub async fn list_dir(path: String) -> Result<DirListing, String> {
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    let rd = fs::read_dir(&path).map_err(|e| format!("Cannot read {path}: {e}"))?;
    for entry in rd.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        let name = entry.file_name().to_string_lossy().to_string();
        let p = entry.path();
        if ft.is_dir() {
            if !ft.is_symlink() {
                dirs.push(p.to_string_lossy().to_string());
            }
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            continue; // symlink to a directory
        }
        let (hidden, system) = attrs(&meta, &name);
        let ext = p
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        files.push(FileEntry {
            path: p.to_string_lossy().to_string(),
            name,
            ext,
            size: meta.len(),
            modified: ms(meta.modified()),
            created: ms(meta.created()),
            hidden,
            system,
        });
    }
    dirs.sort_by_key(|d| d.to_lowercase());
    Ok(DirListing { files, dirs })
}

#[tauri::command]
pub async fn recycle(path: String) -> Result<(), String> {
    trash::delete(&path).map_err(|e| e.to_string())
}

/// Undo for `recycle`: restore the most recently recycled item with this original path.
#[tauri::command]
pub async fn restore(path: String) -> Result<(), String> {
    #[cfg(any(windows, all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android"))))]
    {
        use trash::os_limited::{list, restore_all};
        let target = PathBuf::from(&path);
        let item = list()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|i| i.original_path() == target)
            .max_by_key(|i| i.time_deleted)
            .ok_or("File not found in the Recycle Bin")?;
        restore_all([item]).map_err(|e| e.to_string())
    }
    #[cfg(not(any(windows, all(unix, not(target_os = "macos"), not(target_os = "ios"), not(target_os = "android")))))]
    {
        let _ = path;
        Err("Restoring from the trash is not supported on this platform".into())
    }
}

#[tauri::command]
pub async fn delete_file(path: String) -> Result<(), String> {
    fs::remove_file(&path).map_err(|e| e.to_string())
}

fn unique_target(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let p = Path::new(name);
    let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    (1..)
        .map(|n| dir.join(format!("{stem} ({n}){ext}")))
        .find(|c| !c.exists())
        .unwrap()
}

fn move_path(from: &Path, to: &Path) -> Result<(), String> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    // Different volume (or rename refused): copy, then remove the original.
    fs::copy(from, to).map_err(|e| e.to_string())?;
    if let Err(e) = fs::remove_file(from) {
        let _ = fs::remove_file(to);
        return Err(e.to_string());
    }
    Ok(())
}

/// Moves a file into `dest_dir`, renaming on collision. Returns the new path.
#[tauri::command]
pub async fn move_file(path: String, dest_dir: String) -> Result<String, String> {
    let from = PathBuf::from(&path);
    let name = from.file_name().ok_or("Invalid file")?.to_string_lossy().to_string();
    let dir = PathBuf::from(&dest_dir);
    if !dir.is_dir() {
        return Err(format!("Folder not found: {dest_dir}"));
    }
    let to = unique_target(&dir, &name);
    move_path(&from, &to)?;
    Ok(to.to_string_lossy().to_string())
}

/// Undo for `move_file`.
#[tauri::command]
pub async fn move_back(from: String, to: String) -> Result<(), String> {
    let to_p = PathBuf::from(&to);
    if to_p.exists() {
        return Err("A file already exists at the original location".into());
    }
    move_path(Path::new(&from), &to_p)
}

#[tauri::command]
pub async fn dir_is_empty(path: String) -> bool {
    fs::read_dir(&path).map(|mut r| r.next().is_none()).unwrap_or(false)
}

#[tauri::command]
pub async fn remove_empty_dir(path: String) -> Result<(), String> {
    fs::remove_dir(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn path_exists(path: String) -> bool {
    Path::new(&path).exists()
}

#[derive(Serialize)]
pub struct TextPreview {
    pub text: String,
    pub truncated: bool,
    pub binary: bool,
}

#[tauri::command]
pub async fn read_text(path: String, max_bytes: usize) -> Result<TextPreview, String> {
    let mut f = fs::File::open(&path).map_err(|e| e.to_string())?;
    let mut buf = Vec::with_capacity(max_bytes.min(1 << 20));
    f.by_ref()
        .take(max_bytes as u64 + 1)
        .read_to_end(&mut buf)
        .map_err(|e| e.to_string())?;
    let truncated = buf.len() > max_bytes;
    buf.truncate(max_bytes);
    let binary = buf.iter().take(8192).any(|b| *b == 0);
    let text = if binary { String::new() } else { String::from_utf8_lossy(&buf).to_string() };
    Ok(TextPreview { text, truncated, binary })
}

#[tauri::command]
pub fn open_location(path: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer.exe")
            .raw_arg(format!("/select,\"{path}\""))
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg("-R").arg(&path).spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let p = PathBuf::from(&path);
        let dir = if p.is_dir() { p } else { p.parent().map(|d| d.to_path_buf()).unwrap_or(p) };
        std::process::Command::new("xdg-open").arg(dir).spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}
