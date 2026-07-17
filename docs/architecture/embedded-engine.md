---
title: "Embedded Engine"
description: "How Wren's offline embedded transcription engine works: the disposable worker subprocess, the IPC protocol, model management, and the public module contracts."
---

# Embedded Engine

## Overview

Wren's embedded transcription engine enables offline dictation without requiring a remote API or separate local server. It implements the core [`Transcriber` port](./provider-model.md) directly within a disposable worker subprocess, using the transcribe-rs bindings to whisper.cpp with Parakeet V3 models.

For end users, the experience is straightforward: select a model, download it with one click, and dictate offline. From an architecture perspective, the embedded engine is one of several `ProviderKind` options available at runtime—no separate app edition, no extra flags. Because inference runs in a [disposable worker subprocess](../decisions/0005-disposable-worker-subprocess.md) that exits after each transcription, the resident process avoids the memory cost of the inference runtime, making it practical to embed always-on without burdening cloud-only users.

This design is enabled by the [embedded-engine decision](../decisions/0003-embedded-engine-over-local-server.md) (choosing an embedded adapter over a self-hosted server pattern) and the [inference engine selection](../decisions/0004-inference-engine-selection.md) (transcribe-rs and Parakeet V3).

## Module Layout

The embedded engine lives in the `crates/wren-embedded` crate:

```
crates/wren-embedded/
├─ Cargo.toml
└─ src/
   ├─ lib.rs         # Module declarations and public API re-exports
   ├─ protocol.rs    # IPC message format (parent ↔ worker)
   ├─ worker.rs      # Worker subprocess: links transcribe-rs, loads model, transcribes
   ├─ transcriber.rs # EmbeddedTranscriber adapter: implements Transcriber port via subprocess
   └─ models.rs      # Model catalog, download, caching, and lifecycle
```

## Parent/Worker Subprocess Architecture

The parent process (the main Wren app) and worker process are both instances of the same binary, differentiated by a hidden subcommand marker.

**Parent process** (the adapter, `transcriber.rs`):
- Spawns a worker subprocess via `current_exe` with a hidden subcommand flag
- Does NOT link transcribe-rs or the ONNX runtime
- Sends raw PCM audio via subprocess stdin
- Parses JSON result from subprocess stdout
- Maps subprocess errors to recoverable `PortError`

**Worker process** (`worker.rs`):
- Launched early in `main()`, before Tauri initialization
- Links transcribe-rs and the ONNX inference library
- Loads the model from disk
- Processes PCM samples from stdin
- Writes JSON result to stdout and exits

Because the worker is the Wren binary itself invoked with a subcommand, there is no separate binary to package. The inference runtime exists only within the worker process's memory; once the worker exits, its memory is reclaimed.

## IPC Protocol

### Invocation

The parent spawns the worker as follows:

```bash
<wren_binary_path> __wren-transcribe-worker <MODEL_DIR>
```

The string `__wren-transcribe-worker` is a constant subcommand marker (`WORKER_SUBCOMMAND`). The `<MODEL_DIR>` is the absolute path to the cached model directory.

### Input

The parent writes raw PCM audio samples to the worker's stdin:

- Format: signed 16-bit integer (i16), mono, little-endian
- Sample rate: 16 kHz
- No headers or framing—samples flow continuously until EOF
- The parent closes stdin to signal end-of-audio

Wren's internal `AudioClip` format matches this specification exactly.

### Output: Success

On successful transcription, the worker writes exactly one JSON line to stdout and exits with code 0:

```json
{"text": "...", "language": null, "load_ms": 123, "infer_ms": 456}
```

This corresponds to the `WorkerResult` struct in `protocol.rs`. Fields:
- `text`: The recognized speech (always present)
- `language`: Detected language code, if available (may be `null`)
- `load_ms`: Time to load the model (milliseconds)
- `infer_ms`: Time to run inference (milliseconds)

### Output: Error

