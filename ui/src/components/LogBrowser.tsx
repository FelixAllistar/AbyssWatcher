import { FC, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface LogFile {
    path: string;
    session_start: { secs_since_epoch: number };
    enabled?: boolean;
}

interface LogBrowserProps {
    onClose: () => void;
    onStartReplay: (logs: [string, string][]) => void;
}

const LogBrowser: FC<LogBrowserProps> = ({ onClose, onStartReplay }) => {
    const [currentPath, setCurrentPath] = useState('');
    const [charLogs, setCharLogs] = useState<Record<string, LogFile[]>>({});
    const [selectedLogs, setSelectedLogs] = useState<Set<string>>(new Set());

    const refreshLogs = async (dir: string) => {
        try {
            const logs = await invoke<Record<string, LogFile[]>>('get_logs_by_character', { path: dir });
            // Restore selection state
            const processedLogs: Record<string, LogFile[]> = {};

            Object.entries(logs).forEach(([char, files]) => {
                processedLogs[char] = files.map(f => ({ ...f, enabled: selectedLogs.has(f.path) }));
            });
            setCharLogs(processedLogs);
        } catch (e) {
            console.error(e);
        }
    };

    useEffect(() => {
        const init = async () => {
            const settings = await invoke<{ gamelog_dir: string }>('get_settings');
            setCurrentPath(settings.gamelog_dir);
            refreshLogs(settings.gamelog_dir);
        };
        init();
    }, []);

    const handleToggleLog = (path: string, checked: boolean) => {
        setSelectedLogs(prev => {
            const next = new Set(prev);
            if (checked) next.add(path);
            else next.delete(path);
            return next;
        });

        setCharLogs(prev => {
            const next = { ...prev };
            Object.keys(next).forEach(char => {
                next[char] = next[char].map(log =>
                    log.path === path ? { ...log, enabled: checked } : log
                );
            });
            return next;
        });
    };

    const handleStart = () => {
        const selection: [string, string][] = [];
        Object.entries(charLogs).forEach(([char, logs]) => {
            logs.forEach(log => {
                if (selectedLogs.has(log.path)) {
                    selection.push([char, log.path]);
                }
            });
        });

        if (selection.length === 0) {
            alert("Please select at least one log file.");
            return;
        }
        onStartReplay(selection);
    };

    const handleBrowse = async () => {
        try {
            const path = await invoke<string | null>('pick_gamelog_dir');
            if (path) {
                setCurrentPath(path);
                refreshLogs(path);
            }
        } catch (e) { console.error(e); }
    };

    return (
        <div style={{
            position: 'absolute',
            top: 40, bottom: 60, left: 10, right: 10,
            background: 'rgba(15,15,20,0.95)',
            border: '1px solid var(--border-color)',
            zIndex: 200,
            display: 'flex',
            flexDirection: 'column',
            padding: '10px',
            gap: '10px'
        }}>
            <div style={{ display: 'flex', gap: '8px' }}>
                <button className="icon-btn" onClick={handleBrowse}>Folder...</button>
                <span className="text-dim text-xs" style={{ whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                    {currentPath || 'No directory selected'}
                </span>
            </div>

            <div style={{ flexGrow: 1, overflowY: 'auto', background: 'rgba(0,0,0,0.2)', padding: '5px' }}>
                {Object.entries(charLogs).sort().map(([char, logs]) => (
                    <div key={char} style={{ marginBottom: '10px' }}>
                        <div className="text-dps-out" style={{ fontWeight: 'bold', fontSize: '12px', marginBottom: '4px' }}>{char}</div>
                        {logs.slice(0, 10).map((log, i) => (
                            <div key={log.path} style={{ display: 'flex', fontSize: '10px', alignItems: 'center' }}>
                                <input
                                    type="checkbox"
                                    checked={selectedLogs.has(log.path)}
                                    onChange={(e) => handleToggleLog(log.path, e.target.checked)}
                                    style={{ marginRight: '6px' }}
                                />
                                <span>{new Date(log.session_start.secs_since_epoch * 1000).toLocaleString()}</span>
                            </div>
                        ))}
                        {logs.length > 10 && <div className="text-dim text-xs" style={{ marginLeft: '18px' }}>+ {logs.length - 10} more...</div>}
                    </div>
                ))}
            </div>

            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
                <button className="icon-btn" onClick={onClose}>Cancel</button>
                <button className="icon-btn primary-btn" onClick={handleStart}>Load Selection</button>
            </div>
        </div>
    );
};

export default LogBrowser;
