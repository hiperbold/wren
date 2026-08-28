//! Native bubble overlay (decision 1b, doc 06): a Tauri window **without a
//! webview** (feature `unstable`) + wgpu drawing the pill with a waveform. No
//! WebKit enters the dictation path; the GPU context lives only while the bubble
//! exists and is destroyed with it.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, PhysicalPosition};
use wren_core::{GpuBackendLearning, SettingsStore};

pub const BAR_COUNT: usize = 36;

// Bubble states (mirror the webview overlay).
pub const STATE_RECORDING: u8 = 0;
pub const STATE_TRANSCRIBING: u8 = 1;
pub const STATE_DONE: u8 = 2;
pub const STATE_ERROR: u8 = 3;

// A fresh, unique label per window instead of one fixed label: Tauri only
// frees a destroyed window's label once its `Destroyed` event round-trips
// back through the event loop, which is NOT guaranteed to have happened by
// the time the next session tries to open — even one main-thread turn later
// (confirmed live: a "reclaim" attempt that destroys-then-immediately-builds
// still collided). A unique label per session sidesteps that race
// entirely — there is never anything for a new session to collide with.
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

// GTK/tao imposes a ~200px minimum window height on X11 (even without a
// webview); the window is larger and the shader anchors the 72px pill to the base.
const WIDTH: f64 = 320.0;
const HEIGHT: f64 = 200.0;
const BOTTOM_MARGIN: f64 = 48.0;

const DECAY: f32 = 0.82;
const GAIN: f32 = 4.5;

pub struct NativeOverlay {
    window: tauri::window::Window,
    stop: Arc<AtomicBool>,
    render_thread: Option<JoinHandle<()>>,
    // Signalled when the window's `Destroyed` event round-trips back through
    // the event loop — `destroy()` only asks for that to happen, it doesn't
    // wait for it, and Tauri only frees the window's label once it does.
    // `close()` blocks on this (briefly) so the label is genuinely free by
    // the time it returns, instead of racing the next session's `open()`.
    destroyed_rx: std::sync::mpsc::Receiver<()>,
}

impl NativeOverlay {
    /// Creates the window (call on the main thread) and starts the render thread.
    pub fn open(
        app: &AppHandle,
        level: Arc<AtomicU32>,
        state: Arc<AtomicU8>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let label = format!("overlay-native-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed));
        let window = tauri::window::WindowBuilder::new(app, &label)
            .title("Wren")
            .inner_size(WIDTH, HEIGHT)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .focused(false)
            .shadow(false)
            .visible_on_all_workspaces(true)
            .visible(true)
            .build()?;

        let (destroyed_tx, destroyed_rx) = std::sync::mpsc::channel();
        window.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                let _ = destroyed_tx.send(());
            }
        });

        // `WindowBuilder::build()` returning does not guarantee GTK has
        // realized the underlying GdkWindow yet (X11 window creation is
        // asynchronous) — calling `set_position` before that happens can be
        // silently dropped (confirmed live: the window then sits at whatever
        // position X11 defaulted to, e.g. (0,0), and stays there — nothing
        // keeps re-asserting that position, it's simply that our own
        // set_position call never took effect). Force realization up front,
        // before positioning or any of the platform-specific tweaks below.
        #[cfg(target_os = "linux")]
        {
            use gtk::prelude::*;
            if let Ok(gtk_window) = window.gtk_window() {
                gtk_window.realize();
            }
        }

        log::debug!(
            target: "wren::overlay",
            "native overlay created: inner_size={:?} scale={:?}",
            window.inner_size(),
            window.scale_factor()
        );
        let _ = window.set_ignore_cursor_events(true);

        // Bottom center of the cursor's monitor (same logic as the webview overlay).
        let monitor = window
            .current_monitor()
            .ok()
            .flatten()
            .or_else(|| app.primary_monitor().ok().flatten());
        if let Some(monitor) = monitor {
            let scale = monitor.scale_factor();
            let size = monitor.size();
            let pos = monitor.position();
            let w = (WIDTH * scale) as i32;
            let h = (HEIGHT * scale) as i32;
            let x = pos.x + (size.width as i32 - w) / 2;
            let y = pos.y + size.height as i32 - h - (BOTTOM_MARGIN * scale) as i32;
            let _ = window.set_position(PhysicalPosition::new(x, y));
        }

        // tao's `.focused(false)` doesn't stop GTK from grabbing focus on X11
        // when mapping the window — and a focused overlay = Ctrl+V going to the
        // wrong window. These three calls ensure the user's app stays focused
        // throughout the whole session.
        #[cfg(target_os = "linux")]
        {
            use gtk::prelude::*;
            if let Ok(gtk_window) = window.gtk_window() {
                gtk_window.set_accept_focus(false);
                gtk_window.set_focus_on_map(false);
                gtk_window.set_type_hint(gtk::gdk::WindowTypeHint::Notification);
            }
        }

        // X11: Vulkan doesn't expose compositing alpha (only Opaque), so the
        // transparency of the corners/top comes from X Shape: the window is
        // CLIPPED to the pill shape. On Wayland/macOS/Windows the swapchain's
        // real alpha handles it and the shape is unnecessary.
        #[cfg(target_os = "linux")]
        {
            let size = window
                .inner_size()
                .unwrap_or(tauri::PhysicalSize::new(WIDTH as u32, HEIGHT as u32));
            apply_pill_shape(&window, size.width as i32, size.height as i32);
        }

        let stop = Arc::new(AtomicBool::new(false));
        let render_thread = {
            let window = window.clone();
            let stop = stop.clone();
            std::thread::spawn(move || {
                if let Err(e) = render_loop(window.clone(), stop, level, state) {
                    log::error!(target: "wren::overlay", "native overlay: render failed: {e}");
                    // Without rendering the window would be an invisible/black
                    // ghost on screen during the session — destroy it immediately.
                    let app = window.app_handle().clone();
                    let _ = app.run_on_main_thread(move || {
                        let _ = window.destroy();
                    });
                }
            })
        };

        Ok(NativeOverlay {
            window,
            stop,
            render_thread: Some(render_thread),
            destroyed_rx,
        })
    }

    /// Stops rendering, releases the GPU, and destroys the window. Can be called
    /// from any thread. Blocks briefly for the OS to confirm the window is
    /// actually gone (bounded — never blocks indefinitely) so the "overlay-native"
    /// label is genuinely free before this returns, instead of merely having
    /// asked for the destroy and hoping it lands before the next session opens.
    pub fn close(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.render_thread.take() {
            let _ = handle.join(); // ensures surface/device are dropped before the window
        }
        let _ = self.window.destroy();
        let _ = self.destroyed_rx.recv_timeout(Duration::from_millis(500));
    }
}

