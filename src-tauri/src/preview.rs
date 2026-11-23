// Native document previews on Windows, using the same preview handlers as
// Explorer's preview pane (Word, Excel, PowerPoint, PDF readers, etc.).
//
// All handlers live on one dedicated STA thread with its own message loop, so
// slow handlers (Office) never block the UI. Each preview gets its own owned,
// disabled popup window: it floats over the card area, never takes focus, and
// keyboard input stays with Dustr. Previews can be prepared ahead of time
// (hidden) and shown later. For swipe animations, the frontend takes a
// snapshot of the visible preview, hides the native window and animates the
// snapshot image instead.
//
// On other platforms every command is a no-op and `preview_prepare` returns false.

use serde::Deserialize;

/// Rectangle in CSS pixels, relative to the webview's top-left corner.
#[derive(Deserialize, Clone, Copy, Debug)]
#[cfg_attr(not(windows), allow(dead_code))]
pub struct CssRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[cfg(windows)]
mod imp {
    use super::CssRect;
    use base64::Engine;
    use std::collections::HashMap;
    use std::sync::{mpsc, Mutex, OnceLock};
    use windows::core::{w, Interface, HSTRING, PCWSTR, PWSTR};
    use windows::Win32::Foundation::*;
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::System::Com::*;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Shell::PropertiesSystem::{IInitializeWithFile, IInitializeWithStream};
    use windows::Win32::UI::Shell::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    #[derive(Clone, Copy, Debug)]
    pub struct PRect {
        pub x: i32,
        pub y: i32,
        pub w: i32,
        pub h: i32,
    }

    enum Cmd {
        Prepare { id: String, path: String, owner: isize, rect: PRect, reply: mpsc::Sender<Result<(), String>> },
        Show { id: String, rect: PRect },
        Hide { id: String },
        Release { id: String },
        ReleaseAll,
        Snapshot { id: String, reply: mpsc::Sender<Option<String>> },
    }

    struct Worker {
        tx: mpsc::Sender<Cmd>,
        thread_id: u32,
    }

    static WORKER: OnceLock<Mutex<Worker>> = OnceLock::new();
    const CLASS: PCWSTR = w!("DustrPreviewHost");
    const PREVIEW_HANDLER_IID: PCWSTR = w!("{8895b1c6-b41f-4c1c-a562-0d564250836f}");

    fn send(cmd: Cmd) {
        let worker = WORKER.get_or_init(|| {
            let (tx, rx) = mpsc::channel();
            let (tid_tx, tid_rx) = mpsc::channel();
            std::thread::Builder::new()
                .name("dustr-preview".into())
                .spawn(move || unsafe { run(rx, tid_tx) })
                .expect("spawn preview thread");
            let thread_id = tid_rx.recv().unwrap_or(0);
            Mutex::new(Worker { tx, thread_id })
        });
        let w = worker.lock().unwrap();
        let _ = w.tx.send(cmd);
        unsafe {
            let _ = PostThreadMessageW(w.thread_id, WM_APP, WPARAM(0), LPARAM(0));
        }
    }

    struct Host {
        hwnd: HWND,
        handler: IPreviewHandler,
    }

