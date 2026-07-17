// ─────────────────────────────────────────────────────────────────────────────
// ALL video copy lives here. Switching `LANG` to "pt" localizes the entire video
// in a single line. (Current focus: international leads → "en".)
// ─────────────────────────────────────────────────────────────────────────────

export type Lang = "en" | "pt";
export const LANG: Lang = "en";

type Copy = {
  // Hook (retention in opening seconds)
  hookProblem: string;
  // Cold open
  tagline: string;
  taglineSub: string;
  // Hotkey
  pressHint: string;
  // Recording — words that "emerge" as the person speaks (the hero voiceover)
  spokenWords: string[];
  // The dictated result that lands in the apps
  dictatedEditor: string;
  dictatedBrowser: string;
  dictatedChat: string;
  // Mock app labels
  appEditor: string;
  appBrowser: string;
  appChat: string;
  // "wherever the cursor is"
  anywhere: string;
  // Value props
  props: { title: string; sub: string }[];
  // State labels
  stateListening: string;
  stateThinking: string;
  stateDone: string;
  // CTA
  ctaLine: string;
  ctaRepo: string;
  ctaPlatforms: string;
};

const en: Copy = {
  hookProblem: "Still typing everything?",
  tagline: "Just talk.",
  taglineSub: "Voice-to-text, right where your cursor is.",
  pressHint: "Press to talk",
  spokenWords: [
    "Ship",
    "the",
    "release",
    "notes",
    "before",
    "the",
    "standup.",
  ],
  dictatedEditor: "Ship the release notes before the standup.",
  dictatedBrowser: "flights to lisbon in september",
  dictatedChat: "On it — pushing the fix now, tests are green. ✅",
  appEditor: "main.rs — wren",
  appBrowser: "Search",
  appChat: "#team",
  anywhere: "Wherever your cursor is.",
  // Order: hook-benefit (speed) → emotional (privacy) → brand (lightness).
  // Short headlines: each benefit gets the full screen, one at a time.
  props: [
    { title: "3× faster than typing", sub: "Speak a paragraph in the time you'd type a line." },
    { title: "Private by default", sub: "Your audio never leaves your machine." },
    { title: "Feather-light", sub: "So light you forget it's running." },
  ],
  stateListening: "Listening",
  stateThinking: "Transcribing",
  stateDone: "Done",
  ctaLine: "Free & open source. Download and start talking.",
  ctaRepo: "wren.rafaelvieiras.com",
  ctaPlatforms: "Linux today · macOS & Windows soon",
};

const pt: Copy = {
  hookProblem: "Ainda digitando tudo?",
  tagline: "É só falar.",
  taglineSub: "Voz em texto, exatamente onde o cursor está.",
  pressHint: "Aperte pra falar",
  spokenWords: ["Publicar", "as", "notas", "da", "release", "antes", "da", "daily."],
  dictatedEditor: "Publicar as notas da release antes da daily.",
  dictatedBrowser: "voos para lisboa em setembro",
  dictatedChat: "Pode deixar — subindo o fix agora, testes verdes. ✅",
  appEditor: "main.rs — wren",
  appBrowser: "Buscar",
  appChat: "#time",
  anywhere: "Onde o cursor estiver.",
  // Order: hook-benefit (speed) → emotional (privacy) → brand (lightness).
  // Short headlines: each benefit gets the full screen, one at a time.
  props: [
    { title: "3× mais rápido que digitar", sub: "Fale um parágrafo no tempo de digitar uma linha." },
    { title: "Privado por padrão", sub: "Seu áudio nunca sai da sua máquina." },
    { title: "Leve como pluma", sub: "Tão leve que você esquece que está rodando." },
  ],
  stateListening: "Ouvindo",
  stateThinking: "Transcrevendo",
  stateDone: "Pronto",
  ctaLine: "Grátis e open source. Baixe e comece a falar.",
  ctaRepo: "wren.rafaelvieiras.com",
  ctaPlatforms: "Linux hoje · macOS & Windows em breve",
};

const table: Record<Lang, Copy> = { en, pt };
export const t: Copy = table[LANG];
