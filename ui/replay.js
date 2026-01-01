try {
  const { listen } = window.__TAURI__.event;
  const { invoke } = window.__TAURI__.core;

  const els = {
    browseBtn: document.getElementById("browse-btn"),
    sessionPath: document.getElementById("session-path"),
    sessionSelect: document.getElementById("session-select"),
    sessionDetails: document.getElementById("session-details"),
    
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

  async function refreshSessions(dir) {
    els.sessionPath.textContent = "Scanning...";
    els.sessionSelect.innerHTML = "<option value=''>Scanning...</option>";
    
    try {
        const sessions = await invoke("get_replay_sessions", { path: dir });
        currentSessions = sessions;
        
        els.sessionSelect.innerHTML = "<option value=''>Select a Session...</option>";
        
        sessions.forEach((sess, idx) => {
            // SystemTime from Rust via serde: { secs_since_epoch: ..., nanos_since_epoch: ... }
            const secs = sess.timestamp.secs_since_epoch;
            const date = new Date(secs * 1000);
            
            const label = `${date.toLocaleString()} (${sess.logs.length} chars)`;
            const opt = document.createElement("option");
            opt.value = idx;
            opt.textContent = label;
            els.sessionSelect.appendChild(opt);
        });
        
        els.sessionPath.textContent = dir;
    } catch (e) {
        console.error(e);
        els.sessionPath.textContent = "Error scanning: " + e;
        els.sessionSelect.innerHTML = "<option value=''>Error</option>";
    }
  }

  async function init() {
    // 1. Initial Data Fetch
    try {
        const settings = await invoke("get_settings");
        currentLogDir = settings.gamelog_dir;
        await refreshSessions(currentLogDir);
    } catch (e) { console.error(e); }

    // 2. Event Listeners
    els.browseBtn.onclick = async () => {
        try {
            const path = await invoke("pick_gamelog_dir");
            if (path) {
                currentLogDir = path;
                await refreshSessions(currentLogDir);
            }
        } catch (e) { console.error(e); }
    };
    
    els.sessionSelect.onchange = () => {
        const idx = els.sessionSelect.value;
        els.sessionDetails.innerHTML = "";
        
        if (idx === "") return;
        
        const session = currentSessions[idx];
        if (!session) return;

        session.logs.forEach(log => {
            const div = document.createElement("div");
            div.style.display = "flex";
            div.style.alignItems = "center";
            div.style.marginBottom = "2px";
            
            const cb = document.createElement("input");
            cb.type = "checkbox";
            cb.checked = true; 
            cb.style.marginRight = "6px";
            
            log.enabled = true;
            cb.onchange = () => log.enabled = cb.checked;
            
            const span = document.createElement("span");
            span.textContent = log.character;
            
            div.appendChild(cb);
            div.appendChild(span);
            els.sessionDetails.appendChild(div);
        });
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
    await listen("dps-update", (e) => {
         Components.updateUI(els, e.payload, []);
    });
  }

  init();

} catch (err) {
  document.body.innerHTML = `<h3 style='color:red; background:black'>ERROR: ${err.message}</h3>`;
  console.error(err);
}