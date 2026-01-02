// ui/components/index.js
// Component registry and exports - loads all component modules

const Components = {
    Styles: {
        damage: { color: "var(--color-dps-out)", label: "DPS" },
        repair: { color: "var(--color-rep-out)", label: "HPS" },
        cap: { color: "var(--color-cap-out)", label: "GJ/s" },
        neut: { color: "var(--color-neut-out)", label: "GJ/s" },
        default: { color: "#aaa", label: "" }
    },

    // Character Selection Component
    renderSelection: function (container, characters, onToggle) {
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

    // DPS/Stats UI Update
    updateUI: function (els, data, characters) {
        // Update main stat values
        if (els.outDps) els.outDps.textContent = data.outgoing_dps.toFixed(1);
        if (els.inDps) els.inDps.textContent = data.incoming_dps.toFixed(1);
        if (els.outHps) els.outHps.textContent = (data.outgoing_hps || 0).toFixed(1);
        if (els.inHps) els.inHps.textContent = (data.incoming_hps || 0).toFixed(1);
        if (els.outCap) els.outCap.textContent = (data.outgoing_cap || 0).toFixed(1);
        if (els.inCap) els.inCap.textContent = (data.incoming_cap || 0).toFixed(1);
        if (els.outNeut) els.outNeut.textContent = (data.outgoing_neut || 0).toFixed(1);
        if (els.inNeut) els.inNeut.textContent = (data.incoming_neut || 0).toFixed(1);

        if (!els.activeList) return;
        this.renderActionList(els.activeList, data, characters);
    },

    // Combat Actions List Component
    renderActionList: function (container, data, characters) {
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
            if (dps > 0 || (hps === 0 && cap === 0 && neut === 0)) headerBadges += `<span style="color:${Components.Styles.damage.color}; margin-left:6px">${dps.toFixed(1)}</span>`;
            if (hps > 0) headerBadges += `<span style="color:${Components.Styles.repair.color}; margin-left:6px">REP ${hps.toFixed(1)}</span>`;
            if (cap > 0) headerBadges += `<span style="color:${Components.Styles.cap.color}; margin-left:6px">CAP ${cap.toFixed(1)}</span>`;
            if (neut > 0) headerBadges += `<span style="color:${Components.Styles.neut.color}; margin-left:6px">NEUT ${neut.toFixed(1)}</span>`;

            html += `<div style="margin-bottom:6px; border-bottom:1px solid rgba(255,255,255,0.05)">
          <div style="display:flex; justify-content:space-between; font-weight:700">
            <span>${name}</span>
            <div>${headerBadges}</div>
          </div>`;

            // Sort and render actions
            actions.sort((a, b) => b.value - a.value);

            actions.forEach(act => {
                const style = this.getActionStyle(act.action_type);
                html += `<div style="font-size:9px; color:${style.color}; display:flex; justify-content:space-between; align-items:center">
              <span>${act.name}</span>
              <span>${act.value.toFixed(1)} <span style="font-size:7px; opacity:0.7">${style.label}</span></span>
            </div>`;
            });
            html += `</div>`;
        });

        container.innerHTML = html;
    },

    // Helper: Get style for action type
    getActionStyle: function (actionType) {
        switch (actionType) {
            case "Damage": return this.Styles.damage;
            case "Repair": return this.Styles.repair;
            case "Capacitor": return this.Styles.cap;
            case "Neut": return this.Styles.neut;
            default: return this.Styles.default;
        }
    },

    // Utility: Format number with precision
    formatNumber: function (value, precision = 1) {
        return (value || 0).toFixed(precision);
    }
};
