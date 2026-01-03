import { useEffect, useState } from 'react';
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

// Types matching the Rust backend
export interface DpsUpdate {
  outgoing_dps: number;
  incoming_dps: number;
  outgoing_hps: number;
  incoming_hps: number;
  outgoing_cap: number;
  incoming_cap: number;
  outgoing_neut: number;
  incoming_neut: number;
  combat_actions_by_character: Record<string, CombatAction[]>;
}

export interface CombatAction {
  name: string;
  action_type: 'Damage' | 'Repair' | 'Capacitor' | 'Neut';
  incoming: boolean;
  value: number;
}

export interface CharacterState {
  character: string;
  path: string;
  tracked: boolean;
}

interface Settings {
  gamelog_dir: string;
  dps_window_seconds: number;
}

import WindowFrame from './components/WindowFrame';

function MainApp() {
  const [dpsData, setDpsData] = useState<DpsUpdate | null>(null);
  const [characters, setCharacters] = useState<CharacterState[]>([]);
  const [showSettings, setShowSettings] = useState(false);
  const [showCharacterSelector, setShowCharacterSelector] = useState(false);
  const [settings, setSettings] = useState<Settings>({ gamelog_dir: '', dps_window_seconds: 5 });

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

  // Define controls to pass to the WindowFrame
  const headerControls = (
    <>
      <button
        className="icon-btn"
        onClick={() => {
          setShowCharacterSelector(!showCharacterSelector);
          setShowSettings(false);
        }}
      >
        Chars
      </button>
      <button
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
          <SettingsModal
            settings={settings}
            onSave={handleSaveSettings}
            onCancel={() => setShowSettings(false)}
            onOpenReplay={handleOpenReplay}
          />
        )}

        {showCharacterSelector && (
          <CharacterSelector
            characters={characters}
            onToggle={handleToggleTracking}
          />
        )}

        <div id="data-container">
          <StatusBar data={dpsData} />
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
