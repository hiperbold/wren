//! Adapter for the `Vad` port with earshot (pure-Rust neural VAD, ~110 KiB).
//! Chosen over Silero to avoid paying 11–25 MB of ONNX Runtime per platform
//! (doc 08 §3); Silero via `ort` stays as an opt-in plan B.
//!
//! Beyond the gate and the edge trim, it compresses internal thinking pauses
//! (doc 08 §3): pauses longer than the configured threshold are shortened to a
//! fixed residue — less billed duration and less hallucination territory,
//! without erasing the acoustic sentence-boundary cue the model uses to
//! punctuate.

use wren_core::{AudioClip, Vad, VadOutcome};

/// earshot works on frames of 256 samples @ 16 kHz.
const FRAME_SAMPLES: usize = 256;
/// Duration of one frame: 256 / 16000 = 16 ms.
const FRAME_MS: u64 = 16;
/// A score above this is speech (threshold recommended by the crate).
const SPEECH_THRESHOLD: f32 = 0.5;
/// Trim padding at the END and around internal speech: ~200 ms so as not to
/// cut attack/decay that the detector marked late/early. Short on purpose:
/// leftover silence at the end is the classic hallucination territory (doc 08 §3).
const TRIM_PADDING_FRAMES: usize = 12;
/// Trim padding at the START: ~500 ms. More generous than the end's because
/// Whisper decodes better with a breath of silence before speech — with 200 ms,
/// the golden set baseline regressed (recife-accent: WER 0→7%, the ambiguous
/// final sentence changed); with 500 ms it went back to 0%. Silence BEFORE
/// speech is not hallucination territory, so the cost is only ~0.3 s billed.
const TRIM_LEAD_FRAMES: usize = 31;
/// Residue kept in place of a long pause: 24 frames ≈ 384 ms (~400 ms), half
/// glued to each speech boundary. Not configurable on purpose — it is the
/// minimum that preserves the punctuation cue without paying for the whole
/// pause. Half the residue (12 frames) == TRIM_PADDING_FRAMES, so the ~200 ms
/// protection around each speech stretch stays intact.
const PAUSE_RESIDUE_FRAMES: usize = 2 * TRIM_PADDING_FRAMES;

#[derive(Default)]
pub struct EarshotVad {
    /// Internal pauses longer than this (ms) are compressed to the residue.
    /// `None` = compression off (just gate + trim).
    compress_pauses_over_ms: Option<u64>,
}

impl EarshotVad {
    pub fn new(compress_pauses_over_ms: Option<u64>) -> Self {
        EarshotVad { compress_pauses_over_ms }
    }
}

/// Trims silence off the edges and compresses internal pauses — a pure function
/// (samples + per-frame classification + config → samples), testable without
/// relying on earshot classifying synthetic audio.
///
/// Rules:
/// - **edges**: keeps `TRIM_LEAD_FRAMES` (~500 ms) before the first speech
///   frame and `TRIM_PADDING_FRAMES` (~200 ms) after the last — a deliberate
///   asymmetry, see the constants' comments;
/// - **internal pauses**: silence gaps LONGER than `compress_over_ms` become a
///   residue of `PAUSE_RESIDUE_FRAMES` (~400 ms), half at each boundary — the
///   compression only acts on the core of the gap, beyond the padding;
/// - `compress_over_ms = None` turns compression off (trim only);
/// - **fail-open**: with no speech frame or with flags inconsistent with the
///   samples, it returns the audio intact (the NoSpeech gate is the caller's
///   decision).
pub(crate) fn trim_and_compress_pauses(
    samples: &[i16],
    speech: &[bool],
    compress_over_ms: Option<u64>,
) -> Vec<i16> {
    // Fail-open: flags that do not match the samples indicate a caller bug —
    // never discard dictation over an internal inconsistency.
    if speech.len() != samples.len() / FRAME_SAMPLES {
        return samples.to_vec();
    }
    let Some(first) = speech.iter().position(|&s| s) else {
        return samples.to_vec();
    };
    let last = speech.iter().rposition(|&s| s).unwrap_or(first);

    // Edge trim with padding, clamped to the clip's bounds.
    let start = first.saturating_sub(TRIM_LEAD_FRAMES) * FRAME_SAMPLES;
    let end = ((last + 1 + TRIM_PADDING_FRAMES) * FRAME_SAMPLES).min(samples.len());

    // Kept stretches, as sample ranges [start, end).
    let mut kept: Vec<(usize, usize)> = Vec::new();
    match compress_over_ms {
        None => kept.push((start, end)),
        Some(threshold_ms) => {
            let mut segment_start = start;
            let mut prev_speech = first;
            for (i, _) in speech.iter().enumerate().skip(first + 1).filter(|(_, &s)| s) {
                let gap_frames = i - prev_speech - 1;
                // Only compress if the gap exceeds the threshold AND there is core
                // left beyond the residue — a gap ≤ threshold stays intact.
                if gap_frames as u64 * FRAME_MS > threshold_ms
                    && gap_frames > PAUSE_RESIDUE_FRAMES
                {
                    // Close the stretch half-a-residue after the previous speech…
                    let close = (prev_speech + 1 + PAUSE_RESIDUE_FRAMES / 2) * FRAME_SAMPLES;
                    kept.push((segment_start, close.min(samples.len())));
                    // …and reopen half-a-residue before the next speech.
                    segment_start = (i - PAUSE_RESIDUE_FRAMES / 2) * FRAME_SAMPLES;
                }
                prev_speech = i;
            }
            kept.push((segment_start, end));
        }
    }

    let mut out = Vec::with_capacity(kept.iter().map(|(s, e)| e.saturating_sub(*s)).sum());
    for (s, e) in kept {
        if s < e {
            out.extend_from_slice(&samples[s..e]);
        }
    }
    out
}

