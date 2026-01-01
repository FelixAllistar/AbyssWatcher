const { listen } = window.__TAURI__.event;
const { invoke } = window.__TAURI__.core;

const selectionContainer = document.getElementById("selection-container");
const outDpsEl = document.getElementById("out-dps");
const outHpsEl = document.getElementById("out-hps");
const inDpsEl = document.getElementById("in-dps");
const activeListEl = document.getElementById("active-list");
const toggleBtn = document.getElementById("toggle-settings");

// ...

function updateUI(data) {
  outDpsEl.textContent = data.outgoing_dps.toFixed(1);
  outHpsEl.textContent = (data.outgoing_hps || 0.0).toFixed(1);
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
            // Sort Alphabetically by Weapon Name to prevent UI jumping
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