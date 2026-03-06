use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;


use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, Runtime, State, WebviewBuilder, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, WindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

const MAIN_WINDOW_LABEL: &str = "main";
const SETTINGS_WINDOW_LABEL: &str = "settings";
const SETTINGS_FILE_NAME: &str = "settings.json";
const STARTUP_LOG_FILE_NAME: &str = "startup.log";
const GROK_URL: &str = "https://grok.com?referrer=grok-overlay-tauri";

#[cfg(target_os = "macos")]
const DEFAULT_SHORTCUT: &str = "Alt+Space";
#[cfg(target_os = "windows")]
const DEFAULT_SHORTCUT: &str = "Ctrl+Alt+G";
#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
const DEFAULT_SHORTCUT: &str = "Alt+Space";

const TITLEBAR_WEBVIEW_LABEL: &str = "titlebar";
const CONTENT_WEBVIEW_LABEL: &str = "content";
const TITLEBAR_HEIGHT: f64 = 36.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Settings {
    shortcut: String,
    always_on_top: bool,
    launch_at_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: DEFAULT_SHORTCUT.to_string(),
            always_on_top: true,
            launch_at_login: false,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveSettingsPayload {
    shortcut: String,
    always_on_top: bool,
    launch_at_login: bool,
}

struct AppState {
    settings: Mutex<Settings>,
}

fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .or_else(|_| app.path().app_data_dir())
        .map_err(|e| format!("Unable to resolve app config directory: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| {
        format!(
            "Unable to create app config directory {}: {e}",
            dir.display()
        )
    })?;
    Ok(dir.join(SETTINGS_FILE_NAME))
}

fn load_settings<R: Runtime>(app: &AppHandle<R>) -> Settings {
    let path = match settings_path(app) {
        Ok(path) => path,
        Err(_) => return Settings::default(),
    };
    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str::<Settings>(&content).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

fn persist_settings<R: Runtime>(app: &AppHandle<R>, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app)?;
    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Unable to serialize settings: {e}"))?;
    fs::write(&path, content).map_err(|e| format!("Unable to write {}: {e}", path.display()))
}

fn append_startup_log<R: Runtime>(app: &AppHandle<R>, message: &str) {
    let log_dir = match app
        .path()
        .app_log_dir()
        .or_else(|_| app.path().app_config_dir())
    {
        Ok(path) => path,
        Err(_) => return,
    };
    let _ = fs::create_dir_all(&log_dir);
    let log_path = log_dir.join(STARTUP_LOG_FILE_NAME);
    let mut file = match OpenOptions::new().create(true).append(true).open(log_path) {
        Ok(file) => file,
        Err(_) => return,
    };
    let line = format!("[{:?}] {message}\n", std::time::SystemTime::now());
    let _ = file.write_all(line.as_bytes());
}

fn hide_on_close<R: Runtime>(window: &WebviewWindow<R>) {
    let window_for_handler = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window_for_handler.hide();
        }
    });
}



fn ensure_settings_window<R: Runtime>(app: &AppHandle<R>) -> Result<WebviewWindow<R>, String> {
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        return Ok(window);
    }

    let window = WebviewWindowBuilder::new(
        app,
        SETTINGS_WINDOW_LABEL,
        WebviewUrl::App("settings.html".into()),
    )
    .title("Grok Overlay Settings")
    .inner_size(420.0, 460.0)
    .resizable(false)
    .center()
    .visible(false)
    .build()
    .map_err(|e| format!("Unable to create settings window: {e}"))?;
    hide_on_close(&window);
    Ok(window)
}

fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_window(MAIN_WINDOW_LABEL) else {
        return;
    };
    let visible = window.is_visible().unwrap_or(false);
    if visible {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_window(MAIN_WINDOW_LABEL) else {
        return;
    };
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

fn hide_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_window(MAIN_WINDOW_LABEL) {
        let _ = window.hide();
    }
}

