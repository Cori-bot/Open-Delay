#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod updates;
use open_delay_core::{Action, Command, Config, Preferences};
use open_delay_service::{default_data_dir, RunningService, Service};
use serde::Serialize;
use std::{path::PathBuf, sync::Mutex};
use tauri::{Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[derive(Clone, Serialize)]
struct ServiceInfo {
    api_base: String,
    error: Option<String>,
    native_errors: Vec<String>,
}
struct DesktopState {
    running: Mutex<Option<RunningService>>,
    info: Mutex<ServiceInfo>,
    data: PathBuf,
    web: PathBuf,
    lifecycle: tokio::sync::Mutex<()>,
}

#[tauri::command]
fn service_info(state: State<'_, DesktopState>) -> Result<ServiceInfo, String> {
    state
        .info
        .lock()
        .map(|s| s.clone())
        .map_err(|_| "Desktop state unavailable".into())
}

#[tauri::command]
fn desktop_status(state: State<'_, DesktopState>) -> Result<serde_json::Value, String> {
    Ok(
        serde_json::json!({"errors":state.info.lock().map_err(|_|"Desktop state unavailable")?.native_errors}),
    )
}

#[tauri::command]
async fn retry_service(
    app: tauri::AppHandle,
    api_port: u16,
    rtmp_port: u16,
) -> Result<ServiceInfo, String> {
    let state = app.state::<DesktopState>();
    let _lifecycle = state.lifecycle.lock().await;
    if app.state::<updates::UpdaterState>().is_installing() {
        return Err("An update installation is in progress".into());
    }
    let previous = {
        let mut guard = state
            .running
            .lock()
            .map_err(|_| "Desktop state unavailable")?;
        if let Some(running) = guard.as_ref() {
            if running
                .service
                .controller
                .lock()
                .map_err(|_| "Controller unavailable")?
                .snapshot()
                .routes
                .iter()
                .any(|route| route.is_connected)
            {
                return Err("Stop the stream before restarting the local engine".into());
            }
        }
        guard.take()
    };
    if let Some(previous) = previous {
        previous.stop().await;
    }
    let mut config = Config::load(state.data.join("config.json")).map_err(|e| e.to_string())?;
    config.preferences.api_port = api_port;
    config.preferences.rtmp_port = rtmp_port;
    let running = open_delay_service::start(
        config,
        state.data.join("config.json"),
        state.data.clone(),
        state.web.clone(),
    )
    .await
    .map_err(|e| e.to_string())?;
    let preferences = running
        .service
        .controller
        .lock()
        .map_err(|_| "Controller unavailable")?
        .config()
        .preferences
        .clone();
    let native_errors = apply_native(&app, &preferences);
    let info = ServiceInfo {
        api_base: format!("http://127.0.0.1:{}", running.api_addr.port()),
        error: None,
        native_errors,
    };
    *state
        .running
        .lock()
        .map_err(|_| "Desktop state unavailable")? = Some(running);
    *state.info.lock().map_err(|_| "Desktop state unavailable")? = info.clone();
    emit_desktop_status(&app, &info.native_errors);
    Ok(info)
}

fn service(app: &tauri::AppHandle) -> Option<Service> {
    app.state::<DesktopState>()
        .running
        .lock()
        .ok()?
        .as_ref()
        .map(|r| r.service.clone())
}

fn apply_native(app: &tauri::AppHandle, prefs: &Preferences) -> Vec<String> {
    let mut errors = Vec::new();
    if let Err(error) = app.global_shortcut().unregister_all() {
        errors.push(format!("Cannot unregister old shortcuts: {error}"));
        return errors;
    }
    let actions = [
        (&prefs.hotkeys.start_delay, "start_delay"),
        (&prefs.hotkeys.go_live, "go_live"),
        (&prefs.hotkeys.toggle_overlay, "toggle_overlay"),
        (&prefs.hotkeys.custom_delay, "custom_delay"),
    ];
    for (key, action) in actions {
        if let Some(key) = key {
            let result = app
                .global_shortcut()
                .on_shortcut(key.as_str(), move |app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        if let Some(service) = service(app) {
                            let result = service.controller.lock().ok().map(|mut c| {
                                let resolved_action = match action {
                                    "start_delay" => Action::PrepareDelay {
                                        delay_ms: c.config().preferences.buffer_ms,
                                    },
                                    "custom_delay" => Action::PrepareDelay {
                                        delay_ms: c.config().preferences.hotkeys.custom_delay_ms,
                                    },
                                    "toggle_overlay" => Action::ToggleOverlay,
                                    _ => Action::GoLive,
                                };
                                c.execute(Command {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    route_ids: None,
                                    action: resolved_action,
                                })
                            });
                            if let Some(result) = result {
                                let _ = app.emit("command-result", &result);
                            }
                            service.publish();
                        }
                    }
                });
            if let Err(error) = result {
                errors.push(format!("Shortcut {key}: {error}"));
            }
        }
    }
    let startup = if prefs.startup {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    };
    if let Err(error) = startup {
        errors.push(format!("Windows startup: {error}"));
    }
    errors
}

