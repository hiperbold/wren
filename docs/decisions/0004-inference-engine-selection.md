---
title: "ADR 0004: Inference Engine — transcribe-rs with Parakeet V3"
description: "Why Wren's embedded engine uses the transcribe-rs multi-engine framework with Parakeet V3 as the default CPU-optimized model, validated by a hands-on spike."
---

# ADR 0004: Inference Engine — transcribe-rs with Parakeet V3

## Status

Accepted — finalized 2026-07-11 after validation spike.

## Context

Wren's embedded offline transcription engine needed to select both an inference framework and a default model. Key constraints:

- Target audience: end users without GPUs (CPU-only environments)
- Input format: 16 kHz mono i16 (already Wren's canonical format, defined in `domain.rs`)
- Deployment: desktop app with minimal system dependencies
- Multi-language support: Portuguese (Brazilian) as primary; global support desirable

Existing single-model alternatives (e.g., whisper-rs for Whisper only) did not provide the flexibility needed for model curation and future expansion. The embedded engine required a framework that could support multiple model families and enable user choice without architectural refactoring.

## Decision

Wren adopts **`transcribe-rs` 0.3.11** as the inference framework and **Parakeet V3 (int8 quantization, from HuggingFace)** as the default model.

### Inference Framework: transcribe-rs

`transcribe-rs` is a multi-engine Rust framework (trait-based `SpeechModel`) authored by the creator of Handy. It provides:

- Support for ONNX-based models (Parakeet, Canary, Moonshine, etc.) and Whisper via whisper.cpp
- Optional hardware acceleration (CUDA, ROCm, Metal, Vulkan, DirectML, CoreML)
- MIT license
- Direct integration with Wren's `Transcriber` port via the `SpeechModel` trait

The framework allows `EmbeddedTranscriber` to be a thin wrapper over `SpeechModel`, preserving architectural simplicity and leaving room for future model selection without refactoring.

### Default Model: Parakeet V3

Parakeet V3 (int8 quantization, 640 MB) is selected as the default because:

- **CPU-optimized**: Achieves ~5× realtime performance on consumer hardware without GPU acceleration
- **Multilingual**: Supports Portuguese (Brazilian) with correct accent marks and cedilla handling
- **Deterministic**: Output is reproducible across runs
- **Validated in Handy**: The original multi-engine framework was refined in Handy with Parakeet V3 as a core optimization to avoid the CPU and thermal load that Whisper introduces

Whisper remains available as an option for users with GPU acceleration, but is not the default due to its CPU-heavy nature on CPU-only systems.

## Validation Spike

**Date:** 2026-07-11  
**Hardware:** Ryzen 5 3600 (6-core), no GPU acceleration  
**Setup:** transcribe-rs 0.3.11 (onnx feature) + Parakeet V3 int8 (`smcleod/parakeet-tdt-0.6b-v3-int8` from HuggingFace, 640 MB)

### Results

- **Binary portability:** Compiles without external system library dependencies. ONNX Runtime is statically linked, producing a self-contained 26 MB binary with no separate `.so` files to package or distribute.

- **Multilingual support:** Despite outdated crate documentation claiming English-only, Parakeet V3 is multilingual. Tested with Brazilian Portuguese audio; correctly transcribed accents, cedillas (ç), and diacritics with 100% accuracy.

- **Determinism:** Output is byte-for-byte identical across multiple runs on the same audio.

- **Performance:** Achieves ~11× realtime (463 ms elapsed time to transcribe 5.15 seconds of audio; model load time ~1.8 s single-threaded).

- **Memory pressure:** Peak RSS during transcription ~1 GB — conflicts with Wren's lightweight resource budget. See [Consequences](#consequences).

## Consequences

### Positive

- Eliminates dependency on external inference services for the embedded engine
- Enables offline-first transcription with predictable, minimal latency on consumer hardware
- Parakeet V3 multilingual support opens path to global localization without model switching

### Design Trade-offs

**Memory budget conflict:** The ~1 GB peak RSS during transcription conflicts with Wren's design principle of keeping idle state lightweight (disposable webview architecture).

**Mitigation strategy (ADR 0005):** The `EmbeddedTranscriber` adapter **must load the model on-demand and release it immediately after each session**. Two implementation patterns are viable:

1. Load model into a long-lived `Transcriber` instance within the session, drop when the session ends
2. Spawn a disposable subprocess for transcription (worker pattern), isolating the RSS peak to the subprocess lifecycle

This ensures idle RSS remains low and the main app remains responsive. See [ADR 0005: Disposable Worker Subprocess](./0005-disposable-worker-subprocess.md) for the architectural resolution.

### Operational Considerations

- Model download (640 MB) occurs on first use; requires integrity verification and progress reporting
- Dependency on ort 2.0.0-rc.12 (release candidate); version must be pinned in Cargo.lock
- Framework updates (transcribe-rs, ort) require regression testing of transcription quality and latency

## Implementation Notes

### Type Adaptation

- **Audio input:** `AudioClip` in Wren's core is `i16` (signed 16-bit integers). `transcribe-rs` expects `&[f32]` normalized to [-1, 1] range. Bridge: divide each sample by 32768.0.
- **Method signature:** `SpeechModel::transcribe_with(&mut self)` requires mutable access. To satisfy `Transcriber(&self) + Send + Sync` traits, wrap the model instance in a `Mutex<SpeechModel>`.

### Dependency Pinning

- Pin `ort = "2.0.0-rc.12"` exactly in Cargo.toml (release candidate; stable version may have breaking changes)

### Model Curation

The following decisions remain open for future issues:

- Which models to expose in the UI (curated set vs. all available)
- Download caching, integrity verification, and expiry policies
- Preferred model source (HuggingFace for official models, criteria for community models)

## Related Documentation

- [Embedded Engine Architecture](../architecture/embedded-engine.md) — resulting adapter and module design
- [ADR 0005: Disposable Worker Subprocess](./0005-disposable-worker-subprocess.md) — resolution to the RSS memory budget conflict
- [Resource Budget](../reference/resource-budget.md) — Wren's lightweight state design constraints
