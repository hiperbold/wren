//! wren-bench — transcription benchmark against the golden set
//! (`tests/fixtures/golden/`). Development tool, not distributed.
//!
//! Runs the SAME pipeline as the app (normalize → VAD → FLAC → provider) over
//! each sample in the manifest, in three VAD variants, and compares against the
//! reference: WER (content), PER (punctuation) and capitalization — kept
//! separate on purpose (doc 08 §7). Usage: `scripts/bench-stt.sh --help`.

mod metrics;

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Deserialize;
use wren_adapters::preprocess;
use wren_adapters::{EarshotVad, JsonSettingsStore, RemoteApiTranscriber};
use wren_core::{
    AudioClip, ProviderConfig, SettingsStore, Transcriber, TranscriptionOptions, Vad, VadOutcome,
};

/// App use-case gate: real speech below this isn't worth a request.
const MIN_SPEECH_MS: u64 = 300;
/// Default threshold for pause compression — same as the app (doc 08 §3).
const DEFAULT_COMPRESS_OVER_MS: u64 = 2000;

const USAGE: &str = "\
wren-bench — transcription benchmark against the golden set

Usage: scripts/bench-stt.sh [FLAGS]
       (or: cargo run -p wren-bench --release -- [FLAGS])

Flags:
  --samples id1,id2       run only the listed samples (default: all)
  --variants a,b          variants: sem-vad, vad, vad+pausas (default: all three)
  --compress-over-ms N    threshold (ms) for the vad+pausas variant (default: 2000)
  --quiet                 omit the transcribed texts, table only
  --help                  this message

Provider: the active one from settings.json; overrides via environment variable
(each one replaces only its respective field):
  WREN_BENCH_BASE_URL, WREN_BENCH_MODEL, WREN_BENCH_API_KEY";

// ---------------------------------------------------------------------------
// Golden set manifest
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Manifest {
    samples: Vec<Sample>,
}

/// Only the fields the bench uses; `scenario`/`source` stay in the manifest for
/// humans (serde ignores unknown fields).
#[derive(Deserialize)]
struct Sample {
    id: String,
    audio: String,
    reference: String,
    punctuation: String,
}

// ---------------------------------------------------------------------------
// VAD variants
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Variant {
    SemVad,
    Vad,
    VadPausas,
}

impl Variant {
    const ALL: [Variant; 3] = [Variant::SemVad, Variant::Vad, Variant::VadPausas];

    fn name(self) -> &'static str {
        match self {
            Variant::SemVad => "sem-vad",
            Variant::Vad => "vad",
            Variant::VadPausas => "vad+pausas",
        }
    }

    fn parse(s: &str) -> Option<Variant> {
        Variant::ALL.into_iter().find(|v| v.name() == s)
    }
}

// ---------------------------------------------------------------------------
// Arguments (manual parsing — repo philosophy: minimal dependencies)
// ---------------------------------------------------------------------------

struct Args {
    /// `None` = all samples in the manifest.
    samples: Option<Vec<String>>,
    variants: Vec<Variant>,
    compress_over_ms: u64,
    quiet: bool,
}

/// `Ok(None)` = `--help` was requested (prints and exits without error).
fn parse_args(argv: &[String]) -> Result<Option<Args>, String> {
    let mut args = Args {
        samples: None,
        variants: Variant::ALL.to_vec(),
        compress_over_ms: DEFAULT_COMPRESS_OVER_MS,
        quiet: false,
    };

    let mut i = 0;
    // Accepts both `--flag value` and `--flag=value`.
    let value_of = |i: &mut usize, flag: &str, inline: Option<&str>| -> Result<String, String> {
        if let Some(v) = inline {
            return Ok(v.to_string());
        }
        *i += 1;
        argv.get(*i).cloned().ok_or_else(|| format!("{flag} requires a value (see --help)"))
    };

    while i < argv.len() {
        let arg = &argv[i];
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) => (f, Some(v)),
            None => (arg.as_str(), None),
        };
        match flag {
            "--help" | "-h" => return Ok(None),
            "--quiet" => args.quiet = true,
            "--samples" => {
                let v = value_of(&mut i, "--samples", inline)?;
                args.samples =
                    Some(v.split(',').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect());
            }
            "--variants" => {
                let v = value_of(&mut i, "--variants", inline)?;
                let mut variants = Vec::new();
                for name in v.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                    let variant = Variant::parse(name).ok_or_else(|| {
                        format!("unknown variant: {name} (use sem-vad, vad, vad+pausas)")
                    })?;
                    if !variants.contains(&variant) {
                        variants.push(variant);
                    }
                }
                if variants.is_empty() {
                    return Err("--variants requires at least one variant".into());
                }
                args.variants = variants;
            }
            "--compress-over-ms" => {
                let v = value_of(&mut i, "--compress-over-ms", inline)?;
                args.compress_over_ms = v
                    .parse()
                    .map_err(|_| format!("--compress-over-ms requires an integer in ms, got: {v}"))?;
            }
            other => return Err(format!("unknown flag: {other} (see --help)")),
        }
        i += 1;
    }
    Ok(Some(args))
}

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

