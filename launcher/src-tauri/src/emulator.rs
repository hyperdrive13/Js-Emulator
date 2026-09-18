//! Spawns the Android emulator process, detached from the launcher UI.
//!
//! GPU handling: "-gpu auto" runs the host driver against the emulator's own
//! bundled compatibility blocklist and silently falls back to software
//! rendering on anything it doesn't recognize (reproduced on a real RTX 3090
//! with a current driver — logged as "Your GPU drivers may have a bug").
//! We force "-gpu host" instead, but that's not risk-free on its own: a
//! machine with a genuinely old/broken GPU could crash on real hardware
//! rendering where the blocklist's software fallback would have been the
//! safe choice. So: try host, watch for an early crash, and if it dies
//! quickly, retry once with software rendering — covers both ends without
//! asking anyone to know what "-gpu host" even means.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const EARLY_CRASH_WINDOW: Duration = Duration::from_secs(6);

/// Where the emulator's own stdout/stderr goes (overwritten each launch) —
/// so there's somewhere real to look when something goes wrong, instead of
/// discarding it.
pub fn log_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("android-game-launcher")
        .join("emulator.log")
}

pub fn launch(
    sdk_root: &Path,
    avd_name: &str,
    on_progress: impl Fn(String),
) -> Result<(), String> {
    let bin = crate::sdk::emulator_path(sdk_root);
    if !bin.exists() {
        return Err(format!("emulator binary not found at {}", bin.display()));
    }

    // Explicit override escape hatch for the rare host where even this
    // fallback logic doesn't help.
    if let Ok(forced) = std::env::var("GAME_LAUNCHER_GPU_MODE") {
        on_progress(format!("Launching with forced GPU mode '{forced}'..."));
        spawn(&bin, avd_name, &forced)?;
        return Ok(());
    }

    on_progress("Launching with hardware GPU acceleration...".into());
    let mut child = spawn(&bin, avd_name, "host")?;

    if let Some(exit_status) = wait_briefly(&mut child, EARLY_CRASH_WINDOW) {
        on_progress(format!(
            "Hardware-accelerated launch exited immediately ({exit_status}) — \
             retrying with software rendering (slower, but works everywhere)..."
        ));
        spawn(&bin, avd_name, "swiftshader_indirect")?;
    }
    Ok(())
}

fn spawn(bin: &Path, avd_name: &str, gpu_mode: &str) -> Result<Child, String> {
    let log_path = log_path();
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Truncate once, then reopen twice in *append* mode for the stdout/stderr
    // handles. Two separate File::create handles on the same path would each
    // track their own write offset starting at 0 and clobber each other;
    // O_APPEND makes every write atomically land at end-of-file instead, so
    // the two streams interleave safely.
    File::create(&log_path).map_err(|e| e.to_string())?;
    let stdout_file = std::fs::OpenOptions::new()
        .append(true)
        .open(&log_path)
        .map_err(|e| e.to_string())?;
    let stderr_file = std::fs::OpenOptions::new()
        .append(true)
        .open(&log_path)
        .map_err(|e| e.to_string())?;

    Command::new(bin)
        .arg("-avd")
        .arg(avd_name)
        .arg("-gpu")
        .arg(gpu_mode)
        .arg("-netdelay")
        .arg("none")
        .arg("-netspeed")
        .arg("full")
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .spawn()
        .map_err(|e| format!("failed to start emulator: {e}"))
}

/// Polls a just-spawned child for up to `window`; returns its exit status if
/// it already died (a real crash, not just "still booting" — the emulator
/// takes far longer than this window to boot normally), or None if it's
/// still running, which is the success case.
fn wait_briefly(child: &mut Child, window: Duration) -> Option<std::process::ExitStatus> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {
                if start.elapsed() >= window {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(300));
            }
            Err(_) => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A process that crashes immediately (simulating a GPU driver that
    /// can't handle "-gpu host") must be detected within the window.
    #[test]
    fn detects_early_crash() {
        let mut child = Command::new("sh").arg("-c").arg("exit 7").spawn().unwrap();
        let status = wait_briefly(&mut child, Duration::from_secs(3));
        assert_eq!(status.unwrap().code(), Some(7));
    }

    /// A process that's still running past the window (simulating a
    /// successful boot in progress) must read as "still running", i.e. None
    /// — this is what tells launch() *not* to fall back to software mode.
    #[test]
    fn does_not_flag_a_still_running_process() {
        let mut child = Command::new("sleep").arg("5").spawn().unwrap();
        let status = wait_briefly(&mut child, Duration::from_millis(500));
        assert!(status.is_none(), "a healthy long-running process must not read as crashed");
        let _ = child.kill();
    }
}
