---
title: "Design: Context-Aware Post-Processing"
description: "Design brief for detecting the focused app/window at dictation time and applying user-defined per-destination formatting profiles. Not yet implemented — Phase 3 work."
---

# Design: Context-Aware Post-Processing

> **Design Brief for Future Work**
>
> This document describes a feature planned for Phase 3 of the [Roadmap](../roadmap.md). It is not yet implemented and should not be confused with current Wren behavior. It depends on the `TextPostProcessor` port already designed.

## Core Concept

At the instant the user activates the shortcut, Wren captures a **snapshot of the active window** (`{app, title, [url]}`). If — and only if — the user has explicitly created a **post-processing profile** matching that context, the transcribed text is reformatted according to the profile's rules before injection.

### User Intent Examples

- **Twitter/X context** → User dictates a post; the destination has a ~280-character limit, casual tone, no markdown.
- **WhatsApp Web context** → Informal, no markdown, emojis OK.
- **Obsidian context** → Markdown, line breaks, bullets, note formatting.

The user dictates naturally, and the destination/formatting rules emerge from context—without explicitly saying "for Twitter" aloud.

## The Non-Negotiable Principle

This is the core rule governing the entire feature:

- **Disabled by default.** Wren **never** captures or uses window context unless the user has explicitly created a profile.
- **Enabled only when the user creates a profile.** The existence of at least one active profile is what enables context capture. Without a profile, the capture code does not run.
- **User decides the match.** The user defines the pattern (app name and/or window title substring) that triggers each profile. Nothing is inferred silently.
- **Local, always.** Aligned with Wren's 100% local positioning for this data class—context **never** egresses. The only outbound data is the final text going to the LLM post-processing provider the user configures (which may be local/Ollama).

## Detectable Signals (Three Levels)

Three levels, with increasing cost and reliability:

1. **App/process name** — e.g., `chrome`, `Slack`, `obsidian`, `WhatsApp`.
   Inexpensive, runs once, available on nearly all platforms.
2. **Window title** — delivers most of the signal because titles reveal destination: `(3) WhatsApp`, `Post / X`, `my-notes.md - Obsidian`.
   **This is the primary signal in the design.**
3. **Exact browser URL** — difficult and fragile (requires Accessibility/UIA, permissions, and per-browser code). Useful for disambiguating (Gmail vs. Twitter in the same Chrome window), but treated as **future enhancement**, not a requirement.

### Platform Availability

| | App/Process | Title | Browser URL |
|---|---|---|---|
| **macOS** | `NSWorkspace.frontmostApplication` | Accessibility API (AXUIElement) | Accessibility/AppleScript — requests permission |
| **Windows** | `GetForegroundWindow` + `GetWindowThreadProcessId` | `GetWindowText` | UI Automation (UIA) |
| **Linux/X11** | `_NET_ACTIVE_WINDOW` + `WM_CLASS` | `_NET_WM_NAME` | via AT-SPI, fragile |
| **Linux/Wayland** | ⚠️ limited/unavailable | ⚠️ compositor-dependent | ⚠️ practically unavailable |

**Wayland is the weak point:** By design, there is no standard API for the active window. Workarounds exist in some compositors (`ext-foreign-toplevel`, portals), but are not guaranteed. The current development environment (Pop!_OS, X11/GNOME) works; the design must **degrade gracefully** when context is unavailable—in that case, simply do not apply a profile and inject text normally.

## Timing Challenge (Wren-Specific)

Wren displays an overlay/bubble and spins up a **disposable webview** (see [Architecture Overview](../architecture/overview.md)). If capture happens **after** this, the foreground window will be the Wren window itself—not Twitter.

→ Capture must happen **at the instant the shortcut fires, before any Wren UI appears**. The natural place is `wren::shortcut`: take the snapshot `{app, title, [url]}` and carry it along with the audio through the post-processing stage.

## Proposed Architecture (To Be Confirmed in Phase 3)

- **Snapshot at shortcut time.** Capture `WindowContext { app, title, url? }` in `wren::shortcut`, before the overlay. If capture fails (Wayland, permission denied), return `None` and continue without profile.
- **Profile model** (persisted in settings, editable in UI):
  - `name` — profile label.
  - `match` — matching rule: app pattern and/or substring/regex of title.
  - `rules` — formatting instructions injected as an **explicit hint** in the LLM prompt (character limit, tone, markdown yes/no, line breaks, etc.).
  - `provider` — which post-processing provider to use (may inherit the default).
- **Matching logic.** When transcription completes, find the first profile whose `match` clause matches the `WindowContext`. No match → no contextual post-processing.
- **Explicit hint injection, not inference.** The destination/rules appear **explicitly** in the prompt, rather than letting the model guess:

  ```
  Destination: Twitter/X (limit ~280 chars, casual tone, no markdown)
  Destination: Obsidian (markdown, line breaks, bullets OK)
  Destination: WhatsApp (informal, no markdown, emojis OK)
  ```

- **Reuse existing port.** Passes through the `TextPostProcessor` port already designed; the profile only decides **whether** and **with which rules** to invoke it.

## Privacy and Security

- **Window titles can contain sensitive content.** Store only the minimum needed to match the profile; **never** persist raw titles in telemetry, logs, or history.
- **Per-app opt-out / allowlist.** The user controls which apps are in scope; the default is "none" (nothing captured).
- **No context egress.** The `WindowContext` stays in the Rust process; nothing of it goes to the cloud. Only the final assembled text goes to the LLM provider the user chose.

## Suggested Phasing Within Phase 3

1. **Validation spike** — A command that captures `{app, title}` from the active window on shortcut activation and logs it to the Diagnostics pane, validating in the real environment before designing the UI. (Candidate crates: `active-win-pos-rs` or similar for levels 1+2.)
2. **Profile model + matching by app/title** (the heart of the feature).
3. **UI for profile creation/editing** (what enables context capture).
4. **Browser URL** as an optional enhancement, platform by platform.

## Open Decisions (To Revisit in Phase 3)

- Match by **simple substring** vs. **regex** in title — start simple.
- **Precedence rule** when multiple profiles match (order? specificity?).
- Where **declarative hint** ends and **free-form per-profile prompt** begins.
- **Browser URL strategy** and whether the cost/permissions per platform justify it.
- **Wayland behavior**: accept unavailability vs. invest in portals.
