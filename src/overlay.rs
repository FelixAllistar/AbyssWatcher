use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::core::{log_io, model, state, tracker};
use dioxus::prelude::*;
use dioxus_core::VirtualDom;
use dioxus_desktop::{
    launch::launch_virtual_dom, use_window, use_wry_event_handler, Config, DesktopService,
    LogicalPosition, LogicalSize, WindowBuilder,
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

const DEFAULT_GAMELOG_PATH: &str =
    "/home/felix/Games/eve-online/drive_c/users/felix/My Documents/EVE/logs/Gamelogs";

#[derive(Serialize, Deserialize, Clone)]
struct WindowState {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    #[serde(default)]
    tracked_files: Vec<String>,
}

fn default_window_state() -> WindowState {
    WindowState {
        width: 400,
        height: 200,
        x: 50,
        y: 50,
        tracked_files: Vec::new(),
    }
}

fn load_window_state_from_disk() -> WindowState {
    if let Ok(file_content) = std::fs::read_to_string("app_state.json") {
        if let Ok(state) = serde_json::from_str::<WindowState>(&file_content) {
            return state;
        }
    }

    default_window_state()
}

fn build_window_config(window_state: &WindowState) -> Config {
    let width = window_state.width.max(360);
    let height = window_state.height.max(220);

    Config::new()
        .with_window(
            WindowBuilder::new()
                .with_title("AbyssWatcher DPS Meter")
                .with_transparent(true)
                .with_always_on_top(true)
                .with_decorations(false)
                .with_inner_size(LogicalSize::new(width as f64, height as f64))
                .with_position(LogicalPosition::new(
                    window_state.x as f64,
                    window_state.y as f64,
                )),
        )
        .with_background_color((0, 0, 0, 0))
}

pub fn run_overlay() {
    let window_state = load_window_state_from_disk();
    let config = build_window_config(&window_state);
    launch_virtual_dom(VirtualDom::new(App), config);
}

fn save_window_state(
    desktop: &Rc<DesktopService>,
    overlay_state: &Signal<OverlayViewState, SyncStorage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let inner_size = desktop.window.inner_size();
    let outer_position = desktop.window.outer_position().unwrap_or_default();
    let overlay_snapshot = overlay_state.read();
    let tracked_files: Vec<String> = overlay_snapshot
        .characters
        .iter()
        .filter(|character| character.tracked)
        .map(|character| character.file_path.display().to_string())
        .collect();
    let state = WindowState {
        width: inner_size.width,
        height: inner_size.height,
        x: outer_position.x,
        y: outer_position.y,
        tracked_files,
    };
    let json = serde_json::to_string(&state)?;
    std::fs::write("app_state.json", json)?;
    Ok(())
}

#[derive(Clone)]
pub struct OverlayViewState {
    pub dps_samples: Vec<model::DpsSample>,
    pub total_damage: f32,
    pub gamelog_dir: Option<PathBuf>,
    pub characters: Vec<CharacterInfo>,
    pub dps_window_secs: u64,
}

#[derive(Clone)]
pub struct CharacterInfo {
    pub name: String,
    pub file_path: PathBuf,
    pub last_modified: SystemTime,
    pub tracked: bool,
}

struct WorkerControl {
    stop_tx: mpsc::Sender<()>,
    handle: Mutex<Option<thread::JoinHandle<()>>>,
}

lazy_static! {
    static ref WORKER_CONTROL: Mutex<Option<WorkerControl>> = Mutex::new(None);
}

fn start_worker_if_needed(mut overlay_state: Signal<OverlayViewState, SyncStorage>) {
    let mut guard = WORKER_CONTROL.lock().unwrap();
    if guard.is_some() {
        return;
    }

    let (stop_tx, stop_rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut engine = state::EngineState::new();
        let mut trackers: HashMap<PathBuf, tracker::TrackedGamelog> = HashMap::new();
        let mut events_by_path: HashMap<PathBuf, Vec<model::CombatEvent>> = HashMap::new();
        let mut last_tracked_paths: HashSet<PathBuf> = HashSet::new();
        let mut last_event_timestamp: Option<Duration> = None;
        let mut last_event_wallclock: Option<SystemTime> = None;

        loop {
            if stop_rx.try_recv().is_ok() {
                break;
            }

            let overlay_snapshot = overlay_state.read();
            let window_secs = overlay_snapshot.dps_window_secs.max(1);
            let window = Duration::from_secs(window_secs);
            let tracked_characters: Vec<_> = overlay_snapshot
                .characters
                .iter()
                .filter(|character| character.tracked)
                .map(|character| (character.file_path.clone(), character.name.clone()))
                .collect();
            drop(overlay_snapshot);

            let tracked_paths: HashSet<_> =
                tracked_characters.iter().map(|(path, _)| path.clone()).collect();

            trackers.retain(|path, _| tracked_paths.contains(path));
            events_by_path.retain(|path, _| tracked_paths.contains(path));

            for (file_path, name) in tracked_characters {
                if !trackers.contains_key(&file_path) {
                    if let Ok(tracker_entry) =
                        tracker::TrackedGamelog::new(name, file_path.clone())
                    {
                        trackers.insert(file_path.clone(), tracker_entry);
                    }
                }
                events_by_path.entry(file_path.clone()).or_default();
            }

            if tracked_paths != last_tracked_paths {
                engine = state::EngineState::new();
                last_event_timestamp = None;
                for (path, events) in &events_by_path {
                    if tracked_paths.contains(path) {
                        for event in events {
                            last_event_timestamp = Some(match last_event_timestamp {
                                Some(prev) => std::cmp::max(prev, event.timestamp),
                                None => event.timestamp,
                            });
                            engine.push_event(event.clone());
                        }
                    }
                }
                if last_event_timestamp.is_some() {
                    last_event_wallclock = Some(SystemTime::now());
                } else {
                    last_event_wallclock = None;
                }
                last_tracked_paths = tracked_paths.clone();
            }

            for (path, tracker_entry) in trackers.iter_mut() {
                if let Ok(new_events) = tracker_entry.read_new_events() {
                    let entry_events = events_by_path.entry(path.clone()).or_default();
                    if !new_events.is_empty() {
                        let now = SystemTime::now();
                        for event in new_events {
                            entry_events.push(event.clone());
                            if last_tracked_paths.contains(path) {
                                last_event_timestamp = Some(match last_event_timestamp {
                                    Some(prev) => std::cmp::max(prev, event.timestamp),
                                    None => event.timestamp,
                                });
                                engine.push_event(event);
                            }
                        }
                        last_event_wallclock = Some(now);
                    }
                }
            }

            let end_time = match (last_event_timestamp, last_event_wallclock) {
                (Some(timestamp), Some(seen_at)) => {
                    if let Ok(elapsed) = SystemTime::now().duration_since(seen_at) {
                        timestamp + elapsed
                    } else {
                        timestamp
                    }
                }
                (Some(timestamp), None) => timestamp,
                (None, _) => Duration::from_secs(0),
            };

            let dps_samples = engine.dps_series(window, end_time);
            let total_damage = engine.total_damage();
            overlay_state.with_mut(move |state| {
                state.dps_samples = dps_samples;
                state.total_damage = total_damage;
            });

            thread::sleep(Duration::from_millis(250));
        }
    });

    *guard = Some(WorkerControl {
        stop_tx,
        handle: Mutex::new(Some(handle)),
    });
}

