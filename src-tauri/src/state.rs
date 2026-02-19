use std::path::PathBuf;
use std::sync::atomic::AtomicI32;
use std::sync::Mutex;

use crate::audio::AudioRecorder;
use crate::config::AppConfig;
use crate::model_manager;
use crate::stt::SttEngine;

pub struct AppState {
    pub recorder: Mutex<AudioRecorder>,
    pub engine: Mutex<SttEngine>,
    pub config: Mutex<AppConfig>,
    /// PID of the app that was focused before recording started
    pub previous_app_pid: AtomicI32,
}

pub fn get_model_path_for_config(cfg: &AppConfig) -> PathBuf {
    match cfg.engine.as_str() {
        "parakeet" => model_manager::parakeet_model_dir(),
        _ => model_manager::whisper_model_path(&cfg.model_size),
    }
}
