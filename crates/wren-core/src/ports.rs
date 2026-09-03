//! The ports (doc 02): contracts the core defines and the adapters implement.
//! Synchronous traits — concurrency is the composition layer's responsibility
//! (the app runs the use case on its own thread).

use crate::domain::{
    AudioClip, HistoryEntry, SessionMetrics, Settings, Transcript, TranscriptionOptions, VadOutcome,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PortError {
    #[error("audio device unavailable: {0}")]
    AudioUnavailable(String),
    #[error("provider rejected the request ({status}): {message}")]
    ProviderRejected { status: u16, message: String },
    #[error("network failure talking to the provider: {0}")]
    Network(String),
    #[error("could not deliver the text: {0}")]
    DeliveryFailed(String),
    #[error("persistence failure: {0}")]
    Storage(String),
    #[error("{0}")]
    Other(String),
}

/// The project's central port (doc 03): audio → transcription.
/// Cloud, local server and (future) embedded all implement THIS contract.
pub trait Transcriber: Send + Sync {
    fn transcribe(
        &self,
        clip: &AudioClip,
        options: &TranscriptionOptions,
    ) -> Result<Transcript, PortError>;
}

/// Audio capture from the microphone.
pub trait AudioSource: Send + Sync {
    fn start(&self) -> Result<(), PortError>;
    fn stop(&self) -> Result<AudioClip, PortError>;
    /// Discards the ongoing capture without producing a clip.
    fn abort(&self);
}

/// Speech detection (doc 02, doc 08 §3): gate (don't send silence) and
/// trim (cut the edges). Must not fail the flow — when in doubt, the
/// adapter returns the whole clip as speech.
pub trait Vad: Send + Sync {
    fn gate_and_trim(&self, clip: &AudioClip) -> VadOutcome;
}

/// Delivers the text to the focused app (paste/type).
pub trait TextSink: Send + Sync {
    fn deliver(&self, text: &str) -> Result<(), PortError>;
}

/// History persistence (transcriptions and failures with preserved audio).
pub trait HistoryStore: Send + Sync {
    fn save(&self, entry: &HistoryEntry) -> Result<(), PortError>;
    fn recent(&self, limit: usize) -> Result<Vec<HistoryEntry>, PortError>;
    /// Replaces the failed entry identified by `created_at_ms` with the
    /// transcription obtained on retry.
    fn resolve(&self, created_at_ms: u64, transcript: &Transcript) -> Result<(), PortError>;
}

/// Recordings preserved when transcription fails — the dictation's safety net.
/// The clip goes to the store BEFORE being sent to the API and is deleted after
/// success; only failures leave audio on disk (awaiting retry).
pub trait RecordingStore: Send + Sync {
    /// Persists the clip and returns the key (e.g. path) to retrieve it.
    fn save(&self, clip: &AudioClip) -> Result<String, PortError>;
    fn load(&self, key: &str) -> Result<AudioClip, PortError>;
    /// Best-effort: a failure to delete does not interrupt the flow.
    fn delete(&self, key: &str);
}

/// Settings persistence.
pub trait SettingsStore: Send + Sync {
    fn load(&self) -> Result<Settings, PortError>;
    fn save(&self, settings: &Settings) -> Result<(), PortError>;
}

/// Feedback to the user (sound/overlay/tray). The waveform bubble is an adapter
/// of this port (docs 01 and 07). No method may block the audio flow.
pub trait Feedback: Send + Sync {
    fn recording_started(&self);
    /// Microphone level in [0.0, 1.0] — feeds the wave animation.
    fn audio_level(&self, level: f32);
    fn transcribing(&self);
    fn finished(&self, transcript: &Transcript);
    fn failed(&self, message: &str);
    fn cancelled(&self);
}

/// Clock — a port so the core stays testable without depending on real time.
pub trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}

/// Performance telemetry — **100% local** diagnostics (never leaves the
/// machine). The use case measures each stage's duration via `Clock` and
/// reports here at the end of each session; the adapter persists/displays it.
/// Like the VAD and feedback, it must not fail the flow — implementations
/// swallow their own errors.
pub trait Telemetry: Send + Sync {
    fn record(&self, metrics: &SessionMetrics);

    /// The process's resident RSS in bytes, if the OS exposes it. The default
    /// `None` keeps the core OS-agnostic and the tests free of any time/OS
    /// dependency; the production adapter reads `/proc/self/statm` on Linux.
    fn sample_rss(&self) -> Option<u64> {
        None
    }
}
