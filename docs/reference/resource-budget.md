---
title: "Resource Budget"
description: "Testable memory and latency budgets per app state, plus the benchmark log that produced them."
---

# Resource Budget

Reference platform: desktop Linux x86_64 (X11), measured in RSS via `ps -o rss= -p <pid>` summing **all app processes** (Rust process + webview processes, when present).

## App states

Wren has different memory consumption per state — the budget is **per state**, not a single number:

| State | Description | Live processes |
|---|---|---|
| **S0 — Idle** | Tray only, no windows | Rust process only |
| **S1 — Recording** | Active dictation session, overlay (bubble) visible | Rust + overlay (native wgpu, no webview) |
| **S2 — UI open** | Settings or history window open | Rust + UI webview |
| **S3 — Post-session** | Returned to idle after recording or closing UI | Rust process only |

Note: The overlay is rendered natively with wgpu, not through a webview. This decision, and the measurements that justified it, are documented in the [native overlay rendering decision](../decisions/0002-native-overlay-rendering.md).

## Memory budget (RSS total)

| State | Target | Maximum (fail if exceeded) |
|---|---|---|
| **S0 — Idle** | ≤ 15 MB | **30 MB** |
| **S1 — Recording** | ≤ 80 MB | **120 MB** |
| **S2 — UI open** | ≤ 120 MB | **180 MB** |
| **S3 — Post-session** | ≤ S0 + 5 MB | **S0 + 15 MB** (else is a leak) |

Notes:

- **S3 is the anti-leak metric:** after 10 record→transcribe→paste cycles and opening/closing the UI 10 times, RSS should return to near the S0 baseline. If it does not, some window is being hidden instead of destroyed, or there is a leak in Rust.
- The overlay (S1) only exists during the session and is destroyed at the end of it.
- The embedded engine is **always built into the app**, but only consumes resources when used: inference runs in a **disposable subprocess**, so the **resident Wren process is not affected** — users on cloud providers pay zero for embedded runtime. The ~1 GB Parakeet V3 model lives and dies in the worker; the **resident app stays at ~2.8 MB** (measured, constant across dictations). The in-process variant was discarded (the allocator retains ~700 MB after `drop` and leaks +16 MB per dictation). Budget per state when using the embedded provider: **~1 GB only while transcribing, ~3 MB at idle.** (There is no separate "editing" mode — see the embedded engine reference documentation.) See the worker subprocess decision document.

## Other budgets

| Metric | Target | Maximum |
|---|---|---|
| CPU at S0 (idle) | ~0% (average over 60 s) | 0.5% |
| Shortcut → capture start latency | ≤ 50 ms | 150 ms |
| Shortcut → overlay visible latency | ≤ 300 ms | 700 ms (never delays capture) |
| Linux installer (.deb, base app) | ≤ 15 MB | 30 MB |
| Cold start (login → tray ready) | ≤ 1 s | 3 s |

**Latency invariant:** audio capture begins on the shortcut event in the Rust process, **before and independently** of the overlay appearing. The overlay is asynchronous feedback; it never blocks the audio capture path.

## Measurement procedure

Reference script (to be created as `scripts/measure.sh`):

1. Start the app, wait for tray, measure S0 (RSS summed across all app PIDs).
2. Trigger a dictation session (fake/local provider), measure S1 at peak.
3. Open the UI, measure S2; close it, wait 5 s, measure S3.
4. Repeat the cycle 10 times and compare final S3 against S0.
5. Fail (exit ≠ 0) if any maximum is exceeded.

Until the script exists, manual measurement follows the same steps using `ps -o rss=,comm= -p $(pgrep -f wren)`.

## When to revisit the budgets

- When porting to macOS or Windows (webviews have different memory profiles — define tables per platform).
- When using the embedded provider: the **worker subprocess** has its own budget (~1 GB during transcription), but the **resident process continues** to count against this table — that is the point of the disposable subprocess.

