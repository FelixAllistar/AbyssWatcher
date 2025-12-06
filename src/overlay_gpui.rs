use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use crate::core::{log_io, model, state, tracker};
use gpui::{
    App, Application, ClickEvent, Context, Entity, Render, Subscription, Window,
    WindowBackgroundAppearance, WindowOptions,
};
use gpui::prelude::*;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui::{div, rgb, rgba, Hsla, SharedString};
use gpui_component::plot::{
    scale::{Scale, ScaleLinear, ScalePoint},
    shape::Line as PlotLine,
    AxisText, Grid, Plot, PlotAxis, StrokeStyle, AXIS_GAP,
};
use gpui_component::plot::IntoPlot;
use gpui_component::PixelsExt;
use gpui_component::theme::{ActiveTheme as _, Theme, ThemeMode};
use gpui_component::{h_flex, v_flex};
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

fn save_persisted_state(state: &PersistedState) {
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = fs::write("app_state.json", json);
    }
}

#[derive(Clone)]
struct DpsPoint {
    label: SharedString,
    outgoing: f64,
    incoming: f64,
}

#[derive(IntoPlot)]
struct DpsChart {
    points: Vec<DpsPoint>,
    out_color: Hsla,
    in_color: Hsla,
    tick_margin: usize,
    y_max: f64,
}

impl DpsChart {
    fn new(
        points: Vec<DpsPoint>,
        out_color: Hsla,
        in_color: Hsla,
        tick_margin: usize,
        y_max: f64,
    ) -> Self {
        Self {
            points,
            out_color,
            in_color,
            tick_margin: tick_margin.max(1),
            y_max,
        }
    }
}

impl Plot for DpsChart {
    fn paint(
        &mut self,
        bounds: gpui::Bounds<gpui::Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.points.is_empty() {
            return;
        }

        let width = bounds.size.width.as_f32();
        let height = bounds.size.height.as_f32() - AXIS_GAP;

        // X scale over time labels.
        let x_scale = ScalePoint::new(
            self.points.iter().map(|p| p.label.clone()).collect(),
            vec![0., width],
        );

        // Y scale from 0 to configured max (with headroom), falling back to data max if needed.
        let configured_max = self.y_max.max(0.0);
        let mut y_domain = if configured_max > 0.0 {
            vec![0.0_f64, configured_max]
        } else {
            self.points
                .iter()
                .flat_map(|p| [p.outgoing, p.incoming])
                .chain(std::iter::once(0.0))
                .collect::<Vec<f64>>()
        };
        if y_domain.iter().all(|v| *v == 0.0) {
            y_domain.push(1.0);
        }
        let y_scale = ScaleLinear::new(y_domain.clone(), vec![height, 10.0]);

        // X axis with decimated tick labels.
        let tick_margin = self.tick_margin.max(1);
        let x_labels = self
            .points
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                if (i + 1) % tick_margin != 0 {
                    return None;
                }
                x_scale.tick(&p.label).map(|x_tick| {
                    AxisText::new(p.label.clone(), x_tick, cx.theme().muted_foreground)
                })
            });

        // Y axis labels based on evenly spaced values from 0 to y_max.
        let y_axis_values: Vec<f64> = if configured_max > 0.0 {
            (0..=4)
                .map(|i| configured_max * i as f64 / 4.0)
                .collect()
        } else {
            vec![0.0]
        };

        let y_labels = y_axis_values.iter().filter_map(|v| {
            y_scale.tick(v).map(|y_tick| {
                let text = if configured_max >= 10.0 {
                    format!("{:.0}", v)
                } else {
                    format!("{:.1}", v)
                };
                AxisText::new(text, y_tick, cx.theme().muted_foreground)
            })
        });

        PlotAxis::new()
            .x(height)
            .x_label(x_labels)
            .y(gpui::px(0.0))
            .y_label(y_labels)
            .stroke(cx.theme().border)
            .paint(&bounds, window, cx);

        // Horizontal grid lines aligned with Y labels.
        let grid_y_ticks: Vec<f32> = y_axis_values
            .iter()
            .filter_map(|v| y_scale.tick(v))
            .collect();
        Grid::new()
            .y(grid_y_ticks)
            .stroke(cx.theme().border)
            .dash_array(&[gpui::px(4.), gpui::px(2.)])
            .paint(&bounds, window);

        // Outgoing DPS line.
        let x_scale_out = x_scale.clone();
        let y_scale_out = y_scale.clone();
        let mut out_line = PlotLine::new()
            .data(self.points.clone())
            .x(move |p: &DpsPoint| x_scale_out.tick(&p.label))
            .y(move |p: &DpsPoint| y_scale_out.tick(&p.outgoing))
            .stroke(self.out_color)
            .stroke_style(StrokeStyle::Natural)
            .stroke_width(gpui::px(2.));

        // Incoming DPS line.
        let x_scale_in = x_scale.clone();
        let y_scale_in = y_scale.clone();
        let mut in_line = PlotLine::new()
            .data(self.points.clone())
            .x(move |p: &DpsPoint| x_scale_in.tick(&p.label))
            .y(move |p: &DpsPoint| y_scale_in.tick(&p.incoming))
            .stroke(self.in_color)
            .stroke_style(StrokeStyle::Natural)
            .stroke_width(gpui::px(2.));

        // Only show dots when point count is small to avoid clutter.
        if self.points.len() <= 40 {
            out_line = out_line
                .dot()
                .dot_size(gpui::px(6.))
                .dot_fill_color(self.out_color);
            in_line = in_line
                .dot()
                .dot_size(gpui::px(6.))
                .dot_fill_color(self.in_color);
        }

        out_line.paint(&bounds, window);
        in_line.paint(&bounds, window);
    }
}

