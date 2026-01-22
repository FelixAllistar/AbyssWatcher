import { check, Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { useEffect, useState, useCallback } from 'react';
import './UpdateChecker.css';

interface UpdateInfo {
    version: string;
    notes?: string;
}

export function UpdateChecker() {
    const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
    const [updateObj, setUpdateObj] = useState<Update | null>(null);
    const [status, setStatus] = useState<'idle' | 'downloading' | 'installing'>('idle');
    const [progress, setProgress] = useState(0);
    const [dismissed, setDismissed] = useState(false);

    useEffect(() => {
        const checkForUpdates = async () => {
            try {
                const update = await check();
                if (update) {
                    setUpdateInfo({ version: update.version, notes: update.body ?? undefined });
                    setUpdateObj(update);
                }
            } catch (e) {
                console.error('[UpdateChecker] Check failed:', e);
            }
        };
        checkForUpdates();
    }, []);

    const handleUpdate = useCallback(async () => {
        if (!updateObj) return;

        setStatus('downloading');
        try {
            let downloaded = 0;
            let total = 0;

            await updateObj.downloadAndInstall((event) => {
                switch (event.event) {
                    case 'Started':
                        total = event.data.contentLength ?? 0;
                        break;
                    case 'Progress':
                        downloaded += event.data.chunkLength;
                        if (total > 0) {
                            setProgress(Math.round((downloaded / total) * 100));
                        }
                        break;
                    case 'Finished':
                        setStatus('installing');
                        break;
                }
            });

            // Relaunch after install
            await relaunch();
        } catch (e) {
            console.error('[UpdateChecker] Update failed:', e);
            setStatus('idle');
        }
    }, [updateObj]);

    if (!updateInfo || dismissed) return null;

    return (
        <div className="update-banner">
            <span className="update-text">
                v{updateInfo.version} Update Available
            </span>
            {status === 'idle' && (
                <div className="update-actions">
                    <button className="update-btn install" onClick={handleUpdate}>
                        Install
                    </button>
                    <button className="update-btn dismiss" onClick={() => setDismissed(true)}>
                        Maybe Later
                    </button>
                </div>
            )}
            {status === 'downloading' && (
                <span className="update-progress">{progress}%</span>
            )}
            {status === 'installing' && (
                <span className="update-progress">Installing...</span>
            )}
        </div>
    );
}

export default UpdateChecker;
