---
title: "Product Overview"
description: "What Wren is, who it's for, and what's in and out of scope."
---

# Product Overview

## The Problem

Dictation is faster than typing, but current tools force a false choice:

- **Local apps (e.g., Handy, MacWhisper):** private and offline, but **locked into local engines**. They refuse, on principle, to use an external transcription API — even when the cloud would be faster, cheaper, or more accurate for the user. They also carry heavy ML runtimes in the binary.
- **Cloud apps/services:** convenient, but **lock you into a single provider**, their pricing model, and send your audio wherever they decide, with no choice.

There is no option where **users decide where transcription comes from** — and can switch without changing apps.

## Value Proposition

**"Choose your speech engine. The app doesn't lock you to any one."**

Wren is a desktop dictation app where transcription is a **pluggable provider**:

- Start with an **external API** (cloud): fast to run, tiny app, no model download.
- Run a **local server** by pointing the same app to `localhost` when you want privacy or zero cost.
- In the future, use an **embedded engine** (pure offline) — without changing apps or relearning anything.

All through the **same provider configuration** (endpoint + credential + model).

## Target Audience

- **People who write a lot** (developers, creators, people with RSI or repetitive strain injury) and want to dictate into any system text field.
- **People who care about privacy and cost** and want the power to say "today I use the cloud, tomorrow I use local" without changing tools.
- **Self-hosting enthusiasts** who already have (or want) a transcription server on their local network.

## Basic User Flow

1. User presses a **global shortcut** (toggle or push-to-talk).
2. Wren **captures audio** from the microphone and shows discrete feedback (sound/overlay).
3. On release/stop, audio goes to the **chosen transcription provider** — or, with the **cancel shortcut** (default `Esc`, active only during recording), the session is discarded without transcription.
4. (Optional) text passes through an **LLM post-processor** (fix terminology, punctuation, format).
5. Text is **injected** into the focused app (paste or type).
6. The transcription stays in **local history**.

### Visual Feedback: The Bubble

During recording, Wren shows a **small bubble, centered in the lower part of the screen** (a floating "pill" style, no window border), with a **waveform animation** that reacts to microphone level in real time — the unmistakable signal of "I'm listening to you."

- Appears when the session starts and **disappears (is destroyed) at the end** — it doesn't stay resident (see the resource budget reference documentation).
- Visual states: *recording* (waveform reacting to voice), *transcribing* (waveform in neutral pulse), *error* (brief indication before disappearing).
- Never steals focus and never delays capture: audio starts before the bubble appears.

## MVP Scope (What Goes In First)

- ✅ Audio capture + global shortcut (toggle and push-to-talk).
- ✅ Transcription via **one or more external API providers** (compatible with OpenAI `/audio/transcriptions` standard, e.g., Groq Whisper, OpenAI).
- ✅ **Local HTTP server provider** (same contract, `base_url=localhost`) — included in MVP because **it costs the same code** as cloud.
- ✅ Text injection (paste/type) with accent and special character handling.
- ✅ **Configuration-driven provider registration** (endpoint, credential, model).
- ✅ Local history + persisted settings.
- ✅ Minimal configuration UI (choose provider, shortcut, language).

## Out of MVP Scope (But Planned in Architecture)

- ⏳ **Embedded transcription engine** (whisper.cpp / ONNX running in-process). Becomes a separate module later.
- ⏳ **LLM post-processing.** The port exists from day one, but implementation may come in a later refinement.
- ⏳ Real-time streaming (transcription while you speak).
- ⏳ Diarization, translation, voice commands.

## Non-Objectives (What Wren Does NOT Aim to Be)

- ❌ Not an audio editor or video captioning app.
- ❌ Doesn't try to be the world's fastest STT engine — it **orchestrates** engines, doesn't compete with them.
- ❌ Doesn't embed heavy ML in the base app. Offline is an **optional module**, not the default.
- ❌ Doesn't lock users into a provider, pricing model, or specific cloud.

## Product Principles

1. **User choice above all.** Switching providers is a configuration, not a reinstallation.
2. **Lightweight by default.** Minimal initial install; weight (models, runtimes) is opt-in.
3. **Honest privacy.** The UI makes clear, for each provider, *where audio goes*. Remote is never hidden as if it were local.
4. **Works where you write.** Any OS text field, no special integration needed.
5. **Good desktop citizen.** Low resource consumption at rest; no bloated daemon.

## One-Liner Differentiator

> Where Handy says "transcription is local and stays local," Wren says
> "transcription is a provider — and you choose."
