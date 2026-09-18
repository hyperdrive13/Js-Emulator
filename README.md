# Js Emulator

A downloadable desktop app (Windows + Linux) that gives you and friends your
own Android instance for playing mobile games — one download, no manual
Android Studio/SDK fumbling. Runs entirely on each person's own machine, no
server dependency.

## Why this architecture

Writing an Android emulator from scratch (CPU virtualization, GPU
acceleration, the Android system itself) is a multi-year, full-team effort —
that's literally what BlueStacks/LDPlayer/Genymotion are. Instead, this wraps
**Google's own AOSP emulator** (the same QEMU-based engine bundled with
Android Studio): open source, actively maintained, genuinely cross-platform,
and hardware-accelerated (KVM on Linux, WHPX on Windows).

The part that's actually "ours" is the launcher around it:
- First-run setup that fetches the Android command-line tools + the one
  system image we standardize on, straight from Google — same artifacts
  `sdkmanager` itself would pull, not anything we redistribute ourselves
  (keeps this app clear of Google's SDK license terms).
- A pre-tuned virtual device ("gaming_phone") so nobody has to touch an AVD
  Manager — sensible RAM/CPU/GPU defaults picked for general mobile gaming,
  not per-title tuning.
- One-click "Play" instead of a wall of Android Studio settings.

## Stack

- **Launcher UI**: [Tauri](https://tauri.app) (Rust + vanilla JS/HTML) — small,
  cross-platform, no Electron overhead.
- **Emulation core**: Android SDK cmdline-tools + `emulator` binary, driven
  via `sdkmanager`/`avdmanager` under the hood.
- **Default image**: `system-images;android-36;google_apis_playstore;x86_64`
  (Android 16, Google Play enabled, x86_64 — needed for Play Services
  login/IAP that most games require).

## Status (as of 2026-09-17)

Built and locally verified on this dev machine (already had the Android SDK,
KVM, and Rust toolchain installed/reachable):

- [x] Project scaffolded (`launcher/`, Tauri + vanilla JS template)
- [x] `sdk.rs` — detects an existing usable SDK (env vars) or bootstraps one
      from scratch into app-local data dir; installs platform-tools, emulator,
      platform, system image via `sdkmanager`
- [x] `avd.rs` — creates the "gaming_phone" AVD and patches its config for
      gaming (4GB RAM / up to 4 cores scaled to host, GPU auto, 8GB data
      partition, hardware keyboard passthrough on)
- [x] `emulator.rs` — launches the tuned AVD
- [x] Minimal UI: Set Up → Play, with live progress log
- [x] `cargo check` clean, AVD-creation path proven end-to-end with a real
      integration test (`cargo test creates_and_tunes_gaming_avd`)
- [x] Full boot test on Linux: real Android 16 home screen, Play Store +
      Google apps present, **confirmed actual GPU rendering** (RTX 3090 shows
      up as the graphics adapter in the emulator log, not software fallback)
- [ ] **Windows testing — not done yet.** Everything above has only run on
      Linux. Treat Windows as unverified until this line is updated — don't
      assume it works there just because the code has `cfg!(windows)`
      branches for paths/binary names.
- [ ] Packaging real installers (.exe / .AppImage or .deb)
- [ ] Icon/branding (currently the Tauri default icon)
- [ ] Per-app keymapping presets (mouse/keyboard → touch)
- [ ] Multi-instance support if you ever want more than one Android instance
      running side by side on the same machine

## Known quirks hit so far

- **`-gpu auto` is not safe to use.** The emulator runs the host GPU driver
  against its bundled compatibility blocklist and silently falls back to
  SwiftShader (software rendering) on anything it doesn't recognize —
  reproduced on a real RTX 3090 with a current driver, logged as "Your GPU
  drivers may have a bug." `-gpu host` correctly detected and used the real
  card. The launcher now defaults to `-gpu host`; `GAME_LAUNCHER_GPU_MODE` env
  var is an escape hatch for the rare machine where hardware rendering
  genuinely is broken (set it to e.g. `swiftshader_indirect`). Worth
  surfacing as an in-app "having graphics issues?" toggle later rather than
  an env var only friends won't know about.
- `avdmanager create avd` does **not** take a `--sdk_root` flag the way
  `sdkmanager` does — it reads `ANDROID_SDK_ROOT`/`ANDROID_HOME` from the
  environment instead. (`sdk.rs`/`avd.rs` handle this.)
- `avdmanager` does not set `PlayStore.enabled=yes` on its own even when
  given a `google_apis_playstore` system image — has to be patched into
  `config.ini` after creation, or Play Store never appears on first boot.
- `sdkmanager` itself is flagged deprecated in favor of a newer `android` CLI
  tool — still functions fine today, but worth revisiting if Google drops it.
- The cmdline-tools download URL has no stable "latest" alias — the build
  number in `sdk.rs`'s `CMDLINE_TOOLS_BUILD` constant will need bumping
  periodically (there's no clean fix short of parsing Google's full package
  manifest XML, which felt like overkill for what this needs).

## Licensing note

We never bundle or redistribute Google's system images, platform tools, or
build tools ourselves — the launcher downloads them at first run directly
from Google's own servers, the same way Android Studio's SDK Manager does.
That keeps distributing *this* app free of Google's SDK license restrictions.
