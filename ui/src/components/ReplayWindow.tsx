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

    useEffect(() => {
        const unlistenUpdate = listen<DpsUpdate>('replay-dps-update', (event) => {
            setDpsData(event.payload);
            // Extract chars from data
            const chars = Object.keys(event.payload.combat_actions_by_character || {}).map(name => ({
                character: name,
                path: '',
                tracked: true
            }));
            setCharacters(chars);
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
        };
    }, []);

    interface ReplaySessionInfo {
        duration: number;
        start_time: number;
    }

    const handleStartReplay = async (logs: [string, string][]) => {
        try {
            const info = await invoke<ReplaySessionInfo>('start_replay', { logs });
            setStatus(p => ({ ...p, duration: info.duration }));
            setSessionStartTime(info.start_time);
            setIsPlaying(true);
            setShowLogs(false);

            // Store the first log path for bookmark operations
            if (logs.length > 0) {
                setCurrentGamelogPath(logs[0][1]);
                console.log('Loading bookmarks for:', logs[0][1]);
                // Try to load existing bookmarks
                try {
                    const bks = await invoke<Bookmark[]>('get_session_bookmarks', { gamelogPath: logs[0][1] });
                    console.log('Loaded bookmarks:', bks);
                    setBookmarks(bks);
                } catch (err) {
                    console.error('Failed to load bookmarks:', err);
                    setBookmarks([]);
                }
            }
        } catch (e) {
            console.error(e);
            alert('Error starting replay: ' + e);
        }
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
            alert(`Detected filaments. Found ${bks.filter(b => b.bookmark_type === 'RunStart').length} run(s).`);
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
                <CombatBreakdown data={dpsData} characters={characters} />
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
