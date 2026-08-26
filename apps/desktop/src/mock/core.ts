/**
 * Replacement for `@tauri-apps/api/core` in `--mode mock`. Routes each
 * `invoke()` command to the fake data in `data.ts`, without a Rust backend.
 * Enabled by an alias in vite.config.ts (mock mode only). Write commands
 * (save_settings, clear_logs, deletes) just resolve successfully.
 */
import * as data from "./data";
import { emitMock } from "./event";

export type InvokeArgs = Record<string, unknown>;

/** Simulates the progress of an embedded model download by emitting a few ticks
 * of `embedded://download-progress` (the key moment of the Models tab). Mock
 * mode only — none of this ends up in the real build. */
function simulateDownload(id: string): void {
  const model = data.embeddedCatalog.find((m) => m.id === id);
  const total = model?.sizeBytes ?? 500 * 1024 * 1024;
  // Plausible progress over ~2.6s (mock — just to exercise the download-bar UI;
  // none of this ends up in the real build).
  const steps: { frac: number; at: number }[] = [
    { frac: 0.08, at: 300 },
    { frac: 0.22, at: 700 },
    { frac: 0.4, at: 1100 },
    { frac: 0.58, at: 1500 },
    { frac: 0.78, at: 1950 },
    { frac: 0.93, at: 2300 },
    { frac: 1, at: 2600 },
  ];
  steps.forEach(({ frac, at }) => {
    setTimeout(() => {
      const done = frac >= 1;
      // On completion, mark the model as downloaded (the locals refetch sees it).
      if (done && !data.embeddedLocal.includes(id)) data.embeddedLocal.push(id);
      emitMock("embedded://download-progress", {
        id,
        downloaded: Math.round(total * frac),
        total,
        done,
      });
    }, at);
  });
}

export async function invoke<T>(cmd: string, args?: InvokeArgs): Promise<T> {
  const a = (args ?? {}) as Record<string, unknown>;
  // Small optional delay to simulate IPC (kept at 0 for screenshots).
  switch (cmd) {
    case "get_settings":
      return clone(data.settings) as T;
    case "save_settings":
      return undefined as T;
    case "provider_presets":
      return clone(data.providerPresets) as T;
    case "list_input_devices":
      return clone(data.inputDevices) as T;
    case "list_models":
      return clone(data.modelsFor(String(a.baseUrl ?? ""))) as T;
    case "get_history":
      return clone(data.history) as T;
    case "retry_transcription":
      return "Text successfully reprocessed from the saved audio." as T;
    case "check_for_updates":
      return "You're already on the latest version (0.1.0)." as T;
    case "get_logs":
      return clone(
        data.filteredLogs(
          (a.level as string | null) ?? null,
          (a.query as string | null) ?? null,
        ),
      ) as T;
    case "clear_logs":
      return undefined as T;
    case "log_file_path":
      return data.logFilePath as T;
    case "get_metrics":
      return clone(data.metrics) as T;
    case "embedded_catalog":
      return clone(data.embeddedCatalog) as T;
    case "embedded_local_models":
      return clone(data.embeddedLocal) as T;
    case "embedded_download_model":
      simulateDownload(String(a.id ?? ""));
      return undefined as T;
    case "embedded_delete_model":
      return undefined as T;
    case "hardware_info":
      return clone(data.hardwareInfo) as T;
    // Event-plugin commands (in case something calls via core): resolve.
    case "plugin:event|listen":
    case "plugin:event|unlisten":
      return undefined as T;
    default:
      console.warn("[tauri-mock] unhandled command:", cmd, args);
      return undefined as T;
  }
}

function clone<T>(v: T): T {
  return JSON.parse(JSON.stringify(v));
}

// Some modules import `Channel`/`convertFileSrc`; expose stubs to be safe.
export class Channel<T = unknown> {
  onmessage: ((message: T) => void) | null = null;
}
export function convertFileSrc(path: string): string {
  return path;
}