pub fn run_overlay() {
    let persisted = load_persisted_state();

    Application::new().run(move |app: &mut App| {
        // CRITICAL: Initialize gpui-component before using any components
        gpui_component::init(app);

        // Set dark theme for the overlay
        Theme::change(ThemeMode::Dark, None, app);

        let mut options = WindowOptions::default();
        options.window_background = WindowBackgroundAppearance::Transparent;

        let persisted_clone = persisted.clone();
        app.open_window(options, move |window, cx| {
            cx.new(|cx| AbyssWatcherView::new(window, cx, persisted_clone.clone()))
        })
        .unwrap();
    });
}

struct CharacterEntry {
    name: String,
    file_path: PathBuf,
    last_modified: SystemTime,
    tracked: bool,
}

struct AbyssWatcherView {
    persisted_state: PersistedState,

    gamelog_dir: Option<PathBuf>,
    gamelog_input: String,
    characters: Vec<CharacterEntry>,
    show_characters_menu: bool,

    engine: state::EngineState,
    trackers: HashMap<PathBuf, tracker::TrackedGamelog>,
    events_by_path: HashMap<PathBuf, Vec<model::CombatEvent>>,
    last_tracked_paths: HashSet<PathBuf>,
    last_event_timestamp: Option<Duration>,
    last_event_wallclock: Option<SystemTime>,

    dps_window_secs: u64,
    dps_samples: Vec<model::DpsSample>,
    peak_out_dps: f32,
    peak_in_dps: f32,
    display_max_dps: f32,

    gamelog_input_state: Entity<InputState>,
    _gamelog_sub: Subscription,

    last_update: Instant,
    opacity: f32,
}

impl AbyssWatcherView {
    fn new(window: &mut Window, cx: &mut Context<Self>, persisted: PersistedState) -> Self {
        let input_state = cx.new(|cx| InputState::new(window, cx));

        // Seed input text from persisted/default.
        let initial_gamelog = persisted
            .gamelog_dir
            .clone()
            .unwrap_or_else(|| DEFAULT_GAMELOG_PATH.to_string());
        input_state.update(cx, |state, cx| {
            state.set_value(initial_gamelog.clone(), window, cx);
        });

        // Subscribe to input changes to keep our string in sync.
        let gamelog_sub = cx.subscribe_in(&input_state, window, |this, state, ev: &InputEvent, _window, cx| {
            match ev {
                InputEvent::Change => {
                    this.gamelog_input = state.read(cx).value().to_string();
                }
                _ => {}
            }
        });

        let mut view = Self {
            persisted_state: persisted.clone(),
            gamelog_dir: persisted.gamelog_dir.clone().map(PathBuf::from),
            gamelog_input: initial_gamelog,
            characters: Vec::new(),
            show_characters_menu: false,
            engine: state::EngineState::new(),
            trackers: HashMap::new(),
            events_by_path: HashMap::new(),
            last_tracked_paths: HashSet::new(),
            last_event_timestamp: None,
            last_event_wallclock: None,
            dps_window_secs: persisted.dps_window_secs.max(1),
            dps_samples: Vec::new(),
            peak_out_dps: 0.0,
            peak_in_dps: 0.0,
            display_max_dps: 0.0,
            gamelog_input_state: input_state,
            _gamelog_sub: gamelog_sub,
            last_update: Instant::now(),
            opacity: persisted.opacity,
        };

        view.try_initial_scan();

        view
    }

