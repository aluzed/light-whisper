mod audio;
mod config;
mod model_manager;
mod paste;
mod stt;

use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

struct AppState {
    recorder: Mutex<audio::AudioRecorder>,
    engine: Mutex<stt::SttEngine>,
    config: Mutex<config::AppConfig>,
}

// ── Tauri Commands ──

#[tauri::command]
fn get_config(state: tauri::State<'_, AppState>) -> config::AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_config(
    config: config::AppConfig,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    config::save_config_to_disk(&config)?;

    let old_config = state.config.lock().unwrap().clone();
    *state.config.lock().unwrap() = config.clone();

    // Reload engine if engine type or model size changed
    let engine_changed = old_config.engine != config.engine;
    let model_changed = old_config.model_size != config.model_size;

    if engine_changed {
        let mut engine = state.engine.lock().unwrap();
        *engine = stt::SttEngine::from_engine_name(&config.engine);
        // Try to load model for the new engine
        let model_path = get_model_path_for_config(&config);
        if model_path.exists() {
            let _ = engine.load_model(&model_path);
        }
    } else if model_changed && config.engine == "whisper" {
        let model_path = model_manager::whisper_model_path(&config.model_size);
        if model_path.exists() {
            let mut engine = state.engine.lock().unwrap();
            let _ = engine.load_model(&model_path);
        }
    }

    Ok(())
}

#[tauri::command]
fn list_audio_devices() -> Vec<String> {
    audio::list_input_devices()
}

#[tauri::command]
fn check_model_exists(engine: String, model_size: String) -> bool {
    model_manager::model_exists_for_engine(&engine, &model_size)
}

#[tauri::command]
async fn download_model(engine: String, model_size: String, app: AppHandle) -> Result<(), String> {
    match engine.as_str() {
        "parakeet" => {
            model_manager::download_parakeet_model(app).await?;
        }
        _ => {
            model_manager::download_whisper_model(&model_size, app).await?;
        }
    }
    Ok(())
}

#[tauri::command]
fn change_shortcut(shortcut: String, app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    // Unregister all shortcuts then register the new one
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {}", e))?;

    app.global_shortcut()
        .register(shortcut.as_str())
        .map_err(|e| format!("Invalid shortcut '{}': {}", shortcut, e))?;

    // Update config in memory and on disk
    let mut cfg = state.config.lock().unwrap();
    cfg.shortcut = shortcut;
    config::save_config_to_disk(&cfg).map_err(|e| format!("Failed to save config: {}", e))?;

    Ok(())
}

fn get_model_path_for_config(cfg: &config::AppConfig) -> std::path::PathBuf {
    match cfg.engine.as_str() {
        "parakeet" => model_manager::parakeet_model_dir(),
        _ => model_manager::whisper_model_path(&cfg.model_size),
    }
}

fn emit_error(app: &AppHandle, msg: &str) {
    eprintln!("{}", msg);
    let _ = app.emit("app-error", msg.to_string());
}

fn do_toggle_recording(app: &AppHandle) {
    let state = app.state::<AppState>();
    let is_recording = state.recorder.lock().unwrap().is_recording();

    if is_recording {
        // Stop recording
        let result = state.recorder.lock().unwrap().stop();
        let _ = app.emit("recording-stopped", ());

        // Hide overlay
        if let Some(window) = app.get_webview_window("recorder") {
            let _ = window.hide();
        }

        match result {
            Ok((samples, sample_rate)) => {
                let samples_16k = audio::resample(&samples, sample_rate, 16000);

                let language = state.config.lock().unwrap().language.clone();
                let mut engine = state.engine.lock().unwrap();

                if !engine.is_loaded() {
                    emit_error(app, "STT engine not loaded — download a model in Settings");
                    return;
                }

                match engine.transcribe(&samples_16k, &language) {
                    Ok(text) => {
                        if !text.is_empty() {
                            // Small delay to let the previous app regain focus
                            std::thread::sleep(std::time::Duration::from_millis(200));
                            if let Err(e) = paste::paste_text(&text) {
                                emit_error(app, &format!("Paste failed: {}", e));
                            }
                        }
                    }
                    Err(e) => emit_error(app, &format!("Transcription failed: {}", e)),
                }
            }
            Err(e) => emit_error(app, &format!("Recording failed: {}", e)),
        }
    } else {
        // Start recording
        let device = state.config.lock().unwrap().audio_device.clone();

        // Show overlay
        if let Some(window) = app.get_webview_window("recorder") {
            let _ = window.show();
            let _ = window.set_focus();
            let _ = window.center();
        }

        if let Err(e) = state.recorder.lock().unwrap().start(&device, app.clone()) {
            emit_error(app, &format!("Cannot start recording: {}", e));
        }
    }
}

fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings_item, &quit_item])?;

    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png"))?;

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Light Whisper")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => {
                if let Some(window) = app.get_webview_window("settings") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    let _ = window.center();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    config::ensure_dirs();

    let cfg = config::load_config();

    let mut engine = stt::SttEngine::from_engine_name(&cfg.engine);
    let model_path = get_model_path_for_config(&cfg);
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
        println!(
            "No model found for engine '{}'. Please download via Settings.",
            cfg.engine
        );
        false
    };

    let state = AppState {
        recorder: Mutex::new(audio::AudioRecorder::new()),
        engine: Mutex::new(engine),
        config: Mutex::new(cfg),
    };

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let app = app.clone();
                        // Run on a separate thread to avoid blocking the shortcut handler
                        std::thread::spawn(move || {
                            do_toggle_recording(&app);
                        });
                    }
                })
                .build(),
        )
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            list_audio_devices,
            check_model_exists,
            download_model,
            change_shortcut,
        ])
        .on_window_event(|window, event| {
            // Hide settings window on close instead of destroying it
            if window.label() == "settings" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(move |app| {
            setup_tray(app.handle())?;

            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            let shortcut = app.state::<AppState>().config.lock().unwrap().shortcut.clone();
            app.global_shortcut().register(shortcut.as_str())?;

            // Auto-open settings if no model is available
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
