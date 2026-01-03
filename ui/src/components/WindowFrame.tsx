import { type FC, type ReactNode } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import '../styles/window.css';

interface WindowFrameProps {
    title?: string;
    children: ReactNode;
    variant: 'main' | 'replay';
    /* Actions to render in the title bar (e.g. settings buttons) */
    headerActions?: ReactNode;
}

const WindowFrame: FC<WindowFrameProps> = ({ title = "AbyssWatcher", children, variant, headerActions }) => {
    const appWindow = getCurrentWindow();

    const startResize = (direction: string) => {
        // @ts-ignore - Valid tauri v2 call
        appWindow.startResizeDragging(direction);
    };

    const handleClose = () => appWindow.close();
    const handleMinimize = () => appWindow.minimize();
    const handleMaximize = () => appWindow.toggleMaximize();

    const handleDrag = (e: React.MouseEvent) => {
        // Only drag on left click
        if (e.button === 0) {
            appWindow.startDragging();
        }
    };

    return (
        <div className="window-frame">
            {/* Resize Handles */}
            <div className="resize-handle top" onMouseDown={() => startResize('top')} />
            <div className="resize-handle bottom" onMouseDown={() => startResize('bottom')} />
            <div className="resize-handle left" onMouseDown={() => startResize('left')} />
            <div className="resize-handle right" onMouseDown={() => startResize('right')} />
            <div className="resize-handle top-left" onMouseDown={() => startResize('topLeft')} />
            <div className="resize-handle top-right" onMouseDown={() => startResize('topRight')} />
            <div className="resize-handle bottom-left" onMouseDown={() => startResize('bottomLeft')} />
            <div className="resize-handle bottom-right" onMouseDown={() => startResize('bottomRight')} />

            {/* Title Bar - Manual Drag Handler */}
            <div className="title-bar" onMouseDown={handleDrag}>
                <div className="title-drag-region">
                    {/* Icon or Logo could go here */}
                    <span className="title-text">{title}</span>
                </div>

                <div className="window-controls" onMouseDown={(e) => e.stopPropagation()}>
                    {/* Custom Header Actions (Chars, Settings, Logs etc) */}
                    {headerActions && (
                        <div className="header-actions">
                            {headerActions}
                        </div>
                    )}

                    {/* Window System Buttons */}
                    <button className="sys-btn" onClick={handleMinimize} title="Minimize">─</button>
                    {variant === 'replay' && (
                        <button className="sys-btn" onClick={handleMaximize} title="Maximize">□</button>
                    )}

                    <button className="sys-btn close" onClick={handleClose} title="Close">✕</button>
                </div>
            </div>

            {/* Content */}
            <div className="window-content">
                {children}
            </div>
        </div>
    );
};

export default WindowFrame;
