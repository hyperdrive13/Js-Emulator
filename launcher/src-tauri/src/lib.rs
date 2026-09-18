mod avd;
mod catalog;
mod emulator;
mod sdk;
#[cfg(test)]
mod cold_start_test;

use tauri::Emitter;

#[derive(serde::Serialize)]
struct DeviceOption {
    id: &'static str,
    label: &'static str,
}

#[derive(serde::Serialize)]
struct VersionOption {
    api: &'static str,
    label: &'static str,
}

#[derive(serde::Serialize)]
struct Options {
    devices: Vec<DeviceOption>,
    versions: Vec<VersionOption>,
    default_device: &'static str,
    default_api: &'static str,
}

#[tauri::command]
fn list_options() -> Options {
    Options {
        devices: catalog::DEVICES
            .iter()
            .map(|d| DeviceOption { id: d.id, label: d.label })
            .collect(),
        versions: catalog::VERSIONS
            .iter()
            .map(|v| VersionOption { api: v.api, label: v.label })
            .collect(),
        default_device: catalog::DEFAULT_DEVICE,
        default_api: catalog::DEFAULT_API,
    }
}

#[derive(serde::Serialize)]
struct Status {
    ready: bool,
}

#[tauri::command]
fn check_status(device: String, api: String) -> Result<Status, String> {
    catalog::validate_device(&device)?;
    catalog::validate_api(&api)?;
    let root = sdk::find_sdk_root();
    let ready = sdk::is_engine_ready(&root)
        && sdk::is_image_ready(&root, &api)
        && avd::exists(&catalog::avd_name(&device, &api));
    Ok(Status { ready })
}

#[tauri::command]
async fn run_setup(app: tauri::AppHandle, device: String, api: String) -> Result<(), String> {
    catalog::validate_device(&device)?;
    catalog::validate_api(&api)?;
    tauri::async_runtime::spawn_blocking(move || {
        let root = sdk::find_sdk_root();
        let emit = |msg: String| {
            let _ = app.emit("setup-progress", msg);
        };

        if !sdk::is_engine_ready(&root) {
            std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
            sdk::bootstrap_engine(&root, &emit)?;
        }
        if !sdk::is_image_ready(&root, &api) {
            sdk::install_image(&root, &api, &emit)?;
        }
        avd::ensure(&root, &device, &api, &emit)?;
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn launch_emulator(app: tauri::AppHandle, device: String, api: String) -> Result<(), String> {
    catalog::validate_device(&device)?;
    catalog::validate_api(&api)?;
    tauri::async_runtime::spawn_blocking(move || {
        let root = sdk::find_sdk_root();
        let name = catalog::avd_name(&device, &api);
        let emit = |msg: String| {
            let _ = app.emit("setup-progress", msg);
        };
        emulator::launch(&root, &name, emit)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_options,
            check_status,
            run_setup,
            launch_emulator
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
