---
title: "Architecture Overview"
description: "Hexagonal (ports & adapters) architecture: layers, ports, and how providers stay decoupled from the core."
---

# Architecture Overview

The interfaces below are expressed in neutral pseudocode (not language-specific).

## Style: Clean / Hexagonal (Ports & Adapters)

Wren follows a **hexagonal architecture**. The single, non-negotiable rule:

> **Dependency rule:** the core (domain + use cases) **depends on nothing external**. Everything "outside the system"—cloud provider, microphone, operating system, database—implements a **port** defined by the core. Dependencies point **inward**, never outward.

Practical consequence: swapping Groq for OpenAI, or cloud for local, or X11 for Wayland, **does not touch a single line of the core**. They are different adapters plugged into the same ports.

```
            ┌─────────────────────────────────────────────┐
            │                  ADAPTERS                     │
            │  (outside world — depend on the core)        │
            │                                               │
   input    │   Global shortcut ──▶┌───────────────────┐   │
   (driving)│   UI / CLI ─────────▶│                   │   │
            │                      │       CORE         │   │
            │                      │  Domain + Use      │   │
            │   Microphone ◀───────│     Cases          │   │  output
            │   Transcriber ◀──────│  (knows nothing    │   │  (driven)
            │   PostProcessor ◀────│   about the       │   │
            │   TextSink ◀─────────│   outside world)  │   │
            │   History ◀──────────│                   │   │
            │   Settings ◀─────────└───────────────────┘   │
            └─────────────────────────────────────────────┘
```

## The three layers

### 1. Domain (the center, pure)

Entities and rules that do not depend on I/O:

- **`AudioClip`** — normalized audio (format, encoding, sample rate known) independent of how it was captured.
- **`Transcript`** — the result: text + (optional) segments/words with timestamps + metadata (detected language, confidence, latency, provider used).
- **`DictationSession`** — a dictation session from start (shortcut) to end (text delivered): states, cancellation, errors.
- **`Provider` / `ProviderConfig`** — the description of a provider (id, label, endpoint, credential, model, capabilities). See [the provider model](./provider-model.md).
- **`Profile` / `Settings`** — preferences: shortcut key, language, injection method, active provider.

### 2. Use cases (application, orchestrates the domain via ports)

The main one:

```
use_case PerformDictation:
    audio      = AudioSource.capture(until_stop_signal)          # port
    audio      = Vad.trim_silence(audio)                         # port (optional)
    transcript = Transcriber.transcribe(audio, options)          # port  ◀── the provider
    if post_processing_enabled:
        transcript.text = PostProcessor.refine(transcript, prompt)  # port
    TextSink.deliver(transcript.text)                            # port
    HistoryStore.save(transcript)                                # port
    Feedback.signal(completed)                                   # port
```

Notice: the use case **does not know** if `Transcriber` is Groq, OpenAI, or a local binary. That is the entire point.

### 3. Adapters (the outside, replaceable/discardable)

Each port gets one or more concrete implementations. They are pluggable and testable.

## The ports (contracts the core defines)

### Output ports (driven — the core calls out to the world)

| Port | Responsibility | Example adapters |
|---|---|---|
| **`Transcriber`** | audio → transcription. **The central port of the project.** | `RemoteApiTranscriber` (Groq/OpenAI), `LocalServerTranscriber` (localhost), `EmbeddedTranscriber` |
| **`AudioSource`** | capture audio from the microphone | platform-specific capture adapter |
| **`Vad`** *(optional)* | detect speech: gate + trim edges + compress internal pauses | earshot (pure Rust neural); Silero as optional alternative |
| **`TextPostProcessor`** *(optional)* | refine text via LLM | OpenAI-compatible adapter |
| **`TextSink`** | deliver text to the focused app | paste (Ctrl+V / Ctrl+Shift+V), type (dotool/xdotool/wtype), etc. |
| **`HistoryStore`** | persist history (transcriptions and failures) | local database |
| **`RecordingStore`** | preserve audio when transcription fails (safety net: dictation never lost; deleted after success/retry) | local WAV files |
| **`SettingsStore`** | read/write configuration | local file |
| **`Feedback`** | sound/overlay status | platform-specific adapter |

### Input ports (driving — the world calls the core)

| Port | Responsibility | Example adapters |
|---|---|---|
| **`SessionTrigger`** | start/stop/cancel a dictation | global shortcut, UI button, CLI |

## How the provider stays decoupled from the system

