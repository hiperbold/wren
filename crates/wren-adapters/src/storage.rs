//! Phase 0 persistence adapters: settings in JSON, history in JSONL.
//! (SQLite can replace the history in Phase 1 without touching the core.)

use std::fs;
use std::io::{BufRead, BufReader, Cursor, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use wren_core::{
    AudioClip, HistoryEntry, HistoryStore, PortError, RecordingStore, Settings, SettingsStore,
    Transcript,
};

fn storage_err<E: std::fmt::Display>(e: E) -> PortError {
    PortError::Storage(e.to_string())
}

pub fn default_config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("wren")
}

pub fn default_data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("wren")
}

pub struct JsonSettingsStore {
    path: PathBuf,
}

impl JsonSettingsStore {
    pub fn new(path: PathBuf) -> Self {
        JsonSettingsStore { path }
    }

    pub fn at_default_location() -> Self {
        Self::new(default_config_dir().join("settings.json"))
    }
}

impl SettingsStore for JsonSettingsStore {
    fn load(&self) -> Result<Settings, PortError> {
        if !self.path.exists() {
            return Ok(Settings::default());
        }
        let raw = fs::read_to_string(&self.path).map_err(storage_err)?;
        serde_json::from_str(&raw).map_err(storage_err)
    }

    fn save(&self, settings: &Settings) -> Result<(), PortError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(storage_err)?;
        }
        let raw = serde_json::to_string_pretty(settings).map_err(storage_err)?;
        fs::write(&self.path, raw).map_err(storage_err)
    }
}

pub struct JsonlHistoryStore {
    path: PathBuf,
    lock: Mutex<()>,
}

impl JsonlHistoryStore {
    pub fn new(path: PathBuf) -> Self {
        JsonlHistoryStore { path, lock: Mutex::new(()) }
    }

    pub fn at_default_location() -> Self {
        Self::new(default_data_dir().join("history.jsonl"))
    }
}

impl JsonlHistoryStore {
    /// Reads all valid lines (old plain-`Transcript` lines remain readable —
    /// the new fields have defaults in the domain).
    fn read_all(&self) -> Result<Vec<HistoryEntry>, PortError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = fs::File::open(&self.path).map_err(storage_err)?;
        Ok(BufReader::new(file)
            .lines()
            .filter_map(|line| line.ok())
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect())
    }
}

impl HistoryStore for JsonlHistoryStore {
    fn save(&self, entry: &HistoryEntry) -> Result<(), PortError> {
        let _guard = self.lock.lock().unwrap();
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(storage_err)?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(storage_err)?;
        let line = serde_json::to_string(entry).map_err(storage_err)?;
        writeln!(file, "{line}").map_err(storage_err)
    }

    fn recent(&self, limit: usize) -> Result<Vec<HistoryEntry>, PortError> {
        let _guard = self.lock.lock().unwrap();
        let entries = self.read_all()?;
        let start = entries.len().saturating_sub(limit);
        let mut recent = entries[start..].to_vec();
        recent.reverse();
        Ok(recent)
    }

    fn resolve(&self, created_at_ms: u64, transcript: &Transcript) -> Result<(), PortError> {
        let _guard = self.lock.lock().unwrap();
        let mut entries = self.read_all()?;
        let entry = entries
            .iter_mut()
            .find(|e| e.created_at_ms == created_at_ms)
            .ok_or_else(|| PortError::Storage("entry not found in history".into()))?;
        *entry = HistoryEntry::done(transcript);

        // Atomic rewrite: write everything to a .tmp and rename over the top.
        let mut out = String::new();
        for e in &entries {
            out.push_str(&serde_json::to_string(e).map_err(storage_err)?);
            out.push('\n');
        }
        let tmp = self.path.with_extension("jsonl.tmp");
        fs::write(&tmp, out).map_err(storage_err)?;
        fs::rename(&tmp, &self.path).map_err(storage_err)
    }
}

/// Recordings preserved as WAV files — the safety net against a network/provider
/// failure. They only exist between a failure and the successful resend.
pub struct WavRecordingStore {
    dir: PathBuf,
}

impl WavRecordingStore {
    pub fn new(dir: PathBuf) -> Self {
        WavRecordingStore { dir }
    }

    pub fn at_default_location() -> Self {
        Self::new(default_data_dir().join("recordings"))
    }
}

