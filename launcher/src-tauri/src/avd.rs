//! Creates and tunes a virtual device for a given (device profile, Android
//! version) combo. Defaults are picked so it "just works for most games";
//! people can additionally pick their own device/version instead.

use crate::catalog;
use crate::sdk;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

pub fn avd_dir(name: &str) -> PathBuf {
    avd_home().join(format!("{name}.avd"))
}

pub fn exists(name: &str) -> bool {
    avd_dir(name).join("config.ini").exists()
}

pub fn ensure(
    sdk_root: &Path,
    device: &str,
    api: &str,
    on_progress: impl Fn(String),
) -> Result<(), String> {
    catalog::validate_device(device)?;
    catalog::validate_api(api)?;
    let name = catalog::avd_name(device, api);
    if exists(&name) {
        return Ok(());
    }

    on_progress(format!("Creating virtual device ({device}, Android {api})..."));
    let mut cmd = Command::new(sdk::avdmanager_path(sdk_root));
    // avdmanager takes no --sdk_root flag (unlike sdkmanager) — it reads the
    // SDK location from this env var instead, which matters when we're
    // pointed at our own bootstrapped SDK rather than a system-wide one.
    cmd.env("ANDROID_SDK_ROOT", sdk_root)
        .arg("create")
        .arg("avd")
        .arg("-n")
        .arg(&name)
        .arg("-k")
        .arg(catalog::system_image_id(api))
        .arg("-d")
        .arg(device)
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

    tune_for_gaming(&name)?;
    on_progress("Virtual device ready.".into());
    Ok(())
}

/// Bumps the freshly-created AVD's defaults from avdmanager's conservative
/// baseline (2GB RAM) to something that actually plays games well, scaled
/// to what the host machine has to give.
fn tune_for_gaming(name: &str) -> Result<(), String> {
    let config_path = avd_dir(name).join("config.ini");
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
    /// actually creates/updates the AVD for the default device/version.
    #[test]
    fn creates_and_tunes_gaming_avd() {
        let root = sdk::find_sdk_root();
        if !(sdk::is_engine_ready(&root) && sdk::is_image_ready(&root, catalog::DEFAULT_API)) {
            eprintln!("skipping: no ready Android SDK found at {}", root.display());
            return;
        }
        let name = catalog::avd_name(catalog::DEFAULT_DEVICE, catalog::DEFAULT_API);
        ensure(&root, catalog::DEFAULT_DEVICE, catalog::DEFAULT_API, |msg| {
            println!("[setup] {msg}")
        })
        .expect("ensure() should succeed");
        assert!(exists(&name), "AVD directory should exist after ensure()");

        let config = std::fs::read_to_string(avd_dir(&name).join("config.ini")).unwrap();
        assert!(config.contains("hw.gpu.enabled=yes"));
        assert!(config.contains("PlayStore.enabled=yes"));
        println!("{config}");
    }
}
