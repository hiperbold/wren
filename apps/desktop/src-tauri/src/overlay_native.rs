//! Native bubble overlay (decision 1b, doc 06): a Tauri window **without a
//! webview** (feature `unstable`) + wgpu drawing the pill with a waveform. No
//! WebKit enters the dictation path; the GPU context lives only while the bubble
//! exists and is destroyed with it.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, PhysicalPosition};

pub const BAR_COUNT: usize = 36;

// Bubble states (mirror the webview overlay).
pub const STATE_RECORDING: u8 = 0;
pub const STATE_TRANSCRIBING: u8 = 1;
pub const STATE_DONE: u8 = 2;
pub const STATE_ERROR: u8 = 3;

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
}

impl NativeOverlay {
    /// Creates the window (call on the main thread) and starts the render thread.
    pub fn open(
        app: &AppHandle,
        level: Arc<AtomicU32>,
        state: Arc<AtomicU8>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let window = tauri::window::WindowBuilder::new(app, "overlay-native")
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
        apply_pill_shape(&window);

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
        })
    }

    /// Stops rendering, releases the GPU, and destroys the window. Can be called
    /// from any thread.
    pub fn close(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.render_thread.take() {
            let _ = handle.join(); // ensures surface/device are dropped before the window
        }
        let window = self.window.clone();
        let app = self.window.app_handle().clone();
        let _ = app.run_on_main_thread(move || {
            let _ = window.destroy();
        });
    }
}

/// Undoes the X Shape clipping (used when the backend has real alpha —
/// the shape would cut off the edge anti-aliasing). Call on the main thread.
#[cfg(target_os = "linux")]
fn clear_pill_shape(window: &tauri::window::Window) {
    use gtk::prelude::*;
    let Ok(gtk_window) = window.gtk_window() else { return };
    let Some(gdk_window) = gtk_window.window() else { return };
    gdk_window.shape_combine_region(None, 0, 0);
}

/// Clips the window (X Shape) to the shape of the pill anchored at the base.
/// Call on the main thread, with the window already realized.
#[cfg(target_os = "linux")]
fn apply_pill_shape(window: &tauri::window::Window) {
    use gtk::prelude::*;

    let Ok(gtk_window) = window.gtk_window() else { return };
    let Some(gdk_window) = gtk_window.window() else { return };

    let w = WIDTH as i32;
    let pill_h = 72i32;
    let pill_top = HEIGHT as i32 - pill_h;
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

fn init_gpu(window: &tauri::window::Window) -> Result<Gpu, Box<dyn std::error::Error>> {
    use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
    let raw_display = window.display_handle()?.as_raw();
    let raw_window = window.window_handle()?.as_raw();

    // Candidate without alpha, kept WITHOUT creating a device (creating a device
    // from the proprietary driver costs retained memory; we only pay if it's chosen).
    let mut opaque_fallback: Option<(wgpu::Surface<'static>, wgpu::Adapter, wgpu::CompositeAlphaMode)> =
        None;

    // Order: Vulkan/Metal/DX12 → GL → SOFTWARE renderer (llvmpipe).
    // Real alpha beats hardware: on X11 hardware Vulkan only gives Opaque,
    // while llvmpipe gives PreMultiplied — and for 320x200 the CPU cost is
    // negligible. The bubble works even without a usable GPU.
    let attempts = [
        (wgpu::Backends::PRIMARY, false),
        (wgpu::Backends::GL, false),
        (wgpu::Backends::PRIMARY, true),
    ];
    for (backends, force_software) in attempts {
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
        let adapter = match pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: force_software,
                ..Default::default()
            },
        )) {
            Ok(a) => a,
            Err(e) => {
                log::warn!(
                    target: "wren::overlay",
                    "wgpu {backends:?}{}: no adapter: {e}",
                    if force_software { " (software)" } else { "" }
                );
                continue;
            }
        };

        let caps = surface.get_capabilities(&adapter);
        let alpha = pick_alpha(&caps);
        match alpha {
            Some(alpha_mode) => {
                // Candidate with real alpha: chosen, create the device.
                if let Some(gpu) = finish_gpu(surface, &adapter, alpha_mode, backends) {
                    return Ok(gpu);
                }
            }
            None if opaque_fallback.is_none() => {
                let alpha_mode = caps.alpha_modes[0];
                opaque_fallback = Some((surface, adapter, alpha_mode));
            }
            None => {} // we already have an opaque fallback; only worth it with alpha
        }
    }

    let (surface, adapter, alpha_mode) =
        opaque_fallback.ok_or("no GPU backend available")?;
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
        .find(|f| matches!(f, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm))
        .unwrap_or(caps.formats[0]);

    log::debug!(
        target: "wren::overlay",
        "wgpu: adapter={} alpha={:?}",
        adapter.get_info().name,
        alpha_mode
    );

    Some(Gpu { surface, device, queue, format, alpha_mode })
}

fn render_loop(
    window: tauri::window::Window,
    stop: Arc<AtomicBool>,
    level: Arc<AtomicU32>,
    state: Arc<AtomicU8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let Gpu { surface, device, queue, format, alpha_mode } = init_gpu(&window)?;

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
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode,
        color_space: wgpu::SurfaceColorSpace::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

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

    while !stop.load(Ordering::SeqCst) {
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

        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)
            | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                surface.configure(&device, &config);
                continue;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                std::thread::sleep(Duration::from_millis(16));
                continue;
            }
            other => {
                return Err(format!("unrecoverable surface: {other:?}").into());
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
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
        queue.submit([encoder.finish()]);
        queue.present(frame);

        // ~30 fps: enough for the wave (which rolls every 50 ms) and cuts the
        // CPU cost of software rendering in half.
        std::thread::sleep(Duration::from_millis(33));
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
