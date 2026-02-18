# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
# Development (hot reload frontend, debug backend)
cargo tauri dev

# Production build (creates .app + .dmg on macOS)
cargo tauri build

# Debug build (faster, no bundling optimization)
cargo tauri build --debug

# Build just the Rust backend (no frontend bundling)
cd src-tauri && cargo build
```

**Prerequisites:** Rust toolchain, CMake, C/C++ compiler (for whisper.cpp compilation). First build takes ~5min due to whisper.cpp.

**macOS note:** `CMAKE_OSX_DEPLOYMENT_TARGET=11.0` is required for whisper.cpp's `std::filesystem` usage. This is set automatically via `src-tauri/.cargo/config.toml`.

## Architecture

Tauri v2 app with Rust backend and vanilla JS frontend (no bundler, `withGlobalTauri: true`).

### Data Flow

```
Alt+Space → do_toggle_recording() → AudioRecorder.start()
  → cpal captures on dedicated thread → Arc<Mutex<Vec<f32>>>
  → emits "waveform-update" events to frontend

Alt+Space again → AudioRecorder.stop() → thread joins
  → resample to 16kHz → WhisperEngine.transcribe()
  → paste::paste_text() (clipboard + Cmd/Ctrl+V simulation)
```

### Threading Model

- **Main thread**: Tauri windows, tray, commands
- **Shortcut handler**: spawns a new thread per Alt+Space to avoid blocking
- **Recording thread**: dedicated thread owns `cpal::Stream` (not Send — cannot cross threads)
- **Model download**: async via Tokio

### Shared State

`AppState` holds three `Mutex`-wrapped fields (`recorder`, `whisper`, `config`) managed by Tauri. All access requires `.lock().unwrap()`.

### Backend Modules (src-tauri/src/)

| Module | Role |
|--------|------|
| `lib.rs` | Tauri setup, commands, tray, shortcut handler |
| `audio.rs` | cpal capture on dedicated thread, resample, WAV export |
| `stt.rs` | whisper-rs context management and transcription |
| `paste.rs` | arboard clipboard + enigo keyboard simulation |
| `config.rs` | JSON config I/O, directory paths |
| `model_manager.rs` | HuggingFace model download with streaming progress |

### Frontend (src/)

Two Tauri windows, plain HTML/JS/CSS:
- **recorder** (index.html): 280×80 frameless transparent overlay with waveform canvas
- **settings** (settings.html): device/model/language config + model download UI

Communication: `invoke()` for commands, `event.listen()` for backend→frontend events.

### IPC Events

| Event | Direction | Payload |
|-------|-----------|---------|
| `waveform-update` | backend→frontend | `f32` RMS value |
| `recording-started` | backend→frontend | — |
| `recording-stopped` | backend→frontend | — |
| `download-progress` | backend→frontend | `{percent, downloaded_mb, total_mb}` |
| `download-complete` | backend→frontend | — |

## Key Constraints

- **cpal::Stream is not Send**: audio capture must stay on its spawning thread. `AudioRecorder` uses `unsafe impl Send/Sync` because only `Arc`-wrapped data crosses thread boundaries.
- **Whisper requires 16kHz mono f32**: recording captures at device native rate, then linear-interpolation resamples.
- **Transcription is blocking**: runs synchronously on the shortcut handler thread after recording stops.
- **whisper-rs 0.15 API**: `full_n_segments()` returns `c_int` (not Result); use `state.get_segment(i)` → `Option<WhisperSegment>` → `.to_str()`.
- **Paste timing**: 50ms delay before simulating Ctrl/Cmd+V, 100ms before restoring previous clipboard.

## Storage Paths

```
~/lightwhisper/
├── config.json              # {audio_device, model_size, language}
├── models/ggml-{size}.bin   # Downloaded whisper.cpp models
└── temp/                    # Temporary WAV files (macOS/Linux)
```

Windows uses `%TEMP%\lightwhisper\` for temp files.
