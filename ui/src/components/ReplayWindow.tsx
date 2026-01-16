import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import StatusBar from './StatusBar';
import CombatBreakdown from './CombatBreakdown';
import ReplayControls from './ReplayControls';
import LogBrowser from './LogBrowser';
import RawLogViewer from './RawLogViewer';
import type { DpsUpdate, CharacterState, Bookmark } from '../types';
import '../styles/replay.css';

interface ReplayStatus {
    current_time: number;
    progress: number;
    duration: number;
}

function ReplayWindow() {
    const [dpsData, setDpsData] = useState<DpsUpdate | null>(null);
    const [status, setStatus] = useState<ReplayStatus>({ current_time: 0, progress: 0, duration: 100 });
    const [isPlaying, setIsPlaying] = useState(false);
    const [speed, setSpeed] = useState(1.0);
    const [rawLogs, setRawLogs] = useState<string[]>([]);

    const [showLogs, setShowLogs] = useState(true); // Default to open
    const [showDebug, setShowDebug] = useState(false);

    // Characters are derived from data for replay
    const [characters, setCharacters] = useState<CharacterState[]>([]);

    // Bookmarks for timeline notches
    const [bookmarks, setBookmarks] = useState<Bookmark[]>([]);
    const [sessionStartTime, setSessionStartTime] = useState(0);
    const [currentGamelogPath, setCurrentGamelogPath] = useState<string | null>(null);

    // Hoisted state for log selection
    const [selectedLogs, setSelectedLogs] = useState<Set<string>>(new Set());

    useEffect(() => {
        const unlistenUpdate = listen<DpsUpdate>('replay-dps-update', (event) => {
            setDpsData(event.payload);
            // Extract chars from data and accumulate them
            setCharacters(prev => {
                const newCharNames = Object.keys(event.payload.combat_actions_by_character || {});
                const existingNames = new Set(prev.map(c => c.character));

                const newChars: CharacterState[] = newCharNames
                    .filter(name => !existingNames.has(name))
                    .map(name => ({
                        character: name,
                        path: '', // Replay doesn't need path for now
                        tracked: true
                    }));

                return [...prev, ...newChars];
            });
        });

        const unlistenStatus = listen<{ current_time: number, progress: number }>('replay-status', (event) => {
            setStatus(prev => ({ ...prev, ...event.payload }));
        });

        const unlistenRaw = listen<string[]>('replay-raw-lines', (event) => {
            setRawLogs(prev => [...prev, ...event.payload].slice(-100));
        });

        return () => {
            unlistenUpdate.then(f => f());
            unlistenStatus.then(f => f());
            unlistenRaw.then(f => f());
            invoke('stop_replay').catch(console.error);
        };
    }, []);

    interface ReplaySessionInfo {
        duration: number;
        start_time: number;
    }

    const handleStartReplay = async () => {
        try {
            const settings = await invoke<{ gamelog_dir: string }>('get_settings');
            const logs = await invoke<Record<string, { path: string }[]>>('get_logs_by_character', { path: settings.gamelog_dir });

            const selection: [string, string][] = [];
            Object.entries(logs).forEach(([char, files]) => {
                files.forEach(f => {
                    if (selectedLogs.has(f.path)) {
                        selection.push([char, f.path]);
                    }
                });
            });

            if (selection.length === 0) {
                alert("Please select at least one log file.");
                return;
            }

            const info = await invoke<ReplaySessionInfo>('start_replay', { logs: selection });
            setStatus(p => ({ ...p, duration: info.duration }));
            setSessionStartTime(info.start_time);
            setIsPlaying(true); // Auto-play enabled
            setShowLogs(false);

            if (selection.length > 0) {
                setCurrentGamelogPath(selection[0][1]);
                try {
                    const bks = await invoke<Bookmark[]>('get_session_bookmarks', { gamelogPath: selection[0][1] });
                    setBookmarks(bks);
                } catch (err) {
                    setBookmarks([]);
                }
            }
        } catch (e) {
            console.error(e);
            alert('Error starting replay: ' + e);
        }
    };


    const handleToggleLog = (path: string, checked: boolean) => {
        if (path === '__ALL__' && !checked) {
            setSelectedLogs(new Set());
            return;
        }

        setSelectedLogs(prev => {
            const next = new Set(prev);
            if (checked) next.add(path);
            else next.delete(path);
            return next;
        });
    };

    const handlePlayPause = async () => {
        if (!isPlaying && status.duration === 100 && status.progress === 0) {
            // Not started
            setShowLogs(true);
            return;
        }
        await invoke('toggle_replay_pause');
        setIsPlaying(!isPlaying);
    };

    const handleScrub = async (val: number) => {
        await invoke('seek_replay', { offsetSecs: val });
    };

    const formatTime = (secs: number) => {
        const m = Math.floor(secs / 60);
        const s = Math.floor(secs % 60);
        return `${m}:${s.toString().padStart(2, '0')}`;
    };

    const handleStopReplay = () => {
        invoke('stop_replay').catch(console.error);
        setIsPlaying(false);
        setStatus(p => ({ ...p, progress: 0, current_time: 0 }));
    };

    const handleDetectFilaments = async () => {
        if (!currentGamelogPath) {
            alert('No log loaded. Start a replay first.');
            return;
        }
        try {
            // Trigger filament detection
            await invoke('detect_filaments', { gamelogPath: currentGamelogPath });
            // Reload bookmarks from gamelog
            const bks = await invoke<Bookmark[]>('get_session_bookmarks', { gamelogPath: currentGamelogPath });
            setBookmarks(bks);
            console.log('Reloaded bookmarks:', bks);
            alert(`Detected filaments. Found ${bks.filter(b => b.bookmark_type === 'RUN_START').length} run(s).`);
        } catch (e) {
            console.error('Detect filaments failed:', e);
            alert('Detect filaments failed: ' + e);
        }
    };

    return (
        <div id="app" className="replay-suite" style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
            <div id="header">
                <h1>Replay Suite</h1>
                <div className="header-controls">
                    <button
                        className="icon-btn"
                        onClick={handleDetectFilaments}
                        title="Detect Abyss runs from chat logs"
                        disabled={!currentGamelogPath}
                    >
                        🔍 Detect
                    </button>
                    <button className="icon-btn" onClick={() => setShowDebug(!showDebug)}>Debug</button>
                    <button className="icon-btn" onClick={() => setShowLogs(!showLogs)}>Logs</button>
                </div>
            </div>

            <div id="data-container" style={{ flexGrow: 1, overflowY: 'auto' }}>
                <StatusBar combatActions={dpsData?.combat_actions_by_character ?? null} />
                <CombatBreakdown data={dpsData} characters={characters} defaultExpanded={true} />
            </div>

            <ReplayControls
                isPlaying={isPlaying}
                timeDisplay={`${formatTime(status.progress)} / ${formatTime(status.duration)}`}
                progress={status.progress}
                maxProgress={status.duration}
                speed={speed}
                onPlayPause={handlePlayPause}
                onStep={() => invoke('step_replay')}
                onScrub={handleScrub}
                onSpeedChange={(s) => { invoke('set_replay_speed', { speed: s }); setSpeed(s); }}
                bookmarks={bookmarks}
                sessionStartTime={sessionStartTime}
            />

            {showLogs && (
                <LogBrowser
                    onClose={() => setShowLogs(false)}
                    onStartReplay={handleStartReplay}
                    onStopReplay={handleStopReplay}
                    selectedLogs={selectedLogs}
                    onToggleLog={handleToggleLog}
                />
            )}

            {showDebug && (
                <RawLogViewer
                    logs={rawLogs}
                    onClose={() => setShowDebug(false)}
                />
            )}
        </div>
    );
}

export default ReplayWindow;