/// Undoes the X Shape clipping (used when the backend has real alpha —
/// the shape would cut off the edge anti-aliasing). Call on the main thread.
#[cfg(target_os = "linux")]
fn clear_pill_shape(window: &tauri::window::Window) {
    use gtk::prelude::*;
    let Ok(gtk_window) = window.gtk_window() else {
        return;
    };
    let Some(gdk_window) = gtk_window.window() else {
        return;
    };
    gdk_window.shape_combine_region(None, 0, 0);
    gdk_window.display().flush();
}

/// Clips the window (X Shape) to the shape of the pill anchored at the base
/// of a `width`×`height` (physical pixels) window. Call on the main thread.
///
/// Takes the size explicitly instead of assuming the `WIDTH`/`HEIGHT`
/// constants — confirmed live: GTK's ~200px X11 minimum-height negotiation
/// isn't always settled by the time this runs, caught mid-resize at e.g.
/// 320×237 instead of 320×200. The wgpu shader always anchors the pill to
/// `size.y - pill_h` using the window's REAL current size (`render_loop`'s
/// `config.height`), so a mask built from the stale hardcoded 200 would
/// clip a different band than where the shader actually draws — producing
/// a partially-covered pill with a black gap above it (the intermittent
/// "half pill" bug). Building the mask from the same size the caller used
/// for the swapchain keeps the two always in agreement, whatever that size
/// currently is.
#[cfg(target_os = "linux")]
fn apply_pill_shape(window: &tauri::window::Window, width: i32, height: i32) {
    use gtk::prelude::*;

    let Ok(gtk_window) = window.gtk_window() else {
        return;
    };
    // `open()` already forces realization up front; this second call is a
    // cheap no-op if so (GTK docs: realizing an already-realized widget does
    // nothing) and a defensive fallback if this is ever called on its own.
    gtk_window.realize();
    let Some(gdk_window) = gtk_window.window() else {
        log::warn!(
            target: "wren::overlay",
            "native overlay: GdkWindow still unrealized after realize() — pill shape not clipped this session"
        );
        return;
    };

    let w = width;
    let pill_h = 72i32.min(height);
    let pill_top = height - pill_h;
    let r = (pill_h as f64 / 2.0) - 1.0;

    // Rounded-rect by scanlines (1 rectangle per line in the caps).
    let mut rects = Vec::with_capacity(pill_h as usize);
    for y in 0..pill_h {
        let cy = y as f64 + 0.5;
        let dx = if cy < r {
            r - (r * r - (r - cy) * (r - cy)).sqrt()
        } else if cy > pill_h as f64 - r {
            let d = cy - (pill_h as f64 - r);
            r - (r * r - d * d).max(0.0).sqrt()
        } else {
            0.0
        };
        let x = dx.floor() as i32;
        rects.push(gtk::cairo::RectangleInt::new(x, pill_top + y, w - 2 * x, 1));
    }
    let region = gtk::cairo::Region::create_rectangles(&rects);
    gdk_window.shape_combine_region(Some(&region), 0, 0);
    // Without an explicit flush this request just sits in the client-side X11
    // output buffer until something else happens to flush the connection —
    // confirmed live: the window shows as an unclipped black rectangle (its
    // full bounding box) for ~300-400ms after becoming visible, only turning
    // into the pill once wgpu's own X11 traffic incidentally flushes it.
    gdk_window.display().flush();
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    levels: [[f32; 4]; 9], // BAR_COUNT=36 floats packed into vec4
    // x = time (s), y = state, z = physical width, w = physical height
    info: [f32; 4],
}

