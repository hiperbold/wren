---
title: "Provider Model"
description: "The Transcriber contract and provider configuration model — how cloud, local-server, and embedded transcription stay interchangeable."
---

# Provider Model

## Central Idea

> **"Local is just a provider with a different endpoint."**

All transcription — whether it comes from the cloud, a server on `localhost`, or an embedded engine — flows through **a single port**: `Transcriber`. Switching the source means switching **configuration**, not code.

This idea is inspired by what Handy already does **well** — but only for *post-processing* text (OpenAI-compatible providers with `base_url` + key + model). Wren applies the **same pattern to transcription itself**, which is precisely where Handy refuses to go. See the [Handy research](../research/handy-case-study.md).

## The Transcriber Contract

```
port Transcriber:

    # The essential operation: audio in, transcription out.
    transcribe(audio: AudioClip, options: TranscriptionOptions) -> Result<Transcript, TranscriberError>

    # Metadata for the UI and core to decide what to offer.
    capabilities() -> Capabilities

    # Stable identity of the provider (for config, logs, history).
    id() -> ProviderId
```

### Contract Types

```
AudioClip:
    samples             # PCM (or file/stream reference) — normalized format
    sample_rate
    channels
    duration

TranscriptionOptions:
    language?           # e.g., "pt" — or null for auto-detection
    prompt?             # hint/glossary/terms (when provider supports it)
    model?              # overrides provider's default model
    translate_to_en?    # bool

Transcript:
    text
    segments?           # list of {start, end, text} when available
    words?              # word-level timestamps when available
    detected_language?
    confidence?
    meta                # provider_id, model, latency_ms, estimated_cost?

Capabilities:
    languages           # supported list (or "auto")
    streaming           # bool
    word_timestamps     # bool
    max_duration?       # provider's limit
    sends_audio_externally  # bool  ◀── privacy/egress flag
    requires_credential # bool

TranscriberError:
    kind                # network | authentication | rate_limit | invalid_audio | unavailable | cancelled | unknown
    message
    recoverable         # bool (guides retry/fallback)
```

> The **`sends_audio_externally`** flag is mandatory and exists to alert the user in the UI. It is the cornerstone of the "explicit egress" principle.

## Configuration Model (Data-Driven Registry)

Providers are declared as **data**, not scattered across code. A provider is:

```
ProviderConfig:
    id                  # "groq", "openai", "local-whisper", "my-server"...
    label               # friendly name in UI
    kind                # remote_api | local_server | embedded
    endpoint            # base_url (e.g., https://api.groq.com/openai/v1
                        #            or http://localhost:8080/v1)
    authentication      # none | api_key | bearer | custom-header
    credential_ref?     # reference to safely stored secret (NOT plaintext)
    default_model
    api_format          # openai_audio | custom_v1 | ...  (which adapter to use)
    capabilities_override?  # optional, when provider doesn't expose discovery
```

### Factory Presets (Initial Suggestion, User-Editable)

| id | kind | endpoint | notes |
|---|---|---|---|
| `groq` | remote_api | `https://api.groq.com/openai/v1` | Whisper large-v3-turbo, cheap/fast |
| `openai` | remote_api | `https://api.openai.com/v1` | Whisper / gpt-4o-transcribe |
| `local-server` | local_server | `http://localhost:PORT/v1` | whisper.cpp-server, faster-whisper, etc. |
| `custom` | remote_api | *(user-editable)* | any OpenAI-compatible endpoint |

> The presets are just a starting point. Users can edit `endpoint`/`model` and create their own providers.

**Note:** The "embedded" provider kind (`ProviderKind::Embedded`) has since been added alongside `remote_api`. When `kind = Embedded`, `endpoint` is empty, there is no API key, and `model` holds the ID of a locally downloaded model. See the embedded engine reference documentation.

## How Each Kind Implements the SAME Port

```
                         port Transcriber
                               │
      ┌────────────────────────┼─────────────────────────────┐
      ▼                        ▼                              ▼
 RemoteApiTranscriber   LocalServerTranscriber        EmbeddedTranscriber
 (cloud)                (third-party localhost,       (in-process engine,
  - HTTP to endpoint     same adapter as cloud)        no network)
  - sends_audio_externally=YES  - HTTP to localhost      - sends_audio_externally=NO
  - requires key         - sends_audio_externally=NO    - whisper.cpp / ONNX
                         - usually no key               - via local-engine module
```

Key point: **`RemoteApiTranscriber` and `LocalServerTranscriber` can be the same adapter** (`api_format = openai_audio`), differing only by `endpoint` and `authentication`. This is why a local server **fits in the MVP for free**.

## Discovery, Selection, and Fallback

- **Selection:** the user chooses an active provider per profile. The core knows only "the active `Transcriber`".
- **Fallback (optional, later phase):** declarative policy like "try local; if `unavailable` and the error is `recoverable`, fall back to cloud". Since every provider is the same port, fallback is an orchestration decision, not a tangle of special cases.
- **Model discovery:** when the provider exposes listing (e.g., `/models`), an optional `list_models()` method in the adapter feeds the UI. Without it, use `capabilities_override`.

## Credential Security (Principle, Not Yet Full Implementation)

- Keys **never** in plaintext in the settings file. Store via OS keyring / dedicated secret vault; the `ProviderConfig` carries only a **reference** (`credential_ref`).
- Keys **never** go into logs, LLM prompts, telemetry, or queries.
- Embedded/local-server provider without credentials is the lowest-exposure path — the UI should expose this as a privacy advantage.

> **Current Status:** this is a stated principle, not yet fully implemented. The current code stores the API key in plaintext in `settings.json`. This is tracked as known technical debt; see the [open questions](../reference/open-questions.md) reference.

## Test Contract (To Ensure Parity)

Every `Transcriber` adapter, regardless of kind, must pass the **same contract suite**:

1. Valid short audio → `Transcript` with non-empty text.
2. Silent/empty audio → empty text **without** fatal error.
3. Cancellation mid-flight → `TranscriberError{kind: cancelled}`.
4. Network failure/unavailability → `recoverable` error correctly marked.
5. `capabilities()` coherent with actual behavior (e.g., if it declares `word_timestamps`, it delivers them).

This ensures that switching providers is safe and predictable — the goal of decoupling.
