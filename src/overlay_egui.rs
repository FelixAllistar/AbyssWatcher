use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use crate::core::{log_io, model, state, tracker};
use eframe::{egui, NativeOptions};
use egui_plot::{Line, Plot, PlotPoints};
use serde::{Deserialize, Serialize};

const DEFAULT_GAMELOG_PATH: &str =
    "/home/felix/Games/eve-online/drive_c/users/felix/My Documents/EVE/logs/Gamelogs";

#[derive(Serialize, Deserialize, Clone)]
struct PersistedState {
    width: f32,
    height: f32,
    x: f32,
    y: f32,
    has_position: bool,
    opacity: f32,
    dps_window_secs: u64,
    gamelog_dir: Option<String>,
    tracked_files: Vec<String>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            width: 420.0,
            height: 260.0,
            x: 50.0,
            y: 50.0,
            has_position: false,
            opacity: 0.8,
            dps_window_secs: 5,
            gamelog_dir: None,
            tracked_files: Vec::new(),
        }
    }
}

fn load_persisted_state() -> PersistedState {
    if let Ok(text) = fs::read_to_string("app_state.json") {
        if let Ok(state) = serde_json::from_str::<PersistedState>(&text) {
            return state;
        }
    }
    PersistedState::default()
}

fn save_persisted_state(app: &AbyssWatcherApp, viewport_rect: Option<egui::Rect>) {
    let mut state = PersistedState::default();

    if let Some(rect) = viewport_rect {
        state.width = rect.width().max(260.0);
        state.height = rect.height().max(180.0);
        state.x = rect.left();
        state.y = rect.top();
        state.has_position = true;
    }

    state.opacity = app.opacity;
    state.dps_window_secs = app.dps_window_secs;
    state.gamelog_dir = app.gamelog_dir.as_ref().map(|p| p.display().to_string());
    state.tracked_files = app
        .characters
        .iter()
        .filter(|c| c.tracked)
        .map(|c| c.file_path.display().to_string())
        .collect();

    if let Ok(json) = serde_json::to_string_pretty(&state) {
        let _ = fs::write("app_state.json", json);
    }
}

pub fn run_overlay() {
    let persisted = load_persisted_state();
    let mut viewport_builder = egui::ViewportBuilder::default()
        .with_decorations(true)
        .with_always_on_top()
        .with_inner_size(egui::vec2(
            persisted.width.max(260.0),
            persisted.height.max(180.0),
        ))
        .with_transparent(true);

    if persisted.has_position {
        viewport_builder = viewport_builder.with_position(egui::pos2(persisted.x, persisted.y));
    }

    let options = NativeOptions {
        viewport: viewport_builder,
        ..NativeOptions::default()
    };

    let _ = eframe::run_native(
        "AbyssWatcher DPS Meter",
        options,
        Box::new(move |_cc| Box::new(AbyssWatcherApp::new(persisted.clone()))),
    );
}

struct CharacterEntry {
    name: String,
    file_path: PathBuf,
    last_modified: SystemTime,
    tracked: bool,
}

struct AbyssWatcherApp {
    gamelog_dir: Option<PathBuf>,
    gamelog_input: String,
    characters: Vec<CharacterEntry>,

    engine: state::EngineState,
    trackers: HashMap<PathBuf, tracker::TrackedGamelog>,
    events_by_path: HashMap<PathBuf, Vec<model::CombatEvent>>,
    last_tracked_paths: HashSet<PathBuf>,
    last_event_timestamp: Option<Duration>,
    last_event_wallclock: Option<SystemTime>,

    dps_window_secs: u64,
    dps_samples: Vec<model::DpsSample>,
    total_damage: f32,
    display_max_dps: f32,

    last_update: Instant,
    opacity: f32,
}

impl AbyssWatcherApp {
    fn new(persisted: PersistedState) -> Self {
        let mut app = Self {
            gamelog_dir: persisted.gamelog_dir.clone().map(PathBuf::from),
            gamelog_input: persisted
                .gamelog_dir
                .clone()
                .unwrap_or_else(|| DEFAULT_GAMELOG_PATH.to_string()),
            characters: Vec::new(),
            engine: state::EngineState::new(),
            trackers: HashMap::new(),
            events_by_path: HashMap::new(),
            last_tracked_paths: HashSet::new(),
            last_event_timestamp: None,
            last_event_wallclock: None,
            dps_window_secs: persisted.dps_window_secs.max(1),
            dps_samples: Vec::new(),
            total_damage: 0.0,
            display_max_dps: 0.0,
            last_update: Instant::now(),
            opacity: persisted.opacity,
        };

        app.try_initial_scan(&persisted);

        app
    }

