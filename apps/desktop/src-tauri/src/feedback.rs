//! Tauri adapter for the `Feedback` port: the native **bubble** (wgpu) at the
//! bottom center of the screen. Rules (docs 01 and 07):
//! - the bubble is created when the session starts and DESTROYED at the end
//!   (never stays resident);
//! - no method blocks the audio flow (window created asynchronously on the main
//!   thread; levels via atomics).
//!
//! The overlay is 100% native (decision 1b, doc 06) — no webview enters the
//! dictation path. If the GPU/window fails, the session continues without visual
//! feedback (log only). Audio feedback already exists as a separate adapter
//! (`wren_adapters::ToneFeedback`), run in parallel via `CompositeFeedback`
//! — which also covers this visual-failure case. This adapter remains
//! **visual only**.

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::AppHandle;
use wren_core::{Feedback, Transcript};

use crate::overlay_native::{
    NativeOverlay, STATE_DONE, STATE_ERROR, STATE_RECORDING, STATE_TRANSCRIBING,
};

pub struct TauriFeedback {
    app: AppHandle,
    // Shared with the overlay's render thread.
    level: Arc<AtomicU32>,
    state: Arc<AtomicU8>,
    overlay: Arc<Mutex<Option<NativeOverlay>>>,
}

impl TauriFeedback {
    pub fn new(app: AppHandle) -> Self {
        TauriFeedback {
            app,
            level: Arc::new(AtomicU32::new(0)),
            state: Arc::new(AtomicU8::new(STATE_RECORDING)),
            overlay: Arc::new(Mutex::new(None)),
        }
    }

    fn destroy_overlay_after(&self, delay: Duration) {
        let overlay = self.overlay.clone();
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            // `recording_started` opens the window asynchronously (queued on
            // the main thread); a very short session (e.g. "no speech
            // detected") can finish and land here before that closure has
            // actually run and stored the overlay. A single check-and-take
            // would then find nothing to destroy, silently leaking whatever
            // gets created moments later — poll briefly instead, so a
            // late-but-imminent creation still gets torn down.
            let mut slot = overlay.lock().unwrap().take();
            for _ in 0..10 {
                if slot.is_some() {
                    break;
                }
                drop(slot);
                std::thread::sleep(Duration::from_millis(30));
                slot = overlay.lock().unwrap().take();
            }
            if let Some(overlay) = slot {
                overlay.close();
            }
        });
    }
}

impl Feedback for TauriFeedback {
    fn recording_started(&self) {
        self.level.store(0f32.to_bits(), Ordering::SeqCst);
        self.state.store(STATE_RECORDING, Ordering::SeqCst);

        let level = self.level.clone();
        let state = self.state.clone();
        let slot = self.overlay.clone();
        // Asynchronous on purpose: capture has already started; the bubble
        // appears when it can (target ≤ 300 ms — doc 07).
        let handle = self.app.clone();
        // `run_on_main_thread` itself can fail to even schedule the closure
        // (e.g. the event loop's queue is gone) — without logging that, a
        // session would silently run with no visual feedback and leave no
        // trace anywhere (seen live: a real session with zero overlay-related
        // log output, success or failure).
        if let Err(e) = self.app.run_on_main_thread(move || {
            match NativeOverlay::open(&handle, level, state) {
                Ok(overlay) => {
                    // Defensive: if a previous session's destroy raced and
                    // never got to close what it had opened (see
                    // `destroy_overlay_after`), don't silently drop it here —
                    // that would leak its render thread and leave a stuck
                    // window on screen forever, since nothing else will ever
                    // reference it again. `open()` gives every window its
                    // own unique label, so this is just orphaned cleanup, not
                    // a collision to avoid. `close()` blocks (briefly) for
                    // destroy confirmation, so it runs off this main thread.
                    if let Some(previous) = slot.lock().unwrap().replace(overlay) {
                        std::thread::spawn(move || previous.close());
                    }
                }
                Err(e) => log::warn!(target: "wren::overlay", "overlay unavailable ({e}); session without visual feedback"),
            }
        }) {
            log::warn!(target: "wren::overlay", "failed to schedule overlay open on main thread ({e}); session without visual feedback");
        }
    }

    fn audio_level(&self, level: f32) {
        self.level.store(level.to_bits(), Ordering::SeqCst);
    }

    fn transcribing(&self) {
        self.state.store(STATE_TRANSCRIBING, Ordering::SeqCst);
    }

    fn finished(&self, _transcript: &Transcript) {
        self.state.store(STATE_DONE, Ordering::SeqCst);
        self.destroy_overlay_after(Duration::from_millis(400));
    }

    fn failed(&self, message: &str) {
        log::warn!(target: "wren::session", "session failed: {message}");
        self.state.store(STATE_ERROR, Ordering::SeqCst);
        self.destroy_overlay_after(Duration::from_millis(1600));
    }

    fn cancelled(&self) {
        self.destroy_overlay_after(Duration::ZERO);
    }
}
