/**
 * Settings store with AUTOSAVE (Zustand). This is the central contract for the
 * settings views: they read `settings` and call a setter; the store persists on
 * its own (debounce ~500ms) via `save_settings` and syncs the Query cache.
 *
 * RULE FOR THE VIEWS: never call `save_settings` by hand. Just change the state
 * (setField / updateActiveProvider / update) — the save happens automatically.
 * The feedback ("Saved"/error) is read from `saveState` and shown by the App.
 */
import { create } from "zustand";
import { tauri, type ProviderConfig, type Settings } from "./tauri";
import { queryClient } from "./query-client";
import { qk } from "./queries";

const SAVE_DEBOUNCE_MS = 500;
const SAVED_FLASH_MS = 1600;

export type SaveState = "idle" | "saving" | "saved" | "error";

interface SettingsState {
  /** Editable in-memory state. `null` until hydrated. */
  settings: Settings | null;
  saveState: SaveState;
  saveError: string | null;

  /** Seeds the initial state (from `useSettingsQuery`). Idempotent: does not
   * overwrite edits in progress. */
  hydrate: (s: Settings) => void;

  /** Generic updater (takes the current state, returns the next) + schedules save. */
  update: (updater: (s: Settings) => Settings) => void;
  /** Shortcut to change a top-level field. */
  setField: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
  /** Applies a patch to the ACTIVE provider (by active_provider_id). */
  updateActiveProvider: (patch: Partial<ProviderConfig>) => void;

  /** Forces an immediate save (cancels the debounce). Useful for "instant"
   * actions like activating a model/provider. */
  saveNow: () => void;
}

let debounceTimer: ReturnType<typeof setTimeout> | undefined;
let flashTimer: ReturnType<typeof setTimeout> | undefined;

export const useSettingsStore = create<SettingsState>((set, get) => {
  const persist = async () => {
    const current = get().settings;
    if (!current) return;
    set({ saveState: "saving", saveError: null });
    try {
      await tauri.saveSettings(current);
      // Keeps the read cache consistent with what was just saved.
      queryClient.setQueryData(qk.settings, current);
      set({ saveState: "saved" });
      clearTimeout(flashTimer);
      flashTimer = setTimeout(() => {
        // Only goes back to idle if nobody triggered another save in the meantime.
        if (get().saveState === "saved") set({ saveState: "idle" });
      }, SAVED_FLASH_MS);
    } catch (e) {
      set({ saveState: "error", saveError: String(e) });
    }
  };

  const schedule = () => {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(persist, SAVE_DEBOUNCE_MS);
  };

  return {
    settings: null,
    saveState: "idle",
    saveError: null,

    hydrate: (s) => {
      if (!get().settings) set({ settings: s });
    },

    update: (updater) => {
      const current = get().settings;
      if (!current) return;
      set({ settings: updater(current) });
      schedule();
    },

    setField: (key, value) =>
      get().update((s) => ({ ...s, [key]: value })),

    updateActiveProvider: (patch) =>
      get().update((s) => ({
        ...s,
        providers: s.providers.map((p) =>
          p.id === s.active_provider_id ? { ...p, ...patch } : p,
        ),
      })),

    saveNow: () => {
      clearTimeout(debounceTimer);
      void persist();
    },
  };
});
