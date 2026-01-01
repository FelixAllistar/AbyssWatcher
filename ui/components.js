// ui/components.js

const Components = {
  Styles: {
    damage: { color: "#ff6b6b", label: "DPS" },
    repair: { color: "#6bff6b", label: "HPS" },
    cap: { color: "#6b6bff", label: "GJ/s" },
    neut: { color: "#d26bff", label: "GJ/s" }, // Purpleish
    default: { color: "#aaa", label: "" }
  },

  renderSelection: function(container, characters, onToggle) {
    container.innerHTML = "";
    if (characters.length === 0) {
        container.innerHTML = "<div style='padding:4px; color:#aaa'>No logs found.</div>";
        return;
    }
    characters.forEach(char => {
        const div = document.createElement("div");
        div.className = "char-row";
        const checkbox = document.createElement("input");
        checkbox.type = "checkbox";
        checkbox.checked = char.tracked;
        checkbox.onchange = () => onToggle(char, checkbox.checked);
        div.appendChild(checkbox);
        div.appendChild(document.createTextNode(char.character));
        container.appendChild(div);
    });
  },

  updateUI: function(els, data, characters) {
    if (els.outDps) els.outDps.textContent = data.outgoing_dps.toFixed(1);
    if (els.inDps) els.inDps.textContent = data.incoming_dps.toFixed(1);
    if (els.outHps) els.outHps.textContent = (data.outgoing_hps || 0).toFixed(1);
    if (els.inHps) els.inHps.textContent = (data.incoming_hps || 0).toFixed(1);
    if (els.outCap) els.outCap.textContent = (data.outgoing_cap || 0).toFixed(1);
    if (els.inCap) els.inCap.textContent = (data.incoming_cap || 0).toFixed(1);
    if (els.outNeut) els.outNeut.textContent = (data.outgoing_neut || 0).toFixed(1);
    if (els.inNeut) els.inNeut.textContent = (data.incoming_neut || 0).toFixed(1);

    if (!els.activeList) return;

    // Active List Logic
    let html = "";
    
    // Convert data to Map for easy lookup
    const activeData = new Map(Object.entries(data.combat_actions_by_character || {}));

    // Merge tracked characters that are inactive
    characters.forEach(char => {
        if (char.tracked && !activeData.has(char.character)) {
            activeData.set(char.character, []);
        }
    });

    const chars = Array.from(activeData.entries());
    
    // Sort logic: Alphabetical by character name
    chars.sort((a, b) => a[0].localeCompare(b[0]));

    chars.forEach(([name, actions]) => {
        // Calculate totals by type
        let dps = 0, hps = 0, cap = 0, neut = 0;
        actions.forEach(act => {
            if (act.action_type === "Damage") dps += act.value;
            else if (act.action_type === "Repair") hps += act.value;
            else if (act.action_type === "Capacitor") cap += act.value;
            else if (act.action_type === "Neut") neut += act.value;
        });

        // Build Header with Badges
        let headerBadges = "";
        if (dps > 0 || (hps===0 && cap===0 && neut===0)) headerBadges += `<span style="color:${Components.Styles.damage.color}; margin-left:6px">${dps.toFixed(1)}</span>`;
        if (hps > 0) headerBadges += `<span style="color:${Components.Styles.repair.color}; margin-left:6px">REP ${hps.toFixed(1)}</span>`;
        if (cap > 0) headerBadges += `<span style="color:${Components.Styles.cap.color}; margin-left:6px">CAP ${cap.toFixed(1)}</span>`;
        if (neut > 0) headerBadges += `<span style="color:${Components.Styles.neut.color}; margin-left:6px">NEUT ${neut.toFixed(1)}</span>`;

        html += `<div style="margin-bottom:6px; border-bottom:1px solid rgba(255,255,255,0.05)">
          <div style="display:flex; justify-content:space-between; font-weight:700">
            <span>${name}</span>
            <div>${headerBadges}</div>
          </div>`;
        
        // Actions
        // Sort actions by value descending
        actions.sort((a, b) => b.value - a.value);
        
        actions.forEach(act => {
            let style = Components.Styles.default;

            if (act.action_type === "Damage") {
                style = Components.Styles.damage;
            } else if (act.action_type === "Repair") {
                style = Components.Styles.repair;
            } else if (act.action_type === "Capacitor") {
                style = Components.Styles.cap;
            } else if (act.action_type === "Neut") {
                style = Components.Styles.neut;
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
};
