use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::audio;
use crate::paste;
use crate::state::AppState;

fn emit_error(app: &AppHandle, msg: &str) {
    eprintln!("ERROR: {}", msg);
    let _ = app.emit("app-error", msg.to_string());
    // Show settings window so the user actually sees the toast
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn register_escape(app: &AppHandle) {
    let _ = app.global_shortcut().register("Escape");
}

fn unregister_escape(app: &AppHandle) {
    let _ = app.global_shortcut().unregister("Escape");
}

/// Stop recording and hide overlay without transcribing (ESC cancel).
pub fn cancel_recording(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.recorder.lock().unwrap().is_recording() {
        return;
    }

    let _ = state.recorder.lock().unwrap().stop();
    let _ = app.emit("recording-stopped", ());
    unregister_escape(app);

    if let Some(window) = app.get_webview_window("recorder") {
        let _ = window.hide();
    }
}

pub fn do_toggle_recording(app: &AppHandle) {
    let state = app.state::<AppState>();
    let is_recording = state.recorder.lock().unwrap().is_recording();

    if is_recording {
        // Stop recording
        let result = state.recorder.lock().unwrap().stop();
        let _ = app.emit("recording-stopped", ());
        unregister_escape(app);

        // Hide overlay
        if let Some(window) = app.get_webview_window("recorder") {
            let _ = window.hide();
        }

        match result {
            Ok((samples, sample_rate)) => {
                // Detect silent audio
                let rms = if samples.is_empty() {
                    0.0
                } else {
                    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
                };
                if rms < 1e-6 {
                    let device = state.config.lock().unwrap().audio_device.clone();
                    emit_error(app, &format!(
                        "No audio detected (device: \"{}\"). Check that the device is connected, or grant microphone access in System Settings > Privacy & Security > Microphone",
                        device
                    ));
                    return;
                }

                let samples_16k = audio::resample(&samples, sample_rate, 16000);

                let language = state.config.lock().unwrap().language.clone();

                // Transcribe in a catch_unwind to prevent hard crashes
                let transcription = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut engine = state.engine.lock().unwrap();
                    if !engine.is_loaded() {
                        return Err("STT engine not loaded — download a model in Settings".to_string());
                    }
                    engine.transcribe(&samples_16k, &language)
                }));

                match transcription {
                    Ok(Ok(text)) => {
                        if !text.is_empty() {
                            // Restore focus to the app that was active before recording
                            let pid = state.previous_app_pid.load(Ordering::SeqCst);
                            if pid > 0 {
                                paste::activate_pid(pid);
                            }
                            std::thread::sleep(std::time::Duration::from_millis(200));
                            if let Err(e) = paste::paste_text(&text) {
                                emit_error(app, &format!(
                                    "Paste failed: {}. On macOS, enable Accessibility in System Settings > Privacy & Security > Accessibility",
                                    e
                                ));
                            }
                        }
                    }
                    Ok(Err(e)) => emit_error(app, &format!("Transcription failed: {}", e)),
                    Err(_) => emit_error(app, "Transcription crashed — try a different STT engine or model"),
                }
            }
            Err(e) => emit_error(app, &format!("Recording failed: {}", e)),
        }
    } else {
        // Start recording — capture frontmost app before showing overlay
        let pid = paste::get_frontmost_pid();
        state.previous_app_pid.store(pid, Ordering::SeqCst);

        let device = state.config.lock().unwrap().audio_device.clone();

        // Show overlay
        if let Some(window) = app.get_webview_window("recorder") {
            let _ = window.show();
            let _ = window.set_focus();
            let _ = window.center();
        }

        if let Err(e) = state.recorder.lock().unwrap().start(&device, app.clone()) {
            emit_error(app, &format!("Cannot start recording: {}", e));
        } else {
            register_escape(app);
        }
    }
}
