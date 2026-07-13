use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tokio::sync::RwLock;

use crate::{
    app::window,
    auth::AuthManager,
    domain::{AppSettings, CommandName, SettingsPatch, ViewState},
    playback::PlaybackCoordinator,
    storage::Storage,
};

pub struct AppState {
    pub coordinator: Arc<PlaybackCoordinator>,
    pub auth: Arc<AuthManager>,
    pub storage: Storage,
    pub settings: Arc<RwLock<AppSettings>>,
    pub last_position_save: Mutex<Instant>,
}

#[tauri::command]
pub async fn get_view_state(state: tauri::State<'_, AppState>) -> Result<ViewState, String> {
    Ok(state.coordinator.view_state().await)
}

#[tauri::command]
pub async fn transport_previous(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .coordinator
        .transport(CommandName::Previous)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn transport_toggle_play_pause(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .coordinator
        .transport(CommandName::TogglePlayPause)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn transport_next(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .coordinator
        .transport(CommandName::Next)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn begin_spotify_auth(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .coordinator
        .begin_authorization()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn reconnect_spotify(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state
        .coordinator
        .reconnect()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn show_widget(app: AppHandle) -> Result<(), String> {
    window::show_widget(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn hide_widget(app: AppHandle) -> Result<(), String> {
    window::hide_widget(&app).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.read().await.clone())
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    patch: SettingsPatch,
) -> Result<(), String> {
    let settings = {
        let mut settings = state.settings.write().await;
        if let Some(value) = patch.always_on_top {
            settings.window.always_on_top = value;
        }
        if let Some(value) = patch.launch_at_login {
            settings.startup.launch_at_login = value;
        }
        if let Some(value) = patch.show_spotify_open_time {
            settings.display.show_spotify_open_time = value;
        }
        if let Some(value) = patch.show_listening_time {
            settings.display.show_listening_time = value;
        }
        state
            .storage
            .save_settings(&settings)
            .map_err(|error| error.to_string())?;
        settings.clone()
    };

    if let Some(window) = app.get_webview_window("main") {
        window
            .set_always_on_top(settings.window.always_on_top)
            .map_err(|error| error.to_string())?;
    }
    let autostart = app.autolaunch();
    let enabled = autostart.is_enabled().map_err(|error| error.to_string())?;
    if settings.startup.launch_at_login && !enabled {
        autostart.enable().map_err(|error| error.to_string())?;
    } else if !settings.startup.launch_at_login && enabled {
        autostart.disable().map_err(|error| error.to_string())?;
    }
    state.coordinator.settings_changed(&settings).await;
    Ok(())
}

#[tauri::command]
pub fn open_logs(state: tauri::State<'_, AppState>) -> Result<(), String> {
    open::that_detached(state.storage.log_dir()).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn reset_local_data(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state
        .auth
        .disconnect()
        .await
        .map_err(|error| error.to_string())?;
    state
        .storage
        .reset_local_files()
        .map_err(|error| error.to_string())?;
    app.restart();
}

#[tauri::command]
pub async fn quit_application(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    if let Err(error) = state.coordinator.checkpoint().await {
        tracing::warn!(%error, "final session checkpoint failed");
    }
    if let Some(window) = app.get_webview_window("main") {
        let mut settings = state.settings.write().await;
        if let Err(error) = window::capture_position(&window, &mut settings) {
            tracing::warn!(%error, "final window position capture failed");
        }
        if let Err(error) = state.storage.save_settings(&settings) {
            tracing::warn!(%error, "final settings save failed");
        }
    }
    app.exit(0);
    Ok(())
}
