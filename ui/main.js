const { listen } = window.__TAURI__.event;
const { invoke } = window.__TAURI__.core;

const selectionContainer = document.getElementById("selection-container");
const outDpsEl = document.getElementById("out-dps");
const inDpsEl = document.getElementById("in-dps");
const activeListEl = document.getElementById("active-list");
const logEl = document.getElementById("logs");
const toggleBtn = document.getElementById("toggle-settings");

let characters = [];

async function init() {
  logToScreen("Initializing...");

  // Toggle settings visibility
  toggleBtn.onclick = () => {
    selectionContainer.classList.toggle("hidden");
    // Force a repaint to help clear ghosting artifacts on transparent windows
    document.body.style.display = 'none';
    document.body.offsetHeight; // Trigger reflow
    document.body.style.display = '';
  };

  // 1. Get available characters
  try {
    characters = await invoke("get_available_characters");
    renderSelection();
  } catch (e) {
    logToScreen("Error: " + JSON.stringify(e));
  }

  // 2. Listen for DPS updates
  await listen("dps-update", (event) => {
    updateUI(event.payload);
  });

  // 3. Listen for backend logs
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