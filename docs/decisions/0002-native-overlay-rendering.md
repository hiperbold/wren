---
title: "ADR 0002: Native wgpu Rendering for the Recording Overlay"
description: "Why the recording overlay bubble is rendered natively with wgpu instead of as a webview."
---

# ADR 0002: Native wgpu Rendering for the Recording Overlay

## Status

Accepted — decided 2026-07-02

## Context

Wren's core interaction is a floating "bubble" overlay that appears during dictation sessions, displaying a waveform animation as the user speaks. The initial implementation attempted to render this overlay as a webview component, which introduced significant memory overhead.

Resource profiling revealed that a webview-based overlay consumed approximately 400 MB of memory during recording sessions and retained 50–100 MB after the first session ended (due to lingering WebKitNetworkProcess and WebKit runtime in the main process). This behavior contradicted the app's design goal of maintaining a low memory footprint across session lifecycle states.

Cross-platform rendering without the webview constraint required a unified approach to handle platform-specific graphics APIs: Metal (macOS), Vulkan (Linux), and Direct3D 12 (Windows).

See [Resource Budget](../reference/resource-budget.md) for detailed memory measurements across states S1 and S3, and the full benchmark log.

## Decision

The recording overlay is rendered natively using **wgpu**, a Rust graphics abstraction that compiles to a single codebase targeting Metal, Vulkan, and Direct3D 12. The decision includes:

1. **wgpu as the unified rendering API**
   - Provides a single, portable interface across all three target platforms.
   - Handles transparent surfaces correctly (essential for the bubble's rounded design with alpha blending).
   - GPU context is created and destroyed with each session, avoiding persistent resource retention.

2. **Webview restricted to settings UI**
   - The webview adapter is retained only for the settings window, where rich HTML/CSS UI is valuable.
   - The overlay window is created via `tauri::window::WindowBuilder` (with the `unstable` feature), generating a native window without a webview component.
   - The builder uses the same cross-platform windowing layer (tao) already deployed in the app and exposes `raw-window-handle` for wgpu integration.

3. **Synthesized tone feedback as visual failure fallback**
   - Tone feedback (`ToneFeedback` adapter) was implemented as an always-on complement to visual feedback.
   - Synthesized tones (start, end, and error signals) are played via settings toggle `play_sounds`, providing audio-only notification if GPU initialization or rendering fails.
   - Sessions continue without interruption if visual feedback is unavailable; failures are logged.

4. **Structural integration**
   - The wgpu overlay is implemented as a new adapter of the `Feedback` port.
   - The webview adapter remains available as a fallback if GPU initialization fails.

## Consequences

**Positive:**
- Memory footprint reduced to 250 MB during recording (S1 state), eliminating the 400 MB webview overhead.
- Post-session memory (S3 state) is reduced to 87 PSS, with no resident processes retained.
- Single codebase compiles to all three platforms, reducing maintenance burden and platform-specific rendering bugs.
- Transparent alpha blending is natively supported; the window can be a true 72-pixel pill shape rather than the 200-pixel floor imposed by WebKitGTK constraints.
- Graceful degradation: if GPU fails at runtime, users hear tone feedback and can continue dictating; visual feedback is not critical to the core interaction.

**Negative:**
- Introduces a dependency on wgpu and its underlying graphics drivers (Metal, Vulkan, DX12).
- GPU initialization adds complexity to session startup; failures must be detected and logged.
- Requires platform-specific window creation code (via `tauri::window::WindowBuilder` and `raw-window-handle`).

## Alternatives Considered

**OpenGL (via glutin/glow)**
- Would work on all three platforms today.
- Rejected: Apple deprecated OpenGL on macOS in 2018. Maintaining platform-specific context creation quirks across drivers and OS versions adds unnecessary technical debt.

**CPU-based rasterization (softbuffer + tiny-skia)**
- Would be the lightest implementation in terms of dependency weight.
- Rejected: The softbuffer crate does not support the alpha channel (an open issue), making it impossible to render the bubble's transparent corners. A pure CPU fallback is not feasible without custom framebuffer compositing.

**Retain webview-based overlay everywhere**
- Simplifies the architecture: a single widget type across the app.
- Rejected: The 400 MB memory cost and 50–100 MB retention per session conflict with the app's resource budget and design philosophy of minimal overhead between dictation bursts.
