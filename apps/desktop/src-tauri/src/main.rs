// No console on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    // If this process was spawned as a transcription worker (hidden subcommand),
    // run inference and exit — BEFORE bringing up Tauri (see
    // docs/architecture/embedded-engine.md). In the app's normal flow, returns
    // None and continues.
    if let Some(code) = wren_desktop_lib::run_worker_if_invoked() {
        return code;
    }
    wren_desktop_lib::run();
    std::process::ExitCode::SUCCESS
}
