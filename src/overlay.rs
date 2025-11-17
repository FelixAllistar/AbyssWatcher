use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

use dioxus::prelude::*;
use dioxus_core::VirtualDom;
use dioxus_desktop::{
    launch::launch_virtual_dom, use_window, use_wry_event_handler, Config, DesktopService,
    LogicalPosition, LogicalSize, WindowBuilder,
};
use serde::{Deserialize, Serialize};
use crate::core::{log_io, model, state};

#[derive(Serialize, Deserialize)]
struct WindowState {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
}

fn default_window_state() -> WindowState {
    WindowState {
        width: 400,
        height: 200,
        x: 50,
        y: 50,
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
    Config::new()
        .with_window(
            WindowBuilder::new()
                .with_title("AbyssWatcher DPS Meter")
                .with_transparent(true)
                .with_always_on_top(true)
                .with_decorations(false)
                .with_inner_size(LogicalSize::new(
                    window_state.width as f64,
                    window_state.height as f64,
                ))
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

fn save_window_state(desktop: &Rc<DesktopService>) -> Result<(), Box<dyn std::error::Error>> {
    let inner_size = desktop.window.inner_size();
    let outer_position = desktop.window.outer_position().unwrap_or_default();
    let state = WindowState {
        width: inner_size.width,
        height: inner_size.height,
        x: outer_position.x,
        y: outer_position.y,
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
}

#[derive(Clone)]
pub struct CharacterInfo {
    pub name: String,
    pub file_path: PathBuf,
    pub last_modified: SystemTime,
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
        },
        model::CombatEvent {
            timestamp: Duration::from_secs(1),
            source: "You".to_string(),
            target: "Enemy".to_string(),
            weapon: "Laser".to_string(),
            damage: 120.0,
        },
        model::CombatEvent {
            timestamp: Duration::from_secs(2),
            source: "You".to_string(),
            target: "Enemy".to_string(),
            weapon: "Missile".to_string(),
            damage: 300.0,
        },
    ];

    for event in sample_events {
        engine_state.push_event(event);
    }

    let dps_samples = engine_state.dps_series(Duration::from_secs(1));
    let total_damage = engine_state.total_damage();

    OverlayViewState {
        dps_samples,
        total_damage,
        gamelog_dir: None,
        characters: Vec::new(),
    }
}

#[component]
fn App() -> Element {
    let desktop = use_window();
    let persisted_state = load_window_state_from_disk();
    let mut overlay_state = use_signal(initial_overlay_state);

    use_context_provider(|| overlay_state);

    use_effect({
        let desktop = desktop.clone();
        move || {
            let _ = desktop.window.set_always_on_top(true);
            let _ = desktop.window.set_inner_size(LogicalSize::new(
                persisted_state.width as f64,
                persisted_state.height as f64,
            ));
            let _ = desktop.window.set_outer_position(LogicalPosition::new(
                persisted_state.x as f64,
                persisted_state.y as f64,
            ));
        }
    });

    use_wry_event_handler({
        let desktop = desktop.clone();
        move |event, _| {
            if let tao::event::Event::WindowEvent {
                event: tao::event::WindowEvent::CloseRequested,
                ..
            } = event
            {
                let _ = save_window_state(&desktop);
                desktop.close();
            }
        }
    });

    let mut is_dragging = use_signal(|| false);
    let mut initial_mouse_position = use_signal(|| (0.0, 0.0));
    let mut initial_window_position = use_signal(|| (0.0, 0.0));

    let overlay_opacity = 0.8_f32;
    let container_style = format!(
        "background: rgba(0,0,0,{}); color: white; font-family: monospace; border-radius: 8px; user-select: none;",
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
                        move |_| {
                            let _ = save_window_state(&desktop);
                            desktop.close();
                        }
                    },
                    "×"
                }
            }
            div {
                style: "padding: 15px;",
                GamelogSettings {}
                DpsSummary {}
                CharacterList {}
            }
        }
    }
}

#[component]
fn DpsSummary() -> Element {
    let overlay_state_signal = use_context::<Signal<OverlayViewState>>();
    let overlay_state_value = overlay_state_signal();

    let latest_dps = overlay_state_value
        .dps_samples
        .last()
        .map(|sample| sample.total_dps)
        .unwrap_or(0.0);

    rsx! {
        h2 { "DPS Meter" }
        p { "Current DPS: {latest_dps}" }
        p { "Total Damage: {overlay_state_value.total_damage}" }
    }
}

#[component]
fn GamelogSettings() -> Element {
    let mut overlay_state_signal = use_context::<Signal<OverlayViewState>>();
    let mut path_input = use_signal(|| {
        "/home/felix/Games/eve-online/drive_c/users/felix/My Documents/EVE/logs/Gamelogs"
            .to_string()
    });
    let state_snapshot = overlay_state_signal();
    let folder_label = state_snapshot
        .gamelog_dir
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(not set)".to_string());

    rsx! {
        div {
            style: "margin-bottom: 8px;",
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
                                })
                                .collect();
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
    let overlay_state_signal = use_context::<Signal<OverlayViewState>>();
    let overlay_state_value = overlay_state_signal();

    let items: Vec<(String, String)> = overlay_state_value
        .characters
        .iter()
        .map(|character| {
            let name = character.name.clone();
            let file_name = character
                .file_path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_string();
            (name, file_name)
        })
        .collect();

    rsx! {
        div {
            style: "margin-top: 8px; font-size: 12px;",
            h3 { "Detected Characters" }
            if overlay_state_value.characters.is_empty() {
                p { "No characters detected. Choose a gamelog folder." }
            }
            for (name, file_name) in items {
                div {
                    "{name} - {file_name}"
                }
            }
        }
    }
}
