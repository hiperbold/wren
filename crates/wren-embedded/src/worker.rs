//! The transcription worker (**child** side): runs in the disposable subprocess,
//! loads the model and transcribes. It is the only place that links transcribe-rs
//! (under the `inference` feature). See `docs/architecture/embedded-engine.md`.
//!
//! TRACK A1 implements `run_worker` (the inference pipeline) without changing the
//! public signature of `run_if_worker`.

use std::process::ExitCode;

use crate::protocol::WORKER_SUBCOMMAND;

/// If `args` (typically `std::env::args().collect()`) indicate the worker
/// subcommand, runs the transcription and returns the exit code. The app calls
/// this **very early in `main`**, before bringing up Tauri. Returns `None` when
/// it is not a worker invocation (the app continues its normal flow).
pub fn run_if_worker(args: &[String]) -> Option<ExitCode> {
    if args.get(1).map(String::as_str) != Some(WORKER_SUBCOMMAND) {
        return None;
    }
    #[cfg(feature = "inference")]
    {
        let model_dir = args.get(2).cloned().unwrap_or_default();
        Some(run_worker(&model_dir))
    }
    #[cfg(not(feature = "inference"))]
    {
        eprintln!("wren: the transcription worker requires a build with the `inference` feature");
        Some(ExitCode::FAILURE)
    }
}

/// The worker's inference pipeline (offline edition only).
///
/// Output contract (docs/10 §3.1): **stdout receives exactly one JSON line**
/// [`WorkerResult`] on success — nothing else. All diagnostics go to **stderr**,
/// so as not to contaminate the channel the parent deserializes.
#[cfg(feature = "inference")]
fn run_worker(model_dir: &str) -> ExitCode {
    match transcribe(model_dir) {
        Ok(result) => match serde_json::to_string(&result) {
            // The ONLY write to stdout: the result's JSON line.
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("wren-worker: failed to serialize the result: {err}");
                ExitCode::FAILURE
            }
        },
        Err(err) => {
            eprintln!("wren-worker: {err}");
            ExitCode::FAILURE
        }
    }
}

/// The actual work: reads PCM from stdin, loads Parakeet and transcribes.
/// Errors become `String` (the caller sends them to stderr). Working reference:
/// the spike in `.claude/worktrees/agent-aacf901872234d785/spike/src/bin/worker.rs`.
#[cfg(feature = "inference")]
fn transcribe(model_dir: &str) -> Result<crate::protocol::WorkerResult, String> {
    use std::io::Read;
    use std::path::PathBuf;
    use std::time::Instant;

    use transcribe_rs::onnx::parakeet::{ParakeetModel, ParakeetParams};
    use transcribe_rs::onnx::Quantization;

    if model_dir.is_empty() {
        return Err("MODEL_DIR missing in the worker invocation".to_string());
    }

    // 1. Read ALL of stdin (raw PCM i16 mono 16 kHz LE, until EOF) and normalize
    //    to f32 in [-1,1]. `chunks_exact(2)` drops a residual odd byte — the
    //    protocol guarantees pairs (i16), so there is no loss for well-formed audio.
    let mut raw = Vec::new();
    std::io::stdin()
        .read_to_end(&mut raw)
        .map_err(|e| format!("failed to read the audio from stdin: {e}"))?;
    let samples: Vec<f32> = raw
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
        .collect();

    // 2. Load Parakeet (int8) — measure the load cost (it dominates the RSS peak).
    let load_start = Instant::now();
    let mut model = ParakeetModel::load(&PathBuf::from(model_dir), &Quantization::Int8)
        .map_err(|e| format!("failed to load the model at {model_dir:?}: {e}"))?;
    let load_ms = load_start.elapsed().as_millis() as u64;

    // 3. Transcribe (`transcribe_with` requires `&mut`). Parakeet (transcribe-rs 0.3)
    //    returns only text + segments — there is no detected language, so `language`
    //    stays `None` (the parent does not depend on it to build the Transcript).
    let infer_start = Instant::now();
    let out = model
        .transcribe_with(&samples, &ParakeetParams::default())
        .map_err(|e| format!("inference failed: {e}"))?;
    let infer_ms = infer_start.elapsed().as_millis() as u64;

    Ok(crate::protocol::WorkerResult {
        text: out.text.trim().to_string(),
        language: None,
        load_ms,
        infer_ms,
    })
}
