//! Creation of the **on-demand** windows. Nothing here is resident: the settings
//! window is built when opened and destroyed when closed (doc 07, state S0).
//! The bubble overlay is native and lives in `overlay_native.rs`.

use tauri::{window::Color, AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// Dark theme background (`#171513`) in RGBA. Painted on the window's **first
/// paint**, before the webview loads the HTML, to avoid the white flash.
const THEME_BG: Color = Color(0x17, 0x15, 0x13, 0xFF);

pub const SETTINGS_LABEL: &str = "settings";

/// Settings window — created when opened, destroyed when closed.
pub fn open_settings_window(app: &AppHandle) {
    if let Some(existing) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = existing.set_focus();
        return;
    }

    let result =
        WebviewWindowBuilder::new(app, SETTINGS_LABEL, WebviewUrl::App("index.html".into()))
            .title("Wren — Settings")
            // New layout (redesign): 200px sidebar + content centered up to 840px with
            // side padding — the old design's single 520px column cramped the navigation
            // and the data screens (History/Diagnostics). Size verified visually at
            // ~1000px width.
            .inner_size(1000.0, 780.0)
            .min_inner_size(720.0, 560.0)
            .resizable(true)
            // Dark background from the first paint — without this the webview flashes
            // white before the React bundle mounts and applies the theme.
            .background_color(THEME_BG)
            .build();

    if let Err(e) = result {
        log::error!(target: "wren::window", "failed to create settings window: {e}");
    }
}