On error, the worker writes a message to stderr and exits with a non-zero code. The adapter (`transcriber.rs`) maps this to `PortError::Other`.

## Public API Contracts

### `worker.rs` — Subprocess Entry Point

```rust
/// If args indicate the worker subcommand, execute the worker and return the exit code.
/// Called early in the app's main(), before any Tauri initialization.
pub fn run_if_worker(args: &[String]) -> Option<std::process::ExitCode>;
```

When `run_if_worker` returns `Some(code)`, the main process exits immediately with that code; the Tauri app never starts.

### `transcriber.rs` — Adapter (Parent Side)

```rust
pub struct EmbeddedTranscriber { /* model_dir, worker_exe */ }

impl EmbeddedTranscriber {
    pub fn new(model_dir: std::path::PathBuf) -> Self;
}

impl wren_core::Transcriber for EmbeddedTranscriber {
    // Spawns worker subprocess, sends audio via stdin, parses result.
}
```

The adapter implements the core `Transcriber` port. Calling `transcribe()` on it spawns a worker, sends audio, waits for the result, and returns it as a `TranscriptionResult`.

### `models.rs` — Model Catalog and Lifecycle

```rust
pub struct ModelFile {
    pub name: String,
    pub url: String,
    pub sha256: Option<String>,
}

pub struct ModelInfo {
    pub id: String,
    pub label: String,
    pub language: String,
    pub size_bytes: u64,
    pub files: Vec<ModelFile>,
}

pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

/// Returns the static curated catalog of available models.
pub fn catalog() -> Vec<ModelInfo>;

/// Returns the directory where a (downloaded) model resides within the cache.
pub fn model_dir(cache_root: &std::path::Path, id: &str) -> std::path::PathBuf;

/// Returns IDs of models already present and verified in the cache.
pub fn local_models(cache_root: &std::path::Path) -> Vec<String>;

/// Downloads all files for a model to the cache, reporting aggregated progress.
/// Verifies size and SHA256 (when available). Idempotent.
pub fn download_model(
    cache_root: &std::path::Path,
    id: &str,
    progress: &dyn Fn(DownloadProgress),
) -> Result<(), wren_core::PortError>;

pub fn delete_model(cache_root: &std::path::Path, id: &str) -> Result<(), wren_core::PortError>;
```

## Model Catalog and Management

### MVP Catalog

The initial model catalog contains a single curated model:

| ID | Source | Files | Size | Language |
|---|---|---|---|---|
| `parakeet-v3-int8` | HuggingFace (`smcleod/parakeet-tdt-0.6b-v3-int8/main`) | `config.json`, `vocab.txt`, `nemo128.onnx`, `decoder_joint-model.int8.onnx`, `encoder-model.int8.onnx` | ~640 MB | English/Portuguese |

### Download and Caching

When a user downloads a model, all files are fetched from their `url` to `<cache_root>/<id>/`. The download function:

- Verifies file size
- Verifies SHA256 checksum (when provided in the catalog)
- Is idempotent: re-downloading an already-cached model re-verifies but does not re-fetch
- Emits progress callbacks for UI display

### Lifecycle

Users can:
- List available models via `catalog()`
- List already-downloaded models via `local_models(cache_root)`
- Download a model via `download_model(cache_root, id, progress)`
- Delete a cached model via `delete_model(cache_root, id)`

## Integration with `ProviderConfig`

The core `ProviderConfig` struct gains a `kind` field of type `ProviderKind`:

```rust
pub enum ProviderKind {
    RemoteApi,  // Cloud API (default, for backward compatibility)
    Embedded,   // Offline embedded engine
}
```

For a provider of kind `Embedded`:
- `base_url`: empty string (unused)
- `api_key`: `None` (unused)
- `model`: the model ID (e.g., `"parakeet-v3-int8"`)
- `sends_audio_externally`: `false` (audio stays local)

