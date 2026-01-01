try {
  const { listen } = window.__TAURI__.event;
  const { invoke } = window.__TAURI__.core;

  // Elements
  const els = {
    selectionContainer: document.getElementById("selection-container"),
    toggleBtn: document.getElementById("toggle-settings"),
    toggleConfigBtn: document.getElementById("toggle-config"),
    settingsModal: document.getElementById("settings-modal"),
    logDirInput: document.getElementById("log-dir-input"),
    browseBtn: document.getElementById("browse-btn"),
    dpsWindowInput: document.getElementById("dps-window-input"),
    saveSettingsBtn: document.getElementById("save-settings"),
    cancelSettingsBtn: document.getElementById("cancel-settings"),
    replayBtn: document.getElementById("replay-btn"),
    
    outDps: document.getElementById("out-dps"),
    inDps: document.getElementById("in-dps"),
    outHps: document.getElementById("out-hps"),
    inHps: document.getElementById("in-hps"),
    outCap: document.getElementById("out-cap"),
    inCap: document.getElementById("in-cap"),
    outNeut: document.getElementById("out-neut"),
    inNeut: document.getElementById("in-neut"),
    activeList: document.getElementById("active-list")
  };

  // State
  let characters = [];

  // Logic
  async function init() {
    // 1. Event Listeners
    els.toggleBtn.onclick = () => {
        els.selectionContainer.classList.toggle("hidden");
        if (!els.selectionContainer.classList.contains("hidden")) {
            els.settingsModal.classList.add("hidden");
        }
    };

    els.toggleConfigBtn.onclick = () => {
        els.settingsModal.classList.toggle("hidden");
        if (!els.settingsModal.classList.contains("hidden")) {
            els.selectionContainer.classList.add("hidden");
        }
    };

    els.browseBtn.onclick = async () => {
        try {
            const path = await invoke("pick_gamelog_dir");
            if (path) els.logDirInput.value = path;
        } catch (e) { console.error(e); }
    };

    els.saveSettingsBtn.onclick = async () => {
        try {
            const newSettings = {
                gamelog_dir: els.logDirInput.value,
                dps_window_seconds: parseInt(els.dpsWindowInput.value) || 5
            };
            await invoke("save_settings", { settings: newSettings });
            els.settingsModal.classList.add("hidden");
            characters = await invoke("get_available_characters");
            renderSelection();
        } catch (e) { console.error(e); }
    };

    els.cancelSettingsBtn.onclick = () => els.settingsModal.classList.add("hidden");

    els.replayBtn.onclick = async () => {
        try {
            await invoke("replay_logs");
            els.settingsModal.classList.add("hidden");
        } catch (e) { console.error(e); }
    };

    // 2. Load Data
    try {
        const settings = await invoke("get_settings");
        els.logDirInput.value = settings.gamelog_dir;
        els.dpsWindowInput.value = settings.dps_window_seconds;
        
        characters = await invoke("get_available_characters");
        renderSelection();
    } catch (e) { console.error(e); }

    // 3. Listen
    await listen("dps-update", (e) => updateUI(e.payload));
  }

  function renderSelection() {
    els.selectionContainer.innerHTML = "";
    if (characters.length === 0) {
        els.selectionContainer.innerHTML = "<div style='padding:4px; color:#aaa'>No logs found.</div>";
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
        els.selectionContainer.appendChild(div);
    });
  }

  // Style Constants
  const STYLES = {
    damage: { color: "#ff6b6b", label: "DPS" },
    repair: { color: "#6bff6b", label: "HPS" },
    cap: { color: "#6b6bff", label: "GJ/s" },
    neut: { color: "#d26bff", label: "GJ/s" }, // Purpleish
    default: { color: "#aaa", label: "" }
  };

  function updateUI(data) {
    els.outDps.textContent = data.outgoing_dps.toFixed(1);
    els.inDps.textContent = data.incoming_dps.toFixed(1);
    els.outHps.textContent = (data.outgoing_hps || 0).toFixed(1);
    els.inHps.textContent = (data.incoming_hps || 0).toFixed(1);
    els.outCap.textContent = (data.outgoing_cap || 0).toFixed(1);
    els.inCap.textContent = (data.incoming_cap || 0).toFixed(1);
    els.outNeut.textContent = (data.outgoing_neut || 0).toFixed(1);
    els.inNeut.textContent = (data.incoming_neut || 0).toFixed(1);

    // Active List
    let html = "";
    
    // Sort characters by their total outgoing DPS (legacy sort) or maybe total activity?
    // For now, let's stick to sorting by outgoing DPS if available, else just alphabetical or activity sum.
    // The previous logic used outgoing_by_character (which was just DPS).
    // Let's iterate over the new map: combat_actions_by_character
    
    const chars = Object.entries(data.combat_actions_by_character || {});
    
    // Sort logic: Alphabetical by character name
    chars.sort((a, b) => a[0].localeCompare(b[0]));

    chars.forEach(([name, actions]) => {
        // Calculate total activity for the header
        const totalVal = actions.reduce((acc, act) => acc + act.value, 0);

        html += `<div style="margin-bottom:6px; border-bottom:1px solid rgba(255,255,255,0.05)">
          <div style="display:flex; justify-content:space-between; font-weight:700">
            <span>${name}</span><span style="color:#00d2ff">${totalVal.toFixed(1)}</span>
          </div>`;
        
        // Actions
        // Sort actions by value descending
        actions.sort((a, b) => b.value - a.value);
        
        actions.forEach(act => {
            let style = STYLES.default;

            if (act.action_type === "Damage") {
                style = STYLES.damage;
            } else if (act.action_type === "Repair") {
                style = STYLES.repair;
            } else if (act.action_type === "Capacitor") {
                style = STYLES.cap;
            } else if (act.action_type === "Neut") {
                style = STYLES.neut;
            }

            html += `<div style="font-size:9px; color:${style.color}; display:flex; justify-content:space-between; align-items:center">
              <span>${act.name}</span>
              <span>${act.value.toFixed(1)} <span style="font-size:7px; opacity:0.7">${style.label}</span></span>
            </div>`;
        });
        html += `</div>`;
    });
    
    els.activeList.innerHTML = html;
  }

  init();

} catch (err) {
  document.body.innerHTML = `<h3 style='color:red; background:black'>ERROR: ${err.message}</h3>`;
  console.error(err);
}