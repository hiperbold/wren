---
title: "Case Study: Handy"
description: "Research on Handy, the local-only dictation app that motivated Wren's pluggable-provider design."
---

# Case Study: Handy

Source: analysis of repository `github.com/cjpais/Handy` + local installation.

## What is Handy

Open-source (MIT) voice dictation app for desktop by CJ Pais. **Tauri v2** (Rust backend + React/TypeScript UI), cross-platform (Linux/macOS/Windows). Codebase of ~20–24k lines of Rust + ~13–16k TypeScript. Linux installer ~126 MB (due to embedded ML runtimes).

### How it works (overview)

| Layer | Implementation in Handy |
|---|---|
| Audio capture | `cpal` + `rodio` (resampling via `rubato`) |
| VAD | Silero via `vad-rs` |
| **STT (transcription)** | **100% local**, 2 engines: `transcribe-cpp` (Whisper GGML/GGUF, GPU) and `transcribe-rs` (**ONNX**: Parakeet, Moonshine, SenseVoice, etc.) |
| Global shortcut / PTT | `handy-keys` (evdev) and `rdev` |
| Text injection | `enigo` + `dotool`/`xdotool`/`wtype`; configurable `PasteMethod` enum |
| Post-processing | **Optional external LLM**, OpenAI-compatible, OFF by default |

### The detail that defines everything

Handy is **local-only for transcription**: audio **never** goes to an external API. The only network output is optional **text post-processing** by LLM, with a well-architected provider abstraction:

- Built-in providers: OpenAI, Anthropic, Groq, OpenRouter, Cerebras, Z.AI, Bedrock, and **Custom** (editable `base_url` → points to Ollama on `localhost:11434`).
- OpenAI-compatible client (`POST {base_url}/chat/completions`), with model discovery and structured output.

**In other words: Handy already has the "external provider skeleton"—just applied to text, not audio.** Wren takes exactly this pattern and applies it to transcription.

## The rejected PRs (the gap Wren fills)

The community **tried multiple times** to add external API transcription and/or improve backends—and the maintainer declined, for two distinct reasons:

### (a) Rejection by "local-only" philosophy (external transcription)

- **PR #886** — "OpenAI-compatible local server transcription".
  Rejected: *"This is a local only transcription app and it will stay that way... local transcription is good enough, and local LLM is not good enough."*
- **PR #1131** — "cloud transcription via Groq Whisper API". Closed in ~18 minutes, no comment.
- **PR #1241** — "optional ElevenLabs transcription provider". Closed without comment.

> **This is Wren's central gap:** external API transcription (cloud or local) that Handy rejects on principle.

### (b) Rejection because the author is rebuilding the backend (ONNX exiting the picture)

The owner closed backend PRs stating he was building his own engine behind the scenes (`transcribe.cpp`, based on **ggml**, with a plan to **deprecate ONNX**):

- **PR #1298** (GPU acceleration for **ONNX** models on Linux) — *"This is not going to get pulled in mainly because we're going to be deprecating Onnx in the future."*  ← the PR that most aligns with "I'm working on something with ONNX".
- **PR #957** (new Qwen3-ASR engine) — *"it will come in as part of a separate set of changes I am working on behind the scenes."*
- **PR #985** (ONNX + DirectML on Windows) — *"Closing because I will be submitting a PR for this and pulling it in."*

### Current status of Handy (July 2026)

- **ONNX:** still in use today (via `transcribe-rs`/`ort`), but in **exit regime**—the author is migrating to `transcribe.cpp` (ggml, Metal/Vulkan).
- **External transcription API:** **rejected as policy**. Only remote LLM for post-processing.

## What Wren reuses (conceptually) and what differs

### Reuse (ideas, not a fork—see license note)

- ✅ **The OpenAI-compatible provider pattern** (`base_url` + credential + model + editable `custom`). Wren extends this to **transcription**.
- ✅ The idea of configurable `PasteMethod` (resolves accents/ç—a known pain point on X11).
- ✅ Choices of capture/shortcut/VAD as reference (`cpal`, evdev, Silero)—to be decided if identical.

### Do differently

- 🔄 **Transcription is a pluggable provider**, not local-only. Local and cloud = same interface.
- 🔄 **Light core by default.** No ML runtime in the base app; the local engine is a **separate module** (Handy embeds both engines → ~126 MB).
- 🔄 **Explicit hexagonal architecture** from the start, so provider decoupling is structural, not incidental.

## License note / stance

Handy is **MIT**, so copying code would be legally permitted—**but the decision is not to fork and to think something new instead**. Therefore:

- Wren uses Handy as a **reference for study and product decisions**, not as a code base.
- Where a third-party library (independent of Handy) solves a problem well (capture, shortcut, injection), it can be adopted directly—that is using the ecosystem, not forking Handy.
- Wren's own license is treated as an [open question](../reference/open-questions.md).

## Sources

- `github.com/cjpais/Handy` (README, `Cargo.toml`, `managers/transcription.rs`, `llm_client.rs`, `settings.rs`, `catalog/`).
- Cited PRs: #886, #957, #985, #1131, #1241, #1298 (via GitHub public API).
- Local installation: `~/.local/share/com.pais.handy/` (models `parakeet-tdt-0.6b-v3-int8` and `whisper-medium-q4_1.bin`; `settings_store.json` with `ort_accelerator`, `post_process_providers`, etc.).