struct Gpu {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    alpha_mode: wgpu::CompositeAlphaMode,
    present_mode: wgpu::PresentMode,
}

/// `Fifo` (the only present mode required by the spec) blocks inside
/// `get_current_texture` until the compositor consumes a buffer — measured
/// live on a Linux/X11/NVIDIA/Mutter setup: it flatlines to ~10fps
/// (~99ms/frame) regardless of the display's real 60Hz refresh, with GPU
/// power state, window type hint, and monitor refresh rate all ruled out as
/// the cause, and a brief speed-up observed right after keyboard/X11
/// activity — this looks like Mutter throttling its own idle recomposite
/// rate for a small background overlay, not a hardware or driver limit.
/// **Scoped to Linux on purpose**: this workaround has only been verified
/// there; Windows/macOS haven't shown this stall, and unconditionally
/// preferring a non-blocking mode everywhere would trade a proven Linux fix
/// for unverified tearing risk on platforms where `Fifo` was never the
/// problem. `Mailbox` would be the ideal non-blocking mode but isn't in this
/// surface's supported list (confirmed live: `[Fifo, FifoRelaxed,
/// Immediate]`); `Immediate` is the next best — it never blocks
/// acquire/present either, at the cost of possible tearing, which fits our
/// own ~30fps sleep-based pacing in `render_loop` far better than depending
/// on whatever cadence Fifo's fallback grants.
fn pick_present_mode(caps: &wgpu::SurfaceCapabilities) -> wgpu::PresentMode {
    #[cfg(target_os = "linux")]
    {
        [wgpu::PresentMode::Mailbox, wgpu::PresentMode::Immediate]
            .into_iter()
            .find(|m| caps.present_modes.contains(m))
            .unwrap_or(wgpu::PresentMode::Fifo)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = caps;
        wgpu::PresentMode::Fifo
    }
}

fn pick_alpha(caps: &wgpu::SurfaceCapabilities) -> Option<wgpu::CompositeAlphaMode> {
    [
        wgpu::CompositeAlphaMode::PreMultiplied,
        wgpu::CompositeAlphaMode::PostMultiplied,
    ]
    .into_iter()
    .find(|m| caps.alpha_modes.contains(m))
}

/// Initializes the GPU trying backends in order of TRANSPARENCY preference: on
/// X11 Vulkan usually exposes only `Opaque`, while GL (EGL) inherits the ARGB
/// visual of the GTK window. If no backend gives alpha, it uses the first one
/// that works (opaque pill, with a warning).
/// tao opens a NEW X11 connection on each `display_handle()` (and leaks it, so
/// the pointer stays valid). We capture display+window ONCE and use the same
/// values in the instance and the surface — otherwise wgpu's equality check
/// fails and the GL backend never initializes.
#[derive(Debug)]
struct CapturedDisplay(raw_window_handle::RawDisplayHandle);
impl raw_window_handle::HasDisplayHandle for CapturedDisplay {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Ok(unsafe { raw_window_handle::DisplayHandle::borrow_raw(self.0) })
    }
}
// X11 display pointer usable across threads (dedicated, leaked connection).
unsafe impl Send for CapturedDisplay {}
unsafe impl Sync for CapturedDisplay {}