Three decisions guarantee the requested decoupling:

1. **The provider is an adapter of the `Transcriber` port**, not a branch of `if` statements inside the core. Adding a provider = adding an adapter file + one entry in the registry. **Zero changes to the domain.**
2. **Registry driven by configuration.** Providers are declared in data (endpoint + credential + model + capabilities), not scattered across code.
3. **Local and remote share the SAME port.** "Local" is not a special code path; it is just a `Transcriber` adapter whose endpoint points to the user's machine. This makes the embedded engine a natural extension, not a rewrite.

## The embedded local engine

There are **two** "local" paths, and they do not compete—they serve different audiences:

- **Third-party local server (already available):** a technical user already running an OpenAI-compatible server on `localhost` (faster-whisper, `whisper.cpp-server`, LocalAI…) simply points the `RemoteApiTranscriber` to it—**the same adapter as the cloud**, only changing the `endpoint`. Wren **does not build or serve** this server.
- **Embedded engine (the focus):** for the **non-technical user**, Wren embeds the engine in an `EmbeddedTranscriber` adapter that implements the `Transcriber` port **directly in the process** (no HTTP, no external process), using **existing bindings** from whisper.cpp/rs. The UX is: choose a model, **download with one click**, and dictate. Inference runs in a **disposable subprocess**—the runtime weight lives and dies there, so the engine can be **always embedded** without burdening cloud-only users—the app is a single unified build, not separate editions. See [the embedded-engine decision](../decisions/0003-embedded-engine-over-local-server.md).

In both cases, **the `Transcriber` port does not change**—it is "just another provider following the same format", chosen at runtime:

```
   Single app — provider chosen at runtime
  ┌───────────────┐   Transcriber contract   ┌────────────────────────────────┐
  │ Core          │ ──────────────────────▶ │ EmbeddedTranscriber → spawns   │
  │ Remote/Local/ │   (kind = Embedded)      │  worker subprocess (whisper.rs)│
  │ Embedded      │                          │  + models downloaded on demand │
  └───────────────┘                          └────────────────────────────────┘
        │ HTTP localhost
        └──────────────────────▶  third-party local server (optional)
```

## Technical principles (invariants)

1. **Dependency rule points inward.** Nothing in the core imports an adapter.
2. **Provider parity.** Cloud, local server, and embedded are the same type (`Transcriber`). None is a first-class citizen in the code.
3. **Core has no ML and no OS.** Inference runtime and system calls live only in adapters and external modules.
4. **Explicit egress.** Every `Transcriber` adapter declares whether it sends audio outside the machine; the UI uses this flag to warn the user.
5. **Testable without hardware or network.** The core is exercisable with fake adapters (fake microphone, fake transcriber)—no domain test needs a mic or internet.
6. **Platform boundaries isolated.** X11/Wayland/macOS/Windows appear only in adapters for `AudioSource`, `TextSink`, `SessionTrigger`, and `Feedback`.
7. **Graceful failure and cancelable.** A dictation session can be canceled at any time; a provider error does not crash the app, it becomes feedback.

## Directory structure (conceptual)

```
wren/
├─ crates/
│  ├─ wren-core/      # domain + use cases + PORT DEFINITIONS (no I/O)
│  │  ├─ domain/      # AudioClip, Transcript, DictationSession, Provider...
│  │  ├─ usecases/    # PerformDictation, ManageProviders...
│  │  └─ ports/       # Transcriber, AudioSource, TextSink, ... (interfaces)
│  ├─ wren-adapters/  # concrete implementations of the ports
│  │  ├─ transcriber/ # remote-api/, local-server/, embedded/
│  │  ├─ audio/       # capture per platform
│  │  ├─ textsink/    # paste/type per platform
│  │  ├─ trigger/     # global shortcut
│  │  └─ storage/     # history, settings
│  └─ wren-embedded/  # embedded engine worker subprocess (whisper.rs)
└─ apps/
   └─ desktop/        # composition (wiring), UI, lifecycle
```

**Note:** This diagram is illustrative of the separation of concerns and does not represent a literal file listing. Refer to [AGENTS.md](../../AGENTS.md) or [CONTRIBUTING.md](../../CONTRIBUTING.md) in the repository root for the actual current crate layout and build instructions.

See also [the desktop stack decision](../decisions/0001-desktop-stack.md) for technology choices, and [the embedded engine reference](./embedded-engine.md) for detailed IPC contracts and subprocess lifecycle.
