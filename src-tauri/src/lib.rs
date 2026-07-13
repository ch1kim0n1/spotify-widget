mod app;
mod auth;
mod commands;
mod domain;
mod error;
mod platform;
mod playback;
mod sessions;
mod spotify;
mod storage;

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use app::window;
use auth::{AuthManager, NativeSecretStore, SpotifyConfig};
use commands::AppState;
use playback::PlaybackCoordinator;
use spotify::SpotifyClient;
use storage::Storage;
use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tokio::sync::RwLock;

struct LogGuard {
    _guard: Mutex<Option<tracing_appender::non_blocking::WorkerGuard>>,
}

pub fn run() {
    let storage = Storage::new().expect("failed to initialize application storage");
    let log_guard = app::logging::initialize(&storage.log_dir())
        .expect("failed to initialize application logging");
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "application starting");

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Err(error) = window::show_widget(app) {
                tracing::warn!(%error, "second-instance show request failed");
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_dialog::init());

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    builder
        .setup(move |app| {
            let settings = storage.load_settings();
            let settings = Arc::new(RwLock::new(settings));
            let auth = Arc::new(AuthManager::new(
                SpotifyConfig::from_environment(),
                Arc::new(NativeSecretStore),
            )?);
            let spotify = Arc::new(SpotifyClient::new(Arc::clone(&auth))?);
            #[cfg(windows)]
            let local_media: Arc<dyn platform::PlatformMediaAdapter> =
                Arc::new(platform::windows::WindowsMediaAdapter);
            #[cfg(not(windows))]
            let local_media: Arc<dyn platform::PlatformMediaAdapter> =
                Arc::new(platform::NoLocalMediaAdapter);

            let coordinator = Arc::new(PlaybackCoordinator::new(
                app.handle().clone(),
                Arc::clone(&auth),
                spotify,
                local_media,
                storage.clone(),
                Arc::clone(&settings),
            ));
            app.manage(AppState {
                coordinator: Arc::clone(&coordinator),
                auth,
                storage: storage.clone(),
                settings: Arc::clone(&settings),
                last_position_save: Mutex::new(Instant::now() - Duration::from_secs(1)),
            });
            app.manage(LogGuard {
                _guard: Mutex::new(Some(log_guard)),
            });

            #[cfg(target_os = "macos")]
            platform::macos::configure_panel(app)?;

            let main_window = app
                .get_webview_window("main")
                .ok_or("main window was not created")?;
            let initial_settings = settings.blocking_read().clone();
            window::apply_window_policy(&main_window, initial_settings.window.always_on_top)?;
            window::restore_position(&main_window, &initial_settings)?;

            let app_handle = app.handle().clone();
            main_window.on_window_event(move |event| match event {
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    if let Err(error) = window::hide_widget(&app_handle) {
                        tracing::warn!(%error, "close-to-hide failed");
                    }
                }
                WindowEvent::Moved(_) => {
                    let state = app_handle.state::<AppState>();
                    let should_save = state.last_position_save.lock().is_ok_and(|mut last| {
                        if last.elapsed() < Duration::from_millis(500) {
                            false
                        } else {
                            *last = Instant::now();
                            true
                        }
                    });
                    if should_save {
                        let app_handle = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app_handle.state::<AppState>();
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let mut settings = state.settings.write().await;
                                if let Err(error) = window::capture_position(&window, &mut settings)
                                {
                                    tracing::warn!(%error, "window position capture failed");
                                } else if let Err(error) = state.storage.save_settings(&settings) {
                                    tracing::warn!(%error, "window position save failed");
                                }
                            }
                        });
                    }
                }
                WindowEvent::ScaleFactorChanged { .. } => {
                    let state = app_handle.state::<AppState>();
                    let settings = state.settings.blocking_read();
                    if let Some(window_handle) = app_handle.get_webview_window("main") {
                        let _ = window::apply_window_policy(
                            &window_handle,
                            settings.window.always_on_top,
                        );
                        let _ = window::restore_position(&window_handle, &settings);
                    }
                }
                _ => {}
            });

            app::tray::create(app.handle())?;
            tauri::async_runtime::block_on(coordinator.initialize_state());
            coordinator.start();
            if initial_settings.startup.show_widget_at_launch {
                window::show_widget(app.handle())?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_view_state,
            commands::transport_previous,
            commands::transport_toggle_play_pause,
            commands::transport_next,
            commands::show_widget,
            commands::hide_widget,
            commands::begin_spotify_auth,
            commands::reconnect_spotify,
            commands::get_settings,
            commands::update_settings,
            commands::open_logs,
            commands::reset_local_data,
            commands::quit_application,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Spotify Companion Widget");
}