/// Picks the GPU backend, learning along the way whether this machine's GL
/// attempt is worth trying at cold start (see `GpuBackendLearning`): loads the
/// learned state, runs `try_backends` accordingly, records the outcome if GL
/// was attempted, and — if skipping GL ever causes a total failure — resets
/// the learned state and retries once with GL back in the mix before giving
/// up, so a stale "skip GL" decision (e.g. after a GPU/driver change) can't
/// permanently break the overlay.
fn init_gpu(window: &tauri::window::Window) -> Result<Gpu, Box<dyn std::error::Error>> {
    use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
    let raw_display = window.display_handle()?.as_raw();
    let raw_window = window.window_handle()?.as_raw();

    let store = window
        .app_handle()
        .state::<crate::AppState>()
        .settings_store()
        .clone();
    let mut settings = store.load().unwrap_or_default();
    let learning = settings.gpu_backend_learning;

    let (result, gl_attempted, gl_wasted) =
        try_backends(raw_display, raw_window, learning.skip_gl_probe);

    if result.is_err() && learning.skip_gl_probe {
        log::warn!(
            target: "wren::overlay",
            "native overlay: cold start failed with the learned GL skip active; resetting gpu_backend_learning and retrying with GL included"
        );
        settings.gpu_backend_learning = GpuBackendLearning::default();
        let _ = store.save(&settings);
        let (retry_result, retry_gl_attempted, retry_gl_wasted) =
            try_backends(raw_display, raw_window, false);
        if retry_gl_attempted {
            settings.gpu_backend_learning =
                settings.gpu_backend_learning.record_sample(retry_gl_wasted);
            let _ = store.save(&settings);
        }
        return retry_result;
    }

    if gl_attempted {
        let updated = learning.record_sample(gl_wasted);
        if updated != learning {
            log::debug!(
                target: "wren::overlay",
                "gpu backend learning: sample recorded (sessions_observed={} gl_failures={} gl_wasted={gl_wasted}); skip_gl_probe now {}",
                updated.sessions_observed,
                updated.gl_failures,
                updated.skip_gl_probe,
            );
            settings.gpu_backend_learning = updated;
            let _ = store.save(&settings);
        }
    }

    result
}

