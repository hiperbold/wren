---
title: "ADR 0003: Embedded Transcription Engine, Not a Self-Hosted Server"
description: "Why offline transcription is delivered as an embedded in-process adapter instead of a separately built and packaged local HTTP server."
---

# ADR 0003: Embedded Transcription Engine, Not a Self-Hosted Server

## Status

Accepted — decided 2026-07-11

## Context

Phase 2 of Wren required offline transcription capability without relying on cloud APIs. Two approaches were under consideration:

1. Build and package a separate local HTTP server as part of Wren (middleware server model)
2. Embed the transcription engine directly in-process as an adapter

Both approaches aimed to provide the lay user with a seamless experience: model selection, one-click downloads, and dictation without external processes.

The key tension was between delivering a polished user experience and avoiding unnecessary operational complexity (building, packaging, versioning a standalone HTTP server) that the core product objective did not demand.

## Decision

Wren will deliver offline transcription through an **embedded in-process adapter**, not a self-hosted HTTP server.

The `EmbeddedTranscriber` adapter implements the `Transcriber` port directly within the Wren process. Model runtime weight is isolated in a [disposable worker subprocess](./0005-disposable-worker-subprocess.md), keeping the main application lightweight while enabling full offline capability.

Users select a model from the UI, download it with a single click, and begin dictation without invoking any external processes or protocols.

### No Local HTTP Server

Wren does not build, package, or version its own local HTTP server. Users who prefer an external server architecture already have a solution: they can point Wren to any OpenAI-compatible third-party server (Ollama, faster-whisper) running on localhost. This arrangement works with Wren's cloud adapter unchanged since Phase 1, requiring no additional code.

## Consequences

- **Simplified deployment:** Wren ships as a single, unified application. Users do not manage a separate server process or HTTP endpoints.
- **Unified adapter design:** All transcription backends—cloud, third-party local server, and embedded engine—flow through a single `Transcriber` port, reducing architectural surface area.
- **Memory isolation:** The embedded engine's runtime (model buffers, inference state) remains confined to a worker subprocess. The main application lifecycle remains independent, implementing Wren's [per-state memory budget](../architecture/embedded-engine.md).
- **User experience:** Lay users get the simplest interaction: choose, download, dictate. No HTTP debugging, no server health checks, no protocol incompatibilities.
- **Out of scope:** Wren will not serve models via HTTP or act as a local transcription server for other applications.

## Alternatives Considered

### A. Self-Hosted HTTP Server

Wren would build and package a dedicated local HTTP server, allowing users to run transcription via HTTP on localhost.

**Rejected because:**
- Introduces operational complexity (server build, packaging, versioning) not required by the core product objective.
- Duplicates functionality already available from third-party servers (Ollama, faster-whisper).
- Users who want this arrangement can already point Wren to any OpenAI-compatible server without new code—the cloud adapter handles it.
- Adds HTTP protocol negotiation and endpoint specification to the user interface.
- Requires managing separate process lifecycle (startup, shutdown, health checks).

### B. Embedded In-Process Adapter (Chosen)

Transcription engine runs as an isolated worker subprocess, accessed via the `Transcriber` trait, integrated directly into the Wren application.

**Chosen because:**
- Delivers the desired lay-user experience: no external processes, no HTTP.
- Simplifies Wren's architecture—all backends (cloud, local server, embedded) implement the same port.
- Isolates runtime weight to the worker subprocess without affecting the main application.
- Requires no server protocol decisions or HTTP endpoint management.

## Related Decisions

- **[ADR 0005: Disposable Worker Subprocess](./0005-disposable-worker-subprocess.md)** — How embedded engine memory is isolated and lifecycle managed.
- **[ADR 0004: Inference Engine Selection](./0004-inference-engine-selection.md)** — Which inference backend powers the embedded transcriber.
- **[Embedded Engine Architecture](../architecture/embedded-engine.md)** — Implementation details and runtime constraints.
