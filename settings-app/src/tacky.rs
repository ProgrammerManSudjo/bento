//! Tacky Borders: the coloured outline round windows (its config.yaml: width, colours, animation, effects and the apps it skips).
//! Tacky reloads the file by itself when it changes.

use crate::{theme::*, App};
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use yaml_rust2::Yaml;

fn toggle_row(app: &mut App, ui: &mut Ui, name: &str, hint: &str, path: &[&str]) {
    let mut v = app.tacky.bool(path, false);
    if row(ui, name, hint, |ui| toggle(ui, &mut v)).changed() {
        let r = app.tacky.set_bool(path, v);
        app.done(r);
    }
}

fn slider_row(app: &mut App, ui: &mut Ui, name: &str, hint: &str, path: &[&str], min: i64, max: i64, suffix: &str) {
    let mut v = app.tacky.int(path, min);
    let was = v;
    let response = row(ui, name, hint, |ui| slider(ui, &mut v, min, max, suffix));
    if v != was && !response.dragged() {
        let r = app.tacky.set_int(path, v);
        app.done(r);
    }
}

fn choice_row(app: &mut App, ui: &mut Ui, name: &str, hint: &str, path: &[&str], options: &[&str]) {
    let mut v = app.tacky.string(path);
    if row(ui, name, hint, |ui| segmented(ui, options, &mut v)) {
        let r = app.tacky.set(path, &v); // words like V2 or AboveWindow are written plain
        app.done(r);
    }
}

fn hex_colour(text: &str) -> Option<Color32> {
    let h = text.trim().strip_prefix('#')?;
    let n = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    match h.len() {
        6 => Some(Color32::from_rgb(n(0)?, n(2)?, n(4)?)),
        8 => Some(Color32::from_rgba_unmultiplied(n(0)?, n(2)?, n(4)?, n(6)?)),
        _ => None,
    }
}