impl Vad for EarshotVad {
    fn gate_and_trim(&self, clip: &AudioClip) -> VadOutcome {
        // Fail-open: the detector only understands 16 kHz — a VAD limitation can
        // never discard the user's dictation (passes whole, without compression).
        if clip.sample_rate != 16_000 {
            return VadOutcome::Speech { clip: clip.clone(), speech_ms: clip.duration_ms };
        }

        // Detector on the heap: ~8 KiB of state, and a fresh one per clip avoids
        // leaking context from one recording into the next.
        let mut detector = earshot::Detector::default_boxed();
        let speech: Vec<bool> = clip
            .samples
            .chunks_exact(FRAME_SAMPLES)
            .map(|frame| detector.predict_i16(frame) > SPEECH_THRESHOLD)
            .collect();

        let speech_frames = speech.iter().filter(|&&s| s).count() as u64;
        if speech_frames == 0 {
            return VadOutcome::NoSpeech;
        }

        let samples =
            trim_and_compress_pauses(&clip.samples, &speech, self.compress_pauses_over_ms);
        let duration_ms = samples.len() as u64 * 1000 / clip.sample_rate.max(1) as u64;

        VadOutcome::Speech {
            clip: AudioClip { samples, sample_rate: clip.sample_rate, duration_ms },
            // speech_ms measures real speech (speech frames × 16 ms) — the use
            // case's <300 ms gate is not affected by the compression.
            speech_ms: speech_frames * FRAME_MS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_silence_becomes_no_speech() {
        let clip = AudioClip {
            samples: vec![0i16; 32_000],
            sample_rate: 16_000,
            duration_ms: 2_000,
        };
        assert!(matches!(
            EarshotVad::new(Some(2000)).gate_and_trim(&clip),
            VadOutcome::NoSpeech
        ));
    }

    #[test]
    fn rate_other_than_16k_passes_through_whole_as_speech() {
        // Fail-open: the VAD cannot discard (nor compress) what it does not understand.
        let clip = AudioClip {
            samples: vec![0i16; 48_000],
            sample_rate: 48_000,
            duration_ms: 1_000,
        };
        match EarshotVad::new(Some(2000)).gate_and_trim(&clip) {
            VadOutcome::Speech { clip: out, speech_ms } => {
                assert_eq!(out.samples.len(), 48_000);
                assert_eq!(speech_ms, 1_000);
            }
            VadOutcome::NoSpeech => panic!("fail-open violated"),
        }
    }

    // ---- Tests of the pure compression function ----

    /// Synthetic clip: each whole frame carries its own index as amplitude, to
    /// track which frames survived the compression.
    fn indexed_samples(total_frames: usize) -> Vec<i16> {
        (0..total_frames)
            .flat_map(|f| std::iter::repeat(f as i16).take(FRAME_SAMPLES))
            .collect()
    }

    /// Speech flags from stretches (start, exclusive end) in frames.
    fn flags(total_frames: usize, speech_runs: &[(usize, usize)]) -> Vec<bool> {
        let mut speech = vec![false; total_frames];
        for &(a, b) in speech_runs {
            speech[a..b].iter_mut().for_each(|s| *s = true);
        }
        speech
    }

    /// Frames present in the result (first sample of each frame).
    fn frames_of(samples: &[i16]) -> Vec<i16> {
        samples.chunks_exact(FRAME_SAMPLES).map(|f| f[0]).collect()
    }

    #[test]
    fn internal_gap_above_the_threshold_is_compressed_to_the_residue() {
        // 20 speech frames, 200 of silence (3200 ms > 2000), 20 of speech.
        let total = 240;
        let samples = indexed_samples(total);
        let speech = flags(total, &[(0, 20), (220, 240)]);

        let out = trim_and_compress_pauses(&samples, &speech, Some(2000));
        let kept = frames_of(&out);

        // Kept: speech 0..20 + 12 padding frames after (20..32),
        // then 12 frames before the next speech (208..220) + speech 220..240.
        let expected: Vec<i16> =
            (0..32).chain(208..240).map(|f| f as i16).collect();
        assert_eq!(kept, expected);
        // The total silence residue in place of the gap: 24 frames ≈ 384 ms.
        assert_eq!(kept.len(), 20 + PAUSE_RESIDUE_FRAMES + 20);
    }

    #[test]
    fn gap_at_or_below_the_threshold_stays_intact() {
        // 20 speech + 100 silence (1600 ms ≤ 2000) + 20 speech: nothing changes
        // (speech at the edges ⇒ no trim either).
        let total = 140;
        let samples = indexed_samples(total);
        let speech = flags(total, &[(0, 20), (120, 140)]);

        let out = trim_and_compress_pauses(&samples, &speech, Some(2000));
        assert_eq!(out, samples);
    }

    #[test]
    fn config_none_compresses_nothing() {
        // Same huge gap: with None only the edge trim happens — and here the
        // speech is at the edges, so the clip comes back whole.
        let total = 240;
        let samples = indexed_samples(total);
        let speech = flags(total, &[(0, 20), (220, 240)]);

        let out = trim_and_compress_pauses(&samples, &speech, None);
        assert_eq!(out, samples);
    }

    #[test]
    fn edges_receive_asymmetric_trim() {
        // 50 silence + 20 speech + 50 silence: the start keeps 31 lead-in frames
        // (~500 ms), the end keeps 12 (~200 ms); the compression does not touch
        // the edges.
        let total = 120;
        let samples = indexed_samples(total);
        let speech = flags(total, &[(50, 70)]);

        let out = trim_and_compress_pauses(&samples, &speech, Some(2000));
        let kept = frames_of(&out);
        let expected: Vec<i16> = (19..82).map(|f| f as i16).collect();
        assert_eq!(kept, expected);
    }

    #[test]
    fn two_long_gaps_are_compressed_independently() {
        // speech 0..10, gap of 150 (2400 ms), speech 160..170, gap of 150,
        // speech 320..330.
        let total = 330;
        let samples = indexed_samples(total);
        let speech = flags(total, &[(0, 10), (160, 170), (320, 330)]);

        let out = trim_and_compress_pauses(&samples, &speech, Some(2000));
        let kept = frames_of(&out);
        let expected: Vec<i16> = (0..22)
            .chain(148..182)
            .chain(308..330)
            .map(|f| f as i16)
            .collect();
        assert_eq!(kept, expected);
    }

    #[test]
    fn no_speech_in_flags_returns_samples_intact() {
        // Fail-open of the pure function: the NoSpeech gate is the caller's decision.
        let samples = indexed_samples(40);
        let out = trim_and_compress_pauses(&samples, &vec![false; 40], Some(2000));
        assert_eq!(out, samples);
    }

    #[test]
    fn inconsistent_flags_with_samples_return_intact() {
        // Fail-open: an internal inconsistency never discards dictation.
        let samples = indexed_samples(40);
        let out = trim_and_compress_pauses(&samples, &vec![true; 7], Some(2000));
        assert_eq!(out, samples);
    }
}

#[cfg(test)]
mod debug_probs {
    use super::*;

    /// Temporary diagnostic: per-frame probabilities from the end of the
    /// recife-accent sample. Run with:
    /// cargo test -p wren-adapters --release -- --ignored probs --nocapture
    #[test]
    #[ignore]
    fn probs_at_the_end_of_the_accented_sample() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/golden/public/NordestePEsotaque.wav"
        );
        let mut reader = hound::WavReader::open(path).unwrap();
        let spec = reader.spec();
        let samples: Vec<i16> = reader.samples::<i16>().map(Result::unwrap).collect();
        let (norm, rate) = crate::preprocess::normalize(&samples, spec.sample_rate, spec.channels);
        assert_eq!(rate, 16_000);

        let mut detector = earshot::Detector::default_boxed();
        let probs: Vec<f32> = norm
            .chunks_exact(FRAME_SAMPLES)
            .map(|f| detector.predict_i16(f))
            .collect();
        let total = probs.len();
        println!("total {total} frames ({} ms)", total * 16);
        // Last 3.5 s (219 frames), 1 line per 10 frames (160 ms).
        let start = total.saturating_sub(219);
        for (li, chunk) in probs[start..].chunks(10).enumerate() {
            let t0 = (start + li * 10) * 16;
            let line: Vec<String> = chunk.iter().map(|p| format!("{p:.2}")).collect();
            println!("{:>6} ms: {}", t0, line.join(" "));
        }
        let last_05 = probs.iter().rposition(|&p| p > 0.5).unwrap();
        let last_03 = probs.iter().rposition(|&p| p > 0.3).unwrap();
        let last_015 = probs.iter().rposition(|&p| p > 0.15).unwrap();
        println!("last frame >0.5: {} ({} ms)", last_05, last_05 * 16);
        println!("last frame >0.3: {} ({} ms)", last_03, last_03 * 16);
        println!("last frame >0.15: {} ({} ms)", last_015, last_015 * 16);
    }
}
