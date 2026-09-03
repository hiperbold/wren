# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-08-27

### Added

- First-run setup wizard (onboarding): engine choice, shortcut, and a quick
  dictation test, re-runnable anytime from Settings → System.
- Advanced settings section with a self-learning GPU backend probe: the
  native recording overlay samples over its first ~6 sessions whether trying
  the GL graphics backend on cold start is actually useful on this machine,
  then skips the wasted attempt once it isn't (saves ~300ms per session on
  affected setups). Self-heals — if skipping GL ever causes the overlay to
  fail to open, it resets the learned state and retries with GL included.
  Cross-platform safe by construction: GL is only ever attempted when the
  primary backend doesn't already provide real transparency, so the learning
  never activates on platforms where it isn't needed.
- Release notes (and the landing page's Windows download CTA) now flag that
  Wren isn't code-signed yet, so Windows SmartScreen / macOS Gatekeeper may
  warn the app is "unrecognized" — with the one-click workaround.

### Fixed

- Native overlay: the recording bubble could visibly stutter under GNOME's
  compositor, which throttles window redraws to 33Hz regardless of the app's
  own frame pacing; switched to a present mode that isn't subject to that
  throttle on Linux.
- Native overlay: the pill-shaped window mask could desync from what was
  actually rendered after a resize, briefly showing a stale half-pill shape.
- Native overlay: a previous session's overlay window could leak (stay
  resident) if a new recording started before the old one finished closing.
- Text delivery (paste): if a synthetic key press failed partway through
  (X11 Unicode input is transient-failure-prone), a held Ctrl/Shift modifier
  could get stuck at the X server level, affecting keyboard input outside
  Wren until the session ended. Modifier releases now always run, regardless
  of whether the preceding press succeeded.
- Logging: a per-target log level cap was being ignored, letting some targets
  log more verbosely than the configured level allowed.

## [0.1.0] - 2026-08-21

### Added

- Automated Linux release builds via GitHub Actions: pushing a `v*` tag builds
  `.deb` and `.AppImage` installers with `tauri-apps/tauri-action` and attaches
  them to a draft GitHub Release. Closes #1.
- Internationalization (i18n) with `react-i18next`: English is the default UI
  language, with a Portuguese (`pt-BR`) locale included. UI strings are
  organized into per-view namespaces.
- Open-source project scaffolding: `LICENSE` (MIT), `CONTRIBUTING.md`,
  `CODE_OF_CONDUCT.md`, `SECURITY.md`, `AGENTS.md`, issue/PR templates.

### Changed

- The entire codebase (comments, log/error messages, docs) was translated from
  Portuguese to English ahead of the open-source release. Golden STT test
  fixtures remain in PT-BR by design (they are paired with Portuguese audio).