fn open_main_home<R: Runtime>(app: &AppHandle<R>) {
    if let Some(webview) = app.get_webview(CONTENT_WEBVIEW_LABEL) {
        let _ = webview.eval(&format!("window.location.replace('{GROK_URL}')"));
    }
    if let Some(window) = app.get_window(MAIN_WINDOW_LABEL) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn set_main_always_on_top<R: Runtime>(
    app: &AppHandle<R>,
    always_on_top: bool,
) -> Result<(), String> {
    let Some(window) = app.get_window(MAIN_WINDOW_LABEL) else {
        return Ok(());
    };
    window
        .set_always_on_top(always_on_top)
        .map_err(|e| format!("Unable to apply always-on-top: {e}"))
}

fn set_launch_at_login<R: Runtime>(
    app: &AppHandle<R>,
    launch_at_login: bool,
) -> Result<(), String> {
    fn is_benign_disable_error(message: &str) -> bool {
        let lowered = message.to_lowercase();
        lowered.contains("not found")
            || lowered.contains("cannot find")
            || lowered.contains("os error 2")
    }

    if launch_at_login {
        app.autolaunch()
            .enable()
            .map_err(|e| format!("Unable to enable launch at login: {e}"))
    } else {
        if matches!(app.autolaunch().is_enabled(), Ok(false)) {
            return Ok(());
        }
        match app.autolaunch().disable() {
            Ok(_) => Ok(()),
            Err(e) => {
                let message = e.to_string();
                if is_benign_disable_error(&message) {
                    Ok(())
                } else {
                    Err(format!("Unable to disable launch at login: {e}"))
                }
            }
        }
    }
}

fn register_shortcut<R: Runtime>(app: &AppHandle<R>, shortcut: &str) -> Result<(), String> {
    let parsed_shortcut: Shortcut = shortcut
        .parse()
        .map_err(|e| format!("Invalid shortcut `{shortcut}`: {e}"))?;
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Unable to clear current shortcuts: {e}"))?;
    app.global_shortcut()
        .on_shortcut(parsed_shortcut, move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                toggle_main_window(app);
            }
        })
        .map_err(|e| format!("Unable to register shortcut `{shortcut}`: {e}"))
}

fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let toggle_item = MenuItem::with_id(app, "toggle_main", "Show / Hide Grok", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let home_item = MenuItem::with_id(app, "open_home", "Go To grok.com", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let settings_item = MenuItem::with_id(app, "open_settings", "Settings", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let launch_item = MenuItem::with_id(
        app,
        "toggle_launch_at_login",
        "Toggle Launch At Login",
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let quit_item =
        MenuItem::with_id(app, "quit", "Quit", true, None::<&str>).map_err(|e| e.to_string())?;
    let menu = Menu::with_items(
        app,
        &[
            &toggle_item,
            &home_item,
            &settings_item,
            &launch_item,
            &quit_item,
        ],
    )
    .map_err(|e| format!("Unable to create tray menu: {e}"))?;

    let mut tray_builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("Grok Overlay")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle_main" => toggle_main_window(app),
            "open_home" => open_main_home(app),
            "open_settings" => {
                if let Ok(window) = ensure_settings_window(app) {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "toggle_launch_at_login" => {
                let current = app.autolaunch().is_enabled().unwrap_or(false);
                let target = !current;
                if set_launch_at_login(app, target).is_ok() {
                    if let Ok(mut settings) = app.state::<AppState>().settings.lock() {
                        settings.launch_at_login = target;
                        let _ = persist_settings(app, &settings);
                    }
                }
            }
            "quit" => app.exit(0),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    tray_builder
        .build(app)
        .map_err(|e| format!("Unable to build tray icon: {e}"))?;
    Ok(())
}

#[tauri::command]
fn get_settings(app: AppHandle, state: State<AppState>) -> Result<Settings, String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock settings state".to_string())?
        .clone();
    if let Ok(enabled) = app.autolaunch().is_enabled() {
        settings.launch_at_login = enabled;
    }
    Ok(settings)
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<AppState>,
    payload: SaveSettingsPayload,
) -> Result<Settings, String> {
    let shortcut = payload.shortcut.trim();
    if shortcut.is_empty() {
        return Err("Shortcut cannot be empty".to_string());
    }

    register_shortcut(&app, shortcut)?;
    set_main_always_on_top(&app, payload.always_on_top)?;
    set_launch_at_login(&app, payload.launch_at_login)?;

    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Unable to lock settings state".to_string())?;
    settings.shortcut = shortcut.to_string();
    settings.always_on_top = payload.always_on_top;
    settings.launch_at_login = payload.launch_at_login;
    persist_settings(&app, &settings)?;
    Ok(settings.clone())
}

#[tauri::command]
fn toggle_main_window_cmd(app: AppHandle) {
    toggle_main_window(&app);
}

#[tauri::command]
fn show_main_window_cmd(app: AppHandle) {
    show_main_window(&app);
}

#[tauri::command]
fn hide_main_window_cmd(app: AppHandle) {
    hide_main_window(&app);
}

#[tauri::command]
fn open_settings_window_cmd(app: AppHandle) -> Result<(), String> {
    let settings_window = ensure_settings_window(&app)?;
    settings_window
        .show()
        .map_err(|e| format!("Unable to show settings window: {e}"))?;
    settings_window
        .set_focus()
        .map_err(|e| format!("Unable to focus settings window: {e}"))
}

#[tauri::command]
fn open_main_home_cmd(app: AppHandle) {
    open_main_home(&app);
}

#[tauri::command]
fn start_dragging_cmd(app: AppHandle) {
    if let Some(window) = app.get_window(MAIN_WINDOW_LABEL) {
        let _ = window.start_dragging();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let mut settings = load_settings(&app_handle);

            #[cfg(target_os = "macos")]
            if let Err(error) = app_handle.set_activation_policy(ActivationPolicy::Accessory) {
                append_startup_log(&app_handle, &format!("Set macOS activation policy failed: {error}"));
            }

            let main_window = WindowBuilder::new(&app_handle, MAIN_WINDOW_LABEL)
                .title("")
                .inner_size(550.0, 620.0)
                .min_inner_size(420.0, 460.0)
                .decorations(false)
                .resizable(true)
                .always_on_top(settings.always_on_top)
                .center()
                .build()
                .map_err(|e| format!("Unable to create main window: {e}"))?;

            main_window
                .add_child(
                    WebviewBuilder::new(
                        CONTENT_WEBVIEW_LABEL,
                        WebviewUrl::External(GROK_URL.parse().unwrap()),
                    ),
                    LogicalPosition::new(0.0, TITLEBAR_HEIGHT),
                    LogicalSize::new(550.0, 620.0 - TITLEBAR_HEIGHT),
                )
                .map_err(|e| format!("Unable to create content webview: {e}"))?;

            main_window
                .add_child(
                    WebviewBuilder::new(
                        TITLEBAR_WEBVIEW_LABEL,
                        WebviewUrl::App("titlebar.html".into()),
                    ),
                    LogicalPosition::new(0.0, 0.0),
                    LogicalSize::new(550.0, TITLEBAR_HEIGHT),
                )
                .map_err(|e| format!("Unable to create titlebar webview: {e}"))?;

            let window_for_close = main_window.clone();
            let window_for_scale = main_window.clone();
            let app_for_resize = app_handle.clone();
            main_window.on_window_event(move |event| match event {
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window_for_close.hide();
                }
                WindowEvent::Resized(size) => {
                    let scale = window_for_scale.scale_factor().unwrap_or(1.0);
                    let lw = size.width as f64 / scale;
                    let lh = size.height as f64 / scale;
                    if let Some(tb) = app_for_resize.get_webview(TITLEBAR_WEBVIEW_LABEL) {
                        let _ = tb.set_size(LogicalSize::new(lw, TITLEBAR_HEIGHT));
                    }
                    if let Some(ct) = app_for_resize.get_webview(CONTENT_WEBVIEW_LABEL) {
                        let _ = ct.set_size(LogicalSize::new(lw, lh - TITLEBAR_HEIGHT));
                    }
                }
                _ => {}
            });

            if register_shortcut(&app_handle, &settings.shortcut).is_err() {
                append_startup_log(
                    &app_handle,
                    &format!(
                        "Failed to register saved shortcut `{}`. Falling back to default `{}`.",
                        settings.shortcut, DEFAULT_SHORTCUT
                    ),
                );
                settings.shortcut = DEFAULT_SHORTCUT.to_string();
                if register_shortcut(&app_handle, &settings.shortcut).is_err() {
                    append_startup_log(
                        &app_handle,
                        &format!(
                            "Failed to register fallback shortcut `{}`. App will continue without global shortcut.",
                            settings.shortcut
                        ),
                    );
                }
            }

            if let Err(error) = set_launch_at_login(&app_handle, settings.launch_at_login) {
                append_startup_log(&app_handle, &format!("Autostart apply failed: {error}"));
            }
            if let Ok(enabled) = app_handle.autolaunch().is_enabled() {
                settings.launch_at_login = enabled;
            }

            app.manage(AppState {
                settings: Mutex::new(settings.clone()),
            });
            if let Err(error) = persist_settings(&app_handle, &settings) {
                append_startup_log(&app_handle, &format!("Persist settings failed: {error}"));
            }

            if let Some(settings_window) = app_handle.get_webview_window(SETTINGS_WINDOW_LABEL) {
                hide_on_close(&settings_window);
            }

            if let Err(error) = setup_tray(&app_handle) {
                append_startup_log(&app_handle, &format!("Tray setup failed: {error}"));
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            toggle_main_window_cmd,
            show_main_window_cmd,
            hide_main_window_cmd,
            open_settings_window_cmd,
            open_main_home_cmd,
            start_dragging_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
