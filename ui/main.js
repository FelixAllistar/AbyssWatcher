const { listen } = window.__TAURI__.event;

const appEl = document.getElementById("app");

async function init() {
  console.log("Initializing AbyssWatcher Frontend...");

  const unlisten = await listen("dps-update", (event) => {
    const data = event.payload;
    console.log("Received DPS update:", data);
    updateUI(data);
  });
}

function updateUI(data) {
  // Simple rendering for prototype
  let html = `
    <div class="summary">
      <p>Outgoing DPS: <strong>${data.outgoing_dps.toFixed(1)}</strong></p>
      <p>Incoming DPS: <strong>${data.incoming_dps.toFixed(1)}</strong></p>
    </div>
    <div class="characters">
      <h3>Characters:</h3>
      <ul>
  `;

  for (const [char, dps] of Object.entries(data.outgoing_by_character)) {
    html += `<li>${char}: ${dps.toFixed(1)}</li>`;
  }

  html += `</ul></div>`;
  appEl.innerHTML = html;
}

init();