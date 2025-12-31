const { listen } = window.__TAURI__.event;
const { invoke } = window.__TAURI__.core;

const selectionContainer = document.getElementById("selection-container");
const outDpsEl = document.getElementById("out-dps");
const inDpsEl = document.getElementById("in-dps");
const activeListEl = document.getElementById("active-list");
const logEl = document.getElementById("logs");
const toggleBtn = document.getElementById("toggle-settings");

// Settings Elements
const toggleConfigBtn = document.getElementById("toggle-config");
const settingsModal = document.getElementById("settings-modal");
const logDirInput = document.getElementById("log-dir-input");
const browseBtn = document.getElementById("browse-btn");
const dpsWindowInput = document.getElementById("dps-window-input");
const saveSettingsBtn = document.getElementById("save-settings");
const cancelSettingsBtn = document.getElementById("cancel-settings");

let characters = [];

async function init() {
  logToScreen("Initializing...");

  // Toggle settings visibility
  toggleBtn.onclick = () => {
    selectionContainer.classList.toggle("hidden");
    if (!selectionContainer.classList.contains("hidden")) {
        settingsModal.classList.add("hidden");
    }
  };

  // Settings Logic
  toggleConfigBtn.onclick = () => {
      settingsModal.classList.toggle("hidden");
      if (!settingsModal.classList.contains("hidden")) {
          selectionContainer.classList.add("hidden");
      }
  };

  browseBtn.onclick = async () => {
      try {
        const path = await invoke("pick_gamelog_dir");
        if (path) {
            logDirInput.value = path;
        }
      } catch (e) {
          logToScreen("Error picking dir: " + e);
      }
  };

  saveSettingsBtn.onclick = async () => {
      const newSettings = {
          gamelog_dir: logDirInput.value,
          dps_window_seconds: parseInt(dpsWindowInput.value) || 5
      };
      try {
          await invoke("save_settings", { settings: newSettings });
          settingsModal.classList.add("hidden");
          logToScreen("Settings saved!");
          // Refresh characters as the directory might have changed
          characters = await invoke("get_available_characters");
          renderSelection();
      } catch (e) {
          logToScreen("Error saving: " + e);
      }
  };

  cancelSettingsBtn.onclick = () => {
      settingsModal.classList.add("hidden");
  };

  // 1. Load initial settings
  try {
    const settings = await invoke("get_settings");
    logDirInput.value = settings.gamelog_dir;
    dpsWindowInput.value = settings.dps_window_seconds;
  } catch (e) {
    logToScreen("Error loading settings: " + JSON.stringify(e));
  }

  // 2. Get available characters (using the loaded settings path)
  try {
    characters = await invoke("get_available_characters");
    renderSelection();
  } catch (e) {
    logToScreen("Error getting chars: " + JSON.stringify(e));
  }

  // 3. Listen for DPS updates
  await listen("dps-update", (event) => {
    updateUI(event.payload);
  });

  // 4. Listen for backend logs
  await listen("backend-log", (event) => {
    logToScreen(event.payload);
  });
}

function renderSelection() {
  selectionContainer.innerHTML = "";
  if (characters.length === 0) {
    selectionContainer.innerHTML = "<div style='padding:8px'>No logs found.</div>";
    return;
  }

  characters.forEach(char => {
    const div = document.createElement("div");
    div.className = "char-row";
    
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.onchange = () => invoke("toggle_tracking", { path: char.path });
    
    div.appendChild(checkbox);
    div.appendChild(document.createTextNode(char.character));
    selectionContainer.appendChild(div);
  });
}

function updateUI(data) {
  outDpsEl.textContent = data.outgoing_dps.toFixed(1);
  inDpsEl.textContent = data.incoming_dps.toFixed(1);

  // Update per-character breakdown
  let html = "";
  const chars = Object.entries(data.outgoing_by_character);
  if (chars.length > 1) {
      chars.sort((a, b) => b[1] - a[1]);
      chars.forEach(([name, dps]) => {
        html += `<div class="active-char"><span>${name}</span><strong>${dps.toFixed(1)}</strong></div>`;
      });
  }
  activeListEl.innerHTML = html;
}

function logToScreen(msg) {
    const p = document.createElement("div");
    p.textContent = msg;
    logEl.prepend(p);
    if (logEl.children.length > 5) logEl.lastChild.remove();
}

init();