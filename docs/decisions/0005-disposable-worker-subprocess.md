---
title: "ADR 0005: Disposable Worker Subprocess for Memory Isolation"
description: "Why embedded-engine inference runs in a short-lived child process instead of in-process, based on a benchmarked comparison of memory behavior."
---

# ADR 0005: Disposable Worker Subprocess for Memory Isolation

## Status

Accepted — decided July 11, 2026, based on comparative benchmark.

## Context

The embedded offline transcription engine loads a large ONNX model (~1 GB RSS footprint during inference). The application runs continuously in the system tray, requiring a strategy to prevent unbounded memory accumulation in the resident process.

Two architectural approaches were evaluated via benchmark across five consecutive dictations on a Ryzen 5 3600:

1. **Design A:** Load model in-process → transcribe → explicitly drop model.
2. **Design B:** Spawn a short-lived worker subprocess, perform inference there, and exit.

## Decision

**Design B (disposable worker subprocess) was chosen.**

The resident Wren process maintains a stable 2.8 MB RSS footprint across all dictations. The ~1 GB model footprint lives and dies within ephemeral worker subprocesses; the operating system reclaims 100% of the memory at process teardown (~0 ms).

The design accepts a constant ~0.4 second latency cost per dictation (cold model load) to eliminate unbounded memory creep in the long-running process. Since this latency is incurred only during active transcription and does not impact idle state, it is architecturally acceptable.

## Alternatives Considered

### Design A: In-Process Load → Transcribe → Drop (Rejected)

**Benchmark results:**

- RSS did not return to baseline after model drop; the allocator retained approximately 700 MB.
- `malloc_trim(0)` reduced RSS to 46–119 MB but is glibc-specific and not portable across platforms.
- Residual memory creep of approximately +16 MB per consecutive dictation accumulated without being returned to the operating system.

**Verdict:** No portable in-process solution exists for unbounded session lifetime. glibc-specific workarounds are fragile and do not address the underlying allocator behavior.

### Design B: Disposable Worker Subprocess (Chosen)

**Benchmark results:**

- Resident Wren process: 2.8 MB constant across five consecutive dictations.
- Worker subprocess footprint (~1 GB): fully reclaimed by the operating system on process exit (≈0 ms recovery time).
- Pure spawn+IPC+teardown overhead: 2–5 ms (negligible relative to model load time).
- Dictation latency increase: ~0.4 seconds (cold model load); this cost is paid only during active transcription and has zero impact on idle state.

**Verdict:** Reliable, portable, and memory-safe for indefinite session lifetime.

## Consequences

### Per-State Memory Budget

The design establishes and anchors the per-state memory budget:

- **Idle state:** ~3 MB (resident Wren process only).
- **Transcribing state:** ~1 GB (worker subprocess) + ~3 MB (resident process).

This directly feeds into resource constraints and the disposable-webview lifecycle management (see [Resource Budget](../reference/resource-budget.md)).

### Latency Profile

Each dictation incurs a one-time cold model load (~1.6 seconds), which dominates transcription cost for both designs. For typical usage patterns (isolated dictations separated by idle periods), this is acceptable. Rapid-fire dictations in sequence receive no amortization benefit.

### Future Optimization: Keep-Warm Worker (Optional, Not Yet Implemented)

Model load time (~1.6 seconds) can be amortized across consecutive dictations by extending worker lifetime. A keep-warm strategy would allow the worker to survive 30–60 seconds post-dictation (with a timeout-based self-destruct), reducing subsequent transcription latency to ~450 ms. The worker would release the 1 GB footprint upon timeout expiry.

This refinement is architecturally compatible with the current design and may be implemented without structural changes.

## Implementation Notes

1. **Worker Binary:** The worker subprocess is implemented as the Wren binary itself, invoked with a hidden subcommand (`__wren-transcribe-worker`). No separate binary is required.

2. **IPC Protocol:** Audio is transmitted to the worker via stdin as raw PCM i16 samples (~0 ms latency for typical 161 KB audio). No intermediate file format (e.g., WAV) is written to disk.

3. **See Also:**
   - [Embedded Engine](../architecture/embedded-engine.md) — IPC protocol details and worker process lifecycle.
   - [Resource Budget](../reference/resource-budget.md) — per-state memory constraints and disposable-webview integration.
