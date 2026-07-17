---
title: "Roadmap"
description: "Phased build order — from a vertical slice through the embedded offline engine to future refinements."
---

# Roadmap

No calendar estimates — only order and completion criteria.

## Phase overview

```
Phase 0  Foundation      →  core + ports + 1 working cloud provider
Phase 1  Product MVP     →  usable dictation in daily workflow (shortcut, paste, history, UI)
Phase 2  Embedded engine →  offline EmbeddedTranscriber adapter + one-click model downloads
Phase 3  Refinements     →  LLM post-processing, context awareness, fallback, streaming
```

The unifying principle: **each phase adds adapters; the core domain never changes.**

### Historical note on Phase 2/3 merger

The original roadmap had separate "Phase 2 — local server middleware" (a standalone process exposing HTTP on `localhost`) and "Phase 3 — embedded engine." These were **merged into a single embedded-engine phase**. Reasons:

1. **Pointing to a third-party local server** (Ollama, faster-whisper, `whisper.cpp-server`) **already works since Phase 1** — it uses the same cloud adapter with `endpoint = localhost`, nearly for free. No separate phase needed.
2. The product goal is **offline turnkey for non-technical users** — choose a model, click download, transcribe — and that requires the engine **inside the app**, **not** a separate HTTP server that Wren would have to build, package, and version.

See [the embedded-engine decision](./decisions/0003-embedded-engine-over-local-server.md) for full rationale.

---

## Phase 0 — Foundation (vertical slice)

**Goal:** prove the architecture end-to-end with a thin slice.

- Define the domain (`AudioClip`, `Transcript`, `DictationSession`) and **ports** (traits).
- Implement the `PerformDictation` use case.
- One adapter: `RemoteApiTranscriber` (OpenAI-compatible cloud provider).
- Minimal adapters: audio capture, text injection, global shortcut.
- **No polished UI** — CLI or keyboard-driven.

**Status:** Completed. A user presses the shortcut, speaks, and text appears in the focused app via external API.

---

## Phase 1 — Product MVP

**Goal:** something usable in daily workflow.

- **Configuration-driven provider registry** (presets + custom endpoints).
- **Support for third-party local servers** — because the cloud adapter and `localhost` adapter are the **same code** (only the `endpoint` differs), pointing Wren to an OpenAI-compatible server the user already runs (faster-whisper, `whisper.cpp-server`, LocalAI, etc.) **comes for free here**. This is the offline path for technical users; the embedded engine (Phase 2) is the offline path for non-technical users.
- Toggle **and** push-to-talk; cancellation.
- Robust text injection (paste/type) with correct accents and Portuguese characters. ✅ Delivered: `PasteMethod` = paste (Ctrl+V, default), Ctrl+Shift+V (terminals), type (enigo) and wtype (Wayland); plus `restore_clipboard` setting (restore the previous clipboard after pasting, opt-in).
- Local history + persisted settings.
- Minimal configuration UI (active provider, hotkey, language, microphone).
- Discrete feedback (sound/overlay). Sound: ✅ Delivered (synthesized tones for start/end/error via `ToneFeedback`, `play_sounds` toggle).
- Local observability (diagnostics). ✅ Delivered: **centralized logging** (rotating file + searchable ring buffer) and **performance telemetry** (time per stage + RSS peak, `Telemetry` port), both in **Settings › Diagnostics**. 100% local — never leaves the machine.
- Desktop Linux packaging. ✅ Delivered: `deb`/`appimage` bundles; **autostart** ✅ (`tauri-plugin-autostart`, `launch_at_login` setting); **auto-update** with plumbing ready (plugin + `check_for_updates` command, GitHub Releases endpoint configured, placeholder public key) — infrastructure for release signing/publishing and production key still needed.

**Status:** Shipped. Users can switch between ≥2 cloud providers **or** a local server by configuration alone, and use the app in production.

---

## Phase 2 — Embedded local engine (pure offline, opt-in)

**Goal:** 100% offline transcription for **non-technical users** — choose a model, **download with one click**, load into memory, and dictate.

### Delivered

- **`EmbeddedTranscriber` adapter** implementing the `Transcriber` port. Inference engine: **`transcribe-rs`** (multi-engine framework), with **Parakeet V3** as the default CPU-optimized choice for non-technical users and **Whisper** available for GPU users. See [the inference engine decision](./decisions/0004-inference-engine-selection.md).
- **Per-subprocess memory isolation** (proven by benchmarking): inference does not run in the resident process — an **ephemeral worker** (the Wren binary itself with a hidden subcommand) loads the model, transcribes, and exits, taking the ~1 GB peak with it. No HTTP, no persistent process. The resident app stays at ~2.8 MB. Optional future refinement: keep-warm with idle timeout to amortize the ~1.6 s load cost across consecutive dictations. See [the worker subprocess decision](./decisions/0005-disposable-worker-subprocess.md).
- **Curated model catalog** (small, labeled by size/quality/language): download → store on disk → **load into memory** → select. One click, in user-friendly language.
- Download management (progress, integrity verification, local cache, removal) — **outside the core**; the domain knows nothing about model files.
- **Single app binary:** the engine is **always embedded** and becomes **just another provider** (remote / local server / embedded) chosen at runtime — switching is a config change, not a reinstall. No separate "offline edition": because inference runs in a disposable subprocess, cloud users **pay zero runtime cost** (the resident process never loads the ONNX runtime); the only cost is ~25 MB in the binary.
- Privacy indicator: `sends_audio_outside_machine = false` and no credentials — the UI highlights this as the privacy advantage it is.

**Status:** Implemented. A non-technical user opens the app, picks a model from the list, clicks download, selects the embedded provider, and dictates **without network and without external processes** — switching to/from cloud happens in settings.

**Out of scope:** Wren does **not** serve models via HTTP or run its own local server. Users who want that arrangement already point Wren to an OpenAI-compatible server on `localhost` (Phase 1).

**Still open:** model catalog curation (which models to offer beyond the default), download/cache policy, and expanding the model source.

---

## Phase 3 — Refinements

**Status:** Planned; not yet started.

- **LLM post-processing** (`TextPostProcessor` port already designed): fix terms, punctuation, formatting; OpenAI-compatible providers, including local LLMs (Ollama).
- **Context-aware post-processing profiles** — detect the focused app/window when the shortcut fires and apply formatting rules the user has created per destination (Twitter, WhatsApp, Obsidian, etc.). **Explicit opt-in**, disabled by default, 100% local. See [context-aware post-processing](./design/context-aware-post-processing.md).
- **Declarative fallback** across providers (local → cloud, etc.).
- **Streaming** (real-time transcription) where the provider supports it.
- Glossary/`custom_words`, correction dictionary, sound themes.

---

## What NOT to do early (pitfalls)

- ❌ Embed ML runtime in Phase 0/1 "just to test local" — it kills lightness and couples the core. To exercise the local path without the weight, **point to a third-party local server** (faster-whisper, etc.); the embedded engine has its own phase and isolated module.
- ❌ Optimize for a specific provider before the port is stable.
- ❌ Build rich UI before the end-to-end flow works.
- ❌ Support every platform in the MVP — focus on the primary platform; isolate the rest in adapters.