    fn try_initial_scan(&mut self, persisted: &PersistedState) {
        let path = if let Some(dir) = &persisted.gamelog_dir {
            PathBuf::from(dir)
        } else {
            PathBuf::from(DEFAULT_GAMELOG_PATH)
        };

        if let Ok(logs) = log_io::scan_gamelogs_dir(&path) {
            if !logs.is_empty() {
                self.gamelog_dir = Some(path.clone());

                let tracked_set: HashSet<String> =
                    persisted.tracked_files.iter().cloned().collect();

                self.characters = logs
                    .into_iter()
                    .map(|log| CharacterEntry {
                        name: log.character.clone(),
                        file_path: log.path.clone(),
                        last_modified: log.last_modified,
                        tracked: tracked_set.contains(&log.path.display().to_string()),
                    })
                    .collect();
                self.characters
                    .sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
            }
        }
    }

    fn poll_engine(&mut self) {
        let now_instant = Instant::now();
        if now_instant.duration_since(self.last_update) < Duration::from_millis(250) {
            return;
        }
        self.last_update = now_instant;

        let window = Duration::from_secs(self.dps_window_secs.max(1));

        let tracked_paths: HashSet<PathBuf> = self
            .characters
            .iter()
            .filter(|c| c.tracked)
            .map(|c| c.file_path.clone())
            .collect();

        // Drop trackers and events for untracked paths
        self.trackers.retain(|path, _| tracked_paths.contains(path));
        self.events_by_path
            .retain(|path, _| tracked_paths.contains(path));

        // Ensure trackers exist for all tracked paths
        for entry in self.characters.iter() {
            if !entry.tracked {
                continue;
            }
            if !self.trackers.contains_key(&entry.file_path) {
                if let Ok(tr) =
                    tracker::TrackedGamelog::new(entry.name.clone(), entry.file_path.clone())
                {
                    self.trackers.insert(entry.file_path.clone(), tr);
                }
            }
            self.events_by_path
                .entry(entry.file_path.clone())
                .or_default();
        }

        // If tracked set changed, rebuild engine from cached events
        if tracked_paths != self.last_tracked_paths {
            self.engine = state::EngineState::new();
            self.last_event_timestamp = None;
            for (path, events) in &self.events_by_path {
                if tracked_paths.contains(path) {
                    for event in events {
                        self.last_event_timestamp = Some(match self.last_event_timestamp {
                            Some(prev) => std::cmp::max(prev, event.timestamp),
                            None => event.timestamp,
                        });
                        self.engine.push_event(event.clone());
                    }
                }
            }
            if self.last_event_timestamp.is_some() {
                self.last_event_wallclock = Some(SystemTime::now());
            } else {
                self.last_event_wallclock = None;
            }
            self.last_tracked_paths = tracked_paths.clone();
        }

        // Read new events from trackers
        for (path, tracker) in self.trackers.iter_mut() {
            if let Ok(new_events) = tracker.read_new_events() {
                if new_events.is_empty() {
                    continue;
                }
                let now_wallclock = SystemTime::now();
                let entry_events = self.events_by_path.entry(path.clone()).or_default();
                for event in new_events {
                    entry_events.push(event.clone());
                    if self.last_tracked_paths.contains(path) {
                        self.last_event_timestamp = Some(match self.last_event_timestamp {
                            Some(prev) => std::cmp::max(prev, event.timestamp),
                            None => event.timestamp,
                        });
                        self.engine.push_event(event);
                    }
                }
                self.last_event_wallclock = Some(now_wallclock);
            }
        }

        let end_time = match (self.last_event_timestamp, self.last_event_wallclock) {
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

        self.dps_samples = self.engine.dps_series(window, end_time);
        self.total_damage = self.engine.total_damage();
    }

    fn draw_dps(&mut self, ui: &mut egui::Ui) {
        self.poll_engine();

        let (out_dps, in_dps) = self
            .dps_samples
            .last()
            .map(|s| (s.outgoing_dps, s.incoming_dps))
            .unwrap_or((0.0, 0.0));

        ui.horizontal(|ui| {
            ui.label("DPS");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut value = self.dps_window_secs as f64;
                ui.add(
                    egui::DragValue::new(&mut value)
                        .clamp_range(1.0..=60.0)
                        .speed(0.2)
                        .fixed_decimals(0),
                );
                ui.label("Window (s):");
                self.dps_window_secs = value.round().clamp(1.0, 60.0) as u64;
            });
        });

        ui.label(format!("Out: {:.1} | In: {:.1}", out_dps, in_dps));
        ui.label(format!("Total: {:.0}", self.total_damage));

