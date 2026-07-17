//! [`EmbeddedTranscriber`] — adapter for the `Transcriber` port (**parent** side).
//!
//! Runs inference in a **disposable subprocess** (see
//! `docs/decisions/0005-disposable-worker-subprocess.md` and
//! `docs/architecture/embedded-engine.md`):
//! spawns `current_exe __wren-transcribe-worker <model_dir>`, writes the samples to
//! stdin and reads the result from stdout. Does NOT link transcribe-rs — it only
//! spawns. The ~1 GB peak lives and dies in the worker, keeping Wren's resident
//! process light.
//!
//! TRACK A1 implements `transcribe` (the IPC) without changing the public signature.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use wren_core::{
    partial_transcript, AudioClip, PortError, Transcriber, Transcript, TranscriptionOptions,
};

use crate::protocol::{WorkerResult, WORKER_SUBCOMMAND};

/// `provider_id` recorded in the `Transcript` of embedded transcriptions —
/// distinguishes it from the remote provider id in history/telemetry.
const PROVIDER_ID: &str = "embedded";

pub struct EmbeddedTranscriber {
    /// Directory of the downloaded model (see [`crate::models::model_dir`]).
    model_dir: PathBuf,
    /// The worker is the app's own executable, reinvoked with the hidden subcommand.
    worker_exe: PathBuf,
}

impl EmbeddedTranscriber {
    pub fn new(model_dir: PathBuf) -> Self {
        let worker_exe =
            std::env::current_exe().unwrap_or_else(|_| PathBuf::from("wren-desktop"));
        EmbeddedTranscriber { model_dir, worker_exe }
    }

    /// Accessors for TRACK A1 (avoids an unused-field warning in the skeleton).
    pub fn model_dir(&self) -> &PathBuf {
        &self.model_dir
    }
    pub fn worker_exe(&self) -> &PathBuf {
        &self.worker_exe
    }
}

impl Transcriber for EmbeddedTranscriber {
    fn transcribe(
        &self,
        clip: &AudioClip,
        _options: &TranscriptionOptions,
    ) -> Result<Transcript, PortError> {
        // The worker is Wren's own binary reinvoked with the hidden subcommand;
        // we do NOT link transcribe-rs here — it only spawns and does IPC. The
        // ~1 GB peak lives and dies in the subprocess (docs/decisions/0005).
        let mut child = Command::new(&self.worker_exe)
            .arg(WORKER_SUBCOMMAND)
            .arg(&self.model_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                PortError::Other(format!("failed to start the transcription worker: {e}"))
            })?;

        // Serialize the samples as raw PCM i16 LE — the exact form the worker
        // expects on stdin (docs/architecture/embedded-engine.md §IPC Protocol);
        // `AudioClip` is already i16 mono.
        let mut pcm = Vec::with_capacity(clip.samples.len() * 2);
        for s in &clip.samples {
            pcm.extend_from_slice(&s.to_le_bytes());
        }

