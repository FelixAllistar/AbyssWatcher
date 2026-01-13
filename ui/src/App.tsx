import { useEffect, useState, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { info, error } from '@tauri-apps/plugin-log';
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
import type { DpsUpdate, CharacterState, SettingsWithAlerts, RoomMarkerState, RoomMarkerResponse, AlertEvent } from './types';

// Re-export types for other modules that import from App
export type { DpsUpdate, CharacterState, CombatAction, TargetHit, Settings, Bookmark, BookmarkType, RoomMarkerState } from './types';

import WindowFrame from './components/WindowFrame';
import Tooltip from './components/Tooltip';

function MainApp() {
  const [dpsData, setDpsData] = useState<DpsUpdate | null>(null);
  const [characters, setCharacters] = useState<CharacterState[]>([]);
  const [showSettings, setShowSettings] = useState(false);
  const [showCharacterSelector, setShowCharacterSelector] = useState(false);
  const [settings, setSettings] = useState<SettingsWithAlerts>({
    gamelog_dir: '',
    dps_window_seconds: 5,
    alert_settings: {
      rules: {
        EnvironmentalDamage: { enabled: true, sound: 'Default' },
        FriendlyFire: { enabled: true, sound: 'Default' },
        LogiTakingDamage: { enabled: true, sound: 'Default' },
        NeutSensitiveNeuted: { enabled: true, sound: 'Default' },
        CapacitorFailure: { enabled: true, sound: 'Default' },
        LogiNeuted: { enabled: true, sound: 'Default' },
      },
      roles: { logi_characters: [], neut_sensitive_characters: [] },
    },
  });
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
        const loadedSettings = await invoke<SettingsWithAlerts>('get_settings');
        setSettings(loadedSettings);
        const loadedChars = await invoke<CharacterState[]>('get_available_characters');
        setCharacters(loadedChars);
      } catch (e) {
        console.error('Init failed:', e);
      }
    };
    init();

    // Subscribe to DPS updates
    const unlistenDps = listen<DpsUpdate>('dps-update', (event) => {
      setDpsData(event.payload);
    });

    // Subscribe to Abyss exit events to auto-reset room marker
    const unlistenAbyssExit = listen<{ character: string, location: string }>('abyss-exited', (event) => {
      console.log(`${event.payload.character} exited Abyss to ${event.payload.location}`);
      // Reset room marker state since run ended (auto-closed by backend)
      setRoomMarkerState('Idle');
    });

    // Subscribe to alert events for sound playback
    const unlistenAlert = listen<AlertEvent>('alert-triggered', (event) => {
      const { sound, message } = event.payload;
      console.log('[ALERT]', message);

      // Play sound if specified
      // Play sound via backend (bypasses WebKitGTK limitations on Linux)
      if (sound) {
        info(`[SOUND] Requesting backend playback: ${sound}.wav`);
        invoke('play_alert_sound', { filename: `${sound}.wav` })
          .catch(err => error(`[SOUND ERROR] Backend playback failed: ${err}`));
      }
    });

    return () => {
      unlistenDps.then((fn) => fn());
      unlistenAbyssExit.then((fn) => fn());
      unlistenAlert.then((fn) => fn());
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

  const handleSaveSettings = async (newSettings: SettingsWithAlerts) => {
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

  // Get all tracked characters for bookmark operations
  const getTrackedCharacters = () => {
    return characters.filter(c => c.tracked);
  };

  const handleMarkHighlight = async () => {
    const tracked = getTrackedCharacters();
    if (tracked.length === 0) {
      console.warn('No character tracked, cannot create bookmark');
      return;
    }

    // Create bookmark for ALL tracked characters
    for (const char of tracked) {
      try {
        await invoke('create_highlight_bookmark', {
          gamelogPath: char.path,
          label: null
        });
        console.log(`Highlight bookmark created for ${char.character}`);
      } catch (e) {
        console.error(`Create bookmark failed for ${char.character}:`, e);
      }
    }
  };

  const handleToggleRoom = async () => {
    const tracked = getTrackedCharacters();
    if (tracked.length === 0) {
      console.warn('No character tracked, cannot toggle room marker');
      return;
    }

    // Current room state - will be toggled
    const currentlyInRoom = roomMarkerState === 'InRoom';

    // Toggle room marker for ALL tracked characters
    let isOpen = false;
    for (const char of tracked) {
      try {
        const response = await invoke<RoomMarkerResponse>('toggle_room_marker', {
          gamelogPath: char.path,
          currentlyInRoom: currentlyInRoom
        });
        // Use the last response state (they should all be the same)
        isOpen = response.room_open;
        console.log(`Room marker for ${char.character}:`, response.room_open);
      } catch (e) {
        console.error(`Toggle room marker failed for ${char.character}:`, e);
      }
    }
    setRoomMarkerState(isOpen ? 'InRoom' : 'Idle');
  };

  // Define controls to pass to the WindowFrame
  const headerControls = (
    <>
      {/* Bookmark Buttons - only show if a character is tracked */}
      {characters.some(c => c.tracked) && (
        <>
          <Tooltip text="Add highlight bookmark" position="bottom" align="right">
            <button
              className="icon-btn"
              onClick={handleMarkHighlight}
              aria-label="Add highlight bookmark"
            >
              📍
            </button>
          </Tooltip>
          <Tooltip
            text={roomMarkerState === 'InRoom' ? 'End room marker' : 'Start room marker'}
            position="bottom"
            align="right"
          >
            <button
              className={`icon-btn ${roomMarkerState === 'InRoom' ? 'active' : ''}`}
              onClick={handleToggleRoom}
              style={roomMarkerState === 'InRoom' ? { background: 'var(--accent-green)', color: '#000' } : {}}
              aria-label={roomMarkerState === 'InRoom' ? 'End room marker' : 'Start room marker'}
            >
              {roomMarkerState === 'InRoom' ? '🚪✓' : '🚪'}
            </button>
          </Tooltip>
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
              trackedCharacters={characters}
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
