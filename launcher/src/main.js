const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const STORAGE_KEY = "android-game-launcher.selection";

let setupBtn, playBtn, logEl, deviceSelect, versionSelect;

function log(line) {
  logEl.textContent += line + "\n";
  logEl.scrollTop = logEl.scrollHeight;
}

function currentSelection() {
  return { device: deviceSelect.value, api: versionSelect.value };
}

function saveSelection() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(currentSelection()));
  } catch {
    // localStorage can be unavailable (private mode, etc.) — fine to skip.
  }
}

async function populateOptions() {
  const options = await invoke("list_options");
  let saved = {};
  try {
    saved = JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}");
  } catch {
    saved = {};
  }

  deviceSelect.innerHTML = options.devices
    .map((d) => `<option value="${d.id}">${d.label}</option>`)
    .join("");
  versionSelect.innerHTML = options.versions
    .map((v) => `<option value="${v.api}">${v.label}</option>`)
    .join("");

  deviceSelect.value = saved.device || options.default_device;
  versionSelect.value = saved.api || options.default_api;
}

async function refreshStatus() {
  const { device, api } = currentSelection();
  const status = await invoke("check_status", { device, api });
  setupBtn.hidden = status.ready;
  playBtn.hidden = !status.ready;
  return status;
}

async function runSetup() {
  setupBtn.disabled = true;
  const { device, api } = currentSelection();
  log(`Setting up ${device} / Android ${api} — first time for this combo, may download a few GB...`);
  try {
    await invoke("run_setup", { device, api });
    log("Setup complete.");
  } catch (e) {
    log("Setup failed: " + e);
  } finally {
    setupBtn.disabled = false;
    await refreshStatus();
  }
}

async function play() {
  playBtn.disabled = true;
  const { device, api } = currentSelection();
  log("Launching...");
  try {
    await invoke("launch_emulator", { device, api });
    log("Emulator window should appear shortly.");
  } catch (e) {
    log("Launch failed: " + e);
  } finally {
    playBtn.disabled = false;
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  setupBtn = document.querySelector("#setup-btn");
  playBtn = document.querySelector("#play-btn");
  logEl = document.querySelector("#log");
  deviceSelect = document.querySelector("#device-select");
  versionSelect = document.querySelector("#version-select");

  setupBtn.addEventListener("click", runSetup);
  playBtn.addEventListener("click", play);
  deviceSelect.addEventListener("change", () => {
    saveSelection();
    refreshStatus();
  });
  versionSelect.addEventListener("change", () => {
    saveSelection();
    refreshStatus();
  });

  await listen("setup-progress", (event) => log(event.payload));
  await populateOptions();
  await refreshStatus();
});