/// Tries backends in order of TRANSPARENCY preference: on X11 Vulkan usually
/// exposes only `Opaque`, while GL (EGL) inherits the ARGB visual of the GTK
/// window. If no backend gives alpha, it uses the first one that works
/// (opaque pill, with a warning). `skip_gl` (set once `GpuBackendLearning`
/// converges on this machine) drops GL from the attempt list entirely.
///
/// Returns the `Gpu` result alongside whether GL was attempted this call and,
/// if so, whether the attempt was wasted (it errored, or it succeeded but
/// wasn't the candidate that actually ended up providing alpha) — the raw
/// signal `GpuBackendLearning::record_sample` needs.
///
/// tao opens a NEW X11 connection on each `display_handle()` (and leaks it, so
/// the pointer stays valid). We capture display+window ONCE and use the same
/// values in the instance and the surface — otherwise wgpu's equality check
/// fails and the GL backend never initializes.
fn try_backends(
    raw_display: raw_window_handle::RawDisplayHandle,
    raw_window: raw_window_handle::RawWindowHandle,
    skip_gl: bool,
) -> (Result<Gpu, Box<dyn std::error::Error>>, bool, bool) {
    // Candidate without alpha, kept WITHOUT creating a device (creating a device
    // from the proprietary driver costs retained memory; we only pay if it's chosen).
    let mut opaque_fallback: Option<(
        wgpu::Surface<'static>,
        wgpu::Adapter,
        wgpu::CompositeAlphaMode,
    )> = None;
    let mut gl_attempted = false;
    let mut gl_contributed_alpha = false;

    // Order: Vulkan/Metal/DX12 → GL. No forced-software (llvmpipe) attempt: it
    // reports a working PreMultiplied surface on X11, but the actual presented
    // frames never reach the screen there (confirmed live — the window sits
    // IsViewable, correctly positioned, GPU device alive, and shows nothing;
    // closing it afterward also hangs, since the render thread blocks forever
    // in `get_current_texture` waiting for a buffer that's never consumed).
    // Real hardware Vulkan only exposes `Opaque` on X11, so it falls through to
    // `opaque_fallback` below and relies on X Shape clipping instead of alpha.
    let attempts: &[wgpu::Backends] = if skip_gl {
        &[wgpu::Backends::PRIMARY]
    } else {
        &[wgpu::Backends::PRIMARY, wgpu::Backends::GL]
    };
    for &backends in attempts {
        if backends == wgpu::Backends::GL {
            gl_attempted = true;
        }
        // GL (EGL) needs the display in the instance; the other backends don't.
        let mut desc = if backends == wgpu::Backends::GL {
            wgpu::InstanceDescriptor::new_with_display_handle(Box::new(CapturedDisplay(
                raw_display,
            )))
        } else {
            wgpu::InstanceDescriptor::new_without_display_handle()
        };
        desc.backends = backends;
        let instance = wgpu::Instance::new(desc);

        // SAFETY: `window` (and the leaked X11 connection) outlive the surface —
        // render_loop keeps the Window until the end and drops the surface first.
        let surface = match unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: Some(raw_display),
                raw_window_handle: raw_window,
            })
        } {
            Ok(s) => s,
            Err(e) => {
                log::warn!(target: "wren::overlay", "wgpu {backends:?}: surface failed: {e}");
                continue;
            }
        };
        let adapter =
            match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                // A fresh device is created every session (never resident —
                // see feedback.rs), so `LowPower` risked leaving the driver in
                // a lower clock state between sessions, adding to the cold-start
                // delay/jank on the first frames of each one.
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })) {
                Ok(a) => a,
                Err(e) => {
                    log::warn!(target: "wren::overlay", "wgpu {backends:?}: no adapter: {e}");
                    continue;
                }
            };

        let caps = surface.get_capabilities(&adapter);
        let alpha = pick_alpha(&caps);
        match alpha {
            Some(alpha_mode) => {
                // Candidate with real alpha: chosen, create the device.
                if let Some(gpu) = finish_gpu(surface, &adapter, alpha_mode, backends) {
                    if backends == wgpu::Backends::GL {
                        gl_contributed_alpha = true;
                    }
                    return (Ok(gpu), gl_attempted, gl_attempted && !gl_contributed_alpha);
                }
            }
            None if opaque_fallback.is_none() => {
                let alpha_mode = caps.alpha_modes[0];
                opaque_fallback = Some((surface, adapter, alpha_mode));
            }
            None => {} // we already have an opaque fallback; only worth it with alpha
        }
    }

    let gl_wasted = gl_attempted && !gl_contributed_alpha;
    let result = match opaque_fallback {
        Some((surface, adapter, alpha_mode)) => {
            if cfg!(target_os = "linux") {
                // Expected on X11 (hardware Vulkan only exposes Opaque): the pill
                // clipping is done by X Shape in apply_pill_shape.
                log::debug!(target: "wren::overlay", "native overlay: alpha unavailable; using X Shape for the clipping");
            } else {
                log::warn!(target: "wren::overlay", "native overlay: no compositing alpha — pill without transparency");
            }
            finish_gpu(surface, &adapter, alpha_mode, wgpu::Backends::PRIMARY)
                .ok_or_else(|| "request_device failed on the fallback".into())
        }
        None => Err("no GPU backend available".into()),
    };
    (result, gl_attempted, gl_wasted)
}

/// Creates the device/queue for the chosen candidate and assembles the `Gpu`.
fn finish_gpu(
    surface: wgpu::Surface<'static>,
    adapter: &wgpu::Adapter,
    alpha_mode: wgpu::CompositeAlphaMode,
    backends: wgpu::Backends,
) -> Option<Gpu> {
    let (device, queue) =
        match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())) {
            Ok(pair) => pair,
            Err(e) => {
                // Without a log this would be an invisible failure (it's happened in the field).
                log::warn!(
                    target: "wren::overlay",
                    "wgpu {backends:?}: request_device failed ({}): {e}",
                    adapter.get_info().name
                );
                return None;
            }
        };

    let caps = surface.get_capabilities(adapter);
    let format = caps
        .formats
        .iter()
        .copied()
        .find(|f| {
            matches!(
                f,
                wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
            )
        })
        .unwrap_or(caps.formats[0]);
    let present_mode = pick_present_mode(&caps);

    log::debug!(
        target: "wren::overlay",
        "wgpu: adapter={} alpha={:?} present_mode={:?} available={:?}",
        adapter.get_info().name,
        alpha_mode,
        present_mode,
        caps.present_modes,
    );

    Some(Gpu {
        surface,
        device,
        queue,
        format,
        alpha_mode,
        present_mode,
    })
}

/// Process-wide RSS in bytes (`/proc/self/statm`, resident pages × 4 KiB).
/// Local to this file on purpose — this is ad-hoc render-perf diagnostics,
/// not the session-level metric already recorded by `wren_adapters::telemetry`.
#[cfg(target_os = "linux")]
fn read_rss_bytes() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(pages * 4096)
}

