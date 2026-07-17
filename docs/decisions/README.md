---
title: "Decision Log"
description: "Architecture Decision Records (ADRs) — settled technical and product decisions, with context and consequences."
---

# Decision Log

This directory records Wren's Architecture Decision Records (ADRs): decisions
that have been made, with the context that led to them and their consequences.
Each ADR is numbered sequentially and never rewritten after acceptance — if a
decision is later reversed, a new ADR supersedes it and links back to the one
it replaces.

For decisions that have **not** been made yet, see
[Open Questions & Known Technical Debt](../reference/open-questions.md).

## Index

| ADR | Title | Status |
|---|---|---|
| [0001](./0001-desktop-stack.md) | Desktop Stack — Tauri v2 with Rust Core and React UI | Accepted |
| [0002](./0002-native-overlay-rendering.md) | Native wgpu Rendering for the Recording Overlay | Accepted |
| [0003](./0003-embedded-engine-over-local-server.md) | Embedded Transcription Engine, Not a Self-Hosted Server | Accepted |
| [0004](./0004-inference-engine-selection.md) | Inference Engine — transcribe-rs with Parakeet V3 | Accepted |
| [0005](./0005-disposable-worker-subprocess.md) | Disposable Worker Subprocess for Memory Isolation | Accepted |
| [0006](./0006-unified-single-app.md) | Single Unified App, No Separate Editions | Accepted |
