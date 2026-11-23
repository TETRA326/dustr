// Classic Explorer context menu ("Show more options" on Windows 11).
// Written per-user under HKCU\Software\Classes, so no admin rights are needed.
// The installer writes the same keys; these commands let the user repair/remove them.

#[cfg(windows)]
const ROOTS: [&str; 3] = [
    r"Software\Classes\Directory\shell\Dustr",
    r"Software\Classes\Directory\Background\shell\Dustr",
    r"Software\Classes\Drive\shell\Dustr",
];

#[cfg(windows)]
#[tauri::command]
pub fn shell_menu_set(enabled: bool) -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for root in ROOTS {
        let _ = hkcu.delete_subkey_all(root);
    }
    if !enabled {
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy().to_string();
    let e = |r: std::io::Error| r.to_string();
    for root in ROOTS {
        let (k, _) = hkcu.create_subkey(root).map_err(e)?;
        k.set_value("MUIVerb", &"Dustr").map_err(e)?;
        k.set_value("Icon", &exe).map_err(e)?;
        k.set_value("SubCommands", &"").map_err(e)?;
        for (key, label, flag) in [
            ("1dir", "Dust only this directory", "--dir"),
            ("2rec", "Dust all recursive folders", "--recursive"),
        ] {
            let (sub, _) = hkcu.create_subkey(format!(r"{root}\shell\{key}")).map_err(e)?;
            sub.set_value("MUIVerb", &label).map_err(e)?;
            let (cmd, _) = hkcu.create_subkey(format!(r"{root}\shell\{key}\command")).map_err(e)?;
            cmd.set_value("", &format!("\"{exe}\" {flag} \"%V\"")).map_err(e)?;
        }
    }
    Ok(())
}

#[cfg(windows)]
#[tauri::command]
pub fn shell_menu_status() -> bool {
    use winreg::enums::HKEY_CURRENT_USER;
    winreg::RegKey::predef(HKEY_CURRENT_USER).open_subkey(ROOTS[0]).is_ok()
}

#[cfg(not(windows))]
#[tauri::command]
pub fn shell_menu_set(_enabled: bool) -> Result<(), String> {
    Err("The Explorer context menu is only available on Windows".into())
}

#[cfg(not(windows))]
#[tauri::command]
pub fn shell_menu_status() -> bool {
    false
}
