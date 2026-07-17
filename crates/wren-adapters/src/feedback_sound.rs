//! Adapter for the `Feedback` port that plays **synthesized feedback tones**:
//! recording start, completion/delivery, and error. It is the audible
//! counterpart of the visual overlay — and the safety net when the overlay's
//! GPU/window fails (the user still hears that it recorded/delivered/failed).
//!
//! ## Thread pattern (rodio is `!Send`/`!Sync`)
//! The rodio output device (`MixerDeviceSink`, which owns the cpal stream)
//! cannot cross threads. Instead of holding it in the struct, `ToneFeedback::new()`
//! spawns a **dedicated audio thread** that is the sole owner of the device and
//! runs a loop consuming `Cue`s from an `mpsc` channel. The struct holds only
//! the `Sender<Cue>` (Send + Sync + Clone), so the trait methods just do a
//! non-blocking `send`.
//!
//! ## Fail-open (Wren philosophy)
//! If opening the audio device fails, the thread logs once and exits — the
//! `Receiver` is dropped and subsequent `send`s become silent no-ops. It never
//! panics nor propagates an error: a failure in the sound subsystem NEVER
//! brings dictation down.

use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use rodio::source::SineWave;
use rodio::Source;
use wren_core::{Feedback, Transcript};

/// The three sound events the audio thread knows how to play.
enum Cue {
    Start,
    Done,
    Error,
}

/// A short note: frequency (Hz) and duration (ms).
struct Note {
    freq: f32,
    ms: u64,
}

/// Low, discreet amplitude — feedback, not an alarm.
const AMPLITUDE: f32 = 0.18;

pub struct ToneFeedback {
    tx: Sender<Cue>,
}

impl ToneFeedback {
    /// Brings up the dedicated audio thread. Always returns a usable instance:
    /// if audio does not open, the methods become no-ops (fail-open).
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<Cue>();
        // If even the thread fails to start, `rx` is dropped along with the
        // closure and the `send`s become no-ops — same outcome as fail-open.
        let _ = std::thread::Builder::new()
            .name("wren-tone-feedback".into())
            .spawn(move || audio_loop(rx));
        ToneFeedback { tx }
    }
}

impl Default for ToneFeedback {
    fn default() -> Self {
        Self::new()
    }
}

impl Feedback for ToneFeedback {
    fn recording_started(&self) {
        let _ = self.tx.send(Cue::Start);
    }

    /// Fires on every audio frame — NEVER play a sound here.
    fn audio_level(&self, _level: f32) {}

    /// We chose to mark start + end + error; the transition to transcribing
    /// lives only in the visual overlay.
    fn transcribing(&self) {}

    fn finished(&self, _transcript: &Transcript) {
        let _ = self.tx.send(Cue::Done);
    }

    fn failed(&self, _message: &str) {
        let _ = self.tx.send(Cue::Error);
    }

    /// Cancellation discards the audio without delivering — no tone (the overlay
    /// disappears).
    fn cancelled(&self) {}
}

/// Error callback for the output stream. The feedback sink stays **open for the
/// whole lifetime** (avoids the latency of reopening on every tone), so the
/// ALSA/PipeWire backend reports *buffer underrun/overrun* continuously while it
/// is idle — benign, but rodio's default callback does an `eprintln!` for each
/// one and **floods the log**. Here: we silence underrun/overrun and log only
/// stream errors that actually matter (device gone, invalid config).
fn on_stream_error(err: rodio::cpal::StreamError) {
    use rodio::cpal::StreamError;
    match err {
        StreamError::BufferUnderrun => {}
        other => {
            log::warn!(target: "wren::sound", "feedback audio stream error: {other}")
        }
    }
}

/// Dedicated thread loop: owns the output device, plays each `Cue`.
fn audio_loop(rx: Receiver<Cue>) {
    // Opens on the default device with our error callback (see `on_stream_error`).
    // Without `open_default_sink()`'s multi-fallback on purpose: for feedback
    // tones, if the default device does not open, fail-open is enough.
    let sink = match rodio::DeviceSinkBuilder::from_default_device()
        .and_then(|b| b.with_error_callback(on_stream_error).open_stream())
    {
        Ok(sink) => sink,
        Err(e) => {
            // Fail-open: logs once; `rx` is dropped here and the sends become no-ops.
            log::warn!(target: "wren::sound", "feedback audio unavailable ({e}); dictation continues without tones");
            return;
        }
    };
    let mixer = sink.mixer();

    // `recv` blocks until the next Cue; leaves the loop when all Senders are
    // dropped (ToneFeedback destroyed when the service is rebuilt).
    while let Ok(cue) = rx.recv() {
        let notes: &[Note] = match cue {
            // Start: a short, mid-range note.
            Cue::Start => &[Note { freq: 660.0, ms: 90 }],
            // Done: two ascending notes — sounds "completed".
            Cue::Done => &[Note { freq: 660.0, ms: 70 }, Note { freq: 880.0, ms: 120 }],
            // Error: a low, longer note, distinct from the others.
            Cue::Error => &[Note { freq: 233.0, ms: 180 }],
        };

        // One Player per burst; the thread is dedicated, so blocking on it during
        // the short tone is acceptable (it does not touch the dictation audio flow).
        let player = rodio::Player::connect_new(mixer);
        for note in notes {
            player.append(tone(note.freq, note.ms));
        }
        player.sleep_until_end();
    }
}

/// Synthesizes a discreet, click-free note: a sine wave capped in duration, with
/// a fast attack (`fade_in`) and a linear decay to silence (`fade_out` over the
/// whole duration). Both ends reach zero → no pops.
fn tone(freq: f32, ms: u64) -> impl Source + Send + 'static {
    let dur = Duration::from_millis(ms);
    // Short attack to kill the onset click, capped at ~15 ms.
    let attack = Duration::from_millis((ms / 4).clamp(5, 15));
    SineWave::new(freq)
        .take_duration(dur)
        .amplify(AMPLITUDE)
        .fade_out(dur)
        .fade_in(attack)
}
