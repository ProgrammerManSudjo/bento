//! Bar: everything about a bar in one place: where it sits, its size, how it behaves, its effects and which widgets it shows.

use crate::{layout, theme::*, App};
use eframe::egui::{self, Ui};

fn path<'a>(bar: &'a str, rest: &[&'a str]) -> Vec<&'a str> {
    let mut p = vec!["bars", bar];
    p.extend_from_slice(rest);
    p
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let bars = app.cfg.keys(&["bars"]);
    if bars.is_empty() {
        text(ui, "config.yaml has no bars.", 14.0, MUTED);
        return;
    }
    if !bars.contains(&app.bar) {
        app.bar = bars[0].clone();
    }
    ui.horizontal_wrapped(|ui| {
        for b in &bars {
            let off = !app.cfg.bool(&path(b, &["enabled"]), true);
            let label = if off { format!("{b}  (off)") } else { b.clone() };
            if pill(ui, &label, *b == app.bar).clicked() {
                app.bar = b.clone();
            }
        }
    });
    ui.add_space(6.0);
    let bar = app.bar.clone();
    let b = bar.as_str();

    card(ui, "Layout", |ui| layout::show(app, ui, b));

    card(ui, "This bar", |ui| {
        app.yaml_toggle(ui, "Show this bar", "Turn the bar off without deleting it", &path(b, &["enabled"]));
        screens(app, ui, b);
    });

    card(ui, "Position", |ui| {
        app.yaml_choice(ui, "Edge", "Which edge of the screen the bar sits on", &path(b, &["alignment", "position"]), &["top", "bottom"]);
        app.yaml_choice(ui, "Align", "Where it sits along that edge", &path(b, &["alignment", "align"]), &["left", "center", "right"]);
        app.yaml_text(ui, "Width", "A percentage like 35%, a number of pixels, or auto", &path(b, &["dimensions", "width"]), 120.0);
        app.yaml_slider(ui, "Height", "Pixels", &path(b, &["dimensions", "height"]), 20, 140, " px");
    });

    card(ui, "Padding", |ui| {
        app.yaml_slider(ui, "Top", "Space between the screen edge and the bar", &path(b, &["padding", "top"]), 0, 80, " px");
        app.yaml_slider(ui, "Bottom", "", &path(b, &["padding", "bottom"]), 0, 80, " px");
        app.yaml_slider(ui, "Left", "", &path(b, &["padding", "left"]), 0, 200, " px");
        app.yaml_slider(ui, "Right", "", &path(b, &["padding", "right"]), 0, 200, " px");
    });

    card(ui, "Behaviour", |ui| {
        app.yaml_toggle(ui, "Always on top", "Stay above other windows", &path(b, &["window_flags", "always_on_top"]));
        app.yaml_toggle(ui, "Reserve space", "Act as an app bar: windows do not cover it", &path(b, &["window_flags", "windows_app_bar"]));
        app.yaml_toggle(ui, "Hide on fullscreen", "Get out of the way of games and videos", &path(b, &["window_flags", "hide_on_fullscreen"]));
        app.yaml_toggle(ui, "Auto hide", "Slide away until the pointer touches the edge", &path(b, &["window_flags", "auto_hide"]));
    });

    card(ui, "Effects", |ui| {
        app.yaml_toggle(ui, "Blur", "Blur what is behind the bar", &path(b, &["blur_effect", "enabled"]));
        app.yaml_toggle(ui, "Dark blur", "", &path(b, &["blur_effect", "dark_mode"]));
        app.yaml_toggle(ui, "Round corners", "", &path(b, &["blur_effect", "round_corners"]));
        app.yaml_text(ui, "Border colour", "A colour like #ffffff, or None", &path(b, &["blur_effect", "border_color"]), 120.0);
        app.yaml_toggle(ui, "Animation", "Slide the bar in and out", &path(b, &["animation", "enabled"]));
        app.yaml_slider(ui, "Animation time", "", &path(b, &["animation", "duration"]), 0, 1000, " ms");
    });

    for (side, hint) in [("left", "Start of the bar"), ("center", "Middle of the bar"), ("right", "End of the bar")] {
        card(ui, &format!("As rows: {side}"), |ui| layout(app, ui, b, side, hint));
    }

    card(ui, "Advanced", |ui| {
        app.yaml_text(ui, "Class name", "The name styles.css uses to style this bar", &path(b, &["class_name"]), 160.0);
    });
}

/// The `screens` list: the monitor names as pills (x removes one, typing adds one).
fn screens(app: &mut App, ui: &mut Ui, b: &str) {
    let p = path(b, &["screens"]);
    let items = app.cfg.list(&p);
    if let Some(new) = row_below(ui, "Screens", "Monitor names. * means every screen. Press x to remove, type to add", |ui| chips(ui, &format!("screens-{b}"), &items, "add a monitor...")) {
        let r = app.cfg.set_list(&p, &new);
        app.done(r);
    }
}

/// One side of the bar: its widgets in order, with move / remove buttons and the ones that can still be added.
fn layout(app: &mut App, ui: &mut Ui, b: &str, side: &str, hint: &str) {
    let p = path(b, &["widgets", side]);
    let items = app.cfg.list(&p);
    let all = app.cfg.keys(&["widgets"]);
    let mut change: Option<Vec<String>> = None;
    small(ui, hint);
    ui.add_space(4.0);
    for (i, name) in items.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.set_height(30.0);
            text(ui, name, 14.0, TEXT);
            let kind = app.cfg.string(&["widgets", name, "type"]);
            if !kind.is_empty() {
                small(ui, kind.rsplit('.').next().unwrap_or(&kind).to_string());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if pill(ui, "remove", false).clicked() {
                    let mut v = items.clone();
                    v.remove(i);
                    change = Some(v);
                }
                if i + 1 < items.len() && pill(ui, "down", false).clicked() {
                    let mut v = items.clone();
                    v.swap(i, i + 1);
                    change = Some(v);
                }
                if i > 0 && pill(ui, "up", false).clicked() {
                    let mut v = items.clone();
                    v.swap(i, i - 1);
                    change = Some(v);
                }
            });
        });
        divider(ui);
    }
    if items.is_empty() {
        small(ui, "Nothing here yet.");
    }
    ui.add_space(8.0);
    small(ui, "ADD");
    ui.horizontal_wrapped(|ui| {
        for w in all.iter().filter(|w| !items.contains(w)) {
            if pill(ui, &format!("+ {w}"), false).clicked() {
                let mut v = items.clone();
                v.push(w.clone());
                change = Some(v);
            }
        }
    });
    ui.add_space(8.0);
    if let Some(v) = change {
        let r = app.cfg.set_list(&p, &v);
        app.done(r);
    }
}
