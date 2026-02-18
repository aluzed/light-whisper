const { invoke } = window.__TAURI__.core;
const { event } = window.__TAURI__;

const toastContainer = document.getElementById('toast-container');

function showToast(message) {
  const toast = document.createElement('div');
  toast.className = 'toast';
  toast.textContent = message;
  toast.addEventListener('click', () => toast.remove());
  toastContainer.appendChild(toast);
  setTimeout(() => toast.remove(), 5500);
}

event.listen('app-error', (e) => {
  showToast(e.payload);
});

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
const shortcutDisplay = document.getElementById('shortcut-display');
const shortcutAssignBtn = document.getElementById('shortcut-assign-btn');
const shortcutDefaultBtn = document.getElementById('shortcut-default-btn');
const shortcutError = document.getElementById('shortcut-error');

function updateWhisperOptionsVisibility() {
  whisperOptionsEl.style.display = engineEl.value === 'whisper' ? '' : 'none';
}

async function loadConfig() {
  try {
    const config = await invoke('get_config');
    engineEl.value = config.engine || 'whisper';
    modelSizeEl.value = config.model_size || 'base';
    languageEl.value = config.language || 'auto';

    shortcutDisplay.value = formatShortcutDisplay(config.shortcut || 'Alt+Space');

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
    // Read current config to preserve shortcut (managed separately)
    const current = await invoke('get_config');
    await invoke('save_config', {
      config: {
        audio_device: audioDeviceEl.value,
        model_size: modelSizeEl.value,
        language: languageEl.value,
        engine: engineEl.value,
        shortcut: current.shortcut || 'Alt+Space',
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

// ── Shortcut ──

function formatShortcutDisplay(shortcut) {
  return shortcut.replace(/\+/g, ' + ');
}

const MODIFIER_KEYS = new Set([
  'Alt', 'Control', 'Shift', 'Meta',
  'AltLeft', 'AltRight', 'ControlLeft', 'ControlRight',
  'ShiftLeft', 'ShiftRight', 'MetaLeft', 'MetaRight',
]);

function codeToKey(code) {
  if (code.startsWith('Key')) return code.slice(3);
  if (code.startsWith('Digit')) return code.slice(5);
  if (code.startsWith('Numpad')) return 'Num' + code.slice(6);
  if (code.startsWith('Arrow')) return code.slice(5);
  const map = {
    'Backquote': '`', 'Minus': '-', 'Equal': '=',
    'BracketLeft': '[', 'BracketRight': ']', 'Backslash': '\\',
    'Semicolon': ';', 'Quote': "'", 'Comma': ',', 'Period': '.',
    'Slash': '/',
  };
  return map[code] || code;
}

let isListening = false;

function startListening() {
  isListening = true;
  shortcutDisplay.value = 'Press a key combo...';
  shortcutDisplay.classList.add('listening');
  shortcutAssignBtn.textContent = 'Cancel';
  shortcutError.textContent = '';
}

function stopListening() {
  isListening = false;
  shortcutDisplay.classList.remove('listening');
  shortcutAssignBtn.textContent = 'Assign';
}

shortcutAssignBtn.addEventListener('click', () => {
  if (isListening) {
    // Cancel — restore current config value
    stopListening();
    invoke('get_config').then(config => {
      shortcutDisplay.value = formatShortcutDisplay(config.shortcut || 'Alt+Space');
    });
  } else {
    startListening();
  }
});

shortcutDefaultBtn.addEventListener('click', async () => {
  stopListening();
  shortcutError.textContent = '';
  try {
    await invoke('change_shortcut', { shortcut: 'Alt+Space' });
    shortcutDisplay.value = formatShortcutDisplay('Alt+Space');
  } catch (e) {
    shortcutError.textContent = String(e);
  }
});

document.addEventListener('keydown', async (e) => {
  if (!isListening) return;
  e.preventDefault();
  e.stopPropagation();

  // Ignore modifier-only presses
  if (MODIFIER_KEYS.has(e.code) || MODIFIER_KEYS.has(e.key)) return;

  const parts = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  if (e.metaKey) parts.push('Super');

  parts.push(codeToKey(e.code));

  const shortcut = parts.join('+');
  stopListening();
  shortcutDisplay.value = formatShortcutDisplay(shortcut);
  shortcutError.textContent = '';

  try {
    await invoke('change_shortcut', { shortcut });
  } catch (err) {
    shortcutError.textContent = String(err);
  }
});

loadConfig();
