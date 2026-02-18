use crate::config;
use futures_util::StreamExt;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

const WHISPER_BASE_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

const PARAKEET_BASE_URL: &str =
    "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main";

// ── Whisper helpers ──

pub fn whisper_model_filename(model_size: &str) -> String {
    match model_size {
        "tiny" => "ggml-tiny.bin".to_string(),
        "base" => "ggml-base.bin".to_string(),
        "small" => "ggml-small.bin".to_string(),
        "medium" => "ggml-medium.bin".to_string(),
        _ => "ggml-base.bin".to_string(),
    }
}

pub fn whisper_model_path(model_size: &str) -> PathBuf {
    config::models_dir().join(whisper_model_filename(model_size))
}

pub fn whisper_model_exists(model_size: &str) -> bool {
    whisper_model_path(model_size).exists()
}

// ── Parakeet helpers ──

/// Files needed by parakeet-rs (downloaded as int8 variants, saved with expected names)
const PARAKEET_FILES: &[(&str, &str)] = &[
    ("encoder-model.int8.onnx", "encoder-model.onnx"),
    ("decoder_joint-model.int8.onnx", "decoder_joint-model.onnx"),
    ("vocab.txt", "vocab.txt"),
];

pub fn parakeet_model_dir() -> PathBuf {
    config::parakeet_models_dir()
}

pub fn parakeet_model_exists() -> bool {
    let dir = parakeet_model_dir();
    PARAKEET_FILES
        .iter()
        .all(|(_, local_name)| dir.join(local_name).exists())
}

// ── Unified check ──

pub fn model_exists_for_engine(engine: &str, model_size: &str) -> bool {
    match engine {
        "parakeet" => parakeet_model_exists(),
        _ => whisper_model_exists(model_size),
    }
}

// ── Download helper ──

/// Download a single file from `url` to `dest`, emitting progress events.
/// `offset` and `grand_total` allow aggregating progress across multiple files.
/// Returns (bytes_downloaded, content_length from GET response)
async fn download_file(
    url: &str,
    dest: &PathBuf,
    app: &AppHandle,
    offset: u64,
    grand_total: u64,
) -> Result<(u64, u64), String> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let file_size = response.content_length().unwrap_or(0);
    let effective_total = if grand_total > 0 { grand_total } else { file_size };
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(&dest)
        .await
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download stream error: {}", e))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("File write error: {}", e))?;

        downloaded += chunk.len() as u64;

        let total_downloaded = offset + downloaded;
        let percent = if effective_total > 0 {
            (total_downloaded as f64 / effective_total as f64) * 100.0
        } else {
            0.0
        };
        let downloaded_mb = total_downloaded as f64 / 1_048_576.0;
        let total_mb = effective_total as f64 / 1_048_576.0;

        let _ = app.emit(
            "download-progress",
            serde_json::json!({
                "percent": percent,
                "downloaded_mb": downloaded_mb,
                "total_mb": total_mb,
            }),
        );
    }

    file.flush()
        .await
        .map_err(|e| format!("File flush error: {}", e))?;

    Ok((downloaded, file_size))
}

// ── Public download functions ──

pub async fn download_whisper_model(model_size: &str, app: AppHandle) -> Result<PathBuf, String> {
    let filename = whisper_model_filename(model_size);
    let url = format!("{}/{}", WHISPER_BASE_URL, filename);
    let dest = whisper_model_path(model_size);

    std::fs::create_dir_all(config::models_dir())
        .map_err(|e| format!("Failed to create models dir: {}", e))?;

    // grand_total=0 → download_file uses content_length from GET response
    download_file(&url, &dest, &app, 0, 0).await?;

    let _ = app.emit("download-complete", ());
    Ok(dest)
}

pub async fn download_parakeet_model(app: AppHandle) -> Result<PathBuf, String> {
    let dir = parakeet_model_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create parakeet dir: {}", e))?;

    // Download each file sequentially; each shows its own 0-100% progress.
    // The encoder (~652 MB) dominates download time so UX is smooth.
    for (i, (remote_name, local_name)) in PARAKEET_FILES.iter().enumerate() {
        let _ = app.emit(
            "download-file-info",
            serde_json::json!({
                "file_index": i + 1,
                "file_count": PARAKEET_FILES.len(),
                "file_name": local_name,
            }),
        );
        let url = format!("{}/{}", PARAKEET_BASE_URL, remote_name);
        let dest = dir.join(local_name);
        download_file(&url, &dest, &app, 0, 0).await?;
    }

    let _ = app.emit("download-complete", ());
    Ok(dir)
}
