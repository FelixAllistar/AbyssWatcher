const { listen } = window.__TAURI__.event;
const { invoke } = window.__TAURI__.core;

async function init() {
  const dataContainer = document.getElementById("data-container");
  const selectionContainer = document.getElementById("selection-container");

  function logToScreen(msg) {
    const p = document.createElement("p");
    p.textContent = `[JS LOG]: ${msg}`;
    p.style.fontSize = "10px";
    p.style.color = "#aaa";
    document.body.appendChild(p);
  }

  logToScreen("Initializing AbyssWatcher Frontend...");

  // 1. Get available characters
  try {
    const characters = await invoke("get_available_characters");
    logToScreen(`Found ${characters.length} characters.`);
    
    if (characters.length === 0) {
      selectionContainer.innerHTML = "<p>No characters detected in logs folder.</p>";
    } else {
      const selectionEl = document.createElement("div");
      selectionEl.id = "selection";
      selectionEl.innerHTML = "<h3>Select Characters to Track:</h3>";
      
      characters.forEach(char => {
        const label = document.createElement("label");
        label.style.display = "block";
        label.style.cursor = "pointer";
        
        const checkbox = document.createElement("input");
        checkbox.type = "checkbox";
        checkbox.onchange = async () => {
          logToScreen(`Toggling: ${char.character}`);
          await invoke("toggle_tracking", { path: char.path });
        };
        
        label.appendChild(checkbox);
        label.appendChild(document.createTextNode(` ${char.character}`));
        selectionEl.appendChild(label);
      });
      selectionContainer.appendChild(selectionEl);
    }
  } catch (e) {
    logToScreen("Error getting characters: " + JSON.stringify(e));
  }

  // 2. Listen for DPS updates
  await listen("dps-update", (event) => {
    lastDpsData = event.payload;
    updateUI();
  });

  // 3. Listen for backend logs
  await listen("backend-log", (event) => {
    logToScreen(`[BACKEND]: ${event.payload}`);
  });
}

// Ensure the DOM is fully loaded before running
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", init);
} else {
  init();
}
