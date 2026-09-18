//! Creates and tunes the one "just works for most games" virtual device we
//! ship by default. Power-user tweaks (more RAM, a second instance, etc.) are
//! left as later settings, not something we need to ask up front.

use crate::sdk::{self, SYSTEM_IMAGE_ID};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const AVD_NAME: &str = "gaming_phone";

/// Default-per-user AVD storage location (avdmanager's own default, honoring
/// ANDROID_AVD_HOME if the user already has it set).
fn avd_home() -> PathBuf {
    if let Ok(p) = std::env::var("ANDROID_AVD_HOME") {
        return PathBuf::from(p);
    }
    dirs::home_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(".android")
        .join("avd")
}

pub fn avd_dir() -> PathBuf {
    avd_home().join(format!("{AVD_NAME}.avd"))
}

pub fn exists() -> bool {
    avd_dir().join("config.ini").exists()
}

pub fn ensure(sdk_root: &Path, on_progress: impl Fn(String)) -> Result<(), String> {
    if exists() {
        return Ok(());
    }

    on_progress("Creating virtual device...".into());
    let mut cmd = Command::new(sdk::avdmanager_path(sdk_root));
    // avdmanager takes no --sdk_root flag (unlike sdkmanager) — it reads the
    // SDK location from this env var instead, which matters when we're
    // pointed at our own bootstrapped SDK rather than a system-wide one.
    cmd.env("ANDROID_SDK_ROOT", sdk_root)
        .arg("create")
        .arg("avd")
        .arg("-n")
        .arg(AVD_NAME)
        .arg("-k")
        .arg(SYSTEM_IMAGE_ID)
        .arg("-d")
        .arg("pixel_6")
        .arg("--force")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().map_err(|e| format!("avdmanager failed to start: {e}"))?;
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        // Declines avdmanager's "create a custom hardware profile?" prompt.
        let _ = stdin.write_all(b"no\n");
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("avdmanager exited with an error creating the device".into());
    }

    tune_for_gaming()?;
    on_progress("Virtual device ready.".into());
    Ok(())
}

/// Bumps the freshly-created AVD's defaults from avdmanager's conservative
/// baseline (2GB RAM) to something that actually plays games well, scaled
/// to what the host machine has to give.
fn tune_for_gaming() -> Result<(), String> {
    let config_path = avd_dir().join("config.ini");
    let contents = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;

    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_all();
    let host_cores = sys.cpus().len().max(1);
    let avd_cores = host_cores.clamp(2, 4);

    let mut sys_mem = sysinfo::System::new_all();
    sys_mem.refresh_memory();
    let host_ram_mb = sys_mem.total_memory() / 1024 / 1024;
    // Leave the host at least 4GB; give the AVD up to 4GB.
    let avd_ram_mb = if host_ram_mb > 12_000 { 4096 } else { 2048 };

    let mut out = String::new();
    for line in contents.lines() {
        if let Some((key, _)) = line.split_once('=') {
            let replacement = match key {
                "hw.ramSize" => Some(format!("hw.ramSize={avd_ram_mb}")),
                "hw.cpu.ncore" => Some(format!("hw.cpu.ncore={avd_cores}")),
                "hw.gpu.mode" => Some("hw.gpu.mode=auto".to_string()),
                "hw.gpu.enabled" => Some("hw.gpu.enabled=yes".to_string()),
                "disk.dataPartition.size" => Some("disk.dataPartition.size=8G".to_string()),
                // avdmanager's CLI doesn't set this from the image tag the way
                // Android Studio's AVD Manager GUI does — has to be patched in
                // by hand or Play Store never shows up on first boot.
                "PlayStore.enabled" => Some("PlayStore.enabled=yes".to_string()),
                "hw.keyboard" => Some("hw.keyboard=yes".to_string()),
                _ => None,
            };
            if let Some(r) = &replacement {
                out.push_str(r);
                out.push('\n');
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    std::fs::write(&config_path, out).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercises the real avdmanager + config tuning against whatever SDK is
    /// on the host. Skips (rather than fails) if no SDK is present, since CI
    /// machines won't have one; on a dev box with the SDK already set up this
    /// actually creates/updates the "gaming_phone" AVD.
    #[test]
    fn creates_and_tunes_gaming_avd() {
        let root = sdk::find_sdk_root();
        if !sdk::is_ready(&root) {
            eprintln!("skipping: no ready Android SDK found at {}", root.display());
            return;
        }
        ensure(&root, |msg| println!("[setup] {msg}")).expect("ensure() should succeed");
        assert!(exists(), "AVD directory should exist after ensure()");

        let config = std::fs::read_to_string(avd_dir().join("config.ini")).unwrap();
        assert!(config.contains("hw.gpu.enabled=yes"));
        assert!(config.contains("PlayStore.enabled=yes"));
        println!("{config}");
    }
}
