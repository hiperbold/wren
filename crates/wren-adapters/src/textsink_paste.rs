//! Adapter for the `TextSink` port: delivers the text to the focused app via the
//! configured method (`PasteMethod`).
//!
//! - **Paste / Ctrl+Shift+V** (`Paste`, `CtrlShiftV`): put the text on the
//!   clipboard and emit the paste shortcut (enigo). They preserve accents/ç —
//!   the known pain of synthetic typing on X11 (doc 05). Ctrl+Shift+V is the
//!   path for terminals, which intercept Ctrl+V.
//! - **Type** (`Type`): synthetic key-by-key typing (does not touch the
//!   clipboard). Works where there is no paste, but suffers with accents on X11.
//! - **wtype** (`Wtype`): delegates to the `wtype` binary — the path for
//!   Wayland, where X11's enigo does not reach.
//!
//! ## Clipboard after delivery
//! Phase 0 decision: the transcribed text STAYS on the clipboard after pasting
//! (we do not restore the previous content) — if the automatic paste fails in
//! some app, the user can still paste manually. `restore_clipboard` makes the
//! restoration optional: when enabled, we save the previous content before
//! pasting and put it back afterward. It is best-effort and inherently subject
//! to a race on X11 (the target app fetches the selection asynchronously), which
//! is why we wait `RESTORE_DELAY` before restoring; it is also why it stays OFF
//! by default.

use std::thread::sleep;
use std::time::Duration;

use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};
use wren_core::{PasteMethod, PortError, TextSink};

/// Wait before restoring the previous clipboard, giving the target app time to
/// consume the selection we just pasted (X11 delivers the selection
/// asynchronously). Conservative on purpose — restoring too early would make the
/// target paste the old content.
const RESTORE_DELAY: Duration = Duration::from_millis(400);

pub struct ClipboardPasteSink {
    method: PasteMethod,
    restore_clipboard: bool,
}

impl ClipboardPasteSink {
    /// Default sink: paste via clipboard (Ctrl+V), without restoring the
    /// clipboard. It is what the tray uses in "Copy last transcription" (just
    /// `copy`) and what the tests exercise.
    pub fn new() -> Self {
        ClipboardPasteSink {
            method: PasteMethod::Paste,
            restore_clipboard: false,
        }
    }

    /// Sink configured from the user's preferences (composition).
    pub fn with_config(method: PasteMethod, restore_clipboard: bool) -> Self {
        ClipboardPasteSink {
            method,
            restore_clipboard,
        }
    }
}

impl Default for ClipboardPasteSink {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardPasteSink {
    /// Copies the text to the clipboard WITHOUT pasting (emits no shortcut). It
    /// is what the tray uses in "Copy last transcription": the user pastes
    /// manually, working around focus failures of the automatic paste. It is
    /// independent of the configured `PasteMethod` — it always only copies.
    /// Reuses the same X11 selection persistence used in `deliver`.
    pub fn copy(&self, text: &str) -> Result<(), PortError> {
        set_clipboard_persistent(text.to_string())
    }

