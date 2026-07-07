use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Thread-safe audio recorder that keeps the cpal::Stream on a dedicated thread.
pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    recording: Arc<AtomicBool>,
    sample_rate: Arc<Mutex<u32>>,
    /// Handle to the recording thread (join on stop)
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

// Safety: we never move cpal::Stream across threads — it lives entirely on
// the spawned thread. The fields we share are Arc-wrapped.
unsafe impl Send for AudioRecorder {}
unsafe impl Sync for AudioRecorder {}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            recording: Arc::new(AtomicBool::new(false)),
            sample_rate: Arc::new(Mutex::new(0)),
            thread_handle: None,
        }
    }

    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::SeqCst)
    }

    pub fn start(&mut self, device_name: &str, app: AppHandle) -> Result<(), String> {
        if self.is_recording() {
            return Err("Already recording".to_string());
        }

        self.samples.lock().unwrap().clear();
        *self.sample_rate.lock().unwrap() = 0;
        self.recording.store(true, Ordering::SeqCst);

        let samples = Arc::clone(&self.samples);
        let recording = Arc::clone(&self.recording);
        let sample_rate_out = Arc::clone(&self.sample_rate);
        let device_name = device_name.to_string();
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let recording_for_error = Arc::clone(&self.recording);

        let handle = std::thread::spawn(move || {
            let result = run_recording(
                device_name,
                samples,
                recording,
                sample_rate_out,
                app,
                started_tx.clone(),
            );
            if let Err(e) = result {
                eprintln!("Recording error: {}", e);
                recording_for_error.store(false, Ordering::SeqCst);
                let _ = started_tx.send(Err(e));
            }
        });

        match started_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {
                self.thread_handle = Some(handle);
                Ok(())
            }
            Ok(Err(e)) => {
                let _ = handle.join();
                self.recording.store(false, Ordering::SeqCst);
                Err(e)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                self.recording.store(false, Ordering::SeqCst);
                Err("Timed out while starting audio stream".to_string())
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = handle.join();
                self.recording.store(false, Ordering::SeqCst);
                Err("Audio recording thread exited before starting".to_string())
            }
        }
    }

    pub fn stop(&mut self) -> Result<(Vec<f32>, u32), String> {
        self.recording.store(false, Ordering::SeqCst);

        // Wait for the recording thread to finish
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        let sr = *self.sample_rate.lock().unwrap();
        let samples = std::mem::take(&mut *self.samples.lock().unwrap());

        if samples.is_empty() {
            return Err("No audio recorded".to_string());
        }

        Ok((samples, sr))
    }
}

