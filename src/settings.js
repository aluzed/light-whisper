const { invoke } = window.__TAURI__.core;
const { event } = window.__TAURI__;

const audioDeviceEl = document.getElementById('audio-device');
const engineEl = document.getElementById('engine');
const modelSizeEl = document.getElementById('model-size');
const whisperOptionsEl = document.getElementById('whisper-options');
const languageEl = document.getElementById('language');
const downloadBtn = document.getElementById('download-btn');
const saveBtn = document.getElementById('save-btn');
const modelStatusEl = document.getElementById('model-status');
const progressContainer = document.getElementById('progress-container');
const progressFill = document.getElementById('progress-fill');
const progressText = document.getElementById('progress-text');

function updateWhisperOptionsVisibility() {
  whisperOptionsEl.style.display = engineEl.value === 'whisper' ? '' : 'none';
}

async function loadConfig() {
  try {
    const config = await invoke('get_config');
    engineEl.value = config.engine || 'whisper';
    modelSizeEl.value = config.model_size || 'base';
    languageEl.value = config.language || 'auto';

    updateWhisperOptionsVisibility();

    // Load audio devices
    const devices = await invoke('list_audio_devices');
    audioDeviceEl.innerHTML = '<option value="default">Default</option>';
    devices.forEach(d => {
      const opt = document.createElement('option');
      opt.value = d;
      opt.textContent = d;
      audioDeviceEl.appendChild(opt);
    });
    if (config.audio_device && config.audio_device !== 'default') {
      audioDeviceEl.value = config.audio_device;
    }

    // Check model status
    await checkModelStatus();
  } catch (e) {
    console.error('Failed to load config:', e);
  }
}

async function checkModelStatus() {
  try {
    const engine = engineEl.value;
    const exists = await invoke('check_model_exists', {
      engine: engine,
      modelSize: modelSizeEl.value,
    });
    const label = engine === 'parakeet' ? 'Parakeet TDT v3' : `Whisper "${modelSizeEl.value}"`;
    if (exists) {
      modelStatusEl.textContent = `${label} is downloaded and ready.`;
      modelStatusEl.style.color = '#50c878';
      downloadBtn.textContent = 'Re-download Model';
    } else {
      modelStatusEl.textContent = `${label} is not downloaded.`;
      modelStatusEl.style.color = '#ff8844';
      downloadBtn.textContent = 'Download Model';
    }
  } catch (e) {
    modelStatusEl.textContent = 'Could not check model status.';
    modelStatusEl.style.color = '#ff4444';
  }
}

engineEl.addEventListener('change', () => {
  updateWhisperOptionsVisibility();
  checkModelStatus();
});

modelSizeEl.addEventListener('change', checkModelStatus);

downloadBtn.addEventListener('click', async () => {
  downloadBtn.disabled = true;
  progressContainer.classList.remove('hidden');
  progressFill.style.width = '0%';
  progressText.textContent = 'Starting download...';

  try {
    await invoke('download_model', {
      engine: engineEl.value,
      modelSize: modelSizeEl.value,
    });
  } catch (e) {
    progressText.textContent = `Error: ${e}`;
    downloadBtn.disabled = false;
  }
});

let fileInfo = '';

event.listen('download-file-info', (e) => {
  const { file_index, file_count, file_name } = e.payload;
  fileInfo = file_count > 1 ? `[${file_index}/${file_count}] ` : '';
  progressFill.style.width = '0%';
});

event.listen('download-progress', (e) => {
  const { percent, downloaded_mb, total_mb } = e.payload;
  progressFill.style.width = `${percent}%`;
  progressText.textContent = `${fileInfo}${downloaded_mb.toFixed(1)} / ${total_mb.toFixed(1)} MB (${percent.toFixed(0)}%)`;
});

event.listen('download-complete', () => {
  progressFill.style.width = '100%';
  progressText.textContent = 'Download complete!';
  downloadBtn.disabled = false;
  checkModelStatus();
});

event.listen('download-error', (e) => {
  progressText.textContent = `Error: ${e.payload}`;
  downloadBtn.disabled = false;
});

saveBtn.addEventListener('click', async () => {
  try {
    await invoke('save_config', {
      config: {
        audio_device: audioDeviceEl.value,
        model_size: modelSizeEl.value,
        language: languageEl.value,
        engine: engineEl.value,
      }
    });
    saveBtn.textContent = 'Saved!';
    setTimeout(() => { saveBtn.textContent = 'Save Settings'; }, 1500);
  } catch (e) {
    console.error('Failed to save:', e);
    saveBtn.textContent = 'Error saving';
    setTimeout(() => { saveBtn.textContent = 'Save Settings'; }, 2000);
  }
});

loadConfig();