/// A colour: a small swatch and a box for "#rrggbb", "#rrggbbaa" or "accent". A gradient is only mentioned (edit it in the file).
fn colour_row(app: &mut App, ui: &mut Ui, name: &str, hint: &str, path: &[&str]) {
    if matches!(app.tacky.at(path), Yaml::Hash(_)) {
        row(ui, name, hint, |ui| small(ui, "a gradient: edit it in config.yaml"));
        return;
    }
    let id = egui::Id::new(("tacky-colour", path.join(".")));
    let current = app.tacky.string(path);
    let mut v: String = ui.data_mut(|d| d.get_temp::<(String, String)>(id)).filter(|(s, _)| *s == current).map(|(_, e)| e).unwrap_or_else(|| current.clone());
    let committed = row(ui, name, hint, |ui| {
        let committed = input(ui, &mut v, 130.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
        let shown = if v.trim() == "accent" { Color32::from_rgb(120, 120, 120) } else { hex_colour(&v).unwrap_or(Color32::TRANSPARENT) };
        ui.painter().rect(rect, CornerRadius::same(9), shown, Stroke::new(1.0, LINE_STRONG), StrokeKind::Inside);
        committed
    });
    ui.data_mut(|d| d.insert_temp(id, (current.clone(), v.clone())));
    if committed && v != current && (v.trim() == "accent" || hex_colour(&v).is_some()) {
        let r = app.tacky.set_text(path, v.trim());
        app.done(r);
    }
}

fn number_row(app: &mut App, ui: &mut Ui, name: &str, hint: &str, list: &[&str], idx: usize, field: &str) {
    let id = egui::Id::new(("tacky-number", list.join("."), idx, field.to_string()));
    let current = app.tacky.item_string(list, idx, field);
    let mut v: String = ui.data_mut(|d| d.get_temp::<(String, String)>(id)).filter(|(s, _)| *s == current).map(|(_, e)| e).unwrap_or_else(|| current.clone());
    let committed = row(ui, name, hint, |ui| input(ui, &mut v, 110.0));
    ui.data_mut(|d| d.insert_temp(id, (current.clone(), v.clone())));
    if committed && v != current {
        if v.trim().parse::<f64>().is_ok() {
            let r = app.tacky.set_item_field(list, idx, field, v.trim());
            app.done(r);
        } else {
            app.say("That is not a number");
        }
    }
}

fn item_choice(app: &mut App, ui: &mut Ui, name: &str, hint: &str, list: &[&str], idx: usize, field: &str, options: &[&str]) {
    let mut v = app.tacky.item_string(list, idx, field);
    if row_below(ui, name, hint, |ui| choices(ui, options, &mut v)) {
        let r = app.tacky.set_item_field(list, idx, field, &v);
        app.done(r);
    }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    if let Some(e) = &app.tacky.error {
        text(ui, e.clone(), 13.0, MUTED);
        return;
    }
    if !app.tacky.path.exists() {
        text(ui, "Tacky Borders has no config.yaml yet (start it once).", 14.0, MUTED);
        return;
    }

    card(ui, "Border", |ui| {
        slider_row(app, ui, "Width", "", &["global", "border_width"], 0, 16, " px");
        slider_row(app, ui, "Offset", "Negative moves the border inwards", &["global", "border_offset"], -20, 20, " px");
        // the corner radius is a word (Auto, Square, Round, RoundSmall) or a number
        let current = app.tacky.string(&["global", "border_radius"]);
        let mut named = if ["Auto", "Square", "Round", "RoundSmall"].contains(&current.as_str()) { current.clone() } else { String::new() };
        if row_below(ui, "Corner shape", "A shape, or a number of pixels below", |ui| choices(ui, &["Auto", "Square", "Round", "RoundSmall"], &mut named)) {
            let r = app.tacky.set(&["global", "border_radius"], &named);
            app.done(r);
        }
        if named.is_empty() {
            slider_row(app, ui, "Corner radius", "", &["global", "border_radius"], 0, 40, " px");
        }
        choice_row(app, ui, "Drawn", "Above the window can flicker with some programs", &["global", "border_z_order"], &["AboveWindow", "BelowWindow"]);
        toggle_row(app, ui, "Follow Windows' own borders", "Hide where Windows hides its border (borderless fullscreen and so on)", &["global", "follow_native_border"]);
        slider_row(app, ui, "Delay on new windows", "Waits for the window's opening animation", &["global", "initialize_delay"], 0, 1500, " ms");
        slider_row(app, ui, "Delay on restoring", "Waits for the unminimise animation", &["global", "unminimize_delay"], 0, 1500, " ms");
    });

    card(ui, "Colours", |ui| {
        accent_swatches(app, ui, "active_color", "Focused window", "Click an accent for the border. Switch on \"gradient\" to pick several: the border runs through them in the order you pick (it turns when the animation is a spiral)", true);
        if !matches!(app.tacky.at(&["global", "active_color"]), Yaml::Hash(_)) {
            colour_row(app, ui, "Focused window: custom", "#rrggbb, #rrggbbaa (with opacity) or accent", &["global", "active_color"]);
        }
        accent_swatches(app, ui, "inactive_color", "Other windows", "Pick an accent for the borders of the windows that are not focused", false);
        if !matches!(app.tacky.at(&["global", "inactive_color"]), Yaml::Hash(_)) {
            colour_row(app, ui, "Other windows: custom", "", &["global", "inactive_color"]);
        }
        toggle_row(app, ui, "Colour komorebi's stacks and monocle", "Use the three colours below for those window kinds", &["global", "komorebi_colors", "enabled"]);
        if app.tacky.bool(&["global", "komorebi_colors", "enabled"], false) {
            colour_row(app, ui, "Stack", "", &["global", "komorebi_colors", "stack_color"]);
            colour_row(app, ui, "Monocle", "", &["global", "komorebi_colors", "monocle_color"]);
            colour_row(app, ui, "Floating", "", &["global", "komorebi_colors", "floating_color"]);
        }
    });

    card(ui, "Animation", |ui| {
        toggle_row(app, ui, "Animate the borders", "", &["global", "animations", "enabled"]);
        slider_row(app, ui, "Smoothness", "Frames per second", &["global", "animations", "fps"], 15, 240, " fps");
        for (side, label) in [("active", "Focused window"), ("inactive", "Other windows")] {
            let list = ["global", "animations", side];
            if app.tacky.item_len(&list) == 0 {
                continue;
            }
            item_choice(app, ui, &format!("{label}: kind"), "", &list, 0, "type", &["Fade", "Spiral", "ReverseSpiral"]);
            number_row(app, ui, &format!("{label}: duration"), "Milliseconds", &list, 0, "duration");
            item_choice(app, ui, &format!("{label}: easing"), "", &list, 0, "easing", &["Linear", "EaseInOutQuad", "EaseInOutCubic", "EaseInOutSine", "EaseOutQuad", "EaseInQuad"]);
        }
    });

    card(ui, "Effects", |ui| {
        toggle_row(app, ui, "Shadow and glow", "Needs the V2 renderer. They cost some CPU and GPU", &["global", "effects", "enabled"]);
        choice_row(app, ui, "Renderer", "V2 draws the effects; Legacy is simpler", &["rendering_backend"], &["V2", "Legacy"]);
        for (side, label) in [("active", "Focused window"), ("inactive", "Other windows")] {
            let list = ["global", "effects", side];
            if app.tacky.item_len(&list) == 0 {
                continue;
            }
            let kind = app.tacky.item_string(&list, 0, "type");
            number_row(app, ui, &format!("{label}: {} size", kind.to_lowercase()), "", &list, 0, "radius");
            number_row(app, ui, &format!("{label}: {} strength", kind.to_lowercase()), "0 to 1", &list, 0, "opacity");
        }
    });

    card(ui, "Program", |ui| {
        toggle_row(app, ui, "Reload when the config changes", "", &["watch_config_changes"]);
        toggle_row(app, ui, "Write a log", "Needs a restart of Tacky Borders", &["enable_logging"]);
    });

    rules(app, ui);
}

/// The programs and windows Tacky treats specially: each rule can be switched, and removed; a new one is added by typing its name.
fn rules(app: &mut App, ui: &mut Ui) {
    let n = app.tacky.item_len(&["window_rules"]);
    let mut remove = None;
    let mut flip: Option<(usize, bool)> = None;
    card(ui, "Skipped windows and programs", |ui| {
        small(ui, "A rule set to on forces a border; off means no border for that window.");
        ui.add_space(4.0);
        for i in 0..n {
            let (name, kind) = (app.tacky.item_string(&["window_rules"], i, "name"), app.tacky.item_string(&["window_rules"], i, "match"));
            let enabled = app.tacky.item_string(&["window_rules"], i, "enabled").eq_ignore_ascii_case("true");
            let mut on = enabled;
            row(ui, &name, &kind.to_lowercase(), |ui| {
                if pill(ui, "remove", false).clicked() {
                    remove = Some(i);
                }
                toggle(ui, &mut on)
            });
            if on != enabled {
                flip = Some((i, on));
            }
        }
        // a new rule
        let id = egui::Id::new("tacky-new-rule");
        let (mut buffer, mut kind): (String, String) = ui.data_mut(|d| d.get_temp(id)).unwrap_or((String::new(), "Process".into()));
        let added = row_below(ui, "Add a rule", "The program (like Spotify), the window title or the window class. It starts switched off", |ui| {
            let mut go = false;
            choices(ui, &["Process", "Title", "Class"], &mut kind);
            ui.horizontal(|ui| {
                let edit = ui.add(egui::TextEdit::singleline(&mut buffer).hint_text("name...").desired_width(220.0).font(egui::FontId::new(12.5, mono())).margin(egui::Margin::symmetric(10, 5)));
                go = (edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) | button(ui, "Add", false).clicked();
            });
            go
        });
        if added && !buffer.trim().is_empty() {
            let name = format!("\"{}\"", buffer.trim().replace('"', ""));
            let r = app.tacky.append_item(&["window_rules"], &[("match", &kind), ("name", &name), ("enabled", "False")]);
            app.done(r);
            buffer.clear();
        }
        ui.data_mut(|d| d.insert_temp(id, (buffer, kind)));
    });
    if let Some(i) = remove {
        let r = app.tacky.remove_item(&["window_rules"], i);
        app.done(r);
    }
    if let Some((i, on)) = flip {
        let r = app.tacky.set_item_field(&["window_rules"], i, "enabled", if on { "True" } else { "False" });
        app.done(r);
    }
    let _ = (Pos2::ZERO, Rect::NOTHING);
}

/// The accent shades as swatches, to pick a border colour from. A click picks that colour (one, the plain colour). With `multi`, the "gradient" pill
/// switches to picking several: then a click adds or takes out a shade, and the order is the order round the border.
fn accent_swatches(app: &mut App, ui: &mut Ui, key: &str, name: &str, hint: &str, multi: bool) {
    let path = ["global", key];
    let mut chosen: Vec<String> = app.tacky.list(&["global", key, "colors"]).into_iter().map(|c| c.to_lowercase()).collect();
    let is_gradient = chosen.len() >= 2;
    if chosen.is_empty() {
        let plain = app.tacky.string(&path);
        if plain.starts_with('#') {
            chosen.push(plain.to_lowercase());
        }
    }
    // gradient mode: kept per colour, and on while the border already is a gradient
    let mode_id = egui::Id::new(("tacky-gradient-mode", key));
    let mut gradient: bool = multi && ui.data_mut(|d| d.get_temp::<bool>(mode_id)).unwrap_or(is_gradient);
    let pal = if crate::lucent::on() { crate::lucent::load_with(app.staged.strength()) } else { crate::lucent::mono_with("") };
    let shades: Vec<(String, [u8; 3])> = ["l3", "l2", "l1", "acc", "d1", "d2", "d3"].iter().filter_map(|n| pal.shade(n).map(|c| (format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]), c))).collect();
    let mut next: Option<Vec<String>> = None;
    row_below(ui, name, hint, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
            for (hex, [r, g, b]) in &shades {
                let (rect, response) = ui.allocate_exact_size(Vec2::splat(38.0), Sense::click());
                let place = chosen.iter().position(|c| c == hex);
                ui.painter().rect(rect, CornerRadius::same(12), Color32::from_rgb(*r, *g, *b), Stroke::new(if place.is_some() { 2.5 } else { 1.0 }, if place.is_some() { glow() } else if response.hovered() { MUTED } else { LINE_STRONG }), StrokeKind::Inside);
                if let (Some(n), true) = (place, gradient && chosen.len() > 1) {
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, (n + 1).to_string(), egui::FontId::new(13.0, heading()), if (*r as u32 + *g as u32 + *b as u32) > 380 { BG } else { WHITE });
                }
                if response.clicked() {
                    next = Some(if gradient {
                        let mut list = chosen.clone();
                        match place {
                            Some(n) if list.len() > 1 => { list.remove(n); }
                            Some(_) => {}
                            None => list.push(hex.clone()),
                        }
                        list
                    } else {
                        vec![hex.clone()]
                    });
                }
            }
            if multi && pill(ui, "gradient", gradient).clicked() {
                gradient = !gradient;
                ui.data_mut(|d| d.insert_temp(mode_id, gradient));
                if !gradient && chosen.len() > 1 {
                    next = Some(vec![chosen[0].clone()]); // back to one colour: the first
                }
            }
        });
    });
    // a gradient also has a direction
    let mut dir = {
        let d = app.tacky.string(&["global", key, "direction"]);
        if d.is_empty() { "45deg".to_string() } else { d }
    };
    if multi && chosen.len() >= 2 && next.is_none() && row_below(ui, &format!("{name}: direction"), "The angle the colours are laid out at", |ui| choices(ui, &["0deg", "45deg", "90deg", "135deg"], &mut dir)) {
        next = Some(chosen.clone());
    }
    if let Some(list) = next {
        let quote = |c: &String| format!("\"{c}\"");
        let r = match list.len() {
            0 => Ok(()),
            1 => app.tacky.set_inline(&path, &quote(&list[0])),
            _ => app.tacky.set_inline(&path, &format!("{{ colors: [{}], direction: {dir} }}", list.iter().map(quote).collect::<Vec<_>>().join(", "))),
        };
        app.done(r);
    }
}
