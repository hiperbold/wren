//! IPC protocol between the app (parent) and the transcription worker (child
//! subprocess). See `docs/architecture/embedded-engine.md` §IPC Protocol.
//!
//! - Invocation: `current_exe __wren-transcribe-worker <MODEL_DIR>`.
//! - Input: **raw PCM i16 mono 16 kHz little-endian** samples on stdin, until EOF.
//! - Output (success): one JSON [`WorkerResult`] line on stdout, exit code 0.
//! - Output (error): message on stderr, exit code ≠ 0.

use serde::{Deserialize, Serialize};

/// Marker for the worker's hidden subcommand (it is `args[1]`; `args[2]` is the MODEL_DIR).
pub const WORKER_SUBCOMMAND: &str = "__wren-transcribe-worker";

/// The JSON line the worker emits on stdout when it completes successfully.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    pub text: String,
    pub language: Option<String>,
    pub load_ms: u64,
    pub infer_ms: u64,
}