## Benchmark log

The following measurements justified the [native overlay rendering decision](../decisions/0002-native-overlay-rendering.md). They are presented as historical records showing why a native wgpu overlay replaced the original webview-based bubble.

### First production measurement (2026-07-02, X11/Pop!_OS, **release** build)

| State | RSS measured | PSS measured | Verdict |
|---|---|---|---|
| S0 (fresh, no window) | 55 MB | **24 MB** | RSS above target; PSS acceptable |
| S1 (recording + bubble) | ~409 MB (with webkit) | — | far above |
| S3 (post-session) | 178 MB + 52 MB (WebKitNetworkProcess) | 78 MB | does not return to S0 |

Release binary: 9.5 MB ✓. Functional flow validated (bubble, waveform, transparency, error state, window self-destruction).

**Interpretation:**

- S0 RSS is inflated by shared libraries (GTK is loaded because of the tray). The actual private cost (PSS 24 MB) is acceptable; the budget table should move to **PSS** on Linux, keeping RSS as reference.
- **WebKitWebProcess (renderer) dies with the window** ✓ — "destroy, not hide" works. But **WebKitNetworkProcess survives** (52 MB, belongs to WebContext) and the main process retains ~54 MB private WebKit runtime after the first webview.
- **Architecture consequence:** the bubble as a webview is expensive (S1 ~400 MB; S3 never returns to baseline). Strong candidate: **native overlay** (GTK window drawn in Rust, no webview) to keep the dictation path 100% WebKit-free — the webview would remain only for the settings window, which is rare.
- **Debug builds do not count for the budget** (RSS ~3–5× larger); always measure in release.

### Native wgpu overlay prototype (2026-07-02, release, X11/NVIDIA GTX 1050 Ti)

Direct comparison with webview overlay (same machine, same day):

| State | Webview (WebKit) | Native wgpu (NVIDIA) | Native wgpu (llvmpipe, FINAL) |
|---|---|---|---|
| S0 idle | 55 MB RSS / 24 PSS | 65 MB RSS / 27 PSS | 57 MB RSS / 25 PSS |
| S1 recording | ~409 MB RSS (multi-process) | 250 RSS / 143 PSS | **223 RSS / 127 PSS, zero children** |
| S3 post-session | 230 MB RSS (~130 PSS + NetworkProcess) | 183 RSS / 87 PSS | **187 RSS / 99 PSS, zero children** |
| S3 after 3 cycles | — | 85 PSS (stable) | — |
| CPU during S1 | — | — | ~13% of one core (30 fps; profiling future work) |
| Transparency | true alpha | X Shape (hard edges) | **true alpha + AA** |
| Binary | 9.5 MB | 13.6 MB (+wgpu) | 13.6 MB |

Prototype technical notes:

- **Hardware Vulkan on X11 does not expose composition alpha** (only `Opaque`), but **llvmpipe (Vulkan via software, Mesa) exposes `PreMultiplied`** — true alpha, with anti-aliasing, no hacks. The selection order became: *true alpha beats hardware* (for 320×200 the CPU cost of llvmpipe is negligible, and avoids NVIDIA driver memory retention). **X Shape** remains as a fallback when only opaque backends are available; it is removed when the backend supports alpha.
- The wgpu GL backend found no compatible adapter on NVIDIA/X11 — no GL fallback for now.
- The ~60 MB retained in S3 above S0 (NVIDIA measurement) is from the **NVIDIA driver** (in-process retention after dropping the device). With llvmpipe the expected retention is lower — see numbers above.
- `request_device` can fail in the field; the fallback chain ends in a software renderer and, if everything fails, the window is destroyed immediately (no "ghost") and the session continues without visual feedback.
- tao opens (and leaks) an X11 connection on each `display_handle()`; the adapter captures handles once per session. Follow-up: cache per process or reuse the window/GPU across sessions if the file descriptor rate becomes a problem.
