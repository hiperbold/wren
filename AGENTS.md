# AGENTS.md

Guidance for AI coding agents (and humans who like precise instructions) working
in this repository. This is the machine-readable companion to
[CONTRIBUTING.md](CONTRIBUTING.md); when they overlap, the commands here are
canonical.

## Project layout

Rust workspace + a Tauri v2 desktop app.

```
crates/
  wren-core        Domain model, ports (traits), use-cases. No I/O.
  wren-adapters    Adapters: audio (cpal), VAD, remote transcriber, paste sink,
                   storage, telemetry, logging.
  wren-embedded    Offline inference: worker process, model mgmt, transcriber.
  wren-bench       CLI benchmark for transcription quality/latency.
apps/desktop/
  src/             React + TypeScript settings UI.
  src-tauri/       Tauri backend (crate `wren-desktop`): tray, global shortcut,
                   native wgpu recording overlay, IPC commands.
tests/fixtures/    Golden STT samples (PT-BR audio + reference transcripts).
```

## Build / run / test

```bash
# Rust (from repo root)
cargo build
cargo test
cargo clippy --all-targets --all-features
cargo fmt

# Desktop UI (from apps/desktop)
npm install
npm run app          # tauri dev — full native app
npm run dev:mock     # UI only, mocked backend, http://localhost:5173
npm run build        # tsc --noEmit + vite build
npx tsc --noEmit     # typecheck only
```

Run a single Rust test: `cargo test -p <crate> <test_name>`.

## Language & i18n rules

- **All code, comments, and identifiers are in English.** No Portuguese in
  source (the golden test fixtures under `tests/fixtures/` are the deliberate
  exception — they are PT-BR speech data paired with audio; never translate
  them).
- **User-facing UI strings go through i18n.** Never hardcode display text in a
  component. Use `react-i18next`:
  ```tsx
  const { t } = useTranslation("audio");
  <SectionHeader title={t("title")} />
  ```
- i18n lives in `apps/desktop/src/i18n/`. There is **one namespace per view/area**
  (`common`, `nav`, `shortcuts`, `provider`, `models`, `audio`, `system`,
  `history`, `diagnostics`). When you add a key, add it to **both**
  `locales/en/<ns>.json` and `locales/pt-BR/<ns>.json` with identical key sets.
  English is the source language; keys are flat with dot notation
  (`"activation.hint.toggle"`); interpolate with `{{var}}`.
- Shared human-readable labels (pipeline stages, session outcomes, language
  names, key names) resolve through helpers in `apps/desktop/src/lib/format.ts`,
  which read from the `common` namespace. Reuse those helpers instead of
  re-mapping labels in a view.

## Do-not-touch zones

Behavioral hot spots — change only with explicit intent, never as a drive-by:

- **`apps/desktop/src-tauri/src/overlay_native.rs`** — the native wgpu recording
  bubble. Do not alter rendering logic or numeric constants; translate comments
  only.
- **Epoch-based cancellation** — Escape aborts mid-transcription via an epoch
  counter. Do not refactor the guarded state resumption in the shortcut/session
  handling.
- **Per-state memory budget** — the disposable-webview lifecycle keeps idle RSS
  low. Don't make windows/webviews resident when idle.
- **IPC contracts** — Tauri `#[tauri::command]` names, `emit`/`listen` event
  channels (e.g. `embedded://download-progress`), and serde field/rename values
  are shared with the TypeScript side. Renaming any of them breaks the app.

## Conventions

- **Commits:** Conventional Commits, in English —
  `feat(ui): ...`, `fix(audio): ...`, `refactor(core): ...`, `chore: ...`,
  `docs: ...`.
- **Rust:** `//!` for module/crate docs, `///` for items; keep `cargo clippy`
  and `cargo fmt` clean.
- **TypeScript:** prefer self-documenting code + types; comment the *why*. Match
  the surrounding file's style.
- Before opening a PR, run the build/test/lint commands above and make sure the
  UI still switches cleanly between the `en` and `pt-BR` locales.

## Windows fork (hiperbold)

This clone is the `hiperbold/wren` fork, branch `windows-build`, kept only to
build the Windows installer via GitHub Actions
(`.github/workflows/windows-build.yml`, NSIS bundle, with a retry for the
bundler's transient download timeouts). There is no Rust/MSVC toolchain on
this machine — always build through Actions. `main` tracks the upstream
(`rafaelvieiras/wren`) untouched to keep future syncs trivial.

The full dictation system this app plugs into (local Whisper server on port
9736 + LLM revision + settings reference) lives in the private repo
`hiperbold/wren-whisper-hiperbold` (local folder `F:\Whisper`) — read its
AGENTS.md first. Installers downloaded from CI are kept in
`release-windows/`, outside git.
