//! Smoke-test / diagnostics for the embedded transcription worker, without
//! bringing up Tauri. It is the SAME worker entry point the app uses
//! (`run_if_worker`), exposed in a small binary. Compile with the `inference`
//! feature:
//!
//! ```sh
//! cargo build -p wren-embedded --features inference --example worker_cli --release
//! ffmpeg -i audio.wav -f s16le -ar 16000 -ac 1 pipe:1 \
//!   | ./target/release/examples/worker_cli __wren-transcribe-worker <model_dir>
//! ```
//!
//! Reads raw PCM i16 mono 16 kHz from stdin and emits the `WorkerResult` JSON line.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match wren_embedded::run_if_worker(&args) {
        Some(code) => code,
        None => {
            eprintln!(
                "usage: worker_cli {} <model_dir>  (PCM i16 mono 16k LE via stdin)",
                wren_embedded::WORKER_SUBCOMMAND
            );
            ExitCode::FAILURE
        }
    }
}
