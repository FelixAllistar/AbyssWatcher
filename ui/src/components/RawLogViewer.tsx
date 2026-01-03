import { type FC, useEffect, useRef } from 'react';

interface RawLogViewerProps {
    logs: string[];
    onClose: () => void;
}

const RawLogViewer: FC<RawLogViewerProps> = ({ logs, onClose }) => {
    const endRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        endRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [logs]);

    return (
        <div style={{
            position: 'absolute',
            top: 40, bottom: 60, left: 10, right: 10,
            background: 'rgba(0,0,0,0.9)',
            border: '1px solid var(--color-dps-in)',
            zIndex: 200,
            display: 'flex',
            flexDirection: 'column',
            padding: '10px',
            color: '#0f0',
            fontFamily: 'monospace',
            fontSize: '10px'
        }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '5px' }}>
                <span>RAW LOG STREAM</span>
                <button className="icon-btn" onClick={onClose}>Close</button>
            </div>
            <div style={{ flexGrow: 1, overflowY: 'auto', whiteSpace: 'pre-wrap' }}>
                {logs.map((line, i) => (
                    <div key={i}>{line}</div>
                ))}
                <div ref={endRef} />
            </div>
        </div>
    );
};

export default RawLogViewer;
