// ui/components.js

const Components = {
  // Map EventType + Incoming to CSS class and suffix
  getMetricStyle: function (type, incoming) {
    switch (type) {
      case "Damage":
        return { class: incoming ? "text-dps-in" : "text-dps-out", label: "DPS" };
      case "Repair":
        return { class: incoming ? "text-rep-in" : "text-rep-out", label: "HPS" };
      case "Capacitor":
        return { class: incoming ? "text-cap-in" : "text-cap-out", label: "GJ/s" };
      case "Neut":
        return { class: incoming ? "text-neut-in" : "text-neut-out", label: "GJ/s" };
      default:
        return { class: "text-default", label: "" };
    }
  },

  renderSelection: function (container, characters, onToggle) {
    container.innerHTML = "";
    if (characters.length === 0) {
      container.innerHTML = "<div class='text-dim' style='padding:4px;'>No logs found.</div>";
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

  updateUI: function (els, data, characters) {
    if (els.outDps) els.outDps.textContent = data.outgoing_dps.toFixed(1);
    if (els.inDps) els.inDps.textContent = data.incoming_dps.toFixed(1);
    if (els.outHps) els.outHps.textContent = (data.outgoing_hps || 0).toFixed(1);
    if (els.inHps) els.inHps.textContent = (data.incoming_hps || 0).toFixed(1);
    if (els.outCap) els.outCap.textContent = (data.outgoing_cap || 0).toFixed(1);
    if (els.inCap) els.inCap.textContent = (data.incoming_cap || 0).toFixed(1);
    if (els.outNeut) els.outNeut.textContent = (data.outgoing_neut || 0).toFixed(1);
    if (els.inNeut) els.inNeut.textContent = (data.incoming_neut || 0).toFixed(1);

    if (els.combatBreakdown) {
      this.renderBreakdown(els.combatBreakdown, data, characters);
    }
  },

  renderBreakdown: function (container, data, characters) {
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
    chars.sort((a, b) => a[0].localeCompare(b[0]));

    chars.forEach(([name, actions]) => {
      // Calculate totals by type and direction for badges
      let stats = {
        out: { dps: 0, hps: 0, cap: 0, neut: 0 },
        in: { dps: 0, hps: 0, cap: 0, neut: 0 }
      };

      actions.forEach(act => {
        let dir = act.incoming ? 'in' : 'out';
        if (act.action_type === "Damage") stats[dir].dps += act.value;
        else if (act.action_type === "Repair") stats[dir].hps += act.value;
        else if (act.action_type === "Capacitor") stats[dir].cap += act.value;
        else if (act.action_type === "Neut") stats[dir].neut += act.value;
      });

      // Build Header with Badges
      let headerBadges = "";

      // Helper to add badge if value > 0
      const addBadge = (val, type, dir) => {
        if (val <= 0) return;
        const style = this.getMetricStyle(type, dir === 'in');
        const prefix = (type !== 'Damage') ? type + ' ' : '';
        headerBadges += `<span class="${style.class}" style="margin-left:6px">${prefix}${val.toFixed(1)}</span>`;
      };

      addBadge(stats.out.dps, "Damage", "out");
      addBadge(stats.in.dps, "Damage", "in");
      addBadge(stats.out.hps, "Repair", "out");
      addBadge(stats.in.hps, "Repair", "in");
      addBadge(stats.out.cap, "Capacitor", "out");
      addBadge(stats.in.cap, "Capacitor", "in");
      addBadge(stats.out.neut, "Neut", "out");
      addBadge(stats.in.neut, "Neut", "in");

      // Fallback for idle tracked chars
      if (headerBadges === "") headerBadges = `<span class="text-dps-out" style="margin-left:6px">0.0</span>`;

      html += `<div style="margin-bottom:8px; border-bottom:1px solid var(--border-color-dim); padding-bottom:4px;">
          <div style="display:flex; justify-content:space-between; font-weight:700">
            <span>${name}</span>
            <div>${headerBadges}</div>
          </div>`;

      // Sort actions: Outgoing first, then Incoming. Within each, by value desc.
      actions.sort((a, b) => {
        if (a.incoming !== b.incoming) return a.incoming ? 1 : -1;
        return b.value - a.value;
      });

      actions.forEach(act => {
        const style = this.getMetricStyle(act.action_type, act.incoming);
        const prefix = act.incoming ? "↙ " : "↗ ";

        html += `<div class="${style.class}" style="font-size:9px; display:flex; justify-content:space-between; align-items:center; margin-left: 4px;">
              <span>${prefix}${act.name}</span>
              <span>${act.value.toFixed(1)} <span style="font-size:7px; opacity:0.7">${style.label}</span></span>
            </div>`;
      });
      html += `</div>`;
    });

    container.innerHTML = html;
  }
};
