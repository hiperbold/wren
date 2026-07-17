// Generates SFX + ambient bed as 16-bit WAV — 100% synthetic, royalty-free.
// Run: npm run audio   → writes to public/sfx and public/music
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const PUBLIC = resolve(__dirname, "..", "public");
const SR = 44100;

// ── WAV writer ────────────────────────────────────────────────────────────────
function writeWav(relPath, channels) {
  const numCh = channels.length;
  const numFrames = channels[0].length;
  const bytesPerSample = 2;
  const dataLen = numFrames * numCh * bytesPerSample;
  const buf = Buffer.alloc(44 + dataLen);
  buf.write("RIFF", 0);
  buf.writeUInt32LE(36 + dataLen, 4);
  buf.write("WAVE", 8);
  buf.write("fmt ", 12);
  buf.writeUInt32LE(16, 16);
  buf.writeUInt16LE(1, 20); // PCM
  buf.writeUInt16LE(numCh, 22);
  buf.writeUInt32LE(SR, 24);
  buf.writeUInt32LE(SR * numCh * bytesPerSample, 28);
  buf.writeUInt16LE(numCh * bytesPerSample, 32);
  buf.writeUInt16LE(16, 34);
  buf.write("data", 36);
  buf.writeUInt32LE(dataLen, 40);
  let off = 44;
  for (let i = 0; i < numFrames; i++) {
    for (let c = 0; c < numCh; c++) {
      let s = Math.max(-1, Math.min(1, channels[c][i]));
      buf.writeInt16LE((s * 32767) | 0, off);
      off += 2;
    }
  }
  const abs = resolve(PUBLIC, relPath);
  mkdirSync(dirname(abs), { recursive: true });
  writeFileSync(abs, buf);
  console.log("  ✓", relPath, `(${(dataLen / 1024).toFixed(0)} KB)`);
}

const sine = (t, f) => Math.sin(2 * Math.PI * f * t);
const seconds = (s) => Math.round(s * SR);

// ── 1. click (shortcut) — short, dry tick ─────────────────────────────────────
function click() {
  const n = seconds(0.09);
  const out = new Float32Array(n);
  for (let i = 0; i < n; i++) {
    const t = i / SR;
    const env = Math.exp(-t * 55);
    const body = sine(t, 1800) * 0.5 + sine(t, 2600) * 0.3;
    const noise = (Math.random() * 2 - 1) * Math.exp(-t * 120) * 0.4;
    out[i] = (body * env + noise) * 0.7;
  }
  writeWav("sfx/click.wav", [out]);
}

// ── 2. ding (done) — short bell, cheerful major third ──────────────────────────
function ding() {
  const n = seconds(0.9);
  const out = new Float32Array(n);
  const f1 = 880; // A5
  const f2 = 1108.73; // C#6 (major third)
  for (let i = 0; i < n; i++) {
    const t = i / SR;
    const env = Math.exp(-t * 5.5);
    const partial =
      sine(t, f1) * 0.6 +
      sine(t, f2) * 0.4 +
      sine(t, f1 * 2) * 0.15 * Math.exp(-t * 9);
    out[i] = partial * env * 0.55;
  }
  writeWav("sfx/ding.wav", [out]);
}

// ── 3. whoosh — smooth transition ───────────────────────────────────────────────
function whoosh() {
  const n = seconds(0.35);
  const out = new Float32Array(n);
  let lp = 0;
  for (let i = 0; i < n; i++) {
    const t = i / SR;
    const p = i / n;
    // sine envelope
    const env = Math.sin(Math.PI * p) ** 2;
    // lowpass noise that "opens" and closes
    const cutoff = 0.02 + 0.25 * env;
    const white = Math.random() * 2 - 1;
    lp += (white - lp) * cutoff;
    out[i] = lp * env * 0.5;
  }
  writeWav("sfx/whoosh.wav", [out]);
}

// ── 4. ambient — warm, subtle pad (30s, stereo) ─────────────────────────────────
function ambient() {
  const dur = 30.5;
  const n = seconds(dur);
  const L = new Float32Array(n);
  const R = new Float32Array(n);
  // Am add chord: A2, E3, A3, C4  (amber/cozy)
  const voices = [110.0, 164.81, 220.0, 261.63];
  for (let i = 0; i < n; i++) {
    const t = i / SR;
    // fade in 2.5s / fade out 3.5s
    const fin = Math.min(1, t / 2.5);
    const fout = Math.min(1, (dur - t) / 3.5);
    const fade = Math.min(fin, fout);
    let l = 0;
    let r = 0;
    for (let v = 0; v < voices.length; v++) {
      const f = voices[v];
      // slow vibrato + amplitude LFO per voice (movement)
      const vib = 1 + 0.0015 * sine(t, 0.07 + v * 0.013);
      const lfo = 0.6 + 0.4 * sine(t, 0.05 + v * 0.021);
      const detune = 0.15; // Hz — subtle chorus
      const s =
        (sine(t, f * vib) + sine(t, f * vib + detune)) * 0.5 * lfo;
      // spread voices in stereo
      const pan = (v / (voices.length - 1)) * 2 - 1; // -1..1
      l += s * (0.5 - pan * 0.35);
      r += s * (0.5 + pan * 0.35);
    }
    const g = (fade * 0.16) / voices.length;
    L[i] = l * g;
    R[i] = r * g;
  }
  writeWav("music/ambient.wav", [L, R]);
}

console.log("Generating promotional video audio…");
click();
ding();
whoosh();
ambient();
console.log("Done.");
