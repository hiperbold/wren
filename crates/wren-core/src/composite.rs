//! `CompositeFeedback`: a generic fan-out over the `Feedback` port. It lets
//! several feedback adapters run side by side (e.g. the visual overlay and the
//! audible tones) without the use case knowing there is more than one — it
//! still receives a single `Arc<dyn Feedback>`.
//!
//! Fail-open by composition: each inner is already responsible for not blocking
//! or panicking (the `Feedback` port contract); the composite just forwards to
//! each one.

use std::sync::Arc;

use crate::domain::Transcript;
use crate::ports::Feedback;

/// Dispatches each `Feedback` port call to all inners, in order.
pub struct CompositeFeedback {
    inners: Vec<Arc<dyn Feedback>>,
}

impl CompositeFeedback {
    pub fn new(inners: Vec<Arc<dyn Feedback>>) -> Self {
        CompositeFeedback { inners }
    }
}

impl Feedback for CompositeFeedback {
    fn recording_started(&self) {
        for inner in &self.inners {
            inner.recording_started();
        }
    }

    fn audio_level(&self, level: f32) {
        for inner in &self.inners {
            inner.audio_level(level);
        }
    }

    fn transcribing(&self) {
        for inner in &self.inners {
            inner.transcribing();
        }
    }

    fn finished(&self, transcript: &Transcript) {
        for inner in &self.inners {
            inner.finished(transcript);
        }
    }

    fn failed(&self, message: &str) {
        for inner in &self.inners {
            inner.failed(message);
        }
    }

    fn cancelled(&self) {
        for inner in &self.inners {
            inner.cancelled();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Fake that counts how many times each method was called.
    #[derive(Default)]
    struct CountingFeedback {
        started: AtomicUsize,
        levels: AtomicUsize,
        transcribing: AtomicUsize,
        finished: AtomicUsize,
        failed: AtomicUsize,
        cancelled: AtomicUsize,
    }

    impl Feedback for CountingFeedback {
        fn recording_started(&self) {
            self.started.fetch_add(1, Ordering::SeqCst);
        }
        fn audio_level(&self, _level: f32) {
            self.levels.fetch_add(1, Ordering::SeqCst);
        }
        fn transcribing(&self) {
            self.transcribing.fetch_add(1, Ordering::SeqCst);
        }
        fn finished(&self, _transcript: &Transcript) {
            self.finished.fetch_add(1, Ordering::SeqCst);
        }
        fn failed(&self, _message: &str) {
            self.failed.fetch_add(1, Ordering::SeqCst);
        }
        fn cancelled(&self) {
            self.cancelled.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// A single call to the composite reaches ALL inners.
    #[test]
    fn fan_out_reaches_all_inners() {
        let a = Arc::new(CountingFeedback::default());
        let b = Arc::new(CountingFeedback::default());
        let composite = CompositeFeedback::new(vec![a.clone(), b.clone()]);

        composite.recording_started();
        composite.audio_level(0.5);
        composite.transcribing();
        composite.failed("network error");
        composite.cancelled();

        for inner in [&a, &b] {
            assert_eq!(inner.started.load(Ordering::SeqCst), 1);
            assert_eq!(inner.levels.load(Ordering::SeqCst), 1);
            assert_eq!(inner.transcribing.load(Ordering::SeqCst), 1);
            assert_eq!(inner.failed.load(Ordering::SeqCst), 1);
            assert_eq!(inner.cancelled.load(Ordering::SeqCst), 1);
        }
    }
}