fn env_non_empty(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// Provider from settings (active) with per-field env overrides.
/// Also returns the configured language (default "pt").
/// The api_key is NEVER printed — not here, not in errors.
fn resolve_provider() -> Result<(ProviderConfig, String), String> {
    let settings = JsonSettingsStore::at_default_location().load().ok();
    let language = settings
        .as_ref()
        .and_then(|s| s.language.clone())
        .unwrap_or_else(|| "pt".to_string());

    let base_url = env_non_empty("WREN_BENCH_BASE_URL");
    let model = env_non_empty("WREN_BENCH_MODEL");
    let api_key = env_non_empty("WREN_BENCH_API_KEY");

    let mut provider = match settings.as_ref().and_then(|s| s.active_provider().cloned()) {
        Some(p) => p,
        None => {
            // No active provider: the overrides must suffice on their own.
            let (Some(bu), Some(m)) = (base_url.clone(), model.clone()) else {
                return Err(
                    "no active provider in settings.json and incomplete overrides — \
                     set WREN_BENCH_BASE_URL and WREN_BENCH_MODEL (and, if needed, \
                     WREN_BENCH_API_KEY)"
                        .into(),
                );
            };
            let local = bu.contains("localhost") || bu.contains("127.0.0.1");
            ProviderConfig {
                id: "bench-env".into(),
                label: "Env override".into(),
                kind: wren_core::ProviderKind::RemoteApi,
                base_url: bu,
                api_key: None,
                model: m,
                sends_audio_externally: !local,
            }
        }
    };

    if let Some(bu) = base_url {
        provider.base_url = bu;
    }
    if let Some(m) = model {
        provider.model = m;
    }
    if let Some(k) = api_key {
        provider.api_key = Some(k);
    }
    Ok((provider, language))
}

/// Short, safe summary of an error for the table: one line, truncated, and with
/// the api_key redacted in case some provider echoes it in the response.
fn error_summary(error: &dyn std::fmt::Display, api_key: Option<&str>) -> String {
    let mut msg = error.to_string().replace(['\n', '\r'], " ");
    if let Some(key) = api_key {
        if !key.is_empty() {
            msg = msg.replace(key, "***");
        }
    }
    const MAX: usize = 60;
    if msg.chars().count() > MAX {
        msg = msg.chars().take(MAX).collect::<String>() + "…";
    }
    msg
}

// ---------------------------------------------------------------------------
// Pipeline per sample × variant
// ---------------------------------------------------------------------------

/// Reads a 16-bit PCM WAV (any rate, mono or stereo).
fn read_wav(path: &Path) -> Result<(Vec<i16>, u32, u16), String> {
    let mut reader =
        hound::WavReader::open(path).map_err(|e| format!("open {}: {e}", path.display()))?;
    let spec = reader.spec();
    if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 16 {
        return Err(format!(
            "{}: expected PCM 16-bit int, got {:?} {} bits",
            path.display(),
            spec.sample_format,
            spec.bits_per_sample
        ));
    }
    let samples: Vec<i16> = reader
        .samples::<i16>()
        .collect::<Result<_, _>>()
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok((samples, spec.sample_rate, spec.channels))
}

/// Applies the VAD variant to the normalized clip.
/// `None` = discarded by the gate (NoSpeech or speech < [`MIN_SPEECH_MS`]).
fn clip_for_variant(clip: &AudioClip, variant: Variant, compress_over_ms: u64) -> Option<AudioClip> {
    let compress = match variant {
        Variant::SemVad => return Some(clip.clone()),
        Variant::Vad => None,
        Variant::VadPausas => Some(compress_over_ms),
    };
    match EarshotVad::new(compress).gate_and_trim(clip) {
        VadOutcome::NoSpeech => None,
        VadOutcome::Speech { speech_ms, .. } if speech_ms < MIN_SPEECH_MS => None,
        VadOutcome::Speech { clip, .. } => Some(clip),
    }
}

enum Outcome {
    Transcribed {
        wer: f64,
        /// `None` = reference without marks or `punctuation: "none"` in the manifest.
        per: Option<f64>,
        cap: (usize, usize),
        sent_s: f64,
        latency_ms: u64,
        text: String,
    },
    DiscardedByGate,
    Failed {
        summary: String,
        sent_s: f64,
    },
}

struct Row {
    id: String,
    variant: &'static str,
    /// Dictated duration (post-normalization, pre-VAD) — the "before" of compression.
    recorded_s: f64,
    outcome: Outcome,
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

fn fmt_pct(v: f64) -> String {
    format!("{:.1}", v * 100.0)
}

fn cells(row: &Row) -> [String; 7] {
    let id = row.id.clone();
    let variant = row.variant.to_string();
    match &row.outcome {
        Outcome::Transcribed { wer, per, cap, sent_s, latency_ms, .. } => [
            id,
            variant,
            fmt_pct(*wer),
            per.map(fmt_pct).unwrap_or_else(|| "n/a".into()),
            format!("{}/{}", cap.0, cap.1),
            format!("{:.1}→{:.1}", row.recorded_s, sent_s),
            latency_ms.to_string(),
        ],
        Outcome::DiscardedByGate => [
            id,
            variant,
            "discarded by gate".into(),
            "—".into(),
            "—".into(),
            format!("{:.1}→—", row.recorded_s),
            "—".into(),
        ],
        Outcome::Failed { summary, sent_s } => [
            id,
            variant,
            format!("ERROR ({summary})"),
            "—".into(),
            "—".into(),
            format!("{:.1}→{:.1}", row.recorded_s, sent_s),
            "—".into(),
        ],
    }
}

/// Prints the aligned table; without `--quiet`, the transcribed text goes on
/// its own line below each result, for manual inspection.
fn print_table(rows: &[Row], quiet: bool) {
    const HEADER: [&str; 7] =
        ["SAMPLE", "VARIANT", "WER%", "PER%", "CAP", "DUR(s) rec→sent", "LAT(ms)"];

    let body: Vec<[String; 7]> = rows.iter().map(cells).collect();
    let mut widths: [usize; 7] = HEADER.map(str::len);
    for row in &body {
        for (w, cell) in widths.iter_mut().zip(row) {
            *w = (*w).max(cell.chars().count());
        }
    }

    let print_row = |cells: &[String; 7]| {
        let mut out = String::new();
        for (i, (cell, width)) in cells.iter().zip(widths).enumerate() {
            if i > 0 {
                out.push_str("  ");
            }
            let pad = width - cell.chars().count();
            if i < 2 {
                // id and variant left-aligned; metrics right-aligned.
                out.push_str(cell);
                out.push_str(&" ".repeat(pad));
            } else {
                out.push_str(&" ".repeat(pad));
                out.push_str(cell);
            }
        }
        println!("{}", out.trim_end());
    };

    print_row(&HEADER.map(String::from));
    let total: usize = widths.iter().sum::<usize>() + 2 * (widths.len() - 1);
    println!("{}", "-".repeat(total));
    for (row, cells) in rows.iter().zip(&body) {
        print_row(cells);
        if !quiet {
            if let Outcome::Transcribed { text, .. } = &row.outcome {
                println!("  ↳ {}", text.replace(['\n', '\r'], " ").trim());
            }
        }
    }
}

/// Simple WER/PER averages per variant (PER only over samples that have it).
fn print_averages(rows: &[Row], variants: &[Variant]) {
    println!();
    println!("Averages per variant (simple average; PER only over samples with punctuation):");
    for variant in variants {
        let name = variant.name();
        let wers: Vec<f64> = rows
            .iter()
            .filter(|r| r.variant == name)
            .filter_map(|r| match &r.outcome {
                Outcome::Transcribed { wer, .. } => Some(*wer),
                _ => None,
            })
            .collect();
        let pers: Vec<f64> = rows
            .iter()
            .filter(|r| r.variant == name)
            .filter_map(|r| match &r.outcome {
                Outcome::Transcribed { per, .. } => *per,
                _ => None,
            })
            .collect();
        let average = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let wer_txt = if wers.is_empty() {
            "n/a".to_string()
        } else {
            format!("{}% ({} samples)", fmt_pct(average(&wers)), wers.len())
        };
        let per_txt = if pers.is_empty() {
            "n/a".to_string()
        } else {
            format!("{}% ({} samples)", fmt_pct(average(&pers)), pers.len())
        };
        println!("  {name:<12} WER {wer_txt:<22} PER {per_txt}");
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn run() -> Result<(), String> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let Some(args) = parse_args(&argv)? else {
        println!("{USAGE}");
        return Ok(());
    };

    // The golden set lives in the repo — resolve from the crate, not the cwd.
    let golden_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/golden");
    let manifest_path = golden_dir.join("manifest.json");
    let raw = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("read {}: {e}", manifest_path.display()))?;
    let manifest: Manifest =
        serde_json::from_str(&raw).map_err(|e| format!("invalid manifest.json: {e}"))?;

    let samples: Vec<&Sample> = match &args.samples {
        None => manifest.samples.iter().collect(),
        Some(ids) => {
            for id in ids {
                if !manifest.samples.iter().any(|s| &s.id == id) {
                    return Err(format!("unknown sample in manifest: {id}"));
                }
            }
            manifest.samples.iter().filter(|s| ids.contains(&s.id)).collect()
        }
    };
    if samples.is_empty() {
        return Err("no samples to run".into());
    }

    let (provider, language) = resolve_provider()?;
    let api_key = provider.api_key.clone();
    eprintln!(
        "provider: {} — model {} — language {}",
        provider.base_url, provider.model, language
    );
    let transcriber = RemoteApiTranscriber::new(provider)
        .map_err(|e| error_summary(&e, api_key.as_deref()))?;
    let options = TranscriptionOptions { language: Some(language), prompt: None };

    let total_runs = samples.len() * args.variants.len();
    let mut rows: Vec<Row> = Vec::with_capacity(total_runs);
    let mut n_run = 0usize;

    for sample in &samples {
        // A sample without audio (e.g. a personal one not yet recorded — see the
        // script in personal/) doesn't bring down the run: it warns and moves on.
        let (pcm, rate, channels) = match read_wav(&golden_dir.join(&sample.audio)) {
            Ok(wav) => wav,
            Err(error) => {
                eprintln!("sample {} skipped: {error}", sample.id);
                continue;
            }
        };
        let (norm, norm_rate) = preprocess::normalize(&pcm, rate, channels);
        let clip = AudioClip {
            duration_ms: norm.len() as u64 * 1000 / u64::from(norm_rate),
            samples: norm,
            sample_rate: norm_rate,
        };
        let recorded_s = clip.duration_ms as f64 / 1000.0;
        let reference = std::fs::read_to_string(golden_dir.join(&sample.reference))
            .map_err(|e| format!("read reference of {}: {e}", sample.id))?;

        for &variant in &args.variants {
            n_run += 1;
            eprint!("[{n_run}/{total_runs}] {} × {} … ", sample.id, variant.name());

            let Some(sent) = clip_for_variant(&clip, variant, args.compress_over_ms) else {
                eprintln!("discarded by gate");
                rows.push(Row {
                    id: sample.id.clone(),
                    variant: variant.name(),
                    recorded_s,
                    outcome: Outcome::DiscardedByGate,
                });
                continue;
            };
            let sent_s = sent.duration_ms as f64 / 1000.0;

            let start = Instant::now();
            let outcome = match transcriber.transcribe(&sent, &options) {
                Ok(transcript) => {
                    let latency_ms = start.elapsed().as_millis() as u64;
                    eprintln!("ok ({latency_ms} ms)");
                    // A sample without relevant punctuation doesn't take part in the PER.
                    let per = if sample.punctuation == "none" {
                        None
                    } else {
                        metrics::per(&reference, &transcript.text)
                    };
                    Outcome::Transcribed {
                        wer: metrics::wer(&reference, &transcript.text),
                        per,
                        cap: metrics::cap_mismatches(&reference, &transcript.text),
                        sent_s,
                        latency_ms,
                        text: transcript.text,
                    }
                }
                Err(error) => {
                    let summary = error_summary(&error, api_key.as_deref());
                    eprintln!("ERROR: {summary}");
                    Outcome::Failed { summary, sent_s }
                }
            };
            rows.push(Row {
                id: sample.id.clone(),
                variant: variant.name(),
                recorded_s,
                outcome,
            });
        }
    }

    println!();
    print_table(&rows, args.quiet);
    print_averages(&rows, &args.variants);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