fn shutdown_worker() {
    let mut guard = WORKER_CONTROL.lock().unwrap();
    if let Some(control) = guard.as_ref() {
        let _ = control.stop_tx.send(());
        if let Some(handle) = control.handle.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
    *guard = None;
}

fn initial_overlay_state() -> OverlayViewState {
    let mut engine_state = state::EngineState::new();

    let sample_events = vec![
        model::CombatEvent {
            timestamp: Duration::from_secs(0),
            source: "You".to_string(),
            target: "Enemy".to_string(),
            weapon: "Laser".to_string(),
            damage: 100.0,
            incoming: false,
        },
        model::CombatEvent {
            timestamp: Duration::from_secs(1),
            source: "You".to_string(),
            target: "Enemy".to_string(),
            weapon: "Laser".to_string(),
            damage: 120.0,
            incoming: false,
        },
        model::CombatEvent {
            timestamp: Duration::from_secs(2),
            source: "You".to_string(),
            target: "Enemy".to_string(),
            weapon: "Missile".to_string(),
            damage: 300.0,
            incoming: false,
        },
    ];

    for event in sample_events {
        engine_state.push_event(event);
    }

    let end = engine_state
        .events()
        .iter()
        .map(|event| event.timestamp)
        .max()
        .unwrap_or(Duration::from_secs(0));
    let dps_samples = engine_state.dps_series(Duration::from_secs(5), end);
    let total_damage = engine_state.total_damage();

    OverlayViewState {
        dps_samples,
        total_damage,
        gamelog_dir: None,
        characters: Vec::new(),
        dps_window_secs: 5,
    }
}

#[component]
fn App() -> Element {
    let desktop = use_window();
    let persisted_state = load_window_state_from_disk();
    let overlay_state: Signal<OverlayViewState, SyncStorage> =
        use_signal_sync(initial_overlay_state);

    use_context_provider(|| overlay_state);

    // Try to auto-scan the default gamelog folder on startup.
    use_effect({
        let mut overlay_state = overlay_state.clone();
        let persisted_state = persisted_state.clone();
        move || {
            let default_path = PathBuf::from(DEFAULT_GAMELOG_PATH);
            if let Ok(logs) = log_io::scan_gamelogs_dir(&default_path) {
                if !logs.is_empty() {
                    let tracked_set: HashSet<String> =
                        persisted_state.tracked_files.iter().cloned().collect();
                    overlay_state.with_mut(|state| {
                        state.gamelog_dir = Some(default_path.clone());
                        state.characters = logs
                            .into_iter()
                            .map(|log| CharacterInfo {
                                name: log.character.clone(),
                                file_path: log.path.clone(),
                                last_modified: log.last_modified,
                                tracked: tracked_set
                                    .contains(&log.path.display().to_string()),
                            })
                            .collect();
                        state
                            .characters
                            .sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
                    });
                }
            }
        }
    });

    use_effect({
        let overlay_state = overlay_state.clone();
        move || {
            start_worker_if_needed(overlay_state);
        }
    });

    use_effect({
        let desktop = desktop.clone();
        let persisted_state = persisted_state;
        move || {
            let width = persisted_state.width.max(360);
            let height = persisted_state.height.max(220);
            let _ = desktop.window.set_always_on_top(true);
            let _ = desktop
                .window
                .set_inner_size(LogicalSize::new(width as f64, height as f64));
            let _ = desktop.window.set_outer_position(LogicalPosition::new(
                persisted_state.x as f64,
                persisted_state.y as f64,
            ));
        }
    });

    use_wry_event_handler({
        let desktop = desktop.clone();
        let overlay_state = overlay_state.clone();
        move |event, _| {
            if let tao::event::Event::WindowEvent {
                event: tao::event::WindowEvent::CloseRequested,
                ..
            } = event
            {
                let _ = save_window_state(&desktop, &overlay_state);
                shutdown_worker();
                desktop.close();
            }
        }
    });

    let mut is_dragging = use_signal(|| false);
    let mut initial_mouse_position = use_signal(|| (0.0, 0.0));
    let mut initial_window_position = use_signal(|| (0.0, 0.0));

    let overlay_opacity = 0.8_f32;
    let container_style = format!(
        "background: rgba(0,0,0,{}); color: white; font-family: monospace; border-radius: 8px; user-select: none; min-width: 360px; min-height: 220px; display: flex; flex-direction: column; overflow: hidden; box-shadow: 0 0 10px rgba(0,0,0,0.5);",
        overlay_opacity
    );

    rsx! {
        style { "html, body {{ background: transparent !important; }}" }
        div {
            style: "{container_style}",
            div {
                style: "height: 30px; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: space-between; padding: 0 10px; cursor: move; border-radius: 8px 8px 0 0;",
                onmousedown: {
                    let desktop = desktop.clone();
                    move |event| {
                        *is_dragging.write() = true;
                        *initial_mouse_position.write() = (
                            event.data.client_coordinates().x,
                            event.data.client_coordinates().y,
                        );
                        let outer_position = desktop.window.outer_position().unwrap_or_default();
                        *initial_window_position.write() =
                            (outer_position.x as f64, outer_position.y as f64);
                    }
                },
                onmousemove: {
                    let desktop = desktop.clone();
                    move |event| {
                        if *is_dragging.read() {
                            let current_mouse = (
                                event.data.client_coordinates().x,
                                event.data.client_coordinates().y,
                            );
                            let delta_x = current_mouse.0 - initial_mouse_position.read().0;
                            let delta_y = current_mouse.1 - initial_mouse_position.read().1;
                            let new_x = initial_window_position.read().0 + delta_x;
                            let new_y = initial_window_position.read().1 + delta_y;
                            let _ = desktop.window.set_outer_position(LogicalPosition::new(new_x, new_y));
                        }
                    }
                },
                onmouseup: move |_| {
                    *is_dragging.write() = false;
                },
                "AbyssWatcher DPS Meter"
                button {
                    style: "background: none; border: none; color: white; cursor: pointer; font-size: 16px;",
                    onclick: {
                        let desktop = desktop.clone();
                        let overlay_state = overlay_state.clone();
                        move |_| {
                            let _ = save_window_state(&desktop, &overlay_state);
                            shutdown_worker();
                            desktop.close();
                        }
                    },
                    "×"
                }
            }
            div {
                style: "padding: 10px; display: flex; flex-direction: column; gap: 8px; font-size: 12px;",
                GamelogSettings {}
                DpsSummary {}
                CharacterList {}
            }
        }
    }
}

#[component]
fn DpsSummary() -> Element {
    let mut overlay_state_signal = use_context::<Signal<OverlayViewState, SyncStorage>>();
    let overlay_state_value = overlay_state_signal();

    let (outgoing_dps, incoming_dps) = overlay_state_value
        .dps_samples
        .last()
        .map(|sample| (sample.outgoing_dps, sample.incoming_dps))
        .unwrap_or((0.0, 0.0));

    let latest_sample = overlay_state_value.dps_samples.last();

    let mut top_outgoing_targets: Vec<(String, f32)> = Vec::new();
    let mut top_incoming_sources: Vec<(String, f32)> = Vec::new();

    if let Some(sample) = latest_sample {
        top_outgoing_targets = sample
            .outgoing_by_target
            .iter()
            .map(|(name, dps)| (name.clone(), *dps))
            .collect();
        top_outgoing_targets.sort_by(|a, b| b.1.total_cmp(&a.1));
        top_outgoing_targets.truncate(3);

        top_incoming_sources = sample
            .incoming_by_source
            .iter()
            .map(|(name, dps)| (name.clone(), *dps))
            .collect();
        top_incoming_sources.sort_by(|a, b| b.1.total_cmp(&a.1));
        top_incoming_sources.truncate(3);
    }

    let history = &overlay_state_value.dps_samples;
    let max_points = 60usize;
    let history_len = history.len();
    let slice_start = history_len.saturating_sub(max_points);
    let slice = &history[slice_start..history_len];

    let mut max_dps_value = 0.0_f32;
    for sample in slice {
        max_dps_value = max_dps_value
            .max(sample.outgoing_dps)
            .max(sample.incoming_dps);
    }
    if max_dps_value <= 0.0 {
        max_dps_value = 1.0;
    }

    let mut graph_points: Vec<(f32, f32)> = Vec::new();
    for sample in slice {
        let out_height = ((sample.outgoing_dps / max_dps_value) * 36.0).max(1.0);
        let in_height = ((sample.incoming_dps / max_dps_value) * 36.0).max(1.0);
        graph_points.push((out_height, in_height));
    }

    let window_secs = overlay_state_value.dps_window_secs;

    rsx! {
        div {
            style: "display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px;",
            span { "DPS" }
            div {
                style: "display: flex; align-items: center; gap: 4px; font-size: 11px;",
                span { "Window (s):" }
                input {
                    r#type: "number",
                    min: "1",
                    max: "60",
                    style: "width: 50px; font-size: 11px; padding: 1px 3px; background: #111; color: white; border: 1px solid #555; border-radius: 3px;",
                    value: "{window_secs}",
                    oninput: move |event| {
                        if let Ok(parsed) = event.value().parse::<u64>() {
                            let value = parsed.max(1).min(60);
                            overlay_state_signal.with_mut(|state| {
                                state.dps_window_secs = value;
                            });
                        }
                    }
                }
            }
        }
        div {
            style: "display: flex; flex-direction: column; gap: 2px;",
            span { "Out: {outgoing_dps:.1} | In: {incoming_dps:.1}" }
            span { "Total: {overlay_state_value.total_damage as i32}" }
        }
        if !graph_points.is_empty() {
            div {
                style: "margin-top: 4px; font-size: 11px;",
                span { "History" }
                div {
                    style: "height: 32px; display: flex; align-items: flex-end; gap: 1px;",
                    for (out_height_px, in_height_px) in &graph_points {
                        div {
                            style: "width: 3px; display: flex; flex-direction: column-reverse; align-items: stretch;",
                            div {
                                style: "height: {out_height_px}px; background: rgba(0, 191, 255, 0.9);"
                            }
                            div {
                                style: "height: {in_height_px}px; background: rgba(255, 64, 64, 0.8);"
                            }
                        }
                    }
                }
            }
        }
        if !top_outgoing_targets.is_empty() {
            div {
                style: "margin-top: 4px;",
                span { "Top targets:" }
            }
            for (name, dps) in &top_outgoing_targets {
                span { "- {name}: {dps:.1}" }
            }
        }
        if !top_incoming_sources.is_empty() {
            div {
                style: "margin-top: 2px;",
                span { "Top incoming:" }
            }
            for (name, dps) in &top_incoming_sources {
                span { "- {name}: {dps:.1}" }
            }
        }
    }
}

#[component]
fn GamelogSettings() -> Element {
    let mut overlay_state_signal = use_context::<Signal<OverlayViewState, SyncStorage>>();
    let mut path_input = use_signal(|| DEFAULT_GAMELOG_PATH.to_string());
    let state_snapshot = overlay_state_signal();

    if !state_snapshot.characters.is_empty() {
        return rsx! {  };
    }

    let folder_label = state_snapshot
        .gamelog_dir
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(not set)".to_string());
    rsx! {
        div {
            style: "margin-bottom: 6px;",
            p {
                "Gamelog folder: {folder_label}"
            }
            div {
                style: "margin-top: 4px;",
                input {
                    style: "width: 100%; font-size: 12px; padding: 2px 4px; margin-bottom: 4px; background: #111; color: white; border: 1px solid #555; border-radius: 4px;",
                    value: "{path_input()}",
                    oninput: move |event| {
                        *path_input.write() = event.value();
                    }
                }
            }
            button {
                style: "background: #333; color: white; border: 1px solid #555; border-radius: 4px; padding: 4px 8px; cursor: pointer; font-size: 12px;",
                onclick: move |_| {
                    let path_string = path_input();
                    let path = PathBuf::from(path_string);
                    if let Ok(logs) = log_io::scan_gamelogs_dir(&path) {
                        overlay_state_signal.with_mut(|state| {
                            state.gamelog_dir = Some(path.clone());
                            state.characters = logs
                                .into_iter()
                                .map(|log| CharacterInfo {
                                    name: log.character,
                                    file_path: log.path,
                                    last_modified: log.last_modified,
                                    tracked: false,
                                })
                                .collect();
                            state.characters.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
                        });
                    }
                },
                "Scan Gamelog Folder"
            }
        }
    }
}

#[component]
fn CharacterList() -> Element {
    let mut expanded = use_signal(|| false);
    let overlay_state_signal = use_context::<Signal<OverlayViewState, SyncStorage>>();
    let overlay_state_value = overlay_state_signal();
    let characters_snapshot: Vec<(usize, String, String, String, String)> = overlay_state_value
        .characters
        .iter()
        .enumerate()
        .map(|(idx, character)| {
            let file_name = character
                .file_path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_string();
            let tracked_text = if character.tracked {
                "Untrack"
            } else {
                "Track"
            }
            .to_string();
            let button_color = if character.tracked {
                "background: #1b5e20; color: white;"
            } else {
                "background: #333; color: white;"
            }
            .to_string();
            (
                idx,
                character.name.clone(),
                file_name,
                tracked_text,
                button_color,
            )
        })
        .collect();

    rsx! {
        div {
            style: "margin-top: 4px; font-size: 12px; display: flex; flex-direction: column; gap: 4px;",
            div {
                style: "display: flex; align-items: center; justify-content: space-between; cursor: pointer; padding: 2px 4px;",
                onclick: move |_| {
                    expanded.with_mut(|value| *value = !*value);
                },
                span { "Characters" }
                span {
                    if expanded() {
                        "▾"
                    } else {
                        "▸"
                    }
                }
            }
            if expanded() {
                if overlay_state_value.characters.is_empty() {
                    p { "No characters detected. Choose a gamelog folder." }
                }
                div {
                    style: "max-height: 140px; overflow-y: auto; display: flex; flex-direction: column; gap: 4px; background: rgba(0,0,0,0.75); border-radius: 4px; padding: 2px;",
                    for (idx, name, file_name, tracked_text, button_color) in characters_snapshot {
                        div {
                            style: "display: flex; align-items: center; justify-content: space-between; padding: 3px 6px; background: rgba(255,255,255,0.04); border-radius: 3px;",
                            span {
                                "{name} - {file_name}"
                            }
                            button {
                                style: "{button_color} border: 1px solid #555; border-radius: 4px; padding: 2px 6px; font-size: 12px; cursor: pointer;",
                                onclick: move |_| {
                                    overlay_state_signal.clone().with_mut(|state| {
                                        if let Some(entry) = state.characters.get_mut(idx) {
                                            entry.tracked = !entry.tracked;
                                        }
                                    });
                                },
                                "{tracked_text}"
                            }
                        }
                    }
                }
            }
        }
    }
}