impl RecordingStore for WavRecordingStore {
    fn save(&self, clip: &AudioClip) -> Result<String, PortError> {
        fs::create_dir_all(&self.dir).map_err(storage_err)?;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let mut path = self.dir.join(format!("rec-{now_ms}.wav"));
        let mut n = 1;
        while path.exists() {
            path = self.dir.join(format!("rec-{now_ms}-{n}.wav"));
            n += 1;
        }
        // WAV on disk (not FLAC): simple and playable in any player.
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: clip.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).map_err(storage_err)?;
        for &sample in &clip.samples {
            writer.write_sample(sample).map_err(storage_err)?;
        }
        writer.finalize().map_err(storage_err)?;
        Ok(path.to_string_lossy().into_owned())
    }

    fn load(&self, key: &str) -> Result<AudioClip, PortError> {
        let wav_bytes = fs::read(key).map_err(storage_err)?;
        let mut reader = hound::WavReader::new(Cursor::new(&wav_bytes)).map_err(storage_err)?;
        let spec = reader.spec();
        let raw: Vec<i16> = reader
            .samples::<i16>()
            .collect::<Result<_, _>>()
            .map_err(storage_err)?;
        // Phase 0 files may be stereo (48 kHz) — downmix by averaging.
        // NO resample: FLAC/Groq accept any rate.
        let channels = spec.channels.max(1) as usize;
        let samples: Vec<i16> = if channels == 1 {
            raw
        } else {
            raw.chunks_exact(channels)
                .map(|frame| {
                    (frame.iter().map(|&s| s as i32).sum::<i32>() / channels as i32) as i16
                })
                .collect()
        };
        let duration_ms = samples.len() as u64 * 1000 / spec.sample_rate.max(1) as u64;
        Ok(AudioClip { samples, sample_rate: spec.sample_rate, duration_ms })
    }

    fn delete(&self, key: &str) {
        let _ = fs::remove_file(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wren_core::EntryStatus;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("wren-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn pcm_clip(duration_ms: u64) -> AudioClip {
        let samples: Vec<i16> = (0..(16 * duration_ms)).map(|i| (i % 100) as i16).collect();
        AudioClip { samples, sample_rate: 16_000, duration_ms }
    }

    #[test]
    fn recording_store_roundtrip_and_delete() {
        let dir = temp_dir("recordings");
        let store = WavRecordingStore::new(dir.clone());

        let clip = pcm_clip(1500);
        let key = store.save(&clip).unwrap();
        assert!(PathBuf::from(&key).exists());

        let loaded = store.load(&key).unwrap();
        assert_eq!(loaded.sample_rate, 16_000);
        assert_eq!(loaded.duration_ms, 1500);
        assert_eq!(loaded.samples, clip.samples);

        store.delete(&key);
        assert!(!PathBuf::from(&key).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_of_phase_0_stereo_wav_downmixes() {
        let dir = temp_dir("recordings-stereo");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("legacy.wav");

        // File preserved by Phase 0: 48 kHz stereo.
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for _ in 0..48 {
            writer.write_sample(100i16).unwrap(); // L
            writer.write_sample(300i16).unwrap(); // R
        }
        writer.finalize().unwrap();

        let store = WavRecordingStore::new(dir.clone());
        let loaded = store.load(path.to_str().unwrap()).unwrap();
        assert_eq!(loaded.sample_rate, 48_000); // no resample on load
        assert_eq!(loaded.samples.len(), 48);
        assert!(loaded.samples.iter().all(|&s| s == 200)); // L/R average
        assert_eq!(loaded.duration_ms, 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_replaces_failed_entry_and_preserves_the_rest() {
        let dir = temp_dir("history");
        let store = JsonlHistoryStore::new(dir.join("history.jsonl"));

        // Old line (plain Transcript, no `status`) must remain readable.
        let old_line = r#"{"text":"old","language":"pt","provider_id":"groq","model":"m","audio_duration_ms":1000,"latency_ms":50,"created_at_ms":111}"#;
        fs::write(dir.join("history.jsonl"), format!("{old_line}\n")).unwrap();

        let failed =
            HistoryEntry::failed("network dropped".into(), Some("/tmp/rec.wav".into()), 9000, 222);
        store.save(&failed).unwrap();

        let transcript = Transcript {
            text: "recovered".into(),
            language: Some("pt".into()),
            provider_id: "groq".into(),
            model: "m".into(),
            audio_duration_ms: 9000,
            latency_ms: 80,
            created_at_ms: 222,
            recorded_duration_ms: None,
        };
        store.resolve(222, &transcript).unwrap();

        let recent = store.recent(10).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].status, EntryStatus::Done);
        assert_eq!(recent[0].text, "recovered");
        assert!(recent[0].audio_path.is_none());
        assert_eq!(recent[1].text, "old");
        assert_eq!(recent[1].status, EntryStatus::Done);

        assert!(store.resolve(999, &transcript).is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}

pub struct SystemClock;

impl wren_core::Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}
