try {
  const { listen } = window.__TAURI__.event;
  const { invoke } = window.__TAURI__.core;

  const els = {
    toggleDebugBtn: document.getElementById("toggle-debug"),
    toggleLogsBtn: document.getElementById("toggle-logs"),
    browseBtn: document.getElementById("browse-btn"),
    sessionPath: document.getElementById("session-path"),
    sessionList: document.getElementById("session-list"),
    sessionPanel: document.getElementById("session-panel"),
    debugPanel: document.getElementById("debug-panel"),
    rawLogsList: document.getElementById("raw-logs-list"),
    
    playPauseBtn: document.getElementById("play-pause-btn"),
    stepBtn: document.getElementById("step-btn"),
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
  let isSessionActive = false;
  let isScrubbing = false;
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
    try {
        const settings = await invoke("get_settings");
        currentLogDir = settings.gamelog_dir;
        await refreshLogs(currentLogDir);
    } catch (e) { console.error(e); }

    els.toggleDebugBtn.onclick = () => {
        els.debugPanel.classList.toggle("hidden");
    };

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
        if (!isSessionActive) {
            // Start new session
            const selectedLogs = [];
            Object.entries(characters).forEach(([char, logs]) => {
                logs.forEach(log => {
                    if (log.enabled) {
                        selectedLogs.push([char, log.path]);
                    }
                });
            });

            if (selectedLogs.length === 0) {
                alert("Please select at least one log file.");
                return;
            }

            try {
                const duration = await invoke("start_replay", { logs: selectedLogs });
                isSessionActive = true;
                els.playPauseBtn.textContent = "Pause";
                els.sessionPanel.classList.add("hidden"); 
                els.timeline.max = duration;
                els.timeline.value = 0;
            } catch (e) {
                alert("Error starting replay: " + e);
            }
        } else {
            // Toggle pause
            try {
                const isPaused = await invoke("toggle_replay_pause");
                els.playPauseBtn.textContent = isPaused ? "Play" : "Pause";
            } catch (e) { console.error(e); }
        }
    };

    els.stepBtn.onclick = async () => {
        if (isSessionActive) {
            await invoke("step_replay");
        }
    };
    
    els.speedSelect.onchange = async () => {
        const speed = parseFloat(els.speedSelect.value);
        await invoke("set_replay_speed", { speed });
    };

    els.timeline.onmousedown = () => isScrubbing = true;
    window.addEventListener("mouseup", () => isScrubbing = false);

    let seekTimeout = null;
    els.timeline.oninput = async () => {
        const offset = parseInt(els.timeline.value);
        
        if (seekTimeout) clearTimeout(seekTimeout);
        seekTimeout = setTimeout(async () => {
            await invoke("seek_replay", { offsetSecs: offset });
        }, 50); // 50ms debounce
    };

    // 3. Listen for updates
    await listen("replay-dps-update", (e) => {
         const data = e.payload;
         // Extract character names from combat_actions_by_character keys
         const replayChars = Object.keys(data.combat_actions_by_character || {}).map(name => ({
             character: name,
             tracked: true
         }));
         Components.updateUI(els, data, replayChars);
    });

    await listen("replay-status", (e) => {
        const { current_time, progress } = e.payload;
        // Update time display
        const minutes = Math.floor(progress / 60);
        const seconds = Math.floor(progress % 60);
        els.timeDisplay.textContent = `${minutes}:${seconds.toString().padStart(2, '0')}`;
        
        if (!isScrubbing) {
            els.timeline.value = progress;
        }
    });

    await listen("replay-raw-lines", (e) => {
        const lines = e.payload;
        lines.forEach(line => {
            const div = document.createElement("div");
            div.textContent = line;
            els.rawLogsList.appendChild(div);
        });
        
        // Keep last 100 lines to avoid DOM bloat
        while (els.rawLogsList.children.length > 100) {
            els.rawLogsList.removeChild(els.rawLogsList.firstChild);
        }
        
        els.rawLogsList.scrollTop = els.rawLogsList.scrollHeight;
    });
  }

  init();

} catch (err) {
  document.body.innerHTML = `<h3 style='color:red; background:black'>ERROR: ${err.message}</h3>`;
  console.error(err);
}