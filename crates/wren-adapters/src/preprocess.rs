//! Audio preprocessing (doc 08 §§1–2): normalizes the capture to the Phase 1
//! canonical form (PCM i16 mono 16 kHz) and encodes FLAC for upload — lossless
//! at ~50–60% of the WAV size, accepted by the Whisper APIs.

use rubato::{Fft, FixedSync, Resampler};
use wren_core::PortError;

/// Canonical rate: it is what Whisper consumes internally — sending more just
/// wastes bandwidth (doc 08 §1).
pub const TARGET_RATE: u32 = 16_000;

/// Downmix to mono (channel average) + resample to 16 kHz.
/// Returns `(samples_16k_mono, 16_000)`; if the input is already in canonical
/// form, it passes through at no cost.
pub fn normalize(samples: &[i16], sample_rate: u32, channels: u16) -> (Vec<i16>, u32) {
    let mono = downmix_to_mono(samples, channels);
    if sample_rate == TARGET_RATE {
        return (mono, TARGET_RATE);
    }
    (resample_to_16k(&mono, sample_rate), TARGET_RATE)
}

fn downmix_to_mono(samples: &[i16], channels: u16) -> Vec<i16> {
    let channels = channels.max(1) as usize;
    if channels == 1 {
        return samples.to_vec();
    }
    samples
        .chunks_exact(channels)
        .map(|frame| {
            let sum: i32 = frame.iter().map(|&s| s as i32).sum();
            (sum / channels as i32) as i16
        })
        .collect()
}

fn resample_to_16k(mono: &[i16], in_rate: u32) -> Vec<i16> {
    use audioadapter_buffers::direct::InterleavedSlice;

    let input: Vec<f32> = mono.iter().map(|&s| s as f32 / 32768.0).collect();

    // Synchronous fixed-ratio Fft: more than enough quality for speech and
    // stateless across clips (a fresh resampler per clip is cheap next to
    // recording).
    let chunk = 1024;
    let mut resampler = match Fft::<f32>::new(
        in_rate as usize,
        TARGET_RATE as usize,
        chunk,
        1,
        1,
        FixedSync::Input,
    ) {
        Ok(r) => r,
        // Exotic rate that rubato refuses: better to deliver the original audio
        // (let the provider resample itself) than to lose the dictation.
        Err(_) => return mono.to_vec(),
    };

    let n_out = resampler.process_all_needed_output_len(input.len());
    let mut out = vec![0.0f32; n_out];
    let input_adapter = match InterleavedSlice::new(input.as_slice(), 1, input.len()) {
        Ok(a) => a,
        Err(_) => return mono.to_vec(),
    };
    let mut output_adapter = match InterleavedSlice::new_mut(out.as_mut_slice(), 1, n_out) {
        Ok(a) => a,
        Err(_) => return mono.to_vec(),
    };

    let written = match resampler.process_all_into_buffer(
        &input_adapter,
        &mut output_adapter,
        input.len(),
        None,
    ) {
        Ok((_read, written)) => written,
        Err(_) => return mono.to_vec(),
    };
    out.truncate(written);

    out.iter().map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16).collect()
}

/// Encodes PCM i16 mono as FLAC in memory (smaller upload than WAV,
/// lossless — doc 08 §2).
pub fn encode_flac(samples: &[i16], sample_rate: u32) -> Result<Vec<u8>, PortError> {
    use flacenc::component::BitRepr;
    use flacenc::error::Verify;

    let flac_err = |e: &dyn std::fmt::Debug| PortError::Other(format!("encode FLAC: {e:?}"));

    // The flacenc API takes i32 even for 16-bit (bits_per_sample decides).
    let samples_i32: Vec<i32> = samples.iter().map(|&s| s as i32).collect();
    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|e| flac_err(&e))?;
    let source =
        flacenc::source::MemSource::from_samples(&samples_i32, 1, 16, sample_rate as usize);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|e| flac_err(&e))?;

    let mut sink = flacenc::bitsink::ByteSink::new();
    stream.write(&mut sink).map_err(|e| flac_err(&e))?;
    Ok(sink.as_slice().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_stereo_averages_the_channels() {
        // Interleaved L/R frames: [100, 200], [-50, 50], [0, 1000]
        let stereo = [100i16, 200, -50, 50, 0, 1000];
        let (mono, rate) = normalize(&stereo, 16_000, 2);
        assert_eq!(rate, 16_000);
        assert_eq!(mono, vec![150, 0, 500]);
    }

    #[test]
    fn resample_48k_to_16k_produces_a_third_of_the_samples() {
        // 1 s of 440 Hz sine wave @ 48 kHz.
        let input: Vec<i16> = (0..48_000)
            .map(|i| {
                let t = i as f32 / 48_000.0;
                ((t * 440.0 * std::f32::consts::TAU).sin() * 16_000.0) as i16
            })
            .collect();
        let (out, rate) = normalize(&input, 48_000, 1);
        assert_eq!(rate, 16_000);
        let expected = 16_000f64;
        assert!(
            (out.len() as f64 - expected).abs() / expected < 0.05,
            "expected ~16000 samples, got {}",
            out.len()
        );
    }

    #[test]
    fn sixteen_khz_mono_passes_through_unchanged() {
        let input = vec![1i16, 2, 3, 4, 5];
        let (out, rate) = normalize(&input, 16_000, 1);
        assert_eq!(rate, 16_000);
        assert_eq!(out, input);
    }

    #[test]
    fn encode_flac_starts_with_magic() {
        let samples = vec![0i16; 16_000];
        let flac = encode_flac(&samples, 16_000).unwrap();
        assert_eq!(&flac[..4], b"fLaC");
    }
}