/// Process-wide CPU time (user+sys) in clock ticks, from `/proc/self/stat`
/// fields 14/15 (utime/stime) — indices 11/12 after splitting on the comm
/// field's closing `)`, since `comm` itself can contain spaces/parens.
/// Whole-process, not just the render thread: simplest without pulling in
/// `libc` for `gettid`/per-task `/proc` reads, and the render thread
/// dominates process CPU while the overlay is open.
#[cfg(target_os = "linux")]
fn read_process_cpu_ticks() -> Option<u64> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let after_comm = stat.rsplit_once(')')?.1;
    let fields: Vec<&str> = after_comm.split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some(utime + stime)
}

/// Rolling per-frame timing accumulator, flushed to a log line every
/// `WINDOW` frames (~2s at the 33ms target) instead of logging every frame.
#[derive(Default)]
struct FramePerf {
    count: u32,
    acquire_total: Duration,
    encode_total: Duration,
    present_total: Duration,
    frame_total: Duration,
    max_frame: Duration,
    retries: u32,
}

impl FramePerf {
    const WINDOW: u32 = 60;
    // 2x the 33ms frame budget: a single frame past this is a visible stutter,
    // not just normal vsync jitter — worth its own log line instead of waiting
    // for the windowed average to (maybe) hide it.
    const STUTTER_THRESHOLD: Duration = Duration::from_millis(70);

    fn record(&mut self, acquire: Duration, encode: Duration, present: Duration, total: Duration) {
        self.count += 1;
        self.acquire_total += acquire;
        self.encode_total += encode;
        self.present_total += present;
        self.frame_total += total;
        self.max_frame = self.max_frame.max(total);
        if total > Self::STUTTER_THRESHOLD {
            log::warn!(
                target: "wren::overlay::perf",
                "frame stutter: total={}ms (acquire={}ms encode={}ms present={}ms)",
                total.as_millis(),
                acquire.as_millis(),
                encode.as_millis(),
                present.as_millis(),
            );
        }
        if self.count >= Self::WINDOW {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.count == 0 {
            return;
        }
        let n = self.count as f64;
        log::debug!(
            target: "wren::overlay::perf",
            "frames={} avg_total={:.1}ms (acquire={:.1}ms encode={:.1}ms present={:.1}ms) max={}ms fps~{:.1} retries={}",
            self.count,
            self.frame_total.as_secs_f64() * 1000.0 / n,
            self.acquire_total.as_secs_f64() * 1000.0 / n,
            self.encode_total.as_secs_f64() * 1000.0 / n,
            self.present_total.as_secs_f64() * 1000.0 / n,
            self.max_frame.as_millis(),
            1000.0 / (self.frame_total.as_secs_f64() * 1000.0 / n).max(0.001),
            self.retries,
        );
        *self = FramePerf::default();
    }
}

fn render_loop(
    window: tauri::window::Window,
    stop: Arc<AtomicBool>,
    level: Arc<AtomicU32>,
    state: Arc<AtomicU8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Gpu {
        surface,
        device,
        queue,
        format,
        alpha_mode,
        present_mode,
    } = init_gpu(&window)?;

    // With real alpha the X Shape clipping is unnecessary (and would cut off the
    // edge anti-aliasing) — undo the one applied in open().
    #[cfg(target_os = "linux")]
    if alpha_mode != wgpu::CompositeAlphaMode::Opaque {
        let w = window.clone();
        let app = window.app_handle().clone();
        let _ = app.run_on_main_thread(move || clear_pill_shape(&w));
    }

    let size = window.inner_size()?;
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode,
        alpha_mode,
        color_space: wgpu::SurfaceColorSpace::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    // Re-assert the shape applied in open(): configuring the swapchain on
    // this window (DRI3/Vulkan buffer negotiation) resets the X Shape mask —
    // confirmed live, the window shows as its full unclipped bounding
    // rectangle (solid black) for hundreds of ms otherwise, only becoming the
    // pill once something else happened to touch the shape again.
    //
    // This used to be fire-and-forget (`let _ = run_on_main_thread(...)`)
    // with no wait for it to actually land — the render loop below started
    // presenting frames immediately after queuing it, racing the GTK main
    // thread to get a turn and run the closure. Confirmed live: with a fast
    // present mode the loop can get several frames out before the main
    // thread's next tick, each shown still shaped from BEFORE `configure`
    // reset it — a stale, differently-sized region from the previous
    // surface size (e.g. only partially covering the new one), i.e. exactly
    // the intermittent "half pill" symptom. Block here (bounded, so a stuck
    // main loop can't hang the render thread forever) until the shape is
    // confirmed re-applied before this thread presents anything.
    #[cfg(target_os = "linux")]
    if alpha_mode == wgpu::CompositeAlphaMode::Opaque {
        let reapply_start = Instant::now();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let w = window.clone();
        let app = window.app_handle().clone();
        let scheduled = app
            .run_on_main_thread({
                let (cfg_width, cfg_height) = (config.width as i32, config.height as i32);
                move || {
                    apply_pill_shape(&w, cfg_width, cfg_height);
                    let _ = done_tx.send(());
                }
            })
            .is_ok();
        if scheduled {
            match done_rx.recv_timeout(Duration::from_millis(500)) {
                Ok(()) => log::debug!(
                    target: "wren::overlay::perf",
                    "shape re-apply round-trip: {}ms",
                    reapply_start.elapsed().as_millis()
                ),
                Err(_) => log::warn!(
                    target: "wren::overlay",
                    "native overlay: shape re-apply didn't confirm within 500ms; proceeding anyway"
                ),
            }
        }
    }

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bubble"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bubble-uniforms"),
        size: std::mem::size_of::<Uniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&bind_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bubble"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    // Rolling waveform (same logic as the webview overlay).
    let mut levels = [0f32; BAR_COUNT];
    let mut current = 0f32;
    let started = Instant::now();
    let mut last_scroll = Instant::now();

