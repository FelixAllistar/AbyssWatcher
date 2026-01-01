try {
  const { listen } = window.__TAURI__.event;
  const { invoke } = window.__TAURI__.core;

  const els = {
    browseBtn: document.getElementById("browse-btn"),
    sessionPath: document.getElementById("session-path"),
    sessionSelect: document.getElementById("session-select"),
    
    playPauseBtn: document.getElementById("play-pause-btn"),
    timeline: document.getElementById("timeline"),
    timeDisplay: document.getElementById("time-display"),
    speedSelect: document.getElementById("speed-select"),
    
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
  let currentSessions = [];
  let isPlaying = false;
  let currentLogDir = "";

  async function init() {
    // 1. Initial Data Fetch
    try {
        const settings = await invoke("get_settings");
        currentLogDir = settings.gamelog_dir;
        els.sessionPath.textContent = currentLogDir;
        
        // TODO: invoke("discover_sessions", { path: currentLogDir })
        // For now, placeholder
    } catch (e) { console.error(e); }

    // 2. Event Listeners
    els.browseBtn.onclick = async () => {
        try {
            const path = await invoke("pick_gamelog_dir");
            if (path) {
                currentLogDir = path;
                els.sessionPath.textContent = path;
                // invoke("discover_sessions")
            }
        } catch (e) { console.error(e); }
    };

    els.playPauseBtn.onclick = async () => {
        isPlaying = !isPlaying;
        els.playPauseBtn.textContent = isPlaying ? "Pause" : "Play";
        // invoke("replay_toggle", { state: isPlaying })
    };
    
    els.speedSelect.onchange = async () => {
        const speed = parseFloat(els.speedSelect.value);
        // invoke("replay_set_speed", { speed })
    };

    els.timeline.oninput = async () => {
        // invoke("replay_scrub", { timestamp: ... })
    };

    // 3. Listen for updates
    // Listen for replay-specific updates? Or reuse dps-update?
    // The Spec says: "Visual Parity... must use the same UI components"
    // So we should expect "dps-update" events here too, specifically targeted at this window or broadcasted.
    await listen("dps-update", (e) => {
         // In replay mode, we might want to filter or ensure these are replay events?
         // For now, assume dps-update is the universal visual update.
         Components.updateUI(els, e.payload, []); // Passing empty array for characters for now, logic to be added.
    });
    
    await listen("replay-status", (e) => {
        // Update timeline, time display, play status from backend
    });
  }

  init();

} catch (err) {
  document.body.innerHTML = `<h3 style='color:red; background:black'>ERROR: ${err.message}</h3>`;
  console.error(err);
}
