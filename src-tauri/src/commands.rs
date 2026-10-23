use tauri::{AppHandle, Manager};

use crate::audio;
use crate::config;
use crate::model_manager;
use crate::state::{get_model_path_for_config, AppState};
use crate::stt;

#[tauri::command]
pub fn get_config(state: tauri::State<'_, AppState>) -> config::AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_config(
    config: config::AppConfig,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    config::save_config_to_disk(&config)?;

    let old_config = state.config.lock().unwrap().clone();
    *state.config.lock().unwrap() = config.clone();

    let engine_changed = old_config.engine != config.engine;
    let model_changed = old_config.model_size != config.model_size;
    let needs_load = engine_changed
        || (model_changed && config.engine == "whisper")
        || !state.engine.lock().unwrap().is_loaded();

    if needs_load {
        reload_engine(&state)?;
    }

    Ok(())
}

fn reload_engine(state: &AppState) -> Result<(), String> {
    let config = state.config.lock().unwrap().clone();
    let model_path = get_model_path_for_config(&config);
    let mut engine = stt::SttEngine::from_engine_name(&config.engine);
    if model_path.exists() {
        engine.load_model(&model_path)?;
    }
    *state.engine.lock().unwrap() = engine;
    Ok(())
}

#[tauri::command]
pub fn list_audio_devices() -> Vec<String> {
    audio::list_input_devices()
}

#[tauri::command]
pub fn check_model_exists(engine: String, model_size: String) -> bool {
    model_manager::model_exists_for_engine(&engine, &model_size)
}

#[tauri::command]
pub async fn download_model(engine: String, model_size: String, app: AppHandle) -> Result<(), String> {
    match engine.as_str() {
        "parakeet" => {
            model_manager::download_parakeet_model(app.clone()).await?;
        }
        _ => {
            model_manager::download_whisper_model(&model_size, app.clone()).await?;
        }
    }

    let state = app.state::<AppState>();
    let active = state.config.lock().unwrap().clone();
    if active.engine == engine && (engine == "parakeet" || active.model_size == model_size) {
        reload_engine(&state)?;
    }
    Ok(())
}

#[tauri::command]
pub fn change_shortcut(shortcut: String, app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
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