        // Anti-deadlock: writing to a full pipe blocks until the other side reads,
        // but the worker only finishes writing stdout after reading us — a classic
        // deadlock if we did everything on this thread. So we write stdin on a
        // dedicated thread (closing the pipe on exit, so the worker sees EOF) while
        // `wait_with_output` drains stdout and stderr here.
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| PortError::Other("worker stdin unavailable".to_string()))?;
        let writer = std::thread::spawn(move || {
            // An error here (e.g. worker died before reading ⇒ BrokenPipe) is not
            // fatal: the exit code / stderr collected below is what decides the result.
            let _ = stdin.write_all(&pcm);
            // `stdin` is dropped at the end of scope → closes the pipe → worker sees EOF.
        });

        let output = child
            .wait_with_output()
            .map_err(|e| PortError::Other(format!("failed to wait for the worker: {e}")))?;
        let _ = writer.join();

        // Exit ≠ 0 ⇒ failure: propagate the worker's stderr (docs/architecture/embedded-engine.md).
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let message = if stderr.is_empty() {
                format!("transcription worker failed ({})", output.status)
            } else {
                stderr
            };
            return Err(PortError::Other(message));
        }

        // Success: the JSON line is the LAST non-empty line of stdout. We scan from
        // back to front and take the first that deserializes — robust against any
        // noise the ONNX runtime might dump on stdout before it.
        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: WorkerResult = stdout
            .lines()
            .rev()
            .find_map(|line| serde_json::from_str::<WorkerResult>(line).ok())
            .ok_or_else(|| {
                PortError::Other(format!(
                    "unexpected worker output (no JSON line): {:?}",
                    stdout.trim()
                ))
            })?;

        // `model` = the model id (directory name, e.g. "parakeet-v3-int8").
        let model = self
            .model_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.model_dir.to_string_lossy().into_owned());

        Ok(partial_transcript(result.text, result.language, PROVIDER_ID, &model))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::WorkerResult;

    /// Byte-level contract of the IPC: `WorkerResult` survives a round-trip
    /// through ONE JSON line (what the worker emits and the parent reads).
    /// Independent of the real engine — it covers the protocol serialization itself.
    #[test]
    fn worker_result_round_trips_in_one_line() {
        let original = WorkerResult {
            text: "hello world".to_string(),
            language: Some("pt".to_string()),
            load_ms: 1200,
            infer_ms: 340,
        };
        let line = serde_json::to_string(&original).unwrap();
        assert!(!line.contains('\n'), "the result must fit in a single line");

        let back: WorkerResult = serde_json::from_str(&line).unwrap();
        assert_eq!(back.text, "hello world");
        assert_eq!(back.language.as_deref(), Some("pt"));
        assert_eq!(back.load_ms, 1200);
        assert_eq!(back.infer_ms, 340);
    }

    /// A trivial clip — most IPC tests do not depend on the content.
    fn clip(samples: usize) -> AudioClip {
        AudioClip {
            samples: vec![0i16; samples],
            sample_rate: 16_000,
            duration_ms: (samples as u64) * 1000 / 16_000,
        }
    }

    /// Spawning a nonexistent binary ⇒ `PortError::Other` (does not panic or
    /// hang). Cross-platform: only the spawn-error path.
    #[test]
    fn spawn_of_nonexistent_worker_becomes_porterror() {
        let transcriber = EmbeddedTranscriber {
            model_dir: PathBuf::from("parakeet-v3-int8"),
            worker_exe: PathBuf::from("/path/that/does/not/exist/wren-worker-nonexistent"),
        };
        let err = transcriber
            .transcribe(&clip(16), &TranscriptionOptions::default())
            .unwrap_err();
        assert!(matches!(err, PortError::Other(_)), "got {err:?}");
    }

    // The tests below exercise the IPC end to end with a FAKE WORKER (a shell
    // script) — without the real 640 MB engine. Unix-only since they use shebang/chmod.
    #[cfg(unix)]
    mod fake_worker {
        use super::*;
        use std::os::unix::fs::PermissionsExt;
        use std::sync::atomic::{AtomicU64, Ordering};

        /// Writes a temporary executable script that acts as the worker and
        /// returns its path. The adapter invokes it as `<script>
        /// __wren-transcribe-worker <model_dir>`; the script ignores the args and
        /// follows the protocol via its body.
        fn write_fake_worker(body: &str) -> PathBuf {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("wren-fake-worker-{}-{n}.sh", std::process::id()));
            std::fs::write(&path, body).unwrap();
            let mut perms = std::fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&path, perms).unwrap();
            path
        }

        fn transcriber(worker_exe: PathBuf) -> EmbeddedTranscriber {
            EmbeddedTranscriber {
                model_dir: PathBuf::from("/cache/parakeet-v3-int8"),
                worker_exe,
            }
        }

        /// Happy path: the worker drains stdin and emits the JSON line; the adapter
        /// deserializes and builds the `Transcript` (provider `embedded`, model =
        /// directory name).
        #[test]
        fn success_reads_json_and_builds_transcript() {
            let worker = write_fake_worker(
                "#!/bin/sh\ncat >/dev/null\n\
                 printf '%s\\n' '{\"text\":\"hello from fake\",\"language\":\"pt\",\"load_ms\":10,\"infer_ms\":5}'\n",
            );
            let out = transcriber(worker.clone())
                .transcribe(&clip(16_000), &TranscriptionOptions::default())
                .unwrap();
            assert_eq!(out.text, "hello from fake");
            assert_eq!(out.language.as_deref(), Some("pt"));
            assert_eq!(out.provider_id, "embedded");
            assert_eq!(out.model, "parakeet-v3-int8");
            let _ = std::fs::remove_file(worker);
        }

        /// Noise on stdout BEFORE the JSON (e.g. an ONNX runtime log) does not
        /// break the read: we take the last line that deserializes.
        #[test]
        fn ignores_noise_on_stdout_before_json() {
            let worker = write_fake_worker(
                "#!/bin/sh\ncat >/dev/null\necho 'onnxruntime: some warning'\n\
                 printf '%s\\n' '{\"text\":\"clean\",\"language\":null,\"load_ms\":1,\"infer_ms\":2}'\n",
            );
            let out = transcriber(worker.clone())
                .transcribe(&clip(16), &TranscriptionOptions::default())
                .unwrap();
            assert_eq!(out.text, "clean");
            assert_eq!(out.language, None);
            let _ = std::fs::remove_file(worker);
        }

        /// Exit ≠ 0 ⇒ `PortError::Other` carrying the worker's stderr.
        #[test]
        fn nonzero_exit_becomes_porterror_with_stderr() {
            let worker = write_fake_worker(
                "#!/bin/sh\ncat >/dev/null\necho 'model missing in cache' >&2\nexit 1\n",
            );
            let err = transcriber(worker.clone())
                .transcribe(&clip(16), &TranscriptionOptions::default())
                .unwrap_err();
            match err {
                PortError::Other(msg) => {
                    assert!(msg.contains("model missing in cache"), "msg={msg}")
                }
                other => panic!("expected PortError::Other, got {other:?}"),
            }
            let _ = std::fs::remove_file(worker);
        }

        /// Exit 0 but stdout with no JSON line at all ⇒ `PortError::Other`.
        #[test]
        fn stdout_without_json_becomes_porterror() {
            let worker =
                write_fake_worker("#!/bin/sh\ncat >/dev/null\necho 'not json'\n");
            let err = transcriber(worker.clone())
                .transcribe(&clip(16), &TranscriptionOptions::default())
                .unwrap_err();
            assert!(matches!(err, PortError::Other(_)), "got {err:?}");
            let _ = std::fs::remove_file(worker);
        }

        /// Anti-deadlock proof: the parent pushes ~1 MB of PCM to a worker that
        /// EXITS without reading stdin. Without the writer thread, `write_all` would
        /// fill the pipe and hang forever; with it, `write` just takes a BrokenPipe
        /// (ignored) and the transcription completes from the already-emitted stdout.
        #[test]
        fn large_stdin_with_worker_that_does_not_read_does_not_hang() {
            let worker = write_fake_worker(
                "#!/bin/sh\nprintf '%s\\n' '{\"text\":\"ok\",\"language\":null,\"load_ms\":0,\"infer_ms\":0}'\n",
            );
            // 500k samples = 1 MB of bytes, well above the pipe buffer (~64 KB).
            let out = transcriber(worker.clone())
                .transcribe(&clip(500_000), &TranscriptionOptions::default())
                .unwrap();
            assert_eq!(out.text, "ok");
            let _ = std::fs::remove_file(worker);
        }
    }
}
