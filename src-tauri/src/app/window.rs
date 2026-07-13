use tauri::{LogicalPosition, Manager, PhysicalPosition, Position, WebviewWindow};

use crate::{
    domain::AppSettings,
    error::{AppError, AppResult},
};

const MIN_VISIBLE: i32 = 24;

pub fn show_widget(app: &tauri::AppHandle) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        crate::platform::macos::show_panel(app)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let window = main_window(app)?;
        window
            .set_always_on_top(true)
            .map_err(|error| AppError::Window(error.to_string()))?;
        window
            .show()
            .map_err(|error| AppError::Window(error.to_string()))?;
        window
            .set_focus()
            .map_err(|error| AppError::Window(error.to_string()))
    }
}

pub fn hide_widget(app: &tauri::AppHandle) -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        crate::platform::macos::hide_panel(app)
    }
    #[cfg(not(target_os = "macos"))]
    {
        main_window(app)?
            .hide()
            .map_err(|error| AppError::Window(error.to_string()))
    }
}

pub fn toggle_or_raise_widget(app: &tauri::AppHandle) -> AppResult<()> {
    let window = main_window(app)?;
    let visible = window
        .is_visible()
        .map_err(|error| AppError::Window(error.to_string()))?;
    let focused = window
        .is_focused()
        .map_err(|error| AppError::Window(error.to_string()))?;
    if visible && focused {
        hide_widget(app)
    } else {
        show_widget(app)
    }
}

pub fn apply_window_policy(window: &WebviewWindow, always_on_top: bool) -> AppResult<()> {
    window
        .set_always_on_top(always_on_top)
        .map_err(|error| AppError::Window(error.to_string()))?;
    window
        .set_skip_taskbar(true)
        .map_err(|error| AppError::Window(error.to_string()))?;
    Ok(())
}

pub fn restore_position(window: &WebviewWindow, settings: &AppSettings) -> AppResult<()> {
    let (Some(saved_x), Some(saved_y)) = (settings.window.x, settings.window.y) else {
        return Ok(());
    };
    let size = window
        .outer_size()
        .map_err(|error| AppError::Window(error.to_string()))?;
    let monitors = window
        .available_monitors()
        .map_err(|error| AppError::Window(error.to_string()))?;
    if monitors.is_empty() {
        return Ok(());
    }
    let preferred = settings
        .window
        .monitor_id
        .as_deref()
        .and_then(|name| {
            monitors
                .iter()
                .find(|monitor| monitor.name().map(String::as_str) == Some(name))
        })
        .or_else(|| {
            monitors.iter().find(|monitor| {
                let position = monitor.position();
                let monitor_size = monitor.size();
                let saved = LogicalPosition::new(saved_x, saved_y)
                    .to_physical::<i32>(monitor.scale_factor());
                saved.x >= position.x
                    && saved.x < position.x + monitor_size.width as i32
                    && saved.y >= position.y
                    && saved.y < position.y + monitor_size.height as i32
            })
        })
        .unwrap_or(&monitors[0]);
    let monitor_position = preferred.position();
    let monitor_size = preferred.size();
    let saved = LogicalPosition::new(saved_x, saved_y).to_physical::<i32>(preferred.scale_factor());
    let (x, y) = clamp_position(
        (saved.x, saved.y),
        (size.width as i32, size.height as i32),
        (
            monitor_position.x,
            monitor_position.y,
            monitor_size.width as i32,
            monitor_size.height as i32,
        ),
    );
    window
        .set_position(Position::Physical(PhysicalPosition::new(x, y)))
        .map_err(|error| AppError::Window(error.to_string()))
}

pub fn capture_position(window: &WebviewWindow, settings: &mut AppSettings) -> AppResult<()> {
    let position = window
        .outer_position()
        .map_err(|error| AppError::Window(error.to_string()))?;
    let scale_factor = window
        .scale_factor()
        .map_err(|error| AppError::Window(error.to_string()))?;
    let logical_position = position.to_logical::<i32>(scale_factor);
    let monitor_name = window
        .current_monitor()
        .map_err(|error| AppError::Window(error.to_string()))?
        .and_then(|monitor| monitor.name().cloned());
    settings.window.x = Some(logical_position.x);
    settings.window.y = Some(logical_position.y);
    settings.window.monitor_id = monitor_name;
    Ok(())
}

fn main_window(app: &tauri::AppHandle) -> AppResult<WebviewWindow> {
    app.get_webview_window("main")
        .ok_or_else(|| AppError::Window("main window is unavailable".into()))
}

pub fn clamp_position(
    position: (i32, i32),
    window_size: (i32, i32),
    monitor_bounds: (i32, i32, i32, i32),
) -> (i32, i32) {
    let (x, y) = position;
    let (width, height) = window_size;
    let (monitor_x, monitor_y, monitor_width, monitor_height) = monitor_bounds;
    let min_x = monitor_x - width + MIN_VISIBLE;
    let max_x = monitor_x + monitor_width - MIN_VISIBLE;
    let min_y = monitor_y - height + MIN_VISIBLE;
    let max_y = monitor_y + monitor_height - MIN_VISIBLE;
    (x.clamp(min_x, max_x), y.clamp(min_y, max_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_fully_offscreen_position() {
        assert_eq!(
            clamp_position((5000, -1000), (420, 172), (0, 0, 1920, 1080)),
            (1896, -148)
        );
    }

    #[test]
    fn preserves_valid_position() {
        assert_eq!(
            clamp_position((1200, 800), (420, 172), (0, 0, 1920, 1080)),
            (1200, 800)
        );
    }
}
