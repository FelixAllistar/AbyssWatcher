use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use crate::core::{log_io, model, state, tracker};
use eframe::{egui, NativeOptions};
use egui_plot::{Line, Plot, PlotBounds, PlotPoint, PlotPoints};
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

fn round_up_to_step(value: f32, step: f32) -> f32 {
    if step <= 0.0 {
        return value;
    }
    (value / step).ceil() * step
}

fn nice_rounded_max(value: f32) -> f32 {
    let v = value.max(0.0);
    if v <= 500.0 {
        round_up_to_step(v, 50.0)
    } else if v <= 5_000.0 {
        round_up_to_step(v, 100.0)
    } else {
        let magnitude = 10_f32.powi((v.log10().floor() as i32).saturating_sub(1));
        round_up_to_step(v, magnitude.max(100.0))
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

    // Set explicit window title and icon.
    viewport_builder = viewport_builder.with_title("AbyssWatcher");
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../AbyssWatcher.png")) {
        viewport_builder = viewport_builder.with_icon(icon);
    }

    if persisted.has_position {
        viewport_builder = viewport_builder.with_position(egui::pos2(persisted.x, persisted.y));
    }

    let options = NativeOptions {
        viewport: viewport_builder,
        ..NativeOptions::default()
    };

    let _ = eframe::run_native(
        "AbyssWatcher",
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
    display_max_dps: f32,
    peak_out_dps: f32,
    peak_in_dps: f32,

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
            display_max_dps: 0.0,
            peak_out_dps: 0.0,
            peak_in_dps: 0.0,
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
    }

    fn draw_dps(&mut self, ui: &mut egui::Ui) {
        self.poll_engine();

        ui.horizontal(|ui| {
            let (out_dps, in_dps, peak_out, peak_in) = if let Some(sample) = self.dps_samples.last()
            {
                let current_top_out = sample
                    .outgoing_by_target
                    .values()
                    .fold(0.0_f32, |acc, v| acc.max(*v));
                let current_top_in = sample
                    .incoming_by_source
                    .values()
                    .fold(0.0_f32, |acc, v| acc.max(*v));

                self.peak_out_dps = self.peak_out_dps.max(current_top_out);
                self.peak_in_dps = self.peak_in_dps.max(current_top_in);

                (
                    sample.outgoing_dps,
                    sample.incoming_dps,
                    self.peak_out_dps,
                    self.peak_in_dps,
                )
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };

            ui.label(format!("Out: {:.1}", out_dps));
            ui.label(format!("In: {:.1}", in_dps));
            ui.label(format!("Peak Out: {:.1}", peak_out));
            ui.label(format!("Peak In: {:.1}", peak_in));
        });

        // DPS history chart using egui::plot
        if !self.dps_samples.is_empty() {
            let max_points = 120usize;
            let len = self.dps_samples.len();
            let start = len.saturating_sub(max_points);
            let slice = &self.dps_samples[start..];

            let window_secs_f = self.dps_window_secs.max(1) as f64;

            let mut out_points = Vec::with_capacity(slice.len());
            let mut in_points = Vec::with_capacity(slice.len());

            let last_time = slice.last().map(|s| s.time.as_secs_f64()).unwrap_or(0.0);

            for sample in slice {
                let t_rel = sample.time.as_secs_f64() - last_time; // 0 at "now", negative to the left
                out_points.push([t_rel, sample.outgoing_dps as f64]);
                in_points.push([t_rel, sample.incoming_dps as f64]);
            }

            // Use session peaks to define a stable Y range for the plot,
            // add some headroom so the graph doesn't hug the top edge,
            // then round up to a "nice" value (50/100/etc).
            let peak_max = self.peak_out_dps.max(self.peak_in_dps).max(10.0);
            let with_headroom = (peak_max * 1.15).max(10.0);
            self.display_max_dps = nice_rounded_max(with_headroom);

            let out_line = Line::new(PlotPoints::from(out_points))
                .name("Outgoing DPS")
                .color(egui::Color32::from_rgb(0, 191, 255));
            let in_line = Line::new(PlotPoints::from(in_points))
                .name("Incoming DPS")
                .color(egui::Color32::from_rgb(255, 64, 64));

            ui.add_space(4.0);
            let plot_resp = Plot::new("dps_history")
                .height(140.0)
                .set_margin_fraction(egui::vec2(0.0, 0.0))
                .y_axis_width(3)
                // We draw numeric labels ourselves; keep grid only.
                .show_axes(egui::Vec2b::new(false, false))
                .show_grid(egui::Vec2b::new(true, true))
                .auto_bounds(egui::Vec2b::new(false, false))
                .allow_drag(false)
                .allow_boxed_zoom(false)
                .allow_scroll(false)
                .allow_zoom(false)
                .show(ui, |plot_ui| {
                    let bounds = PlotBounds::from_min_max(
                        [-window_secs_f, 0.0],
                        [0.0, self.display_max_dps.max(1.0) as f64],
                    );
                    plot_ui.set_plot_bounds(bounds);
                    plot_ui.line(out_line);
                    plot_ui.line(in_line);
                });

            // Custom axis labels: X on bottom (seconds relative to now), Y on left (DPS).
            let transform = plot_resp.transform;
            let frame = *transform.frame();
            let text_color = ui.visuals().strong_text_color();
            let font_id = egui::TextStyle::Body.resolve(ui.style());
            let painter = ui.painter();

            // X-axis ticks: 0 (now) and negative seconds at regular intervals,
            // drawn just below the plot frame.
            let x_min = -window_secs_f;
            let x_step = (window_secs_f / 3.0).max(1.0);
            let mut x = 0.0;
            while x >= x_min {
                let pos = transform.position_from_point(&PlotPoint::new(x, 0.0));
                let label = format!("{:.0}", x);
                painter.text(
                    egui::pos2(pos.x, frame.bottom() + 4.0),
                    egui::Align2::CENTER_TOP,
                    label,
                    font_id.clone(),
                    text_color,
                );
                x -= x_step;
            }

            // Y-axis ticks: 0, 1/3, 2/3, and peak, drawn just to the left of the plot frame.
            let y_max = self.display_max_dps.max(1.0);
            let y_step = y_max / 4.0;
            for i in 0..=4 {
                let value = y_step * i as f32;
                let pos = transform.position_from_point(&PlotPoint::new(0.0, value as f64));
                let label = format!("{:.0}", value);
                painter.text(
                    egui::pos2(frame.left() - 14.0, pos.y),
                    egui::Align2::RIGHT_CENTER,
                    label,
                    font_id.clone(),
                    text_color,
                );
            }
        }

        // Detailed targets / incoming / weapon lists based on latest sample
        if let Some(sample) = self.dps_samples.last() {
            ui.add_space(16.0);
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

                        for (name, dps) in entries {
                            ui.label(format!("{name}: {dps:.1}"));
                        }
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

                        for (name, dps) in entries {
                            ui.label(format!("{name}: {dps:.1}"));
                        }
                    }
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.label("Top weapons");
                    let mut entries: Vec<_> = sample
                        .outgoing_by_weapon
                        .iter()
                        .filter(|(name, _)| !name.is_empty())
                        .map(|(name, dps)| (name.as_str(), *dps))
                        .collect();

                    if entries.is_empty() {
                        ui.label("None");
                    } else {
                        entries.sort_by(|a, b| b.1.total_cmp(&a.1));

                        for (name, dps) in entries {
                            ui.label(format!("{name}: {dps:.1}"));
                        }
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
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(
                        0,
                        0,
                        0,
                        (self.opacity * 255.0) as u8,
                    ))
                    .inner_margin(egui::Margin::symmetric(6.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.visuals_mut().override_text_color = Some(egui::Color32::from_rgb(235, 235, 235));
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
            });

        // Main content panel with semi-transparent background
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(
                        0,
                        0,
                        0,
                        (self.opacity * 255.0) as u8,
                    ))
                    .inner_margin(egui::Margin {
                        left: 46.0,
                        right: 12.0,
                        top: 4.0,
                        bottom: 22.0,
                    }),
            )
            .show(ctx, |ui| {
                ui.visuals_mut().override_text_color = Some(egui::Color32::from_rgb(235, 235, 235));
                egui::ScrollArea::vertical()
                    .id_source("main_scroll")
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            self.draw_dps(ui);
                            self.draw_gamelog_settings(ui);
                        });
                    });
            });

        ctx.request_repaint_after(Duration::from_millis(100));

        if close_requested {
            let rect = outer_rect.or(inner_rect);
            save_persisted_state(self, rect);
        }
    }
}
