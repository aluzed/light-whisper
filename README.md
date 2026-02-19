# Light Whisper

A lightweight desktop app for global voice-to-text. Press **Alt+Space** anywhere to start recording, press again to transcribe and paste the result into the active application.

Built with [Tauri v2](https://v2.tauri.app/) (Rust backend, vanilla JS frontend).

![demo](https://github.com/aluzed/light-whisper/raw/main/demo.gif)

## Features

- **Global hotkey** (Alt+Space) works from any application
- **Two STT engines:**
  - [Whisper](https://github.com/openai/whisper) (OpenAI) via whisper.cpp — models from 40 MB to 500 MB
  - [Parakeet TDT v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2) (NVIDIA) via ONNX Runtime — ~670 MB, 25 languages, auto-detection
- **French & English** support (and more with Parakeet)
- **Auto-paste**: transcribed text is automatically pasted via clipboard + keyboard simulation
- **Minimal UI**: frameless overlay during recording, settings accessible from the tray icon
- **Auto-opens settings** on first launch if no model is downloaded

## Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- CMake + C/C++ compiler
- macOS 11+, Windows 10+, or Linux (X11 recommended)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Tauri CLI
cargo install tauri-cli --version "^2"
```

### macOS

```bash
brew install cmake
```

### Linux (Ubuntu 22.04+ / Debian 12+)

```bash
sudo apt-get install -y \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libssl-dev \
  libasound2-dev \
  libxdo-dev \
  cmake \
  build-essential
```

### Linux (Fedora 39+)

```bash
sudo dnf install -y \
  gtk3-devel \
  webkit2gtk4.1-devel \
  libayatana-appindicator-gtk3 \
  librsvg2-devel \
  openssl-devel \
  alsa-lib-devel \
  libxdo-devel \
  cmake \
  gcc-c++
```

## Build & Run

```bash
# Development (hot reload frontend, debug backend)
cargo tauri dev

# Production build (creates .app + .dmg on macOS, .msi on Windows)
cargo tauri build

# Debug build (faster, no bundling optimization)
cargo tauri build --debug

# Rust backend only (no frontend bundling)
cd src-tauri && cargo build
```

First build takes ~5 minutes due to whisper.cpp compilation via CMake.

**macOS note:** `CMAKE_OSX_DEPLOYMENT_TARGET=11.0` is required for whisper.cpp's `std::filesystem` usage. This is set automatically via `src-tauri/.cargo/config.toml`.

## Usage

1. **Launch the app** — if no model is downloaded, the Settings window opens automatically
2. **Choose an STT engine** (Whisper or Parakeet) and download the model
3. **Save settings** and close the window
4. **Record** — press `Alt+Space` to start, `Alt+Space` again to stop
5. **Result** — transcribed text is automatically pasted into the active application
6. Access settings anytime via the **tray icon** (left or right click)

## Available Models

### Whisper (OpenAI)

| Model | Size | Speed | Quality |
|-------|------|-------|---------|
| tiny | ~40 MB | Very fast | Basic |
| base | ~60 MB | Fast | Recommended |
| small | ~200 MB | Medium | Good |
| medium | ~500 MB | Slow | Excellent |

### Parakeet TDT v3 (NVIDIA)

| Model | Size | Speed | Quality |
|-------|------|-------|---------|
| int8 quantized | ~670 MB | Fast | Excellent (WER 7.7% FR) |

Parakeet supports 25 European languages with automatic language detection. Models are downloaded from HuggingFace and stored in `~/lightwhisper/models/`.

## Permissions (macOS)

Light Whisper requests both permissions on first launch.

- **Microphone**: required for audio capture. Go to **System Settings > Privacy & Security > Microphone** and enable Light Whisper. Without this permission, macOS silently feeds empty audio to the app — recording appears to work but the waveform stays flat and no transcription is produced.
- **Accessibility**: required for auto-paste (keyboard simulation). Go to **System Settings > Privacy & Security > Accessibility** and enable Light Whisper. The app will prompt on first launch; if denied, transcribed text cannot be pasted automatically.

## Project Structure

```
light-whisper/
├── src/                        # Frontend (vanilla HTML/JS/CSS)
│   ├── index.html              # Recorder overlay (280x80 frameless window)
│   ├── settings.html           # Settings (engine, model, device, language)
│   └── settings.js / .css
├── src-tauri/                  # Rust backend
│   ├── src/
│   │   ├── lib.rs              # Tauri setup, commands, tray, shortcut handler
│   │   ├── audio.rs            # Audio capture (cpal) on dedicated thread
│   │   ├── stt.rs              # STT engine dispatch (Whisper + Parakeet)
│   │   ├── paste.rs            # Clipboard + keyboard simulation (enigo)
│   │   ├── config.rs           # JSON config I/O, directory paths
│   │   └── model_manager.rs    # Model download with streaming progress
│   └── Cargo.toml
└── README.md
```

## Storage

```
~/lightwhisper/
├── config.json                     # {audio_device, model_size, language, engine}
├── models/
│   ├── ggml-{size}.bin             # Whisper models
│   └── parakeet-tdt/               # Parakeet ONNX models
│       ├── encoder-model.onnx
│       ├── decoder_joint-model.onnx
│       └── vocab.txt
└── temp/                           # Temporary WAV files
```

Windows uses `%TEMP%\lightwhisper\` for temp files.

## License

MIT