    // Runtime perf instrumentation (target "wren::overlay::perf") — added
    // because "feels laggy / high CPU" reports had no hard numbers to
    // root-cause against. `session_cpu_ticks_start` is process-wide (see
    // `read_process_cpu_ticks`), sampled here so the window realization/GPU
    // init above isn't counted against the render loop itself.
    let mut perf = FramePerf::default();
    #[cfg(target_os = "linux")]
    let session_wall_start = Instant::now();
    #[cfg(target_os = "linux")]
    let session_cpu_ticks_start = read_process_cpu_ticks();

    while !stop.load(Ordering::SeqCst) {
        let frame_start = Instant::now();
        let t = started.elapsed().as_secs_f32();
        let st = state.load(Ordering::SeqCst);

        match st {
            STATE_RECORDING => {
                let raw = f32::from_bits(level.load(Ordering::SeqCst));
                let boosted = (raw * GAIN).min(1.0);
                current = boosted.max(current * DECAY);
                if last_scroll.elapsed() >= Duration::from_millis(50) {
                    last_scroll = Instant::now();
                    levels.rotate_left(1);
                    levels[BAR_COUNT - 1] = current;
                    current *= DECAY;
                }
            }
            STATE_TRANSCRIBING => {
                for (i, l) in levels.iter_mut().enumerate() {
                    *l = 0.18 + 0.14 * (t * 7.5 + i as f32 / 2.4).sin();
                }
            }
            _ => {} // done/error freeze the last waveform
        }

        let mut packed = [[0f32; 4]; 9];
        for (i, l) in levels.iter().enumerate() {
            packed[i / 4][i % 4] = *l;
        }
        let uniforms = Uniforms {
            levels: packed,
            info: [t, st as f32, config.width as f32, config.height as f32],
        };
        queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let acquire_start = Instant::now();
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)
            | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                perf.retries += 1;
                surface.configure(&device, &config);
                continue;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                perf.retries += 1;
                std::thread::sleep(Duration::from_millis(16));
                continue;
            }
            other => {
                return Err(format!("unrecoverable surface: {other:?}").into());
            }
        };
        let acquire_dur = acquire_start.elapsed();

        let encode_start = Instant::now();
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        let command_buffer = encoder.finish();
        let encode_dur = encode_start.elapsed();

        let present_start = Instant::now();
        queue.submit([command_buffer]);
        queue.present(frame);
        let present_dur = present_start.elapsed();

        // ~30 fps target: enough for the wave (which rolls every 50 ms) and
        // cuts CPU cost in half versus 60. With `Mailbox` neither acquire nor
        // present block for vsync, so this sleep is now the ONLY pacing —
        // sleep only the remainder of the frame budget, if any.
        let target = Duration::from_millis(33);
        let elapsed = frame_start.elapsed();
        perf.record(acquire_dur, encode_dur, present_dur, elapsed);
        if elapsed < target {
            std::thread::sleep(target - elapsed);
        }
    }

    perf.flush();
    #[cfg(target_os = "linux")]
    if let (Some(start_ticks), Some(end_ticks)) =
        (session_cpu_ticks_start, read_process_cpu_ticks())
    {
        let wall_ms = session_wall_start.elapsed().as_millis().max(1) as u64;
        // Linux/x86 clock ticks are virtually always 100Hz (`getconf CLK_TCK`);
        // assumed here instead of pulling in `libc` just for `sysconf`.
        let cpu_ms = end_ticks.saturating_sub(start_ticks) * 10;
        log::info!(
            target: "wren::overlay::perf",
            "overlay session ended: wall={wall_ms}ms process_cpu={cpu_ms}ms ({:.0}% of 1 core, whole process not just the render thread) rss={}",
            cpu_ms as f64 / wall_ms as f64 * 100.0,
            read_rss_bytes().map(|b| format!("{:.1}MB", b as f64 / (1024.0 * 1024.0))).unwrap_or_else(|| "n/a".into()),
        );
    }

    // Adjust if the window ever changes size (today it's fixed).
    let _ = &mut config;
    Ok(())
}

