//! The one test that matters most for "will this work on a friend's
//! machine": everything else so far reused the Android SDK this dev box
//! already had installed. This exercises the actual first-run path a
//! brand-new machine hits — download cmdline-tools from Google, install
//! platform-tools/emulator/system image via sdkmanager, create an AVD, boot
//! it, and confirm real GPU rendering — all into an empty directory that
//! starts with nothing in it, the same as a friend's machine would.
//!
//! Slow (multi-GB download) and network-dependent, so it's `#[ignore]`d by
//! default. Run explicitly with:
//!   cargo test --lib cold_start -- --ignored --nocapture

use crate::{avd, catalog, emulator, sdk};
use std::process::Command;
use std::time::{Duration, Instant};

// A different device than the one used elsewhere in the test suite/manual
// testing, so this test actually exercises avdmanager creating a new AVD
// from this freshly-bootstrapped SDK, rather than short-circuiting on one
// that already exists from prior testing.
const TEST_DEVICE: &str = "small_phone";
const TEST_API: &str = catalog::DEFAULT_API;

#[test]
#[ignore]
fn cold_start_from_an_empty_directory_boots_with_real_gpu() {
    // Fixed (not random) path: if this fails partway through, re-running
    // picks up from whatever was already downloaded/installed instead of
    // starting the multi-GB download over — same idempotency a real user's
    // second attempt after a failure would get.
    let root = std::env::temp_dir().join("android-launcher-cold-start-test");
    println!("cold-start SDK root: {}", root.display());
    std::fs::create_dir_all(&root).unwrap();

    let progress = |msg: String| println!("[cold-start] {msg}");

    sdk::bootstrap_engine(&root, &progress).expect("engine bootstrap should succeed from scratch");
    assert!(sdk::is_engine_ready(&root), "engine should be ready after bootstrap");

    sdk::install_image(&root, TEST_API, &progress).expect("image install should succeed");
    assert!(sdk::is_image_ready(&root, TEST_API));

    avd::ensure(&root, TEST_DEVICE, TEST_API, &progress).expect("avd creation should succeed");
    let avd_name = catalog::avd_name(TEST_DEVICE, TEST_API);
    assert!(avd::exists(&avd_name));

    emulator::launch(&root, &avd_name, &progress).expect("emulator should launch");

    let adb = sdk::adb_path(&root);
    wait_for_boot(&adb, Duration::from_secs(180));

    let packages = Command::new(&adb)
        .args(["shell", "pm", "list", "packages"])
        .output()
        .expect("adb shell should run");
    let packages = String::from_utf8_lossy(&packages.stdout);
    assert!(
        packages.contains("com.android.vending"),
        "Play Store should be present on a freshly-created AVD"
    );

    let log = std::fs::read_to_string(emulator::log_path()).unwrap_or_default();
    assert!(
        log.contains("Graphics Adapter"),
        "emulator log should report a graphics adapter line"
    );
    assert!(
        !log.contains("Graphics Adapter Android Emulator OpenGL ES Translator (Google SwiftShader)"),
        "cold-start boot should use real GPU rendering, not the software fallback:\n{log}"
    );

    let _ = Command::new(&adb).args(["emu", "kill"]).status();
}

fn wait_for_boot(adb: &std::path::Path, timeout: Duration) {
    let start = Instant::now();
    loop {
        let out = Command::new(adb)
            .args(["shell", "getprop", "sys.boot_completed"])
            .output();
        if let Ok(out) = out {
            if String::from_utf8_lossy(&out.stdout).trim() == "1" {
                return;
            }
        }
        assert!(start.elapsed() < timeout, "emulator did not boot within {timeout:?}");
        std::thread::sleep(Duration::from_secs(2));
    }
}
