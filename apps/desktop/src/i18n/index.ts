import i18n from "i18next";
import { initReactI18next } from "react-i18next";

// English is the default/source language; pt-BR is a shipped locale. Each UI
// section owns its own namespace file so translation work parallelises without
// write conflicts. Keys are flat with dot notation (e.g. "activation.label").
import en_common from "./locales/en/common.json";
import en_nav from "./locales/en/nav.json";
import en_shortcuts from "./locales/en/shortcuts.json";
import en_provider from "./locales/en/provider.json";
import en_models from "./locales/en/models.json";
import en_audio from "./locales/en/audio.json";
import en_system from "./locales/en/system.json";
import en_history from "./locales/en/history.json";
import en_diagnostics from "./locales/en/diagnostics.json";

import ptBR_common from "./locales/pt-BR/common.json";
import ptBR_nav from "./locales/pt-BR/nav.json";
import ptBR_shortcuts from "./locales/pt-BR/shortcuts.json";
import ptBR_provider from "./locales/pt-BR/provider.json";
import ptBR_models from "./locales/pt-BR/models.json";
import ptBR_audio from "./locales/pt-BR/audio.json";
import ptBR_system from "./locales/pt-BR/system.json";
import ptBR_history from "./locales/pt-BR/history.json";
import ptBR_diagnostics from "./locales/pt-BR/diagnostics.json";

export const NAMESPACES = [
  "common",
  "nav",
  "shortcuts",
  "provider",
  "models",
  "audio",
  "system",
  "history",
  "diagnostics",
] as const;

const resources = {
  en: {
    common: en_common,
    nav: en_nav,
    shortcuts: en_shortcuts,
    provider: en_provider,
    models: en_models,
    audio: en_audio,
    system: en_system,
    history: en_history,
    diagnostics: en_diagnostics,
  },
  "pt-BR": {
    common: ptBR_common,
    nav: ptBR_nav,
    shortcuts: ptBR_shortcuts,
    provider: ptBR_provider,
    models: ptBR_models,
    audio: ptBR_audio,
    system: ptBR_system,
    history: ptBR_history,
    diagnostics: ptBR_diagnostics,
  },
} as const;

// A future in-app language selector can persist the choice here; default to en.
const stored =
  typeof localStorage !== "undefined" ? localStorage.getItem("wren.lang") : null;

void i18n.use(initReactI18next).init({
  resources,
  lng: stored ?? "en",
  fallbackLng: "en",
  ns: NAMESPACES as unknown as string[],
  defaultNS: "common",
  interpolation: { escapeValue: false },
  returnNull: false,
});

export default i18n;
