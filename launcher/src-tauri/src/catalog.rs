//! The curated set of device profiles / Android versions we let people pick
//! from. Deliberately not the full avdmanager device list (which includes
//! automotive, Wear OS, TV, XR, desktop, etc.) — just phone/tablet shapes
//! relevant to mobile gaming. Also doubles as an allowlist: device/api
//! strings coming from the frontend are validated against this before ever
//! reaching a shell command.

pub struct Device {
    pub id: &'static str,
    pub label: &'static str,
}

pub struct Version {
    pub api: &'static str,
    pub label: &'static str,
}

pub const DEVICES: &[Device] = &[
    Device { id: "medium_phone", label: "Phone (default)" },
    Device { id: "small_phone", label: "Small Phone" },
    Device { id: "pixel_6", label: "Pixel 6" },
    Device { id: "pixel_6_pro", label: "Pixel 6 Pro (large)" },
    Device { id: "pixel_9_pro", label: "Pixel 9 Pro (large, newer)" },
    Device { id: "pixel_tablet", label: "Tablet" },
];

// All confirmed available as plain "system-images/android-<api>/google_apis_playstore/x86_64"
// packages (no -extN suffix needed) as of 2026-09.
pub const VERSIONS: &[Version] = &[
    Version { api: "30", label: "Android 11 (oldest, best compatibility)" },
    Version { api: "33", label: "Android 13" },
    Version { api: "34", label: "Android 14" },
    Version { api: "35", label: "Android 15" },
    Version { api: "36", label: "Android 16 (latest, default)" },
];

pub const DEFAULT_DEVICE: &str = "pixel_6";
pub const DEFAULT_API: &str = "36";

pub fn validate_device(id: &str) -> Result<(), String> {
    if DEVICES.iter().any(|d| d.id == id) {
        Ok(())
    } else {
        Err(format!("unknown device profile: {id}"))
    }
}

pub fn validate_api(api: &str) -> Result<(), String> {
    if VERSIONS.iter().any(|v| v.api == api) {
        Ok(())
    } else {
        Err(format!("unknown Android version: {api}"))
    }
}

pub fn system_image_id(api: &str) -> String {
    format!("system-images;android-{api};google_apis_playstore;x86_64")
}

pub fn platform_id(api: &str) -> String {
    format!("platforms;android-{api}")
}

pub fn system_image_rel_path(api: &str) -> String {
    format!("system-images/android-{api}/google_apis_playstore/x86_64")
}

/// AVD name unique per (device, api) so switching profiles doesn't clobber
/// an existing one — each combination you've tried stays available.
pub fn avd_name(device: &str, api: &str) -> String {
    format!("game_{device}_api{api}")
}
