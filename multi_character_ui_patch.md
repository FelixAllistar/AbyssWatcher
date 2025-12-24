# Multi-Character UI Enhancement Patch

This file contains the additional UI code needed to visualize multi-character DPS data.
The core data tracking is already complete and working. These changes add:
- Character color palette
- Collapse state tracking  
- Per-character graph lines
- Collapsible character breakdown sections

## Changes needed to `src/overlay_egui.rs`:

### 1. After line 12 (after DEFAULT_GAMELOG_PATH), add:

```rust
// Character color palette for multi-character visualization
const CHARACTER_COLORS: &[(u8, u8, u8)] = &[
    (0, 191, 255),    // Deep sky blue
    (50, 205, 50),    // Lime green
    (255, 215, 0),    // Gold
    (255, 140, 0),    // Dark orange
    (186, 85, 211),   // Medium orchid
    (255, 105, 180),  // Hot pink
];

fn get_character_color(index: usize) -> egui::Color32 {
    let (r, g, b) = CHARACTER_COLORS[index % CHARACTER_COLORS.len()];
    egui::Color32::from_rgb(r, g, b)
}
```

### 2. In `AbyssWatcherApp` struct (after `opacity: f32,`), add field:

```rust
character_sections_expanded: HashMap<String, bool>,
```

### 3. In `AbyssWatcherApp::new()` initialization (after `opacity: persisted.opacity,`), add:

```rust
character_sections_expanded: HashMap::new(),
```

### 4. In `draw_dps()` function, after the plot section (around line 404), replace:

```rust
                    plot_ui.set_plot_bounds(bounds);
                    plot_ui.line(out_line);
                    plot_ui.line(in_line);
                });
```

With:

```rust
                    plot_ui.set_plot_bounds(bounds);
                    plot_ui.line(out_line);
                    plot_ui.line(in_line);
                    
                    // Draw per-character lines
                    if let Some(latest_sample) = self.dps_samples.last() {
                        let characters: Vec<_> = latest_sample.outgoing_by_character.keys().collect();
                        
                        for (char_idx, character) in characters.iter().enumerate() {
                            let mut char_points = Vec::with_capacity(slice.len());
                            
                            for sample in slice {
                                let t_rel = sample.time.as_secs_f64() - last_time;
                                let char_dps = sample.outgoing_by_character
                                    .get(*character)
                                    .copied()
                                    .unwrap_or(0.0) as f64;
                                char_points.push([t_rel, char_dps]);
                            }
                            
                            let char_line = Line::new(PlotPoints::from(char_points))
                                .name(format!("{}", character))
                                .color(get_character_color(char_idx))
                                .width(1.5);
                            plot_ui.line(char_line);
                        }
                    }
                });
```

### 5. At the end of `draw_dps()` (after the weapon/target/incoming sections around line 511), add before the closing brace:

```rust
        // Collapsible character breakdown
        self.draw_character_breakdown(ui);
    }
    
    fn draw_character_breakdown(&mut self, ui: &mut egui::Ui) {
        if let Some(sample) = self.dps_samples.last() {
            if sample.outgoing_by_character.is_empty() {
                return;
            }
            
            ui.add_space(16.0);
            ui.separator();
            ui.label(\"Character Breakdown\");
            ui.add_space(8.0);
            
            // Sort characters by current DPS (descending)
            let mut char_dps: Vec<_> = sample.outgoing_by_character.iter().collect();
            char_dps.sort_by(|a, b| b.1.total_cmp(a.1));
            
            for (char_idx, (character, &dps)) in char_dps.iter().enumerate() {
                let is_expanded = self.character_sections_expanded
                    .entry(character.to_string())
                    .or_insert(true);
                
                let header_text = format!(\"{}  ({:.1} DPS)\", character, dps);
                let color = get_character_color(char_idx);
                
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    
                    // Colored indicator
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(12.0, 12.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, 2.0, color);
                    
                    ui.add_space(4.0);
                    
                    // Collapsible header
                    let header_response = ui.selectable_label(false, header_text);
                    if header_response.clicked() {
                        *is_expanded = !*is_expanded;
                    }
                });
                
                if *is_expanded {
                    ui.indent(character, |ui| {
                        ui.horizontal(|ui| {
                            // Top targets
                            ui.vertical(|ui| {
                                ui.label(\"Top Targets:\");
                                ui.add_space(2.0);
                                
                                let mut targets: Vec<_> = sample.outgoing_by_target.iter()
                                    .map(|(name, dps)| (name.as_str(), *dps))
                                    .collect();
                                targets.sort_by(|a, b| b.1.total_cmp(&a.1));
                                
                                for (name, target_dps) in targets.iter().take(3) {
                                    ui.label(format!(\"  {} ({:.1})\", name, target_dps));
                                }
                                if targets.is_empty() {
                                    ui.label(\"  None\");
                                }
                            });
                            
                            ui.add_space(16.0);
                            
                            // Top weapons  
                            ui.vertical(|ui| {
                                ui.label(\"Top Weapons:\");
                                ui.add_space(2.0);
                                
                                let mut weapons: Vec<_> = sample.outgoing_by_weapon.iter()
                                    .filter(|(name, _)| !name.is_empty())
                                    .map(|(name, dps)| (name.as_str(), *dps))
                                    .collect();
                                weapons.sort_by(|a, b| b.1.total_cmp(&a.1));
                                
                                for (name, weapon_dps) in weapons.iter().take(3) {
                                    ui.label(format!(\"  {} ({:.1})\", name, weapon_dps));
                                }
                                if weapons.is_empty() {
                                    ui.label(\"  None\");
                                }
                            });
                        });
                    });
                }
                
                ui.add_space(4.0);
            }
        }
```

## Status

Core functionality (character tracking in events and DPS aggregation) is ✅ COMPLETE and TESTED.

UI enhancements are OPTIONAL but recommended for the best multi-character experience. The app will work without them, just showing combined DPS as before.

To apply these UI changes, manually edit `src/overlay_egui.rs` following the sections above.
