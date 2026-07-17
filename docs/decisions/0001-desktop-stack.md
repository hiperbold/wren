---
title: "ADR 0001: Desktop Stack — Tauri v2 with Rust Core and React UI"
description: "Why Wren is built on Tauri v2 with the dictation flow living entirely in the Rust process and disposable webviews for UI."
---

# ADR 0001: Desktop Stack — Tauri v2 with Rust Core and React UI

## Status

Accepted — decided 2026-07-02

## Context

Wren is a desktop dictation application that must function as a good citizen of the desktop environment, maintaining low resource usage during idle time. The core dictation flow requires handling complex operations: global keyboard shortcuts, audio capture, speech-to-text transcription, and text injection across multiple platforms (Linux, macOS, Windows) and input contexts.

The platform must support challenging scenarios such as text injection with accents on X11/Wayland, event capture via evdev, and integration with online/offline speech recognition backends. These operations are computationally sensitive and must not be blocked by UI concerns.

[Resource metrics are testable requirements](../reference/resource-budget.md) for the application.

## Decision

Wren is built on **Tauri v2** with the following architecture:

1. **Rust core and adapters.** All dictation logic—global shortcuts, audio capture, transcription orchestration, and text injection—lives in the Rust backend process. This layer has no dependency on the webview and operates independently.

2. **React/TypeScript UI.** User-facing settings and configuration are built with React and TypeScript in a disposable webview. The UI is an optional client to the core; the app functions entirely without it.

3. **Disposable webviews.** UI windows (settings, overlay) are created on demand and destroyed when closed. Windows are never hidden and held in memory. At idle, only the Rust process and system tray remain resident.

4. **Testable resource budget.** Resource metrics (idle memory, transcription latency, cold-start penalty) are measured and enforced as part of the project's acceptance criteria.

## Consequences

**Positive:**
- Low idle memory footprint—only the Rust process and tray resident when UI is closed.
- Dictation pipeline is not blocked by UI initialization or webview lifecycle.
- Rust ecosystem provides battle-tested libraries for core operations (`cpal` for audio, `rdev` for input events, `enigo` for text injection, `whisper.cpp` bindings for offline inference).
- Clear separation of concerns: core logic is testable and UI-independent.

**Negative:**
- Opening the UI incurs a cold-start penalty of ~0.5 seconds (webview creation overhead). This is accepted because the UI is opened infrequently during normal use.
- Tauri's declarative window model requires careful lifecycle management to maintain the disposable webview contract.
- Desktop platform specifics (X11 vs. Wayland, event injection, global shortcuts) remain complex, though Rust's ecosystem is mature for these concerns.

## Alternatives Considered

**Electron**
- Rejected: ~200–300 MB resident memory in idle (embedded Chromium violates the "good desktop citizen" principle).

**Wails (Go backend)**
- Rejected: While idle memory is acceptable, the Go ecosystem is weak in areas critical to Wren:
  - Global keyboard shortcuts and event capture (evdev integration).
  - Text injection with accents on X11/Wayland.
  - Speech recognition integration and model management.
  - Rust's ecosystem (cpal, rdev, enigo, whisper.cpp) is mature and production-proven by Handy, a reference application in the same class.

**Pure native UI (platform-specific)**
- Rejected: The disposable webview design already achieves low idle memory. A pure native UI would triple the code maintenance burden without proportional benefit.

**Handy precedent**
- Handy (an audio transcription app built with Tauri v2 and React) demonstrates the viability and maturity of this stack for real-time audio and text-injection workflows. Its success informed this decision.
