const { event } = window.__TAURI__;

const canvas = document.getElementById('waveform');
const ctx = canvas.getContext('2d');
const timerEl = document.getElementById('timer');

const BAR_WIDTH = 3;
const GAP = 3;
const NUM_BARS = 30;

let startTime = null;
let timerInterval = null;
let bars = new Array(NUM_BARS).fill(0);
let canvasW = 180;
let canvasH = 50;
let dpr = window.devicePixelRatio || 1;

// Set canvas size once
canvas.width = canvasW * dpr;
canvas.height = canvasH * dpr;
canvas.style.width = canvasW + 'px';
canvas.style.height = canvasH + 'px';
ctx.scale(dpr, dpr);

function startTimer() {
  startTime = Date.now();
  timerInterval = setInterval(() => {
    const elapsed = Math.floor((Date.now() - startTime) / 1000);
    const mins = Math.floor(elapsed / 60);
    const secs = elapsed % 60;
    timerEl.textContent = `${mins}:${secs.toString().padStart(2, '0')}`;
  }, 200);
}

function stopTimer() {
  if (timerInterval) {
    clearInterval(timerInterval);
    timerInterval = null;
  }
  timerEl.textContent = '0:00';
  startTime = null;
}

function drawWaveform() {
  ctx.clearRect(0, 0, canvasW, canvasH);

  const centerY = canvasH / 2;

  for (let i = 0; i < bars.length; i++) {
    const amplitude = bars[i] || 0;
    const barHeight = Math.max(3, amplitude * (canvasH * 0.8));
    const x = i * (BAR_WIDTH + GAP);

    const gradient = ctx.createLinearGradient(0, centerY - barHeight / 2, 0, centerY + barHeight / 2);
    gradient.addColorStop(0, 'rgba(100, 180, 255, 0.9)');
    gradient.addColorStop(1, 'rgba(100, 180, 255, 0.3)');

    ctx.fillStyle = gradient;
    ctx.beginPath();
    ctx.roundRect(x, centerY - barHeight / 2, BAR_WIDTH, barHeight, 1.5);
    ctx.fill();
  }
}

// Backend emits a single RMS value per update; shift bars left and append
event.listen('waveform-update', (e) => {
  const rms = e.payload;
  bars.shift();
  bars.push(rms);
  drawWaveform();
});

event.listen('recording-started', () => {
  startTimer();
  bars.fill(0);
  drawWaveform();
});

event.listen('recording-stopped', () => {
  stopTimer();
  bars.fill(0);
  drawWaveform();
});

// Initial draw
drawWaveform();
