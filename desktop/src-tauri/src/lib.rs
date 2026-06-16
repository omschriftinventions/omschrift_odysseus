mod paths;
mod runtime;
mod server;

use std::sync::Arc;
use std::thread;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent, Url, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_single_instance::init as single_instance;

use paths::{resolve_data_root, resolve_repo_root};
use runtime::{has_native_server_bundle, resolve_runtime_root};
use server::{ServerManager, StatusPayload};

struct AppState {
    server: Arc<ServerManager>,
}

fn emit_status(app: &AppHandle, payload: StatusPayload) {
    let _ = app.emit("server-status", payload);
}

fn focus_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }
    if let Some(window) = app.get_webview_window("splash") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn open_main_ui(app: &AppHandle, url: &str) -> Result<(), String> {
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.close();
    }

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let parsed = Url::parse(url).map_err(|e| format!("Invalid app URL: {e}"))?;
    let window = WebviewWindowBuilder::new(
        app,
        "main",
        WebviewUrl::External(parsed),
    )
    .title("Omschrift Odysseus")
    .inner_size(1280.0, 800.0)
    .resizable(true)
    .center()
    .build()
    .map_err(|e| format!("Failed to open Omschrift Odysseus UI: {e}"))?;
    let _ = window.set_focus();
    Ok(())
}

fn focus_splash(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("splash") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn start_server_task(app: AppHandle, server: Arc<ServerManager>, startup_error: Option<String>) {
    std::thread::spawn(move || {
        if let Some(err) = startup_error {
            emit_status(
                &app,
                StatusPayload {
                    message: err,
                    error: true,
                    ..Default::default()
                },
            );
            focus_splash(&app);
            return;
        }

        let app_handle = app.clone();
        let result = server.ensure_running(&app_handle, |payload| {
            emit_status(&app_handle, payload);
        });

        match result {
            Ok(creds) => {
                if let Some(creds) = creds {
                    emit_status(
                        &app,
                        StatusPayload {
                            message: "First-time setup complete. Save these login details:"
                                .into(),
                            admin_user: Some(creds.username.clone()),
                            admin_password: Some(creds.password),
                            ..Default::default()
                        },
                    );
                    thread::sleep(std::time::Duration::from_secs(12));
                }

                emit_status(
                    &app,
                    StatusPayload {
                        message: "Opening Omschrift Odysseus…".into(),
                        ..Default::default()
                    },
                );
                if let Err(err) = open_main_ui(&app, &server.app_url()) {
                    emit_status(
                        &app,
                        StatusPayload {
                            message: err,
                            error: true,
                            ..Default::default()
                        },
                    );
                }
            }
            Err(err) => {
                emit_status(
                    &app,
                    StatusPayload {
                        message: err,
                        error: true,
                        ..Default::default()
                    },
                );
                focus_splash(&app);
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(single_instance(|app, _args, _cwd| {
            focus_main_window(app);
        }))
        .setup(|app| {
            let native_expected = has_native_server_bundle(app.handle());
            let startup_error = if native_expected {
                None
            } else {
                match resolve_runtime_root(app.handle()) {
                    Ok(_) => None,
                    Err(err) => Some(format!(
                        "{err}\n\nInstall Python 3.11+ or use the native Omschrift Odysseus installer that bundles Python."
                    )),
                }
            };

            let venv_root = if native_expected {
                None
            } else {
                resolve_runtime_root(app.handle())
                    .ok()
                    .or_else(|| resolve_repo_root(Some(app.handle())))
            };

            let data_root = resolve_data_root(Some(app.handle()), native_expected);
            let server = Arc::new(ServerManager::new(
                data_root,
                None,
                venv_root,
                native_expected,
            ));
            app.manage(AppState {
                server: server.clone(),
            });

            let show_item =
                MenuItem::with_id(app, "show", "Show Omschrift Odysseus", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip("Omschrift Odysseus")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => focus_main_window(app),
                    "quit" => {
                        if let Some(state) = app.try_state::<AppState>() {
                            state.server.shutdown();
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        focus_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            WebviewWindowBuilder::new(app, "splash", WebviewUrl::App("index.html".into()))
                .title("Omschrift Odysseus")
                .inner_size(480.0, 420.0)
                .resizable(false)
                .center()
                .build()?;

            start_server_task(app.handle().clone(), server, startup_error);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" || window.label() == "splash" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building Omschrift Odysseus desktop app")
        .run(|app_handle, event| {
            if matches!(event, RunEvent::ExitRequested { .. } | RunEvent::Exit) {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    state.server.shutdown();
                }
            }
        });
}
