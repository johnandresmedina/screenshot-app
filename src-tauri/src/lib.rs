mod capture;
pub mod store;

use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// Event sent to the window whenever the history changes.
const CHANGED_EVENT: &str = "captures-changed";
/// ⌘⇧2. macOS uses ⌘⇧3, 4 and 5 for its own screenshots.
const HOTKEY: (Modifiers, Code) = (Modifiers::SUPER.union(Modifiers::SHIFT), Code::Digit2);
const HOTKEY_LABEL: &str = "⌘⇧2";

static CAPTURING: AtomicBool = AtomicBool::new(false);

struct AppState {
    dir: PathBuf,
    hotkey_ok: AtomicBool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    dir: String,
    hotkey: String,
    hotkey_registered: bool,
}

fn dir(app: &AppHandle) -> PathBuf {
    app.state::<AppState>().dir.clone()
}

fn show_history(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Takes a region capture. The history window is hidden while the overlay is
/// up so it doesn't end up in the shot. Returns the new capture, or None if
/// the user cancelled or a capture was already running.
async fn run_capture(app: AppHandle) -> Result<Option<store::Capture>, String> {
    if CAPTURING.swap(true, Ordering::SeqCst) {
        return Ok(None);
    }
    let window = app.get_webview_window("main");
    let was_visible = window.as_ref().and_then(|w| w.is_visible().ok()).unwrap_or(false);
    if was_visible {
        if let Some(w) = &window {
            let _ = w.hide();
        }
        // Let the hide animation finish before the overlay grabs the screen.
        tokio_sleep(Duration::from_millis(250)).await;
    }

    let d = dir(&app);
    let result = tauri::async_runtime::spawn_blocking(move || capture::region(&d))
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r);
    CAPTURING.store(false, Ordering::SeqCst);

    if was_visible {
        show_history(&app);
    }
    let name = match result? {
        Some(n) => n,
        None => return Ok(None),
    };
    let captured = store::list(&dir(&app))
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|c| c.id == name);
    let _ = app.emit(CHANGED_EVENT, &captured);
    Ok(captured)
}

async fn tokio_sleep(d: Duration) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(d)).await;
}

fn spawn_capture(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_capture(app.clone()).await {
            eprintln!("capture failed: {e}");
            let _ = app.emit("capture-error", e);
        }
    });
}

// ---- Commands the frontend calls ----

#[tauri::command]
fn app_info(state: tauri::State<AppState>) -> AppInfo {
    AppInfo {
        dir: state.dir.to_string_lossy().into_owned(),
        hotkey: HOTKEY_LABEL.into(),
        hotkey_registered: state.hotkey_ok.load(Ordering::SeqCst),
    }
}

#[tauri::command]
fn list_captures(state: tauri::State<AppState>) -> Result<Vec<store::Capture>, String> {
    store::list(&state.dir).map_err(|e| e.to_string())
}

#[tauri::command]
async fn capture_region(app: AppHandle) -> Result<Option<store::Capture>, String> {
    run_capture(app).await
}

#[tauri::command]
fn copy_capture(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    capture::copy_to_clipboard(&store::resolve(&state.dir, &id)?)
}

#[tauri::command]
fn open_capture(state: tauri::State<AppState>, id: String, reveal: bool) -> Result<(), String> {
    capture::open(&store::resolve(&state.dir, &id)?, reveal)
}

#[tauri::command]
fn delete_capture(app: AppHandle, state: tauri::State<AppState>, id: String) -> Result<(), String> {
    capture::trash(&store::resolve(&state.dir, &id)?)?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(())
}

#[tauri::command]
fn open_folder(state: tauri::State<AppState>) -> Result<(), String> {
    std::fs::create_dir_all(&state.dir).map_err(|e| e.to_string())?;
    capture::open(&state.dir, false)
}

// ---- App setup ----

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let capture = MenuItem::with_id(app, "capture", "Capture Region", true, Some("Cmd+Shift+2"))?;
    let history = MenuItem::with_id(app, "history", "Show History…", true, None::<&str>)?;
    let folder = MenuItem::with_id(app, "folder", "Open Screenshots Folder", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, Some("Cmd+Q"))?;
    let menu = Menu::with_items(
        app,
        &[&capture, &PredefinedMenuItem::separator(app)?, &history, &folder, &PredefinedMenuItem::separator(app)?, &quit],
    )?;

    TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(include_bytes!("../icons/tray-template.png"))?)
        // Template icons are recoloured by macOS for light/dark menu bars.
        .icon_as_template(true)
        .tooltip("Screenshot")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "capture" => spawn_capture(app),
            "history" => show_history(app),
            "folder" => {
                let d = dir(app);
                let _ = std::fs::create_dir_all(&d);
                let _ = capture::open(&d, false);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let hotkey = Shortcut::new(Some(HOTKEY.0), HOTKEY.1);

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if *shortcut == hotkey && event.state() == ShortcutState::Pressed {
                        spawn_capture(app);
                    }
                })
                .build(),
        )
        .manage(AppState { dir: store::default_dir(), hotkey_ok: AtomicBool::new(false) })
        .setup(move |app| {
            // Menu bar app: no Dock icon, no app menu.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let _ = std::fs::create_dir_all(dir(app.handle()));
            build_tray(app.handle())?;
            // Show the history window at launch, so starting the app always shows something.
            show_history(app.handle());

            match app.global_shortcut().register(hotkey) {
                Ok(()) => app.state::<AppState>().hotkey_ok.store(true, Ordering::SeqCst),
                Err(e) => eprintln!("could not register {HOTKEY_LABEL}: {e}"),
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the history window only hides it; the app lives in the menu bar.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            list_captures,
            capture_region,
            copy_capture,
            open_capture,
            delete_capture,
            open_folder,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {
            // Opening the app again (Finder, Spotlight, Raycast) brings the window back.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                show_history(_app);
            }
        });
}