    unsafe extern "system" fn wndproc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        DefWindowProcW(h, m, wp, lp)
    }

    unsafe fn run(rx: mpsc::Receiver<Cmd>, tid_tx: mpsc::Sender<u32>) {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let hinstance: HINSTANCE = GetModuleHandleW(None).map(Into::into).unwrap_or_default();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            lpszClassName: CLASS,
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as usize as _),
            ..Default::default()
        };
        RegisterClassW(&wc);

        // Make sure this thread has a message queue before anyone posts to it.
        let mut msg = MSG::default();
        let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
        // Fallback wake-up: COM's modal loop can swallow thread messages while
        // a handler is busy, so also drain the queue on a slow timer.
        SetTimer(None, 0, 150, None);
        let _ = tid_tx.send(GetCurrentThreadId());

        let mut hosts: HashMap<String, Host> = HashMap::new();
        loop {
            let r = GetMessageW(&mut msg, None, 0, 0);
            if r.0 <= 0 {
                break;
            }
            if msg.hwnd.0.is_null() && (msg.message == WM_APP || msg.message == WM_TIMER) {
                while let Ok(cmd) = rx.try_recv() {
                    handle(cmd, &mut hosts, hinstance);
                }
                continue;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    unsafe fn handle(cmd: Cmd, hosts: &mut HashMap<String, Host>, hinstance: HINSTANCE) {
        match cmd {
            Cmd::Prepare { id, path, owner, rect, reply } => {
                if hosts.contains_key(&id) {
                    let _ = reply.send(Ok(()));
                    return;
                }
                let res = create_host(&path, owner, rect, hinstance).map(|host| {
                    hosts.insert(id, host);
                });
                let _ = reply.send(res);
            }
            Cmd::Show { id, rect } => {
                for (other, h) in hosts.iter() {
                    if *other != id {
                        let _ = ShowWindow(h.hwnd, SW_HIDE);
                    }
                }
                if let Some(h) = hosts.get(&id) {
                    let _ = SetWindowPos(
                        h.hwnd,
                        Some(HWND_TOP),
                        rect.x,
                        rect.y,
                        rect.w,
                        rect.h,
                        SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    );
                    let r = RECT { left: 0, top: 0, right: rect.w, bottom: rect.h };
                    let _ = h.handler.SetRect(&r);
                }
            }
            Cmd::Hide { id } => {
                if let Some(h) = hosts.get(&id) {
                    let _ = ShowWindow(h.hwnd, SW_HIDE);
                }
            }
            Cmd::Release { id } => {
                if let Some(h) = hosts.remove(&id) {
                    destroy(h);
                }
            }
            Cmd::ReleaseAll => {
                for (_, h) in hosts.drain() {
                    destroy(h);
                }
            }
            Cmd::Snapshot { id, reply } => {
                let img = hosts.get(&id).and_then(|h| snapshot(h.hwnd));
                let _ = reply.send(img);
            }
        }
    }

    unsafe fn destroy(h: Host) {
        let _ = h.handler.Unload();
        let _ = DestroyWindow(h.hwnd);
    }

    unsafe fn handler_clsid(path: &str) -> Result<windows::core::GUID, String> {
        let ext = std::path::Path::new(path)
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();
        let missing = format!("No Windows preview handler is installed for .{ext} files");
        let ext = HSTRING::from(format!(".{ext}"));
        let mut buf = [0u16; 64];
        let mut len = buf.len() as u32;
        AssocQueryStringW(
            ASSOCF_INIT_DEFAULTTOSTAR,
            ASSOCSTR_SHELLEXTENSION,
            &ext,
            PREVIEW_HANDLER_IID,
            Some(PWSTR(buf.as_mut_ptr())),
            &mut len,
        )
        .ok()
        .map_err(|_| missing.clone())?;
        CLSIDFromString(PCWSTR(buf.as_ptr())).map_err(|_| missing)
    }

    unsafe fn init_handler(handler: &IPreviewHandler, path: &str) -> bool {
        let hpath = HSTRING::from(path);
        if let Ok(i) = handler.cast::<IInitializeWithFile>() {
            if i.Initialize(&hpath, STGM_READ.0).is_ok() {
                return true;
            }
        }
        if let Ok(i) = handler.cast::<IInitializeWithItem>() {
            if let Ok(item) = SHCreateItemFromParsingName::<_, _, IShellItem>(&hpath, None) {
                if i.Initialize(&item, STGM_READ.0).is_ok() {
                    return true;
                }
            }
        }
        if let Ok(i) = handler.cast::<IInitializeWithStream>() {
            if let Ok(stream) =
                SHCreateStreamOnFileEx(&hpath, STGM_READ.0 | STGM_SHARE_DENY_NONE.0, 0, false, None)
            {
                if i.Initialize(&stream, STGM_READ.0).is_ok() {
                    return true;
                }
            }
        }
        false
    }

    unsafe fn create_host(path: &str, owner: isize, rect: PRect, hinstance: HINSTANCE) -> Result<Host, String> {
        let clsid = handler_clsid(path)?;
        let handler: IPreviewHandler = CoCreateInstance(&clsid, None, CLSCTX_LOCAL_SERVER)
            .or_else(|_| CoCreateInstance(&clsid, None, CLSCTX_INPROC_SERVER))
            .map_err(|e| format!("The preview handler {clsid:?} could not be started ({e})"))?;
        if !init_handler(&handler, path) {
            return Err("The preview handler could not open this file".into());
        }
        let (w, h) = (rect.w.max(64), rect.h.max(64));
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            CLASS,
            w!(""),
            WS_POPUP | WS_DISABLED | WS_CLIPCHILDREN,
            rect.x,
            rect.y,
            w,
            h,
            Some(HWND(owner as _)),
            None,
            Some(hinstance),
            None,
        )
        .map_err(|e| format!("Could not create the preview window ({e})"))?;
        let r = RECT { left: 0, top: 0, right: w, bottom: h };
        if let Err(e) = handler.SetWindow(hwnd, &r).and_then(|_| handler.DoPreview()) {
            let _ = handler.Unload();
            let _ = DestroyWindow(hwnd);
            return Err(format!("The preview handler failed to render this file ({e})"));
        }
        Ok(Host { hwnd, handler })
    }

    /// Captures the on-screen pixels of a visible host window as a BMP data URL.
    unsafe fn snapshot(hwnd: HWND) -> Option<String> {
        let mut r = RECT::default();
        GetWindowRect(hwnd, &mut r).ok()?;
        let (w, h) = (r.right - r.left, r.bottom - r.top);
        if w <= 0 || h <= 0 {
            return None;
        }
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bmp.into());
        let blit = BitBlt(mem, 0, 0, w, h, Some(screen), r.left, r.top, SRCCOPY);
        SelectObject(mem, old);

        let mut info = BITMAPINFO::default();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let lines = GetDIBits(
            mem,
            bmp,
            0,
            h as u32,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut info,
            DIB_RGB_COLORS,
        );
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        if blit.is_err() || lines == 0 {
            return None;
        }

        // BMP file: 14-byte file header + 40-byte info header + BGRA pixels.
        let header_len = 14 + 40;
        let mut out = Vec::with_capacity(header_len + pixels.len());
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&((header_len + pixels.len()) as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(header_len as u32).to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&w.to_le_bytes());
        out.extend_from_slice(&(-h).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
        out.extend_from_slice(&(pixels.len() as u32).to_le_bytes());
        out.extend_from_slice(&[0u8; 16]);
        // Force alpha opaque; some handlers leave it zero.
        for px in pixels.chunks_exact_mut(4) {
            px[3] = 255;
        }
        out.extend_from_slice(&pixels);
        Some(format!(
            "data:image/bmp;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(out)
        ))
    }

    fn physical(window: &tauri::WebviewWindow, rect: CssRect, absolute: bool) -> PRect {
        let scale = window.scale_factor().unwrap_or(1.0);
        let (ox, oy) = if absolute {
            window.inner_position().map(|p| (p.x, p.y)).unwrap_or((0, 0))
        } else {
            (-32000, -32000)
        };
        PRect {
            x: ox + (rect.x * scale).round() as i32,
            y: oy + (rect.y * scale).round() as i32,
            w: (rect.w * scale).round() as i32,
            h: (rect.h * scale).round() as i32,
        }
    }

    pub async fn prepare(window: tauri::WebviewWindow, id: String, path: String, rect: CssRect) -> Result<(), String> {
        let owner = window.hwnd().map_err(|e| e.to_string())?;
        let owner = owner.0 as isize;
        let rect = physical(&window, rect, false);
        let (tx, rx) = mpsc::channel();
        send(Cmd::Prepare { id, path, owner, rect, reply: tx });
        tauri::async_runtime::spawn_blocking(move || rx.recv().unwrap_or(Err("Preview thread stopped".into())))
            .await
            .unwrap_or_else(|e| Err(e.to_string()))
    }

    pub fn show(window: tauri::WebviewWindow, id: String, rect: CssRect) {
        let rect = physical(&window, rect, true);
        send(Cmd::Show { id, rect });
    }

    pub fn hide(id: String) {
        send(Cmd::Hide { id });
    }

    pub fn release(id: String) {
        send(Cmd::Release { id });
    }

    pub fn release_all() {
        if WORKER.get().is_some() {
            send(Cmd::ReleaseAll);
        }
    }

    pub async fn snapshot_cmd(id: String) -> Option<String> {
        let (tx, rx) = mpsc::channel();
        send(Cmd::Snapshot { id, reply: tx });
        tauri::async_runtime::spawn_blocking(move || rx.recv().ok().flatten())
            .await
            .ok()
            .flatten()
    }
}

