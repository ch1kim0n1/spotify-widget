use std::time::Duration;

use tauri::{
    Manager,
    menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::{app::window, commands::AppState, error::AppResult};

pub fn create(app: &tauri::AppHandle) -> AppResult<()> {
    let show = MenuItemBuilder::with_id("show", "Show Widget").build(app)?;
    let hide = MenuItemBuilder::with_id("hide", "Hide Widget").build(app)?;
    let reconnect = MenuItemBuilder::with_id("reconnect", "Reconnect Spotify").build(app)?;
    let initial_settings = {
        let state = app.state::<AppState>();
        state.settings.blocking_read().clone()
    };
    let launch_at_login = CheckMenuItemBuilder::with_id("launch_at_login", "Launch at Login")
        .checked(initial_settings.startup.launch_at_login)
        .build(app)?;
    let always_on_top = CheckMenuItemBuilder::with_id("always_on_top", "Always on Top")
        .checked(initial_settings.window.always_on_top)
        .build(app)?;
    let logs = MenuItemBuilder::with_id("logs", "Open Logs").build(app)?;
    let about = MenuItemBuilder::with_id("about", "About").build(app)?;
    let quit_label = if cfg!(target_os = "macos") {
        "Quit Application"
    } else {
        "Kill Application"
    };
    let quit = MenuItemBuilder::with_id("quit", quit_label).build(app)?;
    let separator_one = PredefinedMenuItem::separator(app)?;
    let separator_two = PredefinedMenuItem::separator(app)?;
    let separator_three = PredefinedMenuItem::separator(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[
            &show,
            &hide,
            &separator_one,
            &reconnect,
            &launch_at_login,
            &always_on_top,
            &separator_two,
            &logs,
            &about,
            &separator_three,
            &quit,
        ])
        .build()?;

    let launch_item = launch_at_login.clone();
    let top_item = always_on_top.clone();
    TrayIconBuilder::with_id("main-tray")
        .icon(
            app.default_window_icon()
                .cloned()
                .ok_or_else(|| crate::error::AppError::Platform("application icon is missing".into()))?,
        )
        .tooltip("Spotify Companion Widget")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) && let Err(error) = window::toggle_or_raise_widget(tray.app_handle())
            {
                tracing::warn!(%error, "tray click could not toggle widget");
            }
        })
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show" => {
                if let Err(error) = window::show_widget(app) {
                    tracing::warn!(%error, "show menu action failed");
                }
            }
            "hide" => {
                if let Err(error) = window::hide_widget(app) {
                    tracing::warn!(%error, "hide menu action failed");
                }
            }
            "reconnect" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app.state::<AppState>();
                    if let Err(error) = state.coordinator.reconnect().await {
                        tracing::warn!(%error, "reconnect menu action failed");
                    }
                });
            }
            "launch_at_login" => {
                let app = app.clone();
                let item = launch_item.clone();
                tauri::async_runtime::spawn(async move {
                    let checked = item.is_checked().unwrap_or(false);
                    let target = !checked;
                    let autostart = app.autolaunch();
                    let result = if target {
                        autostart.enable()
                    } else {
                        autostart.disable()
                    };
                    if let Err(error) = result {
                        tracing::warn!(%error, "launch-at-login update failed");
                        return;
                    }
                    let _ = item.set_checked(target);
                    let state = app.state::<AppState>();
                    let snapshot = {
                        let mut settings = state.settings.write().await;
                        settings.startup.launch_at_login = target;
                        if let Err(error) = state.storage.save_settings(&settings) {
                            tracing::warn!(%error, "launch-at-login setting save failed");
                        }
                        settings.clone()
                    };
                    state.coordinator.settings_changed(&snapshot).await;
                });
            }
            "always_on_top" => {
                let app = app.clone();
                let item = top_item.clone();
                tauri::async_runtime::spawn(async move {
                    let target = !item.is_checked().unwrap_or(true);
                    let state = app.state::<AppState>();
                    let snapshot = {
                        let mut settings = state.settings.write().await;
                        settings.window.always_on_top = target;
                        if let Err(error) = state.storage.save_settings(&settings) {
                            tracing::warn!(%error, "always-on-top setting save failed");
                        }
                        settings.clone()
                    };
                    if let Some(window) = app.get_webview_window("main")
                        && let Err(error) = window::apply_window_policy(&window, target)
                    {
                        tracing::warn!(%error, "always-on-top window update failed");
                    }
                    let _ = item.set_checked(target);
                    state.coordinator.settings_changed(&snapshot).await;
                });
            }
            "logs" => {
                let state = app.state::<AppState>();
                if let Err(error) = open::that_detached(state.storage.log_dir()) {
                    tracing::warn!(%error, "open logs menu action failed");
                }
            }
            "about" => {
                app.dialog()
                    .message(format!(
                        "Spotify Companion Widget {}\n\nA GPL-3.0 companion for an existing Spotify playback session.\nNot affiliated with Spotify AB.",
                        app.package_info().version
                    ))
                    .title("About Spotify Companion Widget")
                    .kind(MessageDialogKind::Info)
                    .show(|_| {});
            }
            "quit" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app.state::<AppState>();
                    if let Err(error) = state.coordinator.checkpoint().await {
                        tracing::warn!(%error, "final checkpoint failed");
                    }
                    if let Some(window) = app.get_webview_window("main") {
                        let mut settings = state.settings.write().await;
                        let _ = window::capture_position(&window, &mut settings);
                        let _ = state.storage.save_settings(&settings);
                    }
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    app.exit(0);
                });
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}
