import { useEffect, useState, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import StatusBar from './components/StatusBar';
import CombatBreakdown from './components/CombatBreakdown';
import SettingsModal from './components/SettingsModal';
import CharacterSelector from './components/CharacterSelector';
import ReplayWindow from './components/ReplayWindow';
import './styles/theme.css';
import './styles/common.css';
import './styles/main.css';
import type { DpsUpdate, CharacterState, Settings, RoomMarkerState, RoomMarkerResponse } from './types';

// Re-export types for other modules that import from App
export type { DpsUpdate, CharacterState, CombatAction, TargetHit, Settings, Bookmark, Run, BookmarkType, RoomMarkerState } from './types';

import WindowFrame from './components/WindowFrame';

function MainApp() {
  const [dpsData, setDpsData] = useState<DpsUpdate | null>(null);
  const [characters, setCharacters] = useState<CharacterState[]>([]);
  const [showSettings, setShowSettings] = useState(false);
  const [showCharacterSelector, setShowCharacterSelector] = useState(false);
  const [settings, setSettings] = useState<Settings>({ gamelog_dir: '', dps_window_seconds: 5 });
  const [roomMarkerState, setRoomMarkerState] = useState<RoomMarkerState>('Idle');

  const charSelectorRef = useRef<HTMLDivElement>(null);
  const settingsRef = useRef<HTMLDivElement>(null);
  const charBtnRef = useRef<HTMLButtonElement>(null);
  const settingsBtnRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      const target = event.target as Node;

      // Close character selector if clicking outside of it and its toggle button
      if (showCharacterSelector &&
        charSelectorRef.current && !charSelectorRef.current.contains(target) &&
        charBtnRef.current && !charBtnRef.current.contains(target)) {
        setShowCharacterSelector(false);
      }

      // Close settings if clicking outside of it and its toggle button
      if (showSettings &&
        settingsRef.current && !settingsRef.current.contains(target) &&
        settingsBtnRef.current && !settingsBtnRef.current.contains(target)) {
        setShowSettings(false);
      }
    };

    if (showCharacterSelector || showSettings) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [showCharacterSelector, showSettings]);

  useEffect(() => {
    // Load initial data
    const init = async () => {
      try {
        const loadedSettings = await invoke<Settings>('get_settings');
        setSettings(loadedSettings);
        const loadedChars = await invoke<CharacterState[]>('get_available_characters');
        setCharacters(loadedChars);
      } catch (e) {
        console.error('Init failed:', e);
      }
    };
    init();

    // Subscribe to DPS updates
    const unlisten = listen<DpsUpdate>('dps-update', (event) => {
      setDpsData(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleToggleTracking = async (char: CharacterState) => {
    try {
      await invoke('toggle_tracking', { path: char.path });
      setCharacters((prev) =>
        prev.map((c) =>
          c.path === char.path ? { ...c, tracked: !c.tracked } : c
        )
      );
    } catch (e) {
      console.error('Toggle tracking failed:', e);
    }
  };

  const handleSaveSettings = async (newSettings: Settings) => {
    try {
      await invoke('save_settings', { settings: newSettings });
      setSettings(newSettings);
      const loadedChars = await invoke<CharacterState[]>('get_available_characters');
      setCharacters(loadedChars);
      setShowSettings(false);
    } catch (e) {
      console.error('Save settings failed:', e);
    }
  };

  const handleOpenReplay = async () => {
    try {
      await invoke('open_replay_window');
      setShowSettings(false);
    } catch (e) {
      console.error('Open replay failed:', e);
    }
  };

  // Get the first tracked character for bookmark operations
  const getActiveCharacter = () => {
    const tracked = characters.filter(c => c.tracked);
    if (tracked.length === 0) return null;
    // For simplicity, use the first tracked character
    // In a real multibox scenario, you might want a dropdown to select
    return tracked[0];
  };

  const handleMarkHighlight = async () => {
    const char = getActiveCharacter();
    if (!char) {
      console.warn('No character tracked, cannot create bookmark');
      return;
    }
    try {
      // Extract character_id from path (filename pattern: Local_DATE_TIME_ID.txt)
      // For gamelogs, we'll use 0 as fallback since they don't have ID in name
      await invoke('create_highlight_bookmark', {
        characterId: 0,
        characterName: char.character,
        gamelogPath: char.path,
        label: null
      });
      console.log('Highlight bookmark created');
    } catch (e) {
      console.error('Create bookmark failed:', e);
    }
  };

  const handleToggleRoom = async () => {
    const char = getActiveCharacter();
    if (!char) {
      console.warn('No character tracked, cannot toggle room marker');
      return;
    }
    try {
      const response = await invoke<RoomMarkerResponse>('toggle_room_marker', {
        characterId: 0,
        characterName: char.character,
        gamelogPath: char.path
      });
      setRoomMarkerState(response.state);
      console.log('Room marker state:', response.state);
    } catch (e) {
      console.error('Toggle room marker failed:', e);
    }
  };

  // Define controls to pass to the WindowFrame
  const headerControls = (
    <>
      {/* Bookmark Buttons - only show if a character is tracked */}
      {characters.some(c => c.tracked) && (
        <>
          <button
            className="icon-btn"
            onClick={handleMarkHighlight}
            title="Add highlight bookmark"
          >
            📍
          </button>
          <button
            className={`icon-btn ${roomMarkerState === 'InRoom' ? 'active' : ''}`}
            onClick={handleToggleRoom}
            title={roomMarkerState === 'InRoom' ? 'End room marker' : 'Start room marker'}
            style={roomMarkerState === 'InRoom' ? { background: 'var(--accent-green)', color: '#000' } : {}}
          >
            {roomMarkerState === 'InRoom' ? '🚪✓' : '🚪'}
          </button>
        </>
      )}
      <button
        ref={charBtnRef}
        className="icon-btn"
        onClick={() => {
          setShowCharacterSelector(!showCharacterSelector);
          setShowSettings(false);
        }}
      >
        Chars
      </button>
      <button
        ref={settingsBtnRef}
        className="icon-btn"
        onClick={() => {
          setShowSettings(!showSettings);
          setShowCharacterSelector(false);
        }}
      >
        ⚙
      </button>
    </>
  );

  return (
    <WindowFrame variant="main" headerActions={headerControls}>
      <div id="app" className="main-overlay">
        {showSettings && (
          <div ref={settingsRef}>
            <SettingsModal
              settings={settings}
              onSave={handleSaveSettings}
              onCancel={() => setShowSettings(false)}
              onOpenReplay={handleOpenReplay}
            />
          </div>
        )}

        {showCharacterSelector && (
          <div ref={charSelectorRef}>
            <CharacterSelector
              characters={characters}
              onToggle={handleToggleTracking}
            />
          </div>
        )}

        <div id="data-container">
          <StatusBar combatActions={dpsData?.combat_actions_by_character ?? null} />
          <CombatBreakdown
            data={dpsData}
            characters={characters}
          />
        </div>
      </div>
    </WindowFrame>
  );
}


function App() {
  const [isReplay, setIsReplay] = useState(false);

  useEffect(() => {
    const win = getCurrentWindow();
    if (win.label.includes('replay')) {
      setIsReplay(true);
      document.title = "Replay Suite";
    }
  }, []);

  if (isReplay) return <ReplayWindow />;

  return <MainApp />;
}

export default App;
