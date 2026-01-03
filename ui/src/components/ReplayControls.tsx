import { type FC } from 'react';
import type { Bookmark } from '../types';

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
    /** Bookmarks to display as notches on the timeline */
    bookmarks?: Bookmark[];
    /** Session start time in seconds (for calculating notch positions) */
    sessionStartTime?: number;
}

/** Get color for bookmark type */
const getBookmarkColor = (type: string): string => {
    switch (type) {
        case 'RunStart': return 'var(--accent-green)';
        case 'RunEnd': return 'var(--accent-red)';
        case 'RoomStart': return 'var(--accent-blue)';
        case 'RoomEnd': return 'var(--accent-blue)';
        case 'Highlight': return 'var(--accent-yellow, #ffcc00)';
        default: return 'var(--text-main)';
    }
};

const ReplayControls: FC<ReplayControlsProps> = ({
    isPlaying,
    timeDisplay,
    progress,
    maxProgress,
    speed,
    onPlayPause,
    onStep,
    onScrub,
    onSpeedChange,
    bookmarks = [],
    sessionStartTime = 0
}) => {
    // Calculate notch positions as percentages
    const notches = bookmarks.map(b => {
        const relativeTime = b.timestamp_secs - sessionStartTime;
        const percent = maxProgress > 0 ? (relativeTime / maxProgress) * 100 : 0;
        return {
            type: b.bookmark_type,
            percent: Math.max(0, Math.min(100, percent)),
            label: b.label,
            timestamp: relativeTime
        };
    }).filter(n => n.percent >= 0 && n.percent <= 100);

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

            <div id="timeline-container" style={{
                flexGrow: 1,
                display: 'flex',
                alignItems: 'center',
                position: 'relative'
            }}>
                <input
                    type="range"
                    min="0"
                    max={maxProgress}
                    value={progress}
                    onChange={(e) => onScrub(parseInt(e.target.value))}
                    style={{ width: '100%' }}
                />

                {/* Bookmark Notches */}
                {notches.map((notch, i) => (
                    <div
                        key={i}
                        className="timeline-notch"
                        style={{
                            position: 'absolute',
                            left: `${notch.percent}%`,
                            top: '50%',
                            transform: 'translate(-50%, -50%)',
                            width: '4px',
                            height: '16px',
                            background: getBookmarkColor(notch.type),
                            borderRadius: '2px',
                            pointerEvents: 'auto',
                            cursor: 'pointer',
                            opacity: 0.9,
                            zIndex: 10
                        }}
                        title={`${notch.type}${notch.label ? ': ' + notch.label : ''} (${Math.floor(notch.timestamp / 60)}:${(notch.timestamp % 60).toString().padStart(2, '0')})`}
                        onClick={(e) => {
                            e.stopPropagation();
                            onScrub(notch.timestamp);
                        }}
                    />
                ))}
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
