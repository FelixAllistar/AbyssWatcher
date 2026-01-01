const { listen } = window.__TAURI__.event;
const { invoke } = window.__TAURI__.core;

const selectionContainer = document.getElementById("selection-container");
const outDpsEl = document.getElementById("out-dps");
const outHpsEl = document.getElementById("out-hps");
const outCapEl = document.getElementById("out-cap");
const outNeutEl = document.getElementById("out-neut");
const inDpsEl = document.getElementById("in-dps");
const activeListEl = document.getElementById("active-list");
const toggleBtn = document.getElementById("toggle-settings");

// Settings Elements
const toggleConfigBtn = document.getElementById("toggle-config");
const settingsModal = document.getElementById("settings-modal");
const logDirInput = document.getElementById("log-dir-input");
const browseBtn = document.getElementById("browse-btn");
const dpsWindowInput = document.getElementById("dps-window-input");
const saveSettingsBtn = document.getElementById("save-settings");
const cancelSettingsBtn = document.getElementById("cancel-settings");
const replayBtn = document.getElementById("replay-btn");

let characters = [];

async function init() {
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
          console.error("Error picking dir:", e);
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
          // Refresh characters as the directory might have changed
          characters = await invoke("get_available_characters");
          renderSelection();
      } catch (e) {
          console.error("Error saving settings:", e);
      }
  };

  cancelSettingsBtn.onclick = () => {
      settingsModal.classList.add("hidden");
  };

  replayBtn.onclick = async () => {
      try {
          await invoke("replay_logs");
          settingsModal.classList.add("hidden");
      } catch (e) {
          console.error("Error replaying:", e);
      }
  };

  // 1. Load initial settings
  try {
    const settings = await invoke("get_settings");
    logDirInput.value = settings.gamelog_dir;
    dpsWindowInput.value = settings.dps_window_seconds;
  } catch (e) {
    console.error("Error loading settings:", e);
  }

  // 2. Get available characters (using the loaded settings path)
  try {
    characters = await invoke("get_available_characters");
    renderSelection();
  } catch (e) {
    console.error("Error getting chars:", e);
  }

  // 3. Listen for DPS updates
  await listen("dps-update", (event) => {
    updateUI(event.payload);
  });

  // 4. Listen for backend logs
  await listen("backend-log", (event) => {
    console.log("Backend:", event.payload);
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
  outHpsEl.textContent = (data.outgoing_hps || 0.0).toFixed(1);
  outCapEl.textContent = (data.outgoing_cap || 0.0).toFixed(1);
  outNeutEl.textContent = (data.outgoing_neut || 0.0).toFixed(1);
  inDpsEl.textContent = data.incoming_dps.toFixed(1);

  // Update per-character breakdown
  let html = "";
  const chars = Object.entries(data.outgoing_by_character);
  
  if (chars.length > 0) {
      chars.sort((a, b) => b[1] - a[1]);
      chars.forEach(([name, dps]) => {
        html += `<div class="char-section">
          <div class="active-char"><span>${name}</span><strong>${dps.toFixed(1)}</strong></div>`;
        
        // Weapons Breakdown
        const weapons = Object.entries(data.outgoing_by_char_weapon[name] || {});
        if (weapons.length > 0) {
            // Sort Alphabetically by Weapon Name
            weapons.sort((a, b) => a[0].localeCompare(b[0]));
            html += `<div class="breakdown-list">`;
            weapons.forEach(([weapon, weaponDps]) => {
                html += `<div class="breakdown-item"><span>${weapon}</span><span>${weaponDps.toFixed(1)}</span></div>`;
            });
            html += `</div>`;
        }

        // Targets Breakdown (Top 3)
        const targets = Object.entries(data.outgoing_by_char_target[name] || {});
        if (targets.length > 0) {
            // Sort Alphabetically by Target Name
            targets.sort((a, b) => a[0].localeCompare(b[0]));
            html += `<div class="breakdown-list targets">`;
            targets.slice(0, 3).forEach(([target, targetDps]) => {
                html += `<div class="breakdown-item"><span>» ${target}</span><span>${targetDps.toFixed(1)}</span></div>`;
            });
            html += `</div>`;
        }

        html += `</div>`;
      });
  }
  activeListEl.innerHTML = html;
}

init();
