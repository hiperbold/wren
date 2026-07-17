/**
 * Pure formatting and shortcut/provider helpers — extracted from the old
 * `SettingsPage` monolith. Centralised so that ALL views reuse the same logic
 * (no re-implementing formatting per view). Human-facing labels resolve through
 * i18n (`common` namespace) using the active language.
 */
import i18n from "@/i18n";
import type { ProviderConfig, StageName, SessionOutcome } from "./tauri";

/** Locale to use for number/date formatting — follows the active UI language. */
function activeLocale(): string {
  return i18n.language || "en";
}

/** Seconds with at most 1 decimal place. */
export function formatSecs(ms: number): string {
  return (ms / 1000).toLocaleString(activeLocale(), {
    maximumFractionDigits: 1,
  });
}

/** Short date dd/mm hh:mm. */
export function formatWhen(ms: number): string {
  return new Date(ms).toLocaleString(activeLocale(), {
    day: "2-digit",
    month: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** Time with seconds, for logs (high temporal density). */
export function formatLogTime(ms: number): string {
  return new Date(ms).toLocaleTimeString(activeLocale(), {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

/** Bytes as whole MB; `n/a` when absent. */
export function formatMB(bytes?: number): string {
  if (bytes == null) return "n/a";
  return (
    (bytes / (1024 * 1024)).toLocaleString(activeLocale(), {
      maximumFractionDigits: 0,
    }) + " MB"
  );
}

/** Human-readable size: whole MB and GB with 1 decimal. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "—";
  const mb = bytes / (1024 * 1024);
  if (mb >= 1024) {
    return (
      (mb / 1024).toLocaleString(activeLocale(), { maximumFractionDigits: 1 }) +
      " GB"
    );
  }
  return mb.toLocaleString(activeLocale(), { maximumFractionDigits: 0 }) + " MB";
}

/** Language code → localized language name (e.g. "pt" → "Portuguese"). */
export function prettyLang(code: string): string {
  if (!code) return "—";
  const key = code.toLowerCase();
  const known: Record<string, string> = {
    pt: "lang.pt",
    en: "lang.en",
    es: "lang.es",
    fr: "lang.fr",
    de: "lang.de",
    it: "lang.it",
    multi: "lang.multi",
    multilingual: "lang.multi",
  };
  const k = known[key];
  return k ? i18n.t(`common:${k}`) : code;
}

/** The pipeline stages, in canonical order. */
export const STAGE_NAMES: StageName[] = [
  "capture_stop",
  "vad",
  "persist_audio",
  "transcribe",
  "deliver",
];

/** Localized label for a pipeline stage. */
export function stageLabel(stage: StageName): string {
  return i18n.t(`common:stage.${stage}`);
}

/** Background color class (stage token) for the Telemetry bars. */
export const STAGE_BG: Record<StageName, string> = {
  capture_stop: "bg-stage-capture",
  vad: "bg-stage-vad",
  persist_audio: "bg-stage-persist",
  transcribe: "bg-stage-transcribe",
  deliver: "bg-stage-deliver",
};

/** Localized label for a session outcome. */
export function outcomeLabel(outcome: SessionOutcome): string {
  return i18n.t(`common:outcome.${outcome}`);
}

/** Live-derive whether a base_url sends audio off the machine. Local host
 * (localhost/127.0.0.1/::1) ⇒ internal; anything else ⇒ external. */
export function egressIsExternal(baseUrl: string): boolean {
  const url = baseUrl.trim();
  if (!url) return false;
  try {
    const host = new URL(url).hostname.toLowerCase().replace(/^\[|\]$/g, "");
    return !(host === "localhost" || host === "127.0.0.1" || host === "::1");
  } catch {
    const b = url.toLowerCase();
    return !(
      b.includes("localhost") ||
      b.includes("127.0.0.1") ||
      b.includes("::1")
    );
  }
}

/** Generate a unique id from `base`, avoiding collisions (e.g. "groq" ⇒ "groq-2"). */
export function uniqueProviderId(
  base: string,
  providers: ProviderConfig[],
): string {
  const taken = new Set(providers.map((p) => p.id));
  if (!taken.has(base)) return base;
  let n = 2;
  while (taken.has(`${base}-${n}`)) n++;
  return `${base}-${n}`;
}

/** Human label for modifier keys ("ctrl" → "Ctrl"); space/esc are localized. */
const PRETTY_KEYS: Record<string, string> = {
  ctrl: "Ctrl",
  commandorcontrol: "Ctrl",
  alt: "Alt",
  shift: "Shift",
  super: "Super",
};

export function prettyKey(part: string): string {
  const p = part.toLowerCase();
  if (p === "space") return i18n.t("common:key.space");
  if (p === "escape") return i18n.t("common:key.escape");
  const known = PRETTY_KEYS[p];
  if (known) return known;
  if (part.length === 1) return part.toUpperCase();
  return part.charAt(0).toUpperCase() + part.slice(1);
}

/** Convert a `KeyboardEvent.code` into the base key of the global-shortcut
 * plugin syntax (Tauri v2). `null` = a pure modifier key. */
export function baseKeyFromCode(code: string): string | null {
  if (/^(Control|Alt|Shift|Meta)(Left|Right)$/.test(code)) return null;
  if (/^Key[A-Z]$/.test(code)) return code.slice(3).toLowerCase();
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  return code.toLowerCase();
}

/** Pressed modifiers, in the project's canonical order. */
export function modifiersFrom(e: React.KeyboardEvent): string[] {
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("ctrl");
  if (e.altKey) mods.push("alt");
  if (e.shiftKey) mods.push("shift");
  if (e.metaKey) mods.push("super");
  return mods;
}
