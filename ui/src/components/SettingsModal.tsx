import { FC, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';

interface Settings {
    gamelog_dir: string;
    dps_window_seconds: number;
}

interface SettingsModalProps {
    settings: Settings;
    onSave: (settings: Settings) => void;
    onCancel: () => void;
    onOpenReplay: () => void;
}

const SettingsModal: FC<SettingsModalProps> = ({ settings, onSave, onCancel, onOpenReplay }) => {
    const [logDir, setLogDir] = useState(settings.gamelog_dir);
    const [dpsWindow, setDpsWindow] = useState(settings.dps_window_seconds);

    const handleBrowse = async () => {
        try {
            const path = await invoke<string | null>('pick_gamelog_dir');
            if (path) setLogDir(path);
        } catch (e) {
            console.error('Browse failed:', e);
        }
    };

    return (
        <div id="settings-modal">
            <div className="form-group">
                <label>Game Logs Directory</label>
                <div className="form-row">
                    <input type="text" value={logDir} readOnly />
                    <button className="icon-btn" onClick={handleBrowse}>...</button>
                </div>
            </div>
            <div className="form-group">
                <label>DPS Window (Seconds)</label>
                <input
                    type="number"
                    value={dpsWindow}
                    min={1}
                    max={60}
                    onChange={(e) => setDpsWindow(parseInt(e.target.value) || 5)}
                />
            </div>
            <div className="modal-actions">
                <button className="icon-btn" style={{ color: '#ffeb3b' }} onClick={onOpenReplay}>
                    ↺ Replay
                </button>
                <button className="icon-btn" onClick={onCancel}>Cancel</button>
                <button
                    className="icon-btn primary-btn"
                    onClick={() => onSave({ gamelog_dir: logDir, dps_window_seconds: dpsWindow })}
                >
                    Save
                </button>
            </div>
        </div>
    );
};

export default SettingsModal;
