//! Spawns the actual Android emulator process, detached from the launcher UI.

use crate::avd::AVD_NAME;
use crate::sdk;
use std::path::Path;

pub fn launch(sdk_root: &Path) -> Result<(), String> {
    build_command(sdk_root)?
        .spawn()
        .map_err(|e| format!("failed to start emulator: {e}"))?;
    Ok(())
}

fn build_command(sdk_root: &Path) -> Result<std::process::Command, String> {
    let bin = sdk::emulator_path(sdk_root);
    if !bin.exists() {
        return Err(format!("emulator binary not found at {}", bin.display()));
    }
    // "-gpu auto" runs every host GPU driver against the emulator's bundled
    // blocklist and falls back to software rendering on anything it doesn't
    // recognize — confirmed on a real RTX 3090 with a recent driver: "auto"
    // silently used SwiftShader, while "host" correctly picked up the real
    // card. Default to forcing real hardware; leave an escape hatch env var
    // for the rare host where hardware rendering genuinely is broken.
    let gpu_mode = std::env::var("GAME_LAUNCHER_GPU_MODE").unwrap_or_else(|_| "host".into());
    let mut cmd = std::process::Command::new(bin);
    cmd.arg("-avd")
        .arg(AVD_NAME)
        .arg("-gpu")
        .arg(gpu_mode)
        .arg("-netdelay")
        .arg("none")
        .arg("-netspeed")
        .arg("full")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    Ok(cmd)
}