/// The whole pill in a single fragment shader: rounded-rect (SDF) + pulsing
/// state dot + 36 waveform bars. Premultiplied alpha output.
const SHADER: &str = r#"
struct Uniforms {
    levels: array<vec4<f32>, 9>,
    info: vec4<f32>, // x=time, y=state, z=width, w=height
};
@group(0) @binding(0) var<uniform> u: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    return vec4<f32>(pos[vi], 0.0, 1.0);
}

fn sd_rounded_box(p: vec2<f32>, half: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let size = u.info.zw;
    let s = size.x / 320.0;          // scale factor (320 logical px of width)
    let p = frag.xy;
    let t = u.info.x;
    let state = u32(u.info.y);

    // The window is taller than the pill (GTK floor); the 72 logical px pill
    // stays anchored at the base, the rest is transparent.
    let pill_h = 72.0 * s;
    let pill_top = size.y - pill_h;

    // Colors per state
    var bar_color = vec3<f32>(1.0, 0.698, 0.4);       // amber (recording/transcribing)
    var dot_color = vec3<f32>(1.0, 0.42, 0.37);       // recording red
    if (state == 1u) { dot_color = vec3<f32>(1.0, 0.698, 0.4); }
    if (state == 2u) { bar_color = vec3<f32>(0.47, 0.86, 0.59); dot_color = bar_color; }
    if (state == 3u) { bar_color = vec3<f32>(1.0, 0.42, 0.42); dot_color = bar_color; }

    var border_color = vec3<f32>(1.0, 0.698, 0.4);
    if (state == 3u) { border_color = vec3<f32>(1.0, 0.42, 0.42); }

    let center = vec2<f32>(size.x * 0.5, pill_top + pill_h * 0.5);
    let half = vec2<f32>(size.x * 0.5 - 1.5 * s, pill_h * 0.5 - 1.5 * s);
    let radius = half.y;
    let d = sd_rounded_box(p - center, half, radius);

    var color = vec3<f32>(0.0);
    var alpha = 0.0;

    // pill background
    let bg = smoothstep(1.0, -1.0, d);
    color = vec3<f32>(0.094, 0.086, 0.078);
    alpha = bg * 0.92;

    // border
    let border = smoothstep(1.5, 0.0, abs(d)) * 0.25;
    color = mix(color, border_color, border);
    alpha = max(alpha, bg * border);

    // state dot (pulsing), on the left
    let dot_center = vec2<f32>(22.0 * s, pill_top + pill_h * 0.5);
    let pulse = 0.8 + 0.2 * sin(t * 4.5);
    let dot_d = length(p - dot_center) - 4.5 * s * pulse;
    let dot = smoothstep(1.0, -1.0, dot_d) * bg;
    color = mix(color, dot_color, dot);
    alpha = max(alpha, dot * 0.95);

    // waveform: 36 bars between x0 and x1
    let x0 = 40.0 * s;
    let x1 = size.x - 20.0 * s;
    let gap = (x1 - x0) / 36.0;
    let bar_w = max(2.0 * s, gap * 0.55);
    if (p.x >= x0 && p.x < x1) {
        let fi = (p.x - x0) / gap;
        let i = u32(floor(fi));
        let lvl = clamp(u.levels[i / 4u][i % 4u], 0.06, 1.0);
        let bar_h = lvl * pill_h * 0.62;
        let bar_center = vec2<f32>(x0 + (floor(fi) + 0.5) * gap, pill_top + pill_h * 0.5);
        let bd = sd_rounded_box(
            p - bar_center,
            vec2<f32>(bar_w * 0.5, bar_h * 0.5),
            bar_w * 0.5,
        );
        let bar = smoothstep(0.8, -0.8, bd) * bg;
        color = mix(color, bar_color, bar);
        alpha = max(alpha, bar * 0.95);
    }

    // premultiplied alpha
    return vec4<f32>(color * alpha, alpha);
}
"#;
