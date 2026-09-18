mod avd;
mod emulator;
mod sdk;

use tauri::Emitter;

#[derive(serde::Serialize)]
struct Status {
    sdk_ready: bool,
    avd_ready: bool,
}

#[tauri::command]
fn check_status() -> Status {
    let root = sdk::find_sdk_root();
    Status {
        sdk_ready: sdk::is_ready(&root),
        avd_ready: avd::exists(),
    }
}

#[tauri::command]
async fn run_setup(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = sdk::find_sdk_root();
        let emit = |msg: String| {
            let _ = app.emit("setup-progress", msg);
        };

        if !sdk::is_ready(&root) {
            std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
            sdk::bootstrap(&root, &emit)?;
        }
        avd::ensure(&root, &emit)?;
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn launch_emulator() -> Result<(), String> {
    let root = sdk::find_sdk_root();
    emulator::launch(&root)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            check_status,
            run_setup,
            launch_emulator
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