fn run_recording(
    device_name: String,
    samples: Arc<Mutex<Vec<f32>>>,
    recording: Arc<AtomicBool>,
    sample_rate_out: Arc<Mutex<u32>>,
    app: AppHandle,
    started_tx: mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    let host = cpal::default_host();

    let device = if device_name == "default" {
        host.default_input_device()
            .ok_or("No default input device")?
    } else {
        host.input_devices()
            .map_err(|e| format!("Cannot enumerate devices: {}", e))?
            .find(|d| d.name().map(|n| n == device_name).unwrap_or(false))
            .ok_or_else(|| format!("Device '{}' not found", device_name))?
    };

    let config = device
        .default_input_config()
        .map_err(|e| format!("No default input config: {}", e))?;

    let sr = config.sample_rate().0;
    *sample_rate_out.lock().unwrap() = sr;
    let channels = config.channels() as usize;

    let waveform_counter = Arc::new(Mutex::new(0u32));
    let waveform_buf = Arc::new(Mutex::new(Vec::<f32>::new()));

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build_input_stream(
            &device,
            &config.into(),
            channels,
            samples,
            Arc::clone(&recording),
            waveform_counter,
            waveform_buf,
            app.clone(),
            |s: f32| s,
        ),
        cpal::SampleFormat::F64 => build_input_stream(
            &device,
            &config.into(),
            channels,
            samples,
            Arc::clone(&recording),
            waveform_counter,
            waveform_buf,
            app.clone(),
            |s: f64| s as f32,
        ),
        cpal::SampleFormat::I8 => build_input_stream(
            &device,
            &config.into(),
            channels,
            samples,
            Arc::clone(&recording),
            waveform_counter,
            waveform_buf,
            app.clone(),
            |s: i8| s as f32 / 128.0,
        ),
        cpal::SampleFormat::I16 => build_input_stream(
            &device,
            &config.into(),
            channels,
            samples,
            Arc::clone(&recording),
            waveform_counter,
            waveform_buf,
            app.clone(),
            |s: i16| s as f32 / 32768.0,
        ),
        cpal::SampleFormat::I32 => build_input_stream(
            &device,
            &config.into(),
            channels,
            samples,
            Arc::clone(&recording),
            waveform_counter,
            waveform_buf,
            app.clone(),
            |s: i32| s as f32 / 2_147_483_648.0,
        ),
        cpal::SampleFormat::I64 => build_input_stream(
            &device,
            &config.into(),
            channels,
            samples,
            Arc::clone(&recording),
            waveform_counter,
            waveform_buf,
            app.clone(),
            |s: i64| s as f32 / 9_223_372_036_854_775_808.0,
        ),
        cpal::SampleFormat::U8 => build_input_stream(
            &device,
            &config.into(),
            channels,
            samples,
            Arc::clone(&recording),
            waveform_counter,
            waveform_buf,
            app.clone(),
            |s: u8| (s as f32 - 128.0) / 128.0,
        ),
        cpal::SampleFormat::U16 => build_input_stream(
            &device,
            &config.into(),
            channels,
            samples,
            Arc::clone(&recording),
            waveform_counter,
            waveform_buf,
            app.clone(),
            |s: u16| (s as f32 - 32768.0) / 32768.0,
        ),
        cpal::SampleFormat::U32 => build_input_stream(
            &device,
            &config.into(),
            channels,
            samples,
            Arc::clone(&recording),
            waveform_counter,
            waveform_buf,
            app.clone(),
            |s: u32| (s as f32 - 2_147_483_648.0) / 2_147_483_648.0,
        ),
        cpal::SampleFormat::U64 => build_input_stream(
            &device,
            &config.into(),
            channels,
            samples,
            Arc::clone(&recording),
            waveform_counter,
            waveform_buf,
            app.clone(),
            |s: u64| (s as f32 - 9_223_372_036_854_775_808.0) / 9_223_372_036_854_775_808.0,
        ),
        sample_format => return Err(format!("Unsupported sample format: {}", sample_format)),
    }
    .map_err(|e| format!("Failed to build stream: {}", e))?;

    stream
        .play()
        .map_err(|e| format!("Failed to start stream: {}", e))?;

    let _ = app.emit("recording-started", ());
    let _ = started_tx.send(Ok(()));

    // Keep thread alive while recording
    while recording.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Stream is dropped here, stopping capture
    drop(stream);
    Ok(())
}

fn build_input_stream<T, F>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    samples: Arc<Mutex<Vec<f32>>>,
    recording: Arc<AtomicBool>,
    waveform_counter: Arc<Mutex<u32>>,
    waveform_buf: Arc<Mutex<Vec<f32>>>,
    app: AppHandle,
    convert: F,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: cpal::SizedSample + Copy,
    F: Fn(T) -> f32 + Send + 'static + Copy,
{
    device.build_input_stream(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            if !recording.load(Ordering::SeqCst) {
                return;
            }

            let mono: Vec<f32> = data
                .chunks(channels)
                .map(|frame| frame.iter().map(|&s| convert(s)).sum::<f32>() / channels as f32)
                .collect();

            samples.lock().unwrap().extend_from_slice(&mono);

            let mut counter = waveform_counter.lock().unwrap();
            let mut buf = waveform_buf.lock().unwrap();
            buf.extend_from_slice(&mono);
            *counter += mono.len() as u32;

            if *counter >= 800 {
                let rms = (buf.iter().map(|s| s * s).sum::<f32>() / buf.len() as f32)
                    .sqrt()
                    .min(1.0);
                let _ = app.emit("waveform-update", rms);
                buf.clear();
                *counter = 0;
            }
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    )
}

pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devices| devices.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

/// Resample audio from source_rate to target_rate (linear interpolation)
pub fn resample(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if source_rate == target_rate {
        return samples.to_vec();
    }

    let ratio = source_rate as f64 / target_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_idx = i as f64 * ratio;
        let idx = src_idx as usize;
        let frac = src_idx - idx as f64;

        let sample = if idx + 1 < samples.len() {
            samples[idx] as f64 * (1.0 - frac) + samples[idx + 1] as f64 * frac
        } else if idx < samples.len() {
            samples[idx] as f64
        } else {
            0.0
        };

        output.push(sample as f32);
    }

    output
}
