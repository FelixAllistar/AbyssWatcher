try {
    const { listen } = window.__TAURI__.event;
    const { invoke } = window.__TAURI__.core;

    // ===== Linux Ghosting Fix: Debounced Resize =====
    // Strategy: Only trigger the expensive resize when updates PAUSE.
    // This prevents freezing during high-DPS streams.

    let debounceTimer;
    const triggerDebouncedRefresh = () => {
        if (debounceTimer) clearTimeout(debounceTimer);
        debounceTimer = setTimeout(() => {
            invoke('refresh_transparency').catch(e => console.error(e));
            debounceTimer = null;
        }, 200); // Wait for 200ms of silence before refreshing
    };

    const ghostingObserver = new MutationObserver(() => triggerDebouncedRefresh());
    ghostingObserver.observe(document.body, {
        childList: true,
        subtree: true,
        characterData: true,
        attributes: true,
        attributeFilter: ['style', 'class'] // Only watch visual changes
    });
    // ===== End Linux Ghosting Fix =====

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
        combatBreakdown: document.getElementById("combat-breakdown")
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
                console.log("Settings saved. Reloaded characters:", characters);
                renderSelection();
            } catch (e) { console.error(e); }
        };

        els.cancelSettingsBtn.onclick = () => els.settingsModal.classList.add("hidden");

        els.replayBtn.onclick = async () => {
            try {
                await invoke("open_replay_window");
                els.settingsModal.classList.add("hidden");
            } catch (e) { console.error(e); }
        };

        // 2. Load Data
        try {
            const settings = await invoke("get_settings");
            els.logDirInput.value = settings.gamelog_dir;
            els.dpsWindowInput.value = settings.dps_window_seconds;

            characters = await invoke("get_available_characters");
            console.log("Init loaded characters:", characters);
            renderSelection();
        } catch (e) { console.error("Init failed:", e); }

        // 3. Listen
        await listen("dps-update", (e) => Components.updateUI(els, e.payload, characters));
    }

    function renderSelection() {
        Components.renderSelection(els.selectionContainer, characters, (char, isChecked) => {
            char.tracked = isChecked;
            invoke("toggle_tracking", { path: char.path });
        });
    }

    init();

} catch (err) {
    document.body.innerHTML = `<h3 style='color:red; background:black'>ERROR: ${err.message}</h3>`;
    console.error(err);
}