#[tauri::command]
pub async fn preview_prepare(window: tauri::WebviewWindow, id: String, path: String, rect: CssRect) -> Result<(), String> {
    #[cfg(windows)]
    return imp::prepare(window, id, path, rect).await;
    #[cfg(not(windows))]
    {
        let _ = (window, id, path, rect);
        Err("Native previews are only available on Windows".into())
    }
}

#[tauri::command]
pub async fn preview_show(window: tauri::WebviewWindow, id: String, rect: CssRect) {
    #[cfg(windows)]
    imp::show(window, id, rect);
    #[cfg(not(windows))]
    let _ = (window, id, rect);
}

#[tauri::command]
pub async fn preview_hide(id: String) {
    #[cfg(windows)]
    imp::hide(id);
    #[cfg(not(windows))]
    let _ = id;
}

#[tauri::command]
pub async fn preview_release(id: String) {
    #[cfg(windows)]
    imp::release(id);
    #[cfg(not(windows))]
    let _ = id;
}

#[tauri::command]
pub async fn preview_release_all() {
    #[cfg(windows)]
    imp::release_all();
}

#[tauri::command]
pub async fn preview_snapshot(id: String) -> Option<String> {
    #[cfg(windows)]
    return imp::snapshot_cmd(id).await;
    #[cfg(not(windows))]
    {
        let _ = id;
        None
    }
}

pub fn shutdown() {
    #[cfg(windows)]
    imp::release_all();
}
