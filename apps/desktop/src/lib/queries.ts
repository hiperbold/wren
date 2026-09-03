/**
 * Data hooks (TanStack Query) that wrap the Tauri commands. Views do NOT call
 * `invoke` directly — they use these hooks (reads) and the mutations (writes).
 * This file + the settings store are the CONTRACT for the views.
 *
 * Note: settings does NOT have a "usage" read hook here — the editable state
 * lives in the settings store (store.ts, with autosave). `useHydrateSettings`
 * only fetches the initial value and seeds the store.
 */
import {
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { tauri, type Settings } from "./tauri";

/** Centralized query keys. */
export const qk = {
  settings: ["settings"] as const,
  history: ["history"] as const,
  presets: ["provider-presets"] as const,
  inputDevices: ["input-devices"] as const,
  models: (baseUrl: string, apiKey: string | null) =>
    ["models", baseUrl, apiKey] as const,
  embeddedCatalog: ["embedded-catalog"] as const,
  localModels: ["embedded-local"] as const,
  logs: (level: string | null, query: string | null) =>
    ["logs", level, query] as const,
  logPath: ["log-path"] as const,
  metrics: ["metrics"] as const,
  hardwareInfo: ["hardware-info"] as const,
};

/** Initial settings value (to seed the store). See `useHydrateSettings`. */
export function useSettingsQuery() {
  return useQuery({ queryKey: qk.settings, queryFn: tauri.getSettings });
}

export function useHistory() {
  return useQuery({ queryKey: qk.history, queryFn: tauri.getHistory });
}

export function useProviderPresets() {
  return useQuery({ queryKey: qk.presets, queryFn: tauri.providerPresets });
}

export function useInputDevices() {
  return useQuery({
    queryKey: qk.inputDevices,
    queryFn: tauri.listInputDevices,
  });
}

/** Provider's model list (/models endpoint). Disabled without a base_url;
 * `retry:false` to fall back quickly to free text when the server doesn't list. */
export function useModels(baseUrl: string, apiKey: string | null) {
  return useQuery({
    queryKey: qk.models(baseUrl, apiKey),
    queryFn: () => tauri.listModels(baseUrl, apiKey),
    enabled: !!baseUrl.trim(),
    retry: false,
    staleTime: 30_000,
  });
}

export function useEmbeddedCatalog() {
  return useQuery({
    queryKey: qk.embeddedCatalog,
    queryFn: tauri.embeddedCatalog,
    retry: false,
  });
}

export function useLocalModels() {
  return useQuery({
    queryKey: qk.localModels,
    queryFn: tauri.embeddedLocalModels,
    retry: false,
  });
}

/** Filtered logs. `autoRefresh` enables a refetch every 2s (the webview is just
 * a reader; the app runs in another process). */
export function useLogs(
  level: string | null,
  query: string | null,
  autoRefresh: boolean,
) {
  return useQuery({
    queryKey: qk.logs(level, query),
    queryFn: () => tauri.getLogs({ level, query }),
    refetchInterval: autoRefresh ? 2000 : false,
  });
}

export function useLogPath() {
  return useQuery({
    queryKey: qk.logPath,
    queryFn: tauri.logFilePath,
    staleTime: Infinity,
  });
}

export function useMetrics() {
  return useQuery({ queryKey: qk.metrics, queryFn: () => tauri.getMetrics(50) });
}

/** Coarse machine-capability hint, used by the onboarding wizard's local-model
 * step to decide whether to show a "may be slow on this machine" note. */
export function useHardwareInfo() {
  return useQuery({
    queryKey: qk.hardwareInfo,
    queryFn: tauri.hardwareInfo,
    staleTime: Infinity,
  });
}

/* --------------------------------- mutations ------------------------------ */

/** Persists settings. In general you do NOT call this directly — the settings
 * store handles the autosave. Exposed for advanced cases (e.g. flows outside the
 * store). */
export function useSaveSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (settings: Settings) => tauri.saveSettings(settings),
    onSuccess: (_res, settings) => qc.setQueryData(qk.settings, settings),
  });
}

export function useRetryTranscription() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (createdAtMs: number) => tauri.retryTranscription(createdAtMs),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.history }),
  });
}

export function useCheckUpdates() {
  return useMutation({ mutationFn: () => tauri.checkForUpdates() });
}

export function useClearLogs() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => tauri.clearLogs(),
    onSuccess: () =>
      qc.invalidateQueries({ predicate: (q) => q.queryKey[0] === "logs" }),
  });
}

/** Starts the download of an embedded model. PROGRESS arrives via the
 * `embedded://download-progress` event (see `onDownloadProgress` in tauri.ts) —
 * this mutation only fires and resolves when it finishes. On completion, it
 * reloads the local models. */
export function useDownloadModel() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => tauri.embeddedDownloadModel(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.localModels }),
  });
}

export function useDeleteModel() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => tauri.embeddedDeleteModel(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: qk.localModels }),
  });
}
