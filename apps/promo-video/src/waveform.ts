import { shader } from "./theme";

// Deterministic "speech" envelope (without real audio): layers of sinusoids with
// pauses, mimicking syllables. Returns 0..1. Deterministic => reproducible render.
const speechEnvelope = (t: number): number => {
  // fast syllables
  const syl = Math.abs(Math.sin(t * 8.2)) * Math.abs(Math.sin(t * 3.1 + 0.7));
  // phrase undulation (rises/falls over seconds)
  const phrase = 0.55 + 0.45 * Math.sin(t * 0.9 - 0.4);
  // micro detail
  const detail = 0.5 + 0.5 * Math.sin(t * 21.0 + 1.3);
  // occasional pauses (breathing)
  const gate = Math.max(0, Math.sin(t * 1.3 - 0.2)) > 0.08 ? 1 : 0.15;
  return Math.min(1, syl * phrase * (0.7 + 0.3 * detail) * gate);
};

export type BubbleState = "recording" | "processing" | "done";

// Heights of the 36 bars (0..1). `recording` does the app's gain/decay scroll;
// `processing` uses exactly the shader's pulse.
export const barLevels = (
  state: BubbleState,
  t: number,
  count = shader.BAR_COUNT,
): number[] => {
  const out: number[] = [];
  if (state === "processing") {
    for (let i = 0; i < count; i++) {
      out.push(Math.max(0.04, shader.pulse(t, i)));
    }
    return out;
  }
  if (state === "done") {
    // bars low and calm, almost at rest
    for (let i = 0; i < count; i++) {
      out.push(0.08 + 0.03 * Math.sin(t * 2 + i / 3));
    }
    return out;
  }
  // recording: each bar samples the envelope at a shifted instant (scroll)
  for (let i = 0; i < count; i++) {
    const sampleT = t - (count - i) * 0.028;
    const raw = speechEnvelope(sampleT);
    const boosted = Math.min(1, raw * (shader.GAIN * 0.35));
    // slight vertical symmetry (bars from center), with floor
    out.push(Math.max(0.06, boosted));
  }
  return out;
};
