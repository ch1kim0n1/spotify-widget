use tauri::{App, AppHandle, Manager};
use tauri_nspanel::{ManagerExt, WebviewWindowExt, cocoa::appkit::NSWindowCollectionBehavior};

use crate::error::{AppError, AppResult};

pub fn configure_panel(app: &mut App) -> AppResult<()> {
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::Window("main window is unavailable".into()))?;
    let panel = window
        .to_panel()
        .map_err(|error| AppError::Window(error.to_string()))?;

    const NS_FLOATING_WINDOW_LEVEL: i32 = 4;
    const NS_WINDOW_STYLE_MASK_NON_ACTIVATING_PANEL: i32 = 1 << 7;
    panel.set_level(NS_FLOATING_WINDOW_LEVEL);
    panel.set_style_mask(NS_WINDOW_STYLE_MASK_NON_ACTIVATING_PANEL);
    panel.set_collection_behaviour(
        NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces,
    );
    panel.set_floating_panel(true);
    panel.set_hides_on_deactivate(false);
    panel.set_becomes_key_only_if_needed(true);
    panel.set_moveable_by_window_background(false);
    panel.set_opaque(false);
    panel.set_has_shadow(true);
    panel.set_released_when_closed(false);
    Ok(())
}

pub fn show_panel(app: &AppHandle) -> AppResult<()> {
    let panel = app
        .get_webview_panel("main")
        .map_err(|_| AppError::Window("main panel is unavailable".into()))?;
    panel.show();
    Ok(())
}

pub fn hide_panel(app: &AppHandle) -> AppResult<()> {
    let panel = app
        .get_webview_panel("main")
        .map_err(|_| AppError::Window("main panel is unavailable".into()))?;
    panel.order_out(None);
    Ok(())
}
