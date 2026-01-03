import { FC } from 'react';

interface ReplayControlsProps {
    isPlaying: boolean;
    timeDisplay: string;
    progress: number;
    maxProgress: number;
    speed: number;
    onPlayPause: () => void;
    onStep: () => void;
    onScrub: (value: number) => void;
    onSpeedChange: (speed: number) => void;
}

const ReplayControls: FC<ReplayControlsProps> = ({
    isPlaying,
    timeDisplay,
    progress,
    maxProgress,
    speed,
    onPlayPause,
    onStep,
    onScrub,
    onSpeedChange
}) => {
    return (
        <div id="replay-controls" style={{
            display: 'flex',
            alignItems: 'center',
            gap: '8px',
            padding: '8px',
            borderTop: '1px solid var(--border-color)',
            background: 'var(--bg-panel)',
            marginTop: 'auto'
        }}>
            <button className="icon-btn" onClick={onPlayPause}>
                {isPlaying ? 'Pause' : 'Play'}
            </button>
            <button className="icon-btn" onClick={onStep} disabled={isPlaying}>
                Next
            </button>

            <div id="timeline-container" style={{ flexGrow: 1, display: 'flex', alignItems: 'center' }}>
                <input
                    type="range"
                    min="0"
                    max={maxProgress}
                    value={progress}
                    onChange={(e) => onScrub(parseInt(e.target.value))}
                    style={{ width: '100%' }}
                />
            </div>

            <span className="text-sm text-dim" style={{ minWidth: '80px', textAlign: 'center' }}>
                {timeDisplay}
            </span>

            <select
                value={speed}
                onChange={(e) => onSpeedChange(parseFloat(e.target.value))}
                style={{
                    background: 'var(--btn-bg)',
                    color: 'var(--text-main)',
                    border: '1px solid var(--border-color)',
                    borderRadius: '4px',
                    fontSize: '10px',
                    padding: '2px'
                }}
            >
                <option value="0.5">0.5x</option>
                <option value="1.0">1.0x</option>
                <option value="2.0">2.0x</option>
                <option value="5.0">5.0x</option>
                <option value="10.0">10.0x</option>
            </select>
        </div>
    );
};

export default ReplayControls;
