"use client";

// Sci-fi sound effects using Web Audio API — no external files needed.
// Each sound is synthesized on the fly with a retro-futuristic aesthetic.

let audioCtx: AudioContext | null = null;

function getCtx(): AudioContext {
  if (!audioCtx) audioCtx = new AudioContext();
  return audioCtx;
}

function playTone(freq: number, duration: number, type: OscillatorType = "sine", gain = 0.15) {
  const ctx = getCtx();
  const osc = ctx.createOscillator();
  const g = ctx.createGain();
  osc.type = type;
  osc.frequency.setValueAtTime(freq, ctx.currentTime);
  g.gain.setValueAtTime(gain, ctx.currentTime);
  g.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + duration);
  osc.connect(g).connect(ctx.destination);
  osc.start();
  osc.stop(ctx.currentTime + duration);
}

function playSweep(startFreq: number, endFreq: number, duration: number, type: OscillatorType = "sawtooth", gain = 0.1) {
  const ctx = getCtx();
  const osc = ctx.createOscillator();
  const g = ctx.createGain();
  osc.type = type;
  osc.frequency.setValueAtTime(startFreq, ctx.currentTime);
  osc.frequency.exponentialRampToValueAtTime(endFreq, ctx.currentTime + duration);
  g.gain.setValueAtTime(gain, ctx.currentTime);
  g.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + duration);
  osc.connect(g).connect(ctx.destination);
  osc.start();
  osc.stop(ctx.currentTime + duration);
}

function playNoise(duration: number, gain = 0.08) {
  const ctx = getCtx();
  const bufferSize = ctx.sampleRate * duration;
  const buffer = ctx.createBuffer(1, bufferSize, ctx.sampleRate);
  const data = buffer.getChannelData(0);
  for (let i = 0; i < bufferSize; i++) data[i] = Math.random() * 2 - 1;
  const src = ctx.createBufferSource();
  const g = ctx.createGain();
  const filter = ctx.createBiquadFilter();
  filter.type = "bandpass";
  filter.frequency.value = 2000;
  filter.Q.value = 5;
  src.buffer = buffer;
  g.gain.setValueAtTime(gain, ctx.currentTime);
  g.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + duration);
  src.connect(filter).connect(g).connect(ctx.destination);
  src.start();
  src.stop(ctx.currentTime + duration);
}

export const sounds = {
  // Trade opened — ascending double blip
  tradeOpen() {
    playTone(880, 0.08, "square", 0.1);
    setTimeout(() => playTone(1320, 0.12, "square", 0.1), 80);
  },

  // Trade closed — descending tone
  tradeClose() {
    playSweep(1200, 400, 0.25, "sawtooth", 0.08);
  },

  // Stop loss hit — urgent alarm
  stopLoss() {
    playTone(440, 0.15, "square", 0.12);
    setTimeout(() => playTone(330, 0.15, "square", 0.12), 160);
    setTimeout(() => playTone(220, 0.3, "square", 0.12), 320);
  },

  // Take profit hit — success chime
  takeProfit() {
    playTone(1047, 0.1, "sine", 0.12);
    setTimeout(() => playTone(1319, 0.1, "sine", 0.12), 100);
    setTimeout(() => playTone(1568, 0.15, "sine", 0.12), 200);
  },

  // Circuit breaker — heavy warning
  circuitBreaker() {
    playNoise(0.4, 0.12);
    playTone(110, 0.5, "sawtooth", 0.15);
  },

  // Connected — short confirmation beep
  connected() {
    playTone(1000, 0.06, "sine", 0.08);
    setTimeout(() => playTone(1500, 0.08, "sine", 0.08), 70);
  },

  // Disconnected — low warning
  disconnected() {
    playSweep(600, 200, 0.3, "sine", 0.1);
  },

  // Decision — subtle data blip
  decision() {
    playTone(2000, 0.03, "sine", 0.05);
  },

  // Trailing stop moved — soft click
  trailingStop() {
    playTone(1800, 0.04, "triangle", 0.06);
  },
};

export type SoundEvent = keyof typeof sounds;