    /// Pastes via clipboard: saves the previous clipboard (if `restore_clipboard`),
    /// puts the text, emits the `keys` shortcut and — when asked — restores the
    /// previous content. Shared by `Paste` (Ctrl+V) and `CtrlShiftV`.
    fn deliver_via_clipboard(&self, text: &str, keys: PasteShortcut) -> Result<(), PortError> {
        // Best-effort: only text is restorable; an empty clipboard or one holding
        // an image becomes `None` and the restoration is skipped.
        let previous = if self.restore_clipboard {
            read_clipboard_text()
        } else {
            None
        };

        set_clipboard_persistent(text.to_string())?;
        press_paste(keys)?;

        if let Some(previous) = previous {
            sleep(RESTORE_DELAY);
            // A failure to restore is not a delivery failure — the text was already pasted.
            let _ = set_clipboard_persistent(previous);
        }
        Ok(())
    }
}

/// Which paste shortcut to emit.
#[derive(Clone, Copy)]
enum PasteShortcut {
    /// Ctrl+V — standard paste.
    CtrlV,
    /// Ctrl+Shift+V — paste in terminals.
    CtrlShiftV,
}

/// Reads the current clipboard text, or `None` if it is empty/non-text or if the
/// clipboard does not open. Never fails the flow — it is used only for the
/// optional restoration.
fn read_clipboard_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// Emits the paste shortcut via enigo.
///
/// Modifier releases are attempted unconditionally, even if an earlier step
/// (the Press, or the Unicode 'v' Click — X11 synthetic Unicode input requires
/// a live keycode remap and can fail transiently) errored out. `.and_then()`
/// chaining would abandon the Release on any earlier failure, stranding the
/// modifier "held" in the X server's core keyboard state — state that belongs
/// to the X server, not this process, so it survives a Wren restart and only
/// clears on an X session/reboot.
fn press_paste(keys: PasteShortcut) -> Result<(), PortError> {
    let mut enigo = Enigo::new(&EnigoSettings::default())
        .map_err(|e| PortError::DeliveryFailed(e.to_string()))?;

    let modifiers: &[Key] = match keys {
        PasteShortcut::CtrlV => &[Key::Control],
        PasteShortcut::CtrlShiftV => &[Key::Control, Key::Shift],
    };

    let press_and_click = (|| {
        for &m in modifiers {
            enigo.key(m, Direction::Press)?;
        }
        enigo.key(Key::Unicode('v'), Direction::Click)
    })();

    let mut release_err = None;
    for &m in modifiers.iter().rev() {
        if let Err(e) = enigo.key(m, Direction::Release) {
            release_err.get_or_insert(e);
        }
    }

    press_and_click
        .and(release_err.map_or(Ok(()), Err))
        .map_err(|e| PortError::DeliveryFailed(e.to_string()))
}

/// Direct synthetic typing (does not use the clipboard). Accents/layout may come
/// out wrong on X11 (doc 05) — it is the price of not relying on pasting.
fn deliver_via_typing(text: &str) -> Result<(), PortError> {
    let mut enigo = Enigo::new(&EnigoSettings::default())
        .map_err(|e| PortError::DeliveryFailed(e.to_string()))?;
    enigo
        .text(text)
        .map_err(|e| PortError::DeliveryFailed(e.to_string()))
}

/// Delegates the typing to the `wtype` binary (Wayland). Fails clearly if
/// `wtype` is not installed — the message becomes error feedback to the user.
fn deliver_via_wtype(text: &str) -> Result<(), PortError> {
    // `--` ends the flags: a text starting with '-' is not read as an option.
    let status = std::process::Command::new("wtype")
        .arg("--")
        .arg(text)
        .status()
        .map_err(|e| {
            PortError::DeliveryFailed(format!(
                "could not run 'wtype' (install wtype to paste on Wayland): {e}"
            ))
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(PortError::DeliveryFailed(format!(
            "'wtype' exited with an error ({status})"
        )))
    }
}

/// Puts the text on the clipboard and ensures it persists.
///
/// On X11 the clipboard is served by the process that OWNS the selection: if
/// arboard's `Clipboard` is dropped, the text disappears (unless there is a
/// clipboard manager). The thread with `.wait()` serves the selection until
/// another app takes over the clipboard.
fn set_clipboard_persistent(text: String) -> Result<(), PortError> {
    #[cfg(target_os = "linux")]
    {
        use arboard::SetExtLinux;
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
        std::thread::spawn(move || {
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => {
                    let _ = ready_tx.send(Ok(()));
                    // Blocks serving the selection until another owner appears.
                    let _ = clipboard.set().wait().text(text);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e.to_string()));
                }
            }
        });
        ready_rx
            .recv()
            .map_err(|_| PortError::DeliveryFailed("clipboard thread died".into()))?
            .map_err(PortError::DeliveryFailed)?;
        // A little slack for the thread to commit the set() before Ctrl+V.
        sleep(Duration::from_millis(120));
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| PortError::DeliveryFailed(e.to_string()))?;
        clipboard
            .set_text(text)
            .map_err(|e| PortError::DeliveryFailed(e.to_string()))?;
        sleep(Duration::from_millis(120));
        Ok(())
    }
}

impl TextSink for ClipboardPasteSink {
    fn deliver(&self, text: &str) -> Result<(), PortError> {
        match self.method {
            PasteMethod::Paste => self.deliver_via_clipboard(text, PasteShortcut::CtrlV),
            PasteMethod::CtrlShiftV => self.deliver_via_clipboard(text, PasteShortcut::CtrlShiftV),
            PasteMethod::Type => deliver_via_typing(text),
            PasteMethod::Wtype => deliver_via_wtype(text),
        }
    }
}
