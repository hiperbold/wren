---
title: "Open Questions & Known Technical Debt"
description: "Product and technical decisions still pending, plus intentionally-accepted debt with a migration path."
---

# Open Questions & Known Technical Debt

This document tracks product decisions and technical trade-offs that have not yet been settled, as well as known suboptimal implementations that were accepted deliberately with a documented migration path. For decisions already made, see the [decision log](../decisions/README.md).

## Open Product Decisions

### License

The choice of open-source license has not been finalized. Candidates under consideration:

- **Permissive** (MIT, Apache-2.0): maximize adoption, no copyleft obligations
- **Copyleft** (GPL, AGPL): ensure derivative works remain open-source

Both approaches are viable given the project architecture. The choice will reflect the project's stance on derivative works and ecosystem contribution.

### Definitive Name

The current codename is **Wren** (a small songbird—lightweight, domestic, strong voice for its size). Before finalizing, verify:

- Package name collisions (npm, crates.io, PyPI)
- Domain and app store availability
- Trademark conflicts

Alternative names considered during early exploration: *Murmur*, *Lark*, *Cricket*. Renaming is still low-cost at this stage.

### Platform Scope Beyond Linux

macOS and Windows support timing is open. The architecture already isolates platform-specific behavior in adapters (see `crates/wren-adapters`), so this is an effort prioritization decision, not an architectural constraint. The question remains: when and in what order?

### Telemetry & Privacy Stance

**Default policy:** zero external telemetry. No usage data or transcription data leaves the machine. If any external telemetry is introduced in the future, it must be opt-in and explicit.

This applies only to *external* transmission. Local-only diagnostics—logging and performance telemetry that remain on the machine—are already implemented (available in the app's Settings › Diagnostics tab).

**Still open:** long-term retention policy for recordings and session history. Define criteria for automatic cleanup, export, and user control.

### Context-Aware Post-Processing Profiles

The design brief for context-aware post-processing (e.g., app- or window-specific correction profiles) is documented in [Design: Context-Aware Post-Processing](../design/context-aware-post-processing.md). Several implementation details remain open:

- **Title matching strategy:** exact substring match vs. regex patterns
- **Precedence rules:** when multiple profiles match, which takes priority?
- **Declarative vs. freeform:** where does hint-based configuration end and per-profile prompt engineering begin?
- **Browser URL strategy:** cost/platform-specific permission requirements for capturing active browser URLs
- **Wayland behavior:** unavailable on current Wayland, or invest in portal-based solutions?

## Known Technical Debt

### Auto-Update Implementation (Incomplete Plumbing)

The auto-update infrastructure is partially implemented but not yet functional.

**Current state:**
- Plumbing is in place: `tauri-plugin-updater` and `tauri-plugin-process` registered
- `check_for_updates` IPC command exists and responds gracefully (never panics)
- `createUpdaterArtifacts` is intentionally disabled in `tauri.conf.json`—local builds do not require signing keys

**Why accepted:** completing the full signing and release pipeline requires production infrastructure decisions (key management, CI/CD integration, release cadence) that were deferred to first release.

**Migration path (release-time):**

1. Generate the production key pair: `tauri signer generate`
2. Store the **private key** as a secret in CI/CD (e.g., GitHub Actions `TAURI_SIGNING_PRIVATE_KEY`)—never commit to the repository
3. Replace the placeholder `pubkey` in `tauri.conf.json` with the actual public key
4. Enable `bundle.createUpdaterArtifacts: true` in `tauri.conf.json`
5. Implement CI pipeline that:
   - Builds the application
   - Signs the update artifacts
   - Publishes `latest.json` and binaries to GitHub Releases

### Credential Storage: Plaintext API Keys (Technical Debt)

Provider API keys are currently stored in plaintext within `settings.json` (under `ProviderConfig.api_key`). This contradicts the security principle documented in [Provider Model](../architecture/provider-model.md), which calls for storing credentials in the OS keychain with only a reference in the settings file.

**Why accepted:** the first iteration prioritized dynamic provider discovery and CRUD over OS keychain integration. This does not block basic usage; it increases risk only in local-only, single-user contexts (typical for a desktop app).

**Migration path:**

1. Integrate the `keyring` crate for OS keychain access
2. On first load, detect plaintext API keys in `settings.json` and migrate them to the OS keychain
3. Replace the plaintext key with a keyring reference in `settings.json`
4. Update all provider authentication flows to fetch keys from the keychain
5. Reevaluate priority once multiple providers with stored credentials are in active use
