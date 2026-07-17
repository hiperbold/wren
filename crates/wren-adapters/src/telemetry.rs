//! Adapter for the `Telemetry` port: persists each session's performance
//! metrics to JSONL (`<data>/telemetry.jsonl`) and samples the process RSS.
//!
//! **Fully local diagnostics** — the metrics never leave the machine (consistent
//! with the "zero external telemetry" stance of doc 06 §9, which is about
//! egress). Each `record` also emits a summary via `log::info!` on the
//! `wren::telemetry` target, so the breakdown shows up in the Diagnostics tab
//! alongside the other logs.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use wren_core::{SessionMetrics, Telemetry};

use crate::storage::default_data_dir;

pub struct JsonlTelemetryStore {
    path: PathBuf,
    lock: Mutex<()>,
}

impl JsonlTelemetryStore {
    pub fn new(path: PathBuf) -> Self {
        JsonlTelemetryStore { path, lock: Mutex::new(()) }
    }

    pub fn at_default_location() -> Self {
        Self::new(default_data_dir().join("telemetry.jsonl"))
    }

    /// Recent metrics, **most-recent-first**, limited to `limit`.
    /// Never fails the flow — a missing/unreadable file becomes an empty list.
    pub fn recent(&self, limit: usize) -> Vec<SessionMetrics> {
        let _guard = self.lock.lock().unwrap();
        if !self.path.exists() {
            return Vec::new();
        }
        let file = match fs::File::open(&self.path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };
        let mut all: Vec<SessionMetrics> = BufReader::new(file)
            .lines()
            .filter_map(|line| line.ok())
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect();
        let start = all.len().saturating_sub(limit);
        let mut recent = all.split_off(start);
        recent.reverse();
        recent
    }

    fn append(&self, metrics: &SessionMetrics) -> std::io::Result<()> {
        let _guard = self.lock.lock().unwrap();
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(metrics).unwrap_or_default();
        writeln!(file, "{line}")
    }
}

impl Telemetry for JsonlTelemetryStore {
    fn record(&self, metrics: &SessionMetrics) {
        let breakdown: Vec<String> = metrics
            .stages
            .iter()
            .map(|s| format!("{:?}={}ms", s.stage, s.duration_ms))
            .collect();
        log::info!(
            target: "wren::telemetry",
            "session {:?}: total={}ms [{}] rss_peak={}",
            metrics.outcome,
            metrics.total_ms,
            breakdown.join(" "),
            metrics.rss_peak_bytes.map(fmt_bytes).unwrap_or_else(|| "n/a".into()),
        );
        if let Err(e) = self.append(metrics) {
            log::warn!(target: "wren::telemetry", "could not persist metrics: {e}");
        }
    }

    fn sample_rss(&self) -> Option<u64> {
        sample_rss_bytes()
    }
}

/// Process resident RSS in bytes. On Linux it reads `/proc/self/statm` (second
/// field = resident pages × 4 KiB). Off Linux it returns `None`.
#[cfg(target_os = "linux")]
fn sample_rss_bytes() -> Option<u64> {
    let statm = fs::read_to_string("/proc/self/statm").ok()?;
    let resident_pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(resident_pages * 4096)
}

#[cfg(not(target_os = "linux"))]
fn sample_rss_bytes() -> Option<u64> {
    None
}

fn fmt_bytes(bytes: u64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    format!("{mb:.1} MB")
}

#[cfg(test)]
mod tests {
    use super::*;
    use wren_core::{SessionOutcome, Stage, StageTiming};

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("wren-telem-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir.join("telemetry.jsonl")
    }

    fn metrics(total_ms: u64, outcome: SessionOutcome) -> SessionMetrics {
        SessionMetrics {
            created_at_ms: total_ms, // just to tell them apart in the tests
            outcome,
            provider_id: "groq".into(),
            model: "whisper".into(),
            recorded_duration_ms: 1500,
            sent_audio_duration_ms: 1200,
            total_ms,
            stages: vec![StageTiming {
                stage: Stage::Transcribe,
                duration_ms: total_ms,
                audio_bytes: Some(38_400),
            }],
            rss_start_bytes: Some(100 * 1024 * 1024),
            rss_peak_bytes: Some(150 * 1024 * 1024),
        }
    }

    #[test]
    fn persists_and_reads_most_recent_first() {
        let path = temp_path("roundtrip");
        let store = JsonlTelemetryStore::new(path.clone());

        store.record(&metrics(100, SessionOutcome::Delivered));
        store.record(&metrics(200, SessionOutcome::Failed));

        let recent = store.recent(10);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].total_ms, 200);
        assert_eq!(recent[0].outcome, SessionOutcome::Failed);
        assert_eq!(recent[1].total_ms, 100);
        assert_eq!(recent[1].stages[0].stage, Stage::Transcribe);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn recent_respects_the_limit() {
        let path = temp_path("limite");
        let store = JsonlTelemetryStore::new(path.clone());
        for i in 0..5 {
            store.record(&metrics(i, SessionOutcome::Delivered));
        }
        let recent = store.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].total_ms, 4);
        assert_eq!(recent[1].total_ms, 3);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn missing_file_becomes_empty_list() {
        let store = JsonlTelemetryStore::new(temp_path("vazio"));
        // Nothing written yet (temp_path recreates the empty directory).
        std::fs::remove_file(
            std::env::temp_dir()
                .join(format!("wren-telem-vazio-{}", std::process::id()))
                .join("telemetry.jsonl"),
        )
        .ok();
        assert!(store.recent(10).is_empty());
    }
}
