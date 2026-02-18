use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub audio_device: String,
    pub model_size: String,
    pub language: String,
    pub engine: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            audio_device: "default".to_string(),
            model_size: "base".to_string(),
            language: "auto".to_string(),
            engine: "whisper".to_string(),
        }
    }
}

pub fn config_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Cannot find home directory");
    home.join("lightwhisper")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn models_dir() -> PathBuf {
    config_dir().join("models")
}

pub fn parakeet_models_dir() -> PathBuf {
    models_dir().join("parakeet-tdt")
}

pub fn temp_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        std::env::temp_dir().join("lightwhisper")
    } else {
        config_dir().join("temp")
    }
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        }
    } else {
        AppConfig::default()
    }
}

pub fn save_config_to_disk(config: &AppConfig) -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;

    let json =
        serde_json::to_string_pretty(config).map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(config_path(), json).map_err(|e| format!("Failed to write config: {}", e))?;
    Ok(())
}

pub fn ensure_dirs() {
    let _ = fs::create_dir_all(config_dir());
    let _ = fs::create_dir_all(models_dir());
    let _ = fs::create_dir_all(parakeet_models_dir());
    let _ = fs::create_dir_all(temp_dir());
}
