import { type FC, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { SettingsWithAlerts, AlertEngineConfig, CharacterState } from '../types';
import AlertSettings from './AlertSettings';

interface SettingsModalProps {
    settings: SettingsWithAlerts;
    trackedCharacters: CharacterState[];
    onSave: (settings: SettingsWithAlerts) => void;
    onCancel: () => void;
    onOpenReplay: () => void;
}

const SettingsModal: FC<SettingsModalProps> = ({
    settings,
    trackedCharacters,
    onSave,
    onCancel,
    onOpenReplay
}) => {
    const [logDir, setLogDir] = useState(settings.gamelog_dir);
    const [dpsWindow, setDpsWindow] = useState(settings.dps_window_seconds);
    const [alertConfig, setAlertConfig] = useState<AlertEngineConfig>(settings.alert_settings);

    const handleBrowse = async () => {
        try {
            const path = await invoke<string | null>('pick_gamelog_dir');
            if (path) setLogDir(path);
        } catch (e) {
            console.error('Browse failed:', e);
        }
    };

    const handleSave = () => {
        onSave({
            gamelog_dir: logDir,
            dps_window_seconds: dpsWindow,
            alert_settings: alertConfig,
        });
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

            {/* Alert Settings */}
            <AlertSettings
                config={alertConfig}
                trackedCharacters={trackedCharacters}
                onChange={setAlertConfig}
            />

            <div className="modal-actions">
                <button className="icon-btn" style={{ color: '#ffeb3b' }} onClick={onOpenReplay}>
                    ↺ Replay
                </button>
                <button className="icon-btn" onClick={onCancel}>Cancel</button>
                <button className="icon-btn primary-btn" onClick={handleSave}>
                    Save
                </button>
            </div>
        </div>
    );
};

export default SettingsModal;
