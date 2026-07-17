// Palette and constants extracted 1:1 from the real app, so the video matches the product.
// UI colors: apps/desktop/src/settings/settings.css
// Bubble colors/animation: apps/desktop/src-tauri/src/overlay_native.rs (WGSL shader)

export const theme = {
  // Base (settings.css)
  bg: "#171513",
  bgDeep: "#100e0c",
  panel: "#211e1b",
  panelHi: "#2a2622",
  border: "#3a352f",
  ink: "#f4ece4",
  inkMuted: "#cfc4b8",
  inkFaint: "#8d8479",

  // Brand amber (settings + shader: vec3(1.0, 0.698, 0.4))
  amber: "#ffb266",
  amberDeep: "#e07a3f",

  // Bubble states (shader fs_main)
  recDot: "#ff6b5e", // vec3(1.0, 0.42, 0.37) — red dot while recording
  done: "#78db97", // vec3(0.47, 0.86, 0.59) — done
  error: "#ff6b6b", // vec3(1.0, 0.42, 0.42) — error

  // Bubble pill background: vec3(0.094, 0.086, 0.078) @ alpha 0.92
  bubbleBg: "rgba(24, 22, 20, 0.92)",
} as const;

// Shader animation constants (keep in sync with overlay_native.rs)
export const shader = {
  BAR_COUNT: 36,
  GAIN: 4.5,
  DECAY: 0.82,
  // "processing" pulse: 0.18 + 0.14 * sin(t*7.5 + i/2.4)
  pulse: (t: number, i: number) => 0.18 + 0.14 * Math.sin(t * 7.5 + i / 2.4),
} as const;

export const fontStack =
  '"Inter", system-ui, -apple-system, "Segoe UI", Roboto, sans-serif';
export const monoStack =
  '"JetBrains Mono", "SF Mono", ui-monospace, "Cascadia Code", monospace';