fn emit_desktop_status(app: &tauri::AppHandle, errors: &[String]) {
    if let Err(error) = app.emit("desktop-status", serde_json::json!({"errors": errors})) {
        eprintln!("Cannot notify native settings status: {error}");
    }
}

#[tauri::command]
fn apply_desktop_preferences(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let service = service(&app).ok_or("Local engine unavailable")?;
    let prefs = service
        .controller
        .lock()
        .map_err(|_| "Controller unavailable")?
        .config()
        .preferences
        .clone();
    let errors = apply_native(&app, &prefs);
    app.state::<DesktopState>()
        .info
        .lock()
        .map_err(|_| "Desktop state unavailable")?
        .native_errors = errors.clone();
    emit_desktop_status(&app, &errors);
    Ok(serde_json::json!({"errors":errors}))
}

fn request_quit(app: &tauri::AppHandle) {
    let active = service(app)
        .and_then(|s| s.controller.lock().ok().map(|c| c.snapshot().is_connected))
        .unwrap_or(false);
    if active {
        let handle = app.clone();
        app.dialog().message("Un stream est actif. Quitter coupe la sortie. / A stream is active. Quitting stops the output.")
            .title("Open-Delay").buttons(MessageDialogButtons::OkCancel)
            .show(move |confirmed|if confirmed {handle.exit(0);});
    } else {
        app.exit(0);
    }
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    request_quit(&app);
}

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            service_info,
            desktop_status,
            retry_service,
            apply_desktop_preferences,
            quit_app,
            updates::update_status,
            updates::check_update,
            updates::install_update
        ])
        .setup(|app| {
            app.manage(updates::UpdaterState::new(app.handle()));
            let data = default_data_dir();
            let web = if cfg!(debug_assertions) {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ui/dist")
            } else {
                app.path().resource_dir()?.join("web")
            };
            let result = Config::load(data.join("config.json"))
                .map_err(|e| e.to_string())
                .and_then(|config| {
                    tauri::async_runtime::block_on(open_delay_service::start(
                        config,
                        data.join("config.json"),
                        data.clone(),
                        web.clone(),
                    ))
                    .map_err(|e| e.to_string())
                });
            let (running, info) = match result {
                Ok(running) => {
                    let info = ServiceInfo {
                        api_base: format!("http://127.0.0.1:{}", running.api_addr.port()),
                        error: None,
                        native_errors: vec![],
                    };
                    (Some(running), info)
                }
                Err(error) => (
                    None,
                    ServiceInfo {
                        api_base: String::new(),
                        error: Some(error),
                        native_errors: vec![],
                    },
                ),
            };
            app.manage(DesktopState {
                running: Mutex::new(running),
                info: Mutex::new(info),
                data,
                web,
                lifecycle: tokio::sync::Mutex::new(()),
            });
            let show =
                tauri::menu::MenuItem::with_id(app, "show", "Ouvrir / Open", true, None::<&str>)?;
            let live = tauri::menu::MenuItem::with_id(
                app,
                "live",
                "Retour au direct / Go live",
                true,
                None::<&str>,
            )?;
            let quit =
                tauri::menu::MenuItem::with_id(app, "quit", "Quitter / Quit", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&show, &live, &quit])?;
            let mut tray = tauri::tray::TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Open-Delay")
                .show_menu_on_left_click(true);
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.on_menu_event(|app, event| match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.unminimize();
                        let _ = window.set_focus();
                    }
                }
                "live" => {
                    if let Some(s) = service(app) {
                        if let Ok(mut c) = s.controller.lock() {
                            let result = c.execute(Command {
                                id: uuid::Uuid::new_v4().to_string(),
                                route_ids: None,
                                action: Action::GoLive,
                            });
                            let _ = app.emit("command-result", &result);
                        }
                        s.publish();
                    }
                }
                "quit" => request_quit(app),
                _ => {}
            })
            .build(app)?;
            if let Some(s) = service(app.handle()) {
                let prefs = s
                    .controller
                    .lock()
                    .map_err(|_| "Controller unavailable")?
                    .config()
                    .preferences
                    .clone();
                let errors = apply_native(app.handle(), &prefs);
                emit_desktop_status(app.handle(), &errors);
                if let Ok(mut info) = app.state::<DesktopState>().info.lock() {
                    info.native_errors = errors;
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let quit = service(window.app_handle())
                    .and_then(|s| {
                        s.controller
                            .lock()
                            .ok()
                            .map(|c| c.config().preferences.quit_on_close)
                    })
                    .unwrap_or(true);
                if quit {
                    request_quit(window.app_handle());
                } else {
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("Cannot create the Open-Delay desktop window");
    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event {
            let state = handle.state::<DesktopState>();
            let running = state.running.lock().ok().and_then(|mut r| r.take());
            if let Some(running) = running {
                tauri::async_runtime::block_on(running.stop());
            }
        }
    });
}