    fn try_initial_scan(&mut self) {
        let path = if let Some(dir) = &self.persisted_state.gamelog_dir {
            PathBuf::from(dir)
        } else {
            PathBuf::from(DEFAULT_GAMELOG_PATH)
        };

        if let Ok(logs) = log_io::scan_gamelogs_dir(&path) {
            if !logs.is_empty() {
                self.gamelog_dir = Some(path.clone());

                let tracked_set: HashSet<String> =
                    self.persisted_state.tracked_files.iter().cloned().collect();

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

    fn update_persisted_from_self(&mut self) {
        self.persisted_state.opacity = self.opacity;
        self.persisted_state.dps_window_secs = self.dps_window_secs;
        self.persisted_state.gamelog_dir =
            self.gamelog_dir.as_ref().map(|p| p.display().to_string());
        self.persisted_state.tracked_files = self
            .characters
            .iter()
            .filter(|c| c.tracked)
            .map(|c| c.file_path.display().to_string())
            .collect();
    }

    fn persist(&mut self) {
        self.update_persisted_from_self();
        save_persisted_state(&self.persisted_state);
    }
}

impl Render for AbyssWatcherView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.refresh();
        self.poll_engine();

        // Keep persisted bounds in sync so close/save works.
        let bounds = window.bounds();
        self.persisted_state.width = f32::from(bounds.size.width).max(260.0);
        self.persisted_state.height = f32::from(bounds.size.height).max(180.0);
        self.persisted_state.x = f32::from(bounds.origin.x);
        self.persisted_state.y = f32::from(bounds.origin.y);
        self.persisted_state.has_position = true;

        // Force continuous repaint to keep data flowing even without interaction.
        window.on_next_frame(|window, _cx| {
            window.refresh();
        });

        let (out_dps, in_dps, peak_out, peak_in) = if let Some(sample) = self.dps_samples.last() {
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

        let peak_max = self.peak_out_dps.max(self.peak_in_dps).max(10.0);
        let with_headroom = (peak_max * 1.15).max(10.0);
        self.display_max_dps = nice_rounded_max(with_headroom);

        let theme = cx.theme();
        
        // Colors
        // Background: #141414 with variable opacity
        // Top Bar: #0a0a0a with slightly higher opacity
        // Text: #ebebeb
        let alpha_byte = (self.opacity * 255.0) as u32;
        let bar_alpha_byte = ((self.opacity * 0.9 + 0.1).min(1.0) * 255.0) as u32; // Slightly more opaque
        
        // gpui::rgba takes u32 as 0xRRGGBBAA
        let bg_color = rgba((0x14 << 24) | (0x14 << 16) | (0x14 << 8) | alpha_byte);
        let bar_bg_color = rgba((0x0a << 24) | (0x0a << 16) | (0x0a << 8) | bar_alpha_byte);
        let text_color = rgb(0xebebeb);
        let border_color = rgba(0x333333ff); // Subtle border

        let mut root = v_flex()
            .size_full()
            .bg(bg_color)
            .text_color(text_color);

        // Top bar
        let top_bar = {
            let mut bar = h_flex()
                .items_center()
                .justify_between() // Space out left and right items
                .px_3()
                .py_1()
                .bg(bar_bg_color)
                .border_b_1()
                .border_color(border_color);

            // Left: Characters Menu
            let chars_label = if self.characters.is_empty() {
                "Characters".to_string()
            } else {
                let count = self.characters.iter().filter(|c| c.tracked).count();
                if count == 0 {
                    "Characters".to_string()
                } else {
                    format!("Characters ({})", count)
                }
            };

            let chars_button = Button::new("characters-btn")
                .label(chars_label)
                .ghost()
                .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                    this.show_characters_menu = !this.show_characters_menu;
                    cx.notify();
                }));

            // Right: Controls
            let controls = h_flex()
                .items_center()
                .gap_4()
                // Opacity
                .child(
                    h_flex()
                        .items_center()
                        .gap_1()
                        .child("Opacity")
                        .child(format!("{:.0}%", self.opacity * 100.0))
                        .child(
                            Button::new("opacity-dec")
                                .label("-")
                                .compact()
                                .ghost()
                                .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                    this.opacity = (this.opacity - 0.05).max(0.2);
                                    this.persist();
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("opacity-inc")
                                .label("+")
                                .compact()
                                .ghost()
                                .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                    this.opacity = (this.opacity + 0.05).min(1.0);
                                    this.persist();
                                    cx.notify();
                                })),
                        )
                )
                // Window
                .child(
                    h_flex()
                        .items_center()
                        .gap_1()
                        .child("Window")
                        .child(format!("{}s", self.dps_window_secs))
                        .child(
                            Button::new("window-dec")
                                .label("-")
                                .compact()
                                .ghost()
                                .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                    if this.dps_window_secs > 1 {
                                        this.dps_window_secs -= 1;
                                        this.persist();
                                        cx.notify();
                                    }
                                })),
                        )
                        .child(
                            Button::new("window-inc")
                                .label("+")
                                .compact()
                                .ghost()
                                .on_click(cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                    if this.dps_window_secs < 60 {
                                        this.dps_window_secs += 1;
                                        this.persist();
                                        cx.notify();
                                    }
                                })),
                        )
                );

            bar = bar.child(chars_button).child(controls);
            bar
        };
        root = root.child(top_bar);

        // Body scrollable content
        let mut body = v_flex()
            .id("body_scroll")
            .gap_4()
            .pl(gpui::px(46.0)) // Left padding ~46px
            .pr(gpui::px(12.0)) // Right padding ~12px
            .pt_3()
            .pb_6()
            .flex_1();


        // DPS summary row
        let header = h_flex()
            .gap_6()
            .child(format!("Out: {:.1}", out_dps))
            .child(format!("In: {:.1}", in_dps))
            .child(format!("Peak Out: {:.1}", peak_out))
            .child(format!("Peak In: {:.1}", peak_in));
        body = body.child(header);

        // DPS chart
        if !self.dps_samples.is_empty() {
            let window_secs = self.dps_window_secs.max(1) as f32;
            let max_points = 180usize;
            let len = self.dps_samples.len();
            let start = len.saturating_sub(max_points);
            let slice = &self.dps_samples[start..];

            let last_time = slice
                .last()
                .map(|s| s.time.as_secs_f64() as f32)
                .unwrap_or(0.0);
            let x_min = -window_secs;

            let mut points: Vec<DpsPoint> = Vec::with_capacity(slice.len());
            for sample in slice {
                let t_rel = sample.time.as_secs_f64() as f32 - last_time;
                // Only keep points within the current window.
                if t_rel < x_min {
                    continue;
                }
                let label = SharedString::from(format!("{:.0}", t_rel));
                points.push(DpsPoint {
                    label,
                    outgoing: sample.outgoing_dps as f64,
                    incoming: sample.incoming_dps as f64,
                });
            }

            if !points.is_empty() {
                let desired_ticks = 8usize;
                let tick_margin = (points.len() / desired_ticks).max(1);

                let chart = DpsChart::new(
                    points,
                    theme.chart_1,
                    theme.chart_2,
                    tick_margin,
                    self.display_max_dps as f64,
                );

                let chart_container = div()
                    .min_h(gpui::px(140.0))
                    .flex_1()
                    .child(chart);

                body = body.child(chart_container);
            }
        }

        // Characters menu dropdown
        if self.show_characters_menu {
            let mut menu = v_flex()
                .gap_1()
                .p_2()
                .bg(theme.popover)
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .shadow_lg();

            if self.characters.is_empty() {
                menu = menu.child("No characters detected");
            } else {
                for (i, entry) in self.characters.iter().enumerate() {
                    let file_name = entry
                        .file_path
                        .file_name()
                        .and_then(|v| v.to_str())
                        .unwrap_or_default()
                        .to_string();
                    let label = format!(
                        "[{}] {} ({})",
                        if entry.tracked { "x" } else { " " },
                        entry.name,
                        file_name
                    );
                    let idx = i;
                    menu = menu.child(
                        div()
                            .id(("char-menu", idx))
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.accent))
                            .p_1()
                            .rounded_sm()
                            .child(label)
                            .on_click(cx.listener(move |this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                if let Some(e) = this.characters.get_mut(idx) {
                                    e.tracked = !e.tracked;
                                    this.last_update = Instant::now() - Duration::from_millis(250);
                                    this.persist();
                                    cx.notify();
                                }
                            })),
                    );
                }
            }
            body = body.child(menu);
        }
        
        // Re-structure body construction to be cleaner
        // ... (I will do this in the actual code block below)

        // Detailed targets / incoming / weapon lists
        if let Some(sample) = self.dps_samples.last() {
            let mut stats_row = h_flex().gap_6().items_start();

            // Helper for columns
            let make_column = |title: &str, items: Vec<(&str, f32)>| {
                let mut col = v_flex().gap_1().flex_1();
                col = col.child(
                    div()
                        .child(title.to_string())
                        .font_weight(gpui::FontWeight::BOLD)
                        .pb_1()
                        .border_b_1()
                        .border_color(rgba(0x555555ff))
                );
                
                if items.is_empty() {
                    col = col.child("None");
                } else {
                    for (name, dps) in items {
                        col = col.child(
                            h_flex()
                                .justify_between()
                                .child(name.to_string())
                                .child(format!("{:.1}", dps))
                        );
                    }
                }
                col
            };

            // Top targets
            let mut target_entries: Vec<_> = sample
                .outgoing_by_target
                .iter()
                .map(|(name, dps)| (name.as_str(), *dps))
                .collect();
            target_entries.sort_by(|a, b| b.1.total_cmp(&a.1));
            stats_row = stats_row.child(make_column("Top Targets", target_entries));

            // Top incoming
            let mut incoming_entries: Vec<_> = sample
                .incoming_by_source
                .iter()
                .map(|(name, dps)| (name.as_str(), *dps))
                .collect();
            incoming_entries.sort_by(|a, b| b.1.total_cmp(&a.1));
            stats_row = stats_row.child(make_column("Top Incoming", incoming_entries));

            // Top weapons
            let mut weapon_entries: Vec<_> = sample
                .outgoing_by_weapon
                .iter()
                .filter(|(name, _)| !name.is_empty())
                .map(|(name, dps)| (name.as_str(), *dps))
                .collect();
            weapon_entries.sort_by(|a, b| b.1.total_cmp(&a.1));
            stats_row = stats_row.child(make_column("Top Weapons", weapon_entries));

            body = body.child(stats_row);
        }

        // Gamelog folder input when nothing detected
        if self.characters.is_empty() {
            let mut gamelog_ui = v_flex().gap_2().pt_4();
            gamelog_ui = gamelog_ui.child("Gamelog folder:");
            gamelog_ui = gamelog_ui.child(Input::new(&self.gamelog_input_state));
            gamelog_ui = gamelog_ui.child(
                Button::new("scan-gamelog-btn")
                    .label("Scan Gamelog Folder")
                    .primary()
                    .on_click(
                    cx.listener(|this, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                        let path = PathBuf::from(this.gamelog_input.clone());
                        if let Ok(logs) = log_io::scan_gamelogs_dir(&path) {
                            this.gamelog_dir = Some(path.clone());
                            this.characters = logs
                                .into_iter()
                                .map(|log| CharacterEntry {
                                    name: log.character,
                                    file_path: log.path,
                                    last_modified: log.last_modified,
                                    tracked: false,
                                })
                                .collect();
                            this.characters.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
                            this.persist();
                            cx.notify();
                        }
                    }),
                ),
            );

            body = body.child(gamelog_ui);
        }

        root = root.child(body);
        root
    }
}
