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

  // Persistent state for collapsibility
  collapseState: new Set(),

  // Store latest render data for click handler
  _lastData: null,
  _lastCharacters: null,

  toggleCollapse: function (id, container) {
    if (this.collapseState.has(id)) {
      this.collapseState.delete(id);
    } else {
      this.collapseState.add(id);
    }

    // Use CSS class toggle instead of full re-render to avoid ghosting issues
    const header = container.querySelector(`[data-collapse-id="${id}"]`);
    if (header) {
      const indicator = header.querySelector('.collapse-indicator');
      const isCollapsed = this.collapseState.has(id);

      if (indicator) {
        indicator.textContent = isCollapsed ? '▶' : '▼';
      }

      // Find the content sibling and toggle its visibility
      const content = header.nextElementSibling;
      if (content) {
        content.classList.toggle('collapsed-content', isCollapsed);
      }
    }
  },

  renderBreakdown: function (container, data, characters) {
    // Store for click handler
    this._lastData = data;
    this._lastCharacters = characters;

    if (!container._hasCollapseListener) {
      container.addEventListener('click', (e) => {
        // Use mousedown target workaround - stop if mid-render
        const header = e.target.closest('[data-collapse-id]');
        if (header) {
          e.preventDefault();
          e.stopPropagation();
          const id = header.dataset.collapseId;
          this.toggleCollapse(id, container);
        }
      });
      container._hasCollapseListener = true;
    }

    let html = "";
    const activeData = new Map(Object.entries(data.combat_actions_by_character || {}));

    characters.forEach(char => {
      if (char.tracked && !activeData.has(char.character)) {
        activeData.set(char.character, []);
      }
    });

    const chars = Array.from(activeData.entries());
    chars.sort((a, b) => a[0].localeCompare(b[0]));

    chars.forEach(([name, actions]) => {
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

      let badgesHtml = "";
      const addBadge = (outVal, inVal, type) => {
        if (outVal <= 0 && inVal <= 0) return;
        const outStyle = this.getMetricStyle(type, false);
        const inStyle = this.getMetricStyle(type, true);

        badgesHtml += `<div class="badge">
          ${outVal > 0 ? `<span class="${outStyle.class}">↗ ${outVal.toFixed(0)}</span>` : `<span class="text-dim" style="opacity:0.4">↗ 0</span>`}
          <span class="badge-sep">|</span>
          ${inVal > 0 ? `<span class="${inStyle.class}">${inVal.toFixed(0)} ↙</span>` : `<span class="text-dim" style="opacity:0.4">0 ↙</span>`}
        </div>`;
      };

      addBadge(stats.out.dps, stats.in.dps, "Damage");
      addBadge(stats.out.hps, stats.in.hps, "Repair");
      addBadge(stats.out.cap, stats.in.cap, "Capacitor");
      addBadge(stats.out.neut, stats.in.neut, "Neut");

      if (badgesHtml === "") badgesHtml = `<span class="badge text-dim" style="opacity:0.4">IDLE</span>`;

      const charCollapseId = `char-${name}`;
      const isCharCollapsed = this.collapseState.has(charCollapseId);

      html += `<div class="breakdown-char-card">
          <div class="breakdown-header" data-collapse-id="${charCollapseId}">
            <span class="char-name">${name}${isCharCollapsed ? ' <span class="collapse-indicator">▶</span>' : ' <span class="collapse-indicator">▼</span>'}</span>
            <div class="badge-container">${badgesHtml}</div>
          </div>
          <div class="char-content ${isCharCollapsed ? 'collapsed-content' : ''}">`;

      // Group actions by type
      const groups = {
        "Damage": [],
        "Repair": [],
        "Capacitor": [],
        "Neut": []
      };

      actions.forEach(act => {
        if (groups[act.action_type]) {
          groups[act.action_type].push(act);
        }
      });

      // Render each group
      Object.entries(groups).forEach(([type, items]) => {
        if (items.length === 0) return;

        const groupCollapseId = `group-${name}-${type}`;
        const isGroupCollapsed = this.collapseState.has(groupCollapseId);
        const label = type === 'Damage' ? 'DPS' : type;

        // Sort items: Outgoing first, then value desc
        items.sort((a, b) => {
          if (a.incoming !== b.incoming) return a.incoming ? 1 : -1;
          return b.value - a.value;
        });

        html += `<div class="category-section">
          <div class="category-header" data-collapse-id="${groupCollapseId}">
            <span>${label}</span>
            <span class="collapse-indicator">${isGroupCollapsed ? '▶' : '▼'}</span>
          </div>
          <div class="category-content ${isGroupCollapsed ? 'collapsed-content' : ''}">`;

        items.forEach(act => {
          const style = this.getMetricStyle(act.action_type, act.incoming);
          const icon = act.incoming ? "↙" : "↗";

          html += `<div class="action-row">
                <div class="action-name ${style.class}">
                  <span>${icon}</span>
                  <span>${act.name}</span>
                </div>
                <div class="action-value ${style.class}">
                  ${act.value.toFixed(1)}<span class="action-unit">${style.label}</span>
                </div>
              </div>`;
        });
        html += `</div></div>`;
      });
      html += `</div></div>`;
    });

    container.innerHTML = html;
  }
};
