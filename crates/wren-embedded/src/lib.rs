//! `wren-embedded` — embedded local transcription engine.
//!
//! Always built into the app (see `docs/decisions/0006-unified-single-app.md`);
//! the `inference` feature only exists to let `wren-embedded`'s own tests skip
//! compiling the ONNX runtime. See `docs/architecture/embedded-engine.md`.
//!
//! Inference runs in a **disposable subprocess** (see
//! `docs/decisions/0005-disposable-worker-subprocess.md`):
//! the [`EmbeddedTranscriber`] adapter (parent side) only spawns the worker and
//! does IPC; [`worker::run_if_worker`] (child side) is what links the engine and
//! loads the model, taking the ~1 GB peak with it when it dies.

pub mod models;
pub mod protocol;
pub mod transcriber;
pub mod worker;

pub use models::{
    catalog, delete_model, download_model, local_models, model_dir, DownloadProgress, ModelFile,
    ModelInfo,
};
pub use protocol::{WorkerResult, WORKER_SUBCOMMAND};
pub use transcriber::EmbeddedTranscriber;
pub use worker::run_if_worker;