        // DPS history chart using egui::plot
        if !self.dps_samples.is_empty() {
            let max_points = 120usize;
            let len = self.dps_samples.len();
            let start = len.saturating_sub(max_points);
            let slice = &self.dps_samples[start..];

            let mut max_val = 0.0_f32;
            let mut out_points = Vec::with_capacity(slice.len());
            let mut in_points = Vec::with_capacity(slice.len());

            for sample in slice {
                let t = sample.time.as_secs_f64();
                out_points.push([t, sample.outgoing_dps as f64]);
                in_points.push([t, sample.incoming_dps as f64]);
                max_val = max_val.max(sample.outgoing_dps).max(sample.incoming_dps);
            }

            let target_max = max_val.max(10.0);
            if self.display_max_dps <= 0.0 {
                self.display_max_dps = target_max;
            } else {
                // Smoothly approach the new max to avoid jumpy zooming.
                let lerp_up = 0.2;
                let lerp_down = 0.05;
                if target_max > self.display_max_dps {
                    self.display_max_dps += (target_max - self.display_max_dps) * lerp_up;
                } else {
                    self.display_max_dps += (target_max - self.display_max_dps) * lerp_down;
                }
            }

            let out_line = Line::new(PlotPoints::from(out_points))
                .name("Outgoing DPS")
                .color(egui::Color32::from_rgb(0, 191, 255));
            let in_line = Line::new(PlotPoints::from(in_points))
                .name("Incoming DPS")
                .color(egui::Color32::from_rgb(255, 64, 64));

            ui.add_space(4.0);
            Plot::new("dps_history")
                .height(140.0)
                .allow_boxed_zoom(false)
                .allow_scroll(false)
                .include_y(0.0)
                .include_y(self.display_max_dps as f64)
                .show(ui, |plot_ui| {
                    plot_ui.line(out_line);
                    plot_ui.line(in_line);
                });
        }

        // Detailed targets / incoming lists based on latest sample
        if let Some(sample) = self.dps_samples.last() {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("Top targets");
                    if sample.outgoing_by_target.is_empty() {
                        ui.label("None");
                    } else {
                        let mut entries: Vec<_> = sample
                            .outgoing_by_target
                            .iter()
                            .map(|(name, dps)| (name.as_str(), *dps))
                            .collect();
                        entries.sort_by(|a, b| b.1.total_cmp(&a.1));

                        egui::ScrollArea::vertical()
                            .max_height(120.0)
                            .show(ui, |ui| {
                                for (name, dps) in entries {
                                    ui.label(format!("{name}: {dps:.1}"));
                                }
                            });
                    }
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.label("Top incoming");
                    if sample.incoming_by_source.is_empty() {
                        ui.label("None");
                    } else {
                        let mut entries: Vec<_> = sample
                            .incoming_by_source
                            .iter()
                            .map(|(name, dps)| (name.as_str(), *dps))
                            .collect();
                        entries.sort_by(|a, b| b.1.total_cmp(&a.1));

                        egui::ScrollArea::vertical()
                            .max_height(120.0)
                            .show(ui, |ui| {
                                for (name, dps) in entries {
                                    ui.label(format!("{name}: {dps:.1}"));
                                }
                            });
                    }
                });
            });
        }
    }

    fn draw_gamelog_settings(&mut self, ui: &mut egui::Ui) {
        if !self.characters.is_empty() {
            return;
        }

        ui.separator();
        ui.label("Gamelog folder:");
        ui.text_edit_singleline(&mut self.gamelog_input);
        if ui.button("Scan Gamelog Folder").clicked() {
            let path = PathBuf::from(self.gamelog_input.clone());
            if let Ok(logs) = log_io::scan_gamelogs_dir(&path) {
                self.gamelog_dir = Some(path.clone());
                self.characters = logs
                    .into_iter()
                    .map(|log| CharacterEntry {
                        name: log.character,
                        file_path: log.path,
                        last_modified: log.last_modified,
                        tracked: false,
                    })
                    .collect();
                self.characters
                    .sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
            }
        }
    }
}

impl eframe::App for AbyssWatcherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (outer_rect, inner_rect, close_requested) = ctx.input(|i| {
            let vp = i.viewport();
            (vp.outer_rect, vp.inner_rect, vp.close_requested())
        });

        // Custom menu bar (window title is handled by native decorations)
        egui::TopBottomPanel::top("menu_bar")
            .frame(
                egui::Frame::none().fill(egui::Color32::from_rgba_unmultiplied(
                    0,
                    0,
                    0,
                    (self.opacity * 255.0) as u8,
                )),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("View", |ui| {
                        ui.label("Opacity");
                        ui.add(
                            egui::Slider::new(&mut self.opacity, 0.2..=1.0).clamp_to_range(true),
                        );
                    });

                    ui.menu_button("Characters", |ui| {
                        if self.characters.is_empty() {
                            ui.label("No characters detected");
                        } else {
                            for entry in &mut self.characters {
                                let label = format!(
                                    "{} ({})",
                                    entry.name,
                                    entry
                                        .file_path
                                        .file_name()
                                        .and_then(|v| v.to_str())
                                        .unwrap_or_default()
                                );
                                let mut tracked = entry.tracked;
                                if ui.checkbox(&mut tracked, label).changed() {
                                    entry.tracked = tracked;
                                    self.last_update = Instant::now() - Duration::from_millis(250);
                                }
                            }
                        }
                    });
                });
            });

        // Main content panel with semi-transparent background
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none().fill(egui::Color32::from_rgba_unmultiplied(
                    0,
                    0,
                    0,
                    (self.opacity * 255.0) as u8,
                )),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    self.draw_dps(ui);
                    self.draw_gamelog_settings(ui);
                });
            });

        ctx.request_repaint_after(Duration::from_millis(100));

        if close_requested {
            let rect = outer_rect.or(inner_rect);
            save_persisted_state(self, rect);
        }
    }
}
