const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let setupBtn, playBtn, logEl;

function log(line) {
  logEl.textContent += line + "\n";
  logEl.scrollTop = logEl.scrollHeight;
}

async function refreshStatus() {
  const status = await invoke("check_status");
  if (status.sdk_ready && status.avd_ready) {
    setupBtn.hidden = true;
    playBtn.hidden = false;
  } else {
    setupBtn.hidden = false;
    playBtn.hidden = true;
  }
  return status;
}

async function runSetup() {
  setupBtn.disabled = true;
  log("Starting setup — first time only, downloads a few GB...");
  try {
    await invoke("run_setup");
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
  log("Launching...");
  try {
    await invoke("launch_emulator");
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

  setupBtn.addEventListener("click", runSetup);
  playBtn.addEventListener("click", play);

  await listen("setup-progress", (event) => log(event.payload));
  await refreshStatus();
});