The `kind` field defaults to `RemoteApi` for backward compatibility with existing configuration.

## Tauri Commands

The Tauri backend (`apps/desktop/src-tauri`) exposes these commands for model management:

- **`embedded_catalog() -> Vec<ModelInfoDTO>`**  
  Returns the list of available models, including id, label, language, and size in bytes.

- **`embedded_local_models() -> Vec<String>`**  
  Returns the list of model IDs that are currently downloaded and cached.

- **`embedded_download_model(id: String)` (async)**  
  Downloads the specified model. Emits progress events via the `embedded://download-progress` event channel with payload:
  ```json
  {"id": "...", "downloaded": 123456, "total": 789012, "done": false, "error": null}
  ```
  On completion or error, `done` is set to `true`.

- **`embedded_delete_model(id: String)`**  
  Deletes the specified model from the cache.

- **Model selection** uses the existing `save_settings` command: the UI creates or activates a provider with `kind: Embedded`, sets its `model` field to the desired model ID, and makes it the `active_provider_id`.

## Build and Cache Locations

### Build

The embedded engine is **always built and included**, with no separate editions or feature flags for users:

```bash
npm run app          # tauri dev
npm run build        # production build
```

Internally, the `wren-embedded` crate is a normal dependency of the app with `features = ["inference"]` enabled. The inference feature causes the crate to link transcribe-rs and the ONNX runtime (`ort-sys`).

- **First build:** takes ~1 minute (ort-sys downloads and compiles the pre-built ONNX runtime)
- **Subsequent builds:** use cached ONNX runtime
- **Debug builds:** inference is slow; use `--release` to measure real latency

### Model Cache Location

Models are stored in:

- **Linux:** `~/.local/share/wren/models/<id>/`
- **macOS:** `~/Library/Application Support/wren/models/<id>/`
- **Windows:** `%APPDATA%\wren\models\<id>\`

These paths correspond to the platform's standard data directories as returned by `directories` or similar APIs.

### Testing Models Without Compilation

To run model tests without compiling the ONNX runtime:

```bash
cargo test -p wren-embedded
```

When running tests in isolation (the `-p` flag), the crate's `inference` feature is not activated, avoiding the expensive compilation step. This is useful for testing catalog logic, download idempotence, and cache integrity without pulling in the inference library.

## Testing

### Transcriber Contract

The `transcriber.rs` and `worker.rs` modules are tested via the core `Transcriber` contract suite:

- Valid audio → correct text
- Cancellation (via epoch counter) → graceful interruption
- Worker errors → recoverable `PortError`

End-to-end tests with real models run only in the integration suite. Unit tests use a mock worker to validate the IPC contract without expensive inference.

### Models Catalog and Lifecycle

Tests in `models.rs` verify:

- Catalog is well-formed and static
- `download_model` is idempotent
- Integrity verification rejects corrupted files
- `local_models` reflects the current cache state
- Download progress callbacks are emitted correctly

### Integration Tests

Full app integration tests:

- Single app build (no editions)
- Real end-to-end offline dictation
- Memory measurement: resident process RSS should return to baseline after transcription (validating that the worker subprocess is truly disposable)

## Resource Budget

As detailed in the [Resource Budget](../reference/resource-budget.md), the memory cost of the embedded engine is managed through the disposable worker design. The resident process never loads the ONNX inference runtime (the worker does); after each transcription, the worker exits and its memory is freed. The only persistent cost to the app is ~25 MB for the ONNX runtime binary in the app bundle, which is acceptable.

## Future Work

The following items are deferred beyond the initial MVP:

- **Expanded model catalog:** Additional models (Whisper/GPU, smaller parameter variants) can be added to the catalog and hosted by the same HuggingFace mechanism.
- **Keep-warm with idle timeout:** Refinement to reduce latency on repeated transcriptions (currently each transcription is cold-started).
- **Configurable cache location:** Allow users to override the default model cache path.
