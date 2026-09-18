//! Locates or bootstraps an Android SDK good enough to run the emulator.
//!
//! Strategy: if the host already has a usable SDK (ANDROID_HOME / ANDROID_SDK_ROOT,
//! or a common install path — e.g. this dev machine already has one), reuse it.
//! Otherwise, download Google's official cmdline-tools zip straight from
//! dl.google.com (same artifact `sdkmanager` itself would fetch) into our own
//! app-data folder, and use it to pull just the packages we need.
//!
//! NOTE on licensing: we never bundle/redistribute Google's system images or
//! build tools ourselves. We download them at first-run, from Google, the same
//! way Android Studio's own SDK Manager does. That keeps distribution of this
//! launcher itself clean of Google's SDK license terms.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

// Bump this when Google ships a newer cmdline-tools release. There's no stable
// "latest" URL — the SDK's own manifest (repository2-3.xml) is the canonical
// source of truth, but parsing that multi-MB XML file is more machinery than
// this launcher needs; a periodically-updated constant is the pragmatic
// tradeoff. Verified reachable as of writing.
const CMDLINE_TOOLS_BUILD: &str = "15859902";

const PLATFORM: &str = "android-36";
const SYSTEM_IMAGE: &str = "system-images;android-36;google_apis_playstore;x86_64";

fn host_os_tag() -> &'static str {
    if cfg!(target_os = "windows") {
        "win"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    }
}

fn bin_name(base: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{base}.bat")
    } else {
        base.to_string()
    }
}

/// Our own app-data SDK location, used only when no usable system SDK exists.
fn bundled_sdk_root() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("android-game-launcher")
        .join("sdk")
}

/// Find an existing, usable SDK: env vars first, then our own bundled one.
pub fn find_sdk_root() -> PathBuf {
    for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Ok(p) = std::env::var(var) {
            let p = PathBuf::from(p);
            if p.join("platform-tools").exists() {
                return p;
            }
        }
    }
    bundled_sdk_root()
}

pub fn sdkmanager_path(root: &Path) -> PathBuf {
    root.join("cmdline-tools")
        .join("latest")
        .join("bin")
        .join(bin_name("sdkmanager"))
}

pub fn avdmanager_path(root: &Path) -> PathBuf {
    root.join("cmdline-tools")
        .join("latest")
        .join("bin")
        .join(bin_name("avdmanager"))
}

pub fn emulator_path(root: &Path) -> PathBuf {
    root.join("emulator").join(bin_name("emulator"))
}

pub fn is_ready(root: &Path) -> bool {
    emulator_path(root).exists()
        && root.join("platform-tools").exists()
        && root.join(SYSTEM_IMAGE.replace(';', "/")).exists()
}

/// Downloads cmdline-tools (if `root` doesn't already have them) and installs
/// platform-tools + emulator + the one system image we standardize on.
/// Reports progress lines through `on_progress`.
pub fn bootstrap(root: &Path, on_progress: impl Fn(String)) -> Result<(), String> {
    let cmdline_tools_dir = root.join("cmdline-tools").join("latest");
    if !sdkmanager_path(root).exists() {
        on_progress("Downloading Android command-line tools...".into());
        let url = format!(
            "https://dl.google.com/android/repository/commandlinetools-{}-{}_latest.zip",
            host_os_tag(),
            CMDLINE_TOOLS_BUILD
        );
        let bytes = reqwest::blocking::get(&url)
            .map_err(|e| format!("download failed: {e}"))?
            .bytes()
            .map_err(|e| format!("download read failed: {e}"))?;

        on_progress("Extracting command-line tools...".into());
        // The zip contains a top-level "cmdline-tools/" folder; Google's own
        // layout expects it renamed to ".../cmdline-tools/latest/".
        let extract_to = root.join("cmdline-tools-extract-tmp");
        std::fs::create_dir_all(&extract_to).map_err(|e| e.to_string())?;
        let mut archive =
            zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
        archive.extract(&extract_to).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(root.join("cmdline-tools")).map_err(|e| e.to_string())?;
        std::fs::rename(extract_to.join("cmdline-tools"), &cmdline_tools_dir)
            .map_err(|e| e.to_string())?;
        let _ = std::fs::remove_dir_all(&extract_to);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(entries) = std::fs::read_dir(cmdline_tools_dir.join("bin")) {
                for entry in entries.flatten() {
                    let _ = std::fs::set_permissions(
                        entry.path(),
                        std::fs::Permissions::from_mode(0o755),
                    );
                }
            }
        }
    }

    on_progress("Accepting SDK licenses...".into());
    let mut accept = Command::new(sdkmanager_path(root));
    accept
        .arg(format!("--sdk_root={}", root.display()))
        .arg("--licenses")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if let Ok(mut child) = accept.spawn() {
        use std::io::Write;
        if let Some(stdin) = child.stdin.as_mut() {
            // sdkmanager --licenses prompts once per unaccepted license; "y\n"
            // repeated covers however many there are.
            let _ = stdin.write_all("y\n".repeat(50).as_bytes());
        }
        let _ = child.wait();
    }

    on_progress("Installing platform-tools, emulator, and system image (this is several GB, may take a while)...".into());
    let status = Command::new(sdkmanager_path(root))
        .arg(format!("--sdk_root={}", root.display()))
        .arg("platform-tools")
        .arg("emulator")
        .arg(format!("platforms;{PLATFORM}"))
        .arg(SYSTEM_IMAGE)
        .status()
        .map_err(|e| format!("sdkmanager failed to run: {e}"))?;

    if !status.success() {
        return Err("sdkmanager exited with an error installing packages".into());
    }

    on_progress("Android SDK ready.".into());
    Ok(())
}

pub const SYSTEM_IMAGE_ID: &str = SYSTEM_IMAGE;
