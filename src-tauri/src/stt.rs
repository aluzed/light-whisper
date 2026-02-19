use parakeet_rs::Transcriber;
use std::path::Path;
use std::sync::{Arc, Mutex};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct WhisperEngine {
    ctx: Option<Arc<Mutex<WhisperContext>>>,
}

impl WhisperEngine {
    pub fn new() -> Self {
        // Suppress verbose whisper.cpp C library logging
        unsafe {
            whisper_rs::set_log_callback(None, std::ptr::null_mut());
        }
        Self { ctx: None }
    }

    pub fn load_model(&mut self, model_path: &Path) -> Result<(), String> {
        if !model_path.exists() {
            return Err(format!("Model not found: {}", model_path.display()));
        }

        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or("Invalid model path")?,
            WhisperContextParameters::default(),
        )
        .map_err(|e| format!("Failed to load whisper model: {}", e))?;

        self.ctx = Some(Arc::new(Mutex::new(ctx)));
        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        self.ctx.is_some()
    }

    pub fn transcribe(&self, samples: &[f32], language: &str) -> Result<String, String> {
        let ctx = self.ctx.as_ref().ok_or("Whisper model not loaded")?;
        let ctx = ctx.lock().map_err(|e| format!("Lock error: {}", e))?;

        let mut state = ctx
            .create_state()
            .map_err(|e| format!("Failed to create state: {}", e))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        if language != "auto" {
            params.set_language(Some(language));
        }

        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_single_segment(false);
        params.set_n_threads(4);

        state
            .full(params, samples)
            .map_err(|e| format!("Transcription failed: {}", e))?;

        let num_segments = state.full_n_segments();

        let mut text = String::new();
        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                if let Ok(s) = segment.to_str() {
                    text.push_str(s);
                }
            }
        }

        Ok(text.trim().to_string())
    }
}

pub struct ParakeetEngine {
    model: Option<parakeet_rs::ParakeetTDT>,
}

impl ParakeetEngine {
    pub fn new() -> Self {
        Self { model: None }
    }

    /// Load from a directory containing encoder-model.onnx, decoder_joint-model.onnx, vocab.txt
    pub fn load_model(&mut self, model_dir: &Path) -> Result<(), String> {
        if !model_dir.exists() {
            return Err(format!("Parakeet model dir not found: {}", model_dir.display()));
        }

        let config = parakeet_rs::ExecutionConfig::new().with_intra_threads(4);

        let model = parakeet_rs::ParakeetTDT::from_pretrained(model_dir, Some(config))
            .map_err(|e| format!("Failed to load Parakeet model: {}", e))?;

        self.model = Some(model);
        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        self.model.is_some()
    }

    /// Transcribe 16kHz mono f32 samples. Language param is ignored (Parakeet v3 auto-detects).
    pub fn transcribe(&mut self, samples: &[f32], _language: &str) -> Result<String, String> {
        let model = self.model.as_mut().ok_or("Parakeet model not loaded")?;

        let result = model
            .transcribe_samples(samples.to_vec(), 16000, 1, None)
            .map_err(|e| format!("Parakeet transcription failed: {}", e))?;

        Ok(result.text.trim().to_string())
    }
}

// ── Unified Engine ──

enum EngineInner {
    Whisper(WhisperEngine),
    Parakeet(ParakeetEngine),
}

pub struct SttEngine {
    inner: EngineInner,
}

impl SttEngine {
    pub fn new_whisper() -> Self {
        Self {
            inner: EngineInner::Whisper(WhisperEngine::new()),
        }
    }

    pub fn new_parakeet() -> Self {
        Self {
            inner: EngineInner::Parakeet(ParakeetEngine::new()),
        }
    }

    pub fn from_engine_name(name: &str) -> Self {
        match name {
            "parakeet" => Self::new_parakeet(),
            _ => Self::new_whisper(),
        }
    }

    pub fn load_model(&mut self, path: &Path) -> Result<(), String> {
        match &mut self.inner {
            EngineInner::Whisper(w) => w.load_model(path),
            EngineInner::Parakeet(p) => p.load_model(path),
        }
    }

    pub fn is_loaded(&self) -> bool {
        match &self.inner {
            EngineInner::Whisper(w) => w.is_loaded(),
            EngineInner::Parakeet(p) => p.is_loaded(),
        }
    }

    pub fn transcribe(&mut self, samples: &[f32], language: &str) -> Result<String, String> {
        match &mut self.inner {
            EngineInner::Whisper(w) => w.transcribe(samples, language),
            EngineInner::Parakeet(p) => p.transcribe(samples, language),
        }
    }
}
