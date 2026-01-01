try {
  const { listen } = window.__TAURI__.event;
  const { invoke } = window.__TAURI__.core;

  const els = {
    toggleLogsBtn: document.getElementById("toggle-logs"),
    browseBtn: document.getElementById("browse-btn"),
    sessionPath: document.getElementById("session-path"),
    sessionList: document.getElementById("session-list"),
    sessionPanel: document.getElementById("session-panel"),
    
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
  let characters = {}; // Grouped logs
  let isPlaying = false;
  let currentLogDir = "";

  function renderLogRow(log) {
    const logRow = document.createElement("div");
    logRow.style.display = "flex";
    logRow.style.alignItems = "center";
    logRow.style.fontSize = "10px";
    logRow.style.marginBottom = "2px";
    
    const cb = document.createElement("input");
    cb.type = "checkbox";
    cb.style.marginRight = "6px";
    cb.checked = log.enabled || false;
    cb.onchange = () => {
        log.enabled = cb.checked;
    };
    
    const secs = log.session_start.secs_since_epoch;
    const date = new Date(secs * 1000);
    const label = document.createElement("span");
    label.textContent = date.toLocaleString();
    
    logRow.appendChild(cb);
    logRow.appendChild(label);
    return logRow;
  }

  async function refreshLogs(dir) {
    els.sessionPath.textContent = "Scanning...";
    els.sessionList.innerHTML = "";
    
    try {
        const charLogs = await invoke("get_logs_by_character", { path: dir });
        characters = charLogs;
        
        const sortedChars = Object.keys(charLogs).sort();
        
        if (sortedChars.length === 0) {
            els.sessionList.innerHTML = "<div style='color:#aaa; padding:8px'>No logs found in this directory.</div>";
        }

        sortedChars.forEach(char => {
            const logs = charLogs[char];
            const charDiv = document.createElement("div");
            charDiv.style.marginBottom = "12px";
            charDiv.style.background = "rgba(255,255,255,0.02)";
            charDiv.style.padding = "6px";
            charDiv.style.borderRadius = "4px";
            charDiv.innerHTML = `<div style="font-weight:bold; color:var(--accent-out); margin-bottom:6px; font-size:12px">${char}</div>`;
            
            const logsDiv = document.createElement("div");
            logsDiv.style.paddingLeft = "8px";
            
            const visibleCount = 10;
            const initialLogs = logs.slice(0, visibleCount);
            initialLogs.forEach(log => logsDiv.appendChild(renderLogRow(log)));
            
            if (logs.length > visibleCount) {
                const moreBtn = document.createElement("button");
                moreBtn.className = "icon-btn";
                moreBtn.style.marginTop = "4px";
                moreBtn.style.fontSize = "9px";
                moreBtn.textContent = `Show ${logs.length - visibleCount} more...`;
                moreBtn.onclick = () => {
                    logs.slice(visibleCount).forEach(log => logsDiv.appendChild(renderLogRow(log)));
                    moreBtn.remove();
                };
                logsDiv.appendChild(moreBtn);
            }
            
            charDiv.appendChild(logsDiv);
            els.sessionList.appendChild(charDiv);
        });
        
        els.sessionPath.textContent = dir;
    } catch (e) {
        console.error(e);
        els.sessionPath.textContent = "Error scanning: " + e;
    }
  }

  async function init() {
    // 1. Initial Data Fetch
    try {
        const settings = await invoke("get_settings");
        currentLogDir = settings.gamelog_dir;
        await refreshLogs(currentLogDir);
    } catch (e) { console.error(e); }

    // 2. Event Listeners
    els.toggleLogsBtn.onclick = () => {
        els.sessionPanel.classList.toggle("hidden");
    };

    els.browseBtn.onclick = async () => {
        try {
            const path = await invoke("pick_gamelog_dir");
            if (path) {
                currentLogDir = path;
                await refreshLogs(currentLogDir);
            }
        } catch (e) { console.error(e); }
    };
    
    els.playPauseBtn.onclick = async () => {
        isPlaying = !isPlaying;
        els.playPauseBtn.textContent = isPlaying ? "Pause" : "Play";
        // TODO: Start replay with selected logs
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
