mod audio;
mod commands;
mod config;
mod model_manager;
mod paste;
mod recording;
mod state;
mod stt;
mod tray;

use std::sync::Mutex;
use tauri::Manager;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    config::ensure_dirs();

    let cfg = config::load_config();

    let mut engine = stt::SttEngine::from_engine_name(&cfg.engine);
    let model_path = state::get_model_path_for_config(&cfg);
    let has_model = if model_path.exists() {
        match engine.load_model(&model_path) {
            Ok(()) => {
                println!("STT engine [{}] loaded: {}", cfg.engine, model_path.display());
                true
            }
            Err(e) => {
                eprintln!("Failed to load STT engine: {}", e);
                false
            }
        }
    } else {
        println!("No model found for engine '{}'. Please download via Settings.", cfg.engine);
        false
    };

    let app_state = AppState {
        recorder: Mutex::new(audio::AudioRecorder::new()),
        engine: Mutex::new(engine),
        config: Mutex::new(cfg),
        previous_app_pid: std::sync::atomic::AtomicI32::new(-1),
    };

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let app = app.clone();
                        std::thread::spawn(move || {
                            recording::do_toggle_recording(&app);
                        });
                    }
                })
                .build(),
        )
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::list_audio_devices,
            commands::check_model_exists,
            commands::download_model,
            commands::change_shortcut,
        ])
        .on_window_event(|window, event| {
            if window.label() == "settings" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(move |app| {
            // Hide from Dock — tray-only app
            app.handle().set_activation_policy(tauri::ActivationPolicy::Accessory)?;

            // Prompt for Accessibility permission if not already granted
            // (required for simulating Cmd+V paste)
            paste::ensure_accessibility_permission();

            tray::setup_tray(app.handle())?;

            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            let shortcut = app.state::<AppState>().config.lock().unwrap().shortcut.clone();
            app.global_shortcut().register(shortcut.as_str())?;

            if !has_model {
                if let Some(window) = app.get_webview_window("settings") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    let _ = window.center();
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Error while running Light Whisper");
}
