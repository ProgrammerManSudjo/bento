//! Templates: the Lucent theme, which writes the Windows accent colour into other programs' theme files.
//! It happens when you press Apply now, or right after you pick a wallpaper on the Wallpaper page (while the theme is on). Never in the background.

use crate::{lucent, theme::*, App};
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

pub fn show(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let pal = lucent::load_with(app.staged.strength());

    // the same switch tile as on the home page
    let on = app.staged.lucent_on();
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 150.0), Sense::hover());
    if crate::home::switch_tile(ui, area, "LUCENT THEME", "Your programs take the accent. Updated when you apply a new wallpaper", on) {
        crate::sound::play(if !on { crate::sound::Sound::On } else { crate::sound::Sound::Off });
        app.staged.set_lucent(!on);
    }
    ui.add_space(8.0);
    let theme_id = egui::Id::new("lucent-settings-open");
    register_section(ui, theme_id, "Lucent theme", false);
    let mut theme_open = ui.data_mut(|d| d.get_temp::<bool>(theme_id)).unwrap_or(false);
    if program_header(ui, "lucent", "Lucent theme", "Settings".to_string(), app.staged.lucent_on(), theme_open) {
        theme_open = !theme_open;
        ui.data_mut(|d| d.insert_temp(theme_id, theme_open));
    }
    if theme_open {
        card(ui, "", |ui| {
            // how much of Windows' saturation the colours keep: lower is softer (waits for Apply)
            let mut keep = app.staged.strength();
            row(ui, "Softness", "How much of the accent's colour is kept. Lower is softer and greyer. Press Apply to write it into the programs", |ui| slider(ui, &mut keep, 0, 100, " %"));
            if keep != app.staged.strength() {
                app.staged.set_strength(keep);
            }
            let mut effect = app.staged.effect();
            row(ui, "Lucent effect", "How much brighter the bar and taskbar capsules get: 1% is as bright as now, 100% is 35% brighter. Press Apply", |ui| slider(ui, &mut effect, 1, 100, " %"));
            if effect != app.staged.effect() {
                app.staged.set_effect(effect);
            }
            // the global accent: Windows' own, or one picked here
            let windows = lucent::load_for(app.staged.strength(), "");
            let current = app.staged.get("lucent_accent");
            let mut options = variants("Accent", &windows);
            options.extend(lucent::PRESETS.iter().map(|(n, c)| (*n, c.to_string(), lucent::rgb_of(c).unwrap_or([128; 3]))));
            if let Some(v) = swatch_row(ui, "Accent", "The colour every program follows, unless it has its own. The first one is the accent Windows picked", &current, &options) {
                app.staged.set("lucent_accent", &v);
            }
            let _ = &pal;
            row(ui, "Apply now", "Write the colour into the programs switched on below", |ui| {
                if button(ui, "Apply now", true).clicked() {
                    crate::spawn(app, &ctx, "Applying the theme...", || {
                        let results = lucent::apply_all();
                        lucent::remember(&results);
                        lucent::summary(&results)
                    });
                }
            });
        });
    }
    ui.add_space(8.0);

    if !app.staged.lucent_on() {
        mono_card(app, ui);
    }
    for (index, a) in lucent::APPS.iter().enumerate() {
        let open_id = egui::Id::new(("program-open", a.id));
        register_section(ui, open_id, a.name, false);
        let mut open = ui.data_mut(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
        if program_header(ui, a.id, a.name, format!("{:02} / {:02}", index + 1, lucent::APPS.len()), lucent::enabled(a.id), open) {
            open = !open;
            ui.data_mut(|d| d.insert_temp(open_id, open));
        }
        if !open {
            ui.add_space(8.0);
            continue;
        }
        card(ui, "", |ui| {
            let mut on = lucent::enabled(a.id);
            let note = lucent::last(a.id);
            let changed = row(ui, "Theme this program", a.hint, |ui| {
                let t = toggle(ui, &mut on);
                if let Some(n) = note {
                    small(ui, n);
                }
                t
            }).changed();
            if changed {
                lucent::set_enabled(a.id, on);
            }
            let (mode_key, accent_key, mono_key) = (format!("app_{}_mode", a.id), format!("app_{}_accent", a.id), format!("app_{}_mono", a.id));
            let mut follow = app.staged.get(&mode_key) != "mono";
            if row(ui, "Follow Lucent", "On: the accent colours this program. Off: it stays monochrome", |ui| toggle(ui, &mut follow)).changed() {
                app.staged.set(&mode_key, if follow { "" } else { "mono" });
            }
            // with the whole theme off every program is monochrome, whatever its own switch says
            let accent_now = follow && app.staged.lucent_on();
            ui.add_enabled_ui(accent_now, |ui| {
                let global = lucent::load_for(app.staged.strength(), &app.staged.get("lucent_accent"));
                let mut options = variants("Accent", &global);
                options.extend(lucent::PRESETS.iter().map(|(n, c)| (*n, c.to_string(), lucent::rgb_of(c).unwrap_or([128; 3]))));
                let current = app.staged.get(&accent_key);
                if let Some(v) = swatch_row(ui, "Accent", "This program's own accent colour", &current, &options) {
                    app.staged.set(&accent_key, &v);
                }
            });
            ui.add_enabled_ui(!accent_now, |ui| {
                let options: Vec<(&str, String, [u8; 3])> = lucent::TONES.iter().map(|(n, c)| (*n, if *n == "Grey" { String::new() } else { c.to_string() }, lucent::rgb_of(c).unwrap_or([128; 3]))).collect();
                let current = app.staged.get(&mono_key);
                if let Some(v) = swatch_row(ui, "Monochrome", "The grey this program uses while it is monochrome", &current, &options) {
                    app.staged.set(&mono_key, &v);
                }
            });
        });
    }
    advanced(app, ui);
    let _ = egui::Id::NULL;
}

/// The dark capsules (the bar's tiles, the taskbar's buttons): each can take the accent, as dark as before, on its own or together.
fn advanced(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        dots(ui);
        ui.label(egui::RichText::new("ADVANCED").font(egui::FontId::new(12.5, heading())).color(MUTED));
    });
    small(ui, "The capsules, the text and the highlights take the accent, and stay dark. The bar and the taskbar, separately or together. Changes here wait for Apply.");
    ui.add_space(6.0);
    let (gap, height) = (12.0, 176.0);
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let width = (area.width() - gap) / 2.0;
    for (i, (key, name, hint)) in [("yasb", "YASB BAR", "Capsules of the bar"), ("windhawk", "WINDHAWK", "Taskbar buttons")].into_iter().enumerate() {
        let rect = Rect::from_min_size(Pos2::new(area.min.x + i as f32 * (width + gap), area.min.y), Vec2::new(width, height));
        let on = app.staged.capsule_on(key);
        let clicked = crate::home::switch_tile(ui, rect, name, hint, on);
        // what it would look like: resting, active and clock
        let shown = lucent::capsule_with(key, true, app.staged.strength(), app.staged.effect());
        for (n, c) in [&shown.rest, &shown.active, &shown.clock].into_iter().enumerate() {
            let [r, g, b] = [1, 3, 5].map(|k| u8::from_str_radix(&c[k..k + 2], 16).unwrap_or(0));
            let swatch = Rect::from_min_size(Pos2::new(rect.min.x + 20.0 + n as f32 * 30.0, rect.min.y + 74.0), Vec2::splat(24.0));
            ui.painter().rect(swatch, CornerRadius::same(8), Color32::from_rgb(r, g, b), Stroke::new(1.0, LINE_STRONG), StrokeKind::Inside);
        }
        if clicked {
            crate::sound::play(if !on { crate::sound::Sound::On } else { crate::sound::Sound::Off });
            app.staged.set_capsule(key, !on);
        }
    }
    // a colour of their own for each, and the button to re-apply the taskbar (only while Windhawk is on)
    let (yasb_on, windhawk_on) = (app.staged.capsule_on("yasb"), app.staged.capsule_on("windhawk"));
    if yasb_on || windhawk_on {
        ui.add_space(8.0);
        card(ui, "", |ui| {
            let global = lucent::load_for(app.staged.strength(), &app.staged.get("lucent_accent"));
            for (key, shown, name, hint) in [("yasb", yasb_on, "Bar accent", "The colour of the bar's capsules. Press Apply"), ("windhawk", windhawk_on, "Taskbar accent", "The colour of the taskbar's buttons. Press Apply")] {
                if !shown {
                    continue;
                }
                // with the theme on a colour of its own; with it off a grey of its own
                let (setting, options, name) = if app.staged.lucent_on() {
                    let mut options = variants("Accent", &global);
                    options.extend(lucent::PRESETS.iter().map(|(n, c)| (*n, c.to_string(), lucent::rgb_of(c).unwrap_or([128; 3]))));
                    (format!("{key}_accent"), options, name.to_string())
                } else {
                    let options: Vec<(&str, String, [u8; 3])> = lucent::TONES.iter().map(|(n, c)| (*n, if *n == "Grey" { String::new() } else { c.to_string() }, lucent::rgb_of(c).unwrap_or([128; 3]))).collect();
                    (format!("{key}_mono"), options, name.replace("accent", "monochrome"))
                };
                let current = app.staged.get(&setting);
                if let Some(v) = swatch_row(ui, &name, hint, &current, &options) {
                    app.staged.set(&setting, &v);
                }
            }
            if windhawk_on {
                let press = row(ui, "Taskbar", "Re-apply the taskbar's colours now (Windhawk asks for administrator permission each time)", |ui| button(ui, "Apply to taskbar", false).clicked());
                if press {
                    crate::spawn(app, &ctx, "Updating the taskbar: allow the administrator prompt", || lucent::windhawk_capsule(true));
                }
            }
        });
    }
}

/// A colour setting: one button showing the colour; it opens a small picker with the presets and a box for a hex code.
/// The options are (name, value, shown colour); the value "" is the default. Returns the value that was picked.
fn swatch_row(ui: &mut Ui, name: &str, hint: &str, current: &str, options: &[(&str, String, [u8; 3])]) -> Option<String> {
    let mut picked = None;
    let found = options.iter().find(|(_, v, _)| v.eq_ignore_ascii_case(current));
    let (label, [r, g, b]) = match (found, lucent::rgb_of(current)) {
        (Some((n, _, c)), _) => (n.to_string(), *c),
        (None, Some(c)) => (current.to_lowercase(), c),
        _ => (options[0].0.to_string(), options[0].2),
    };
    row(ui, name, hint, |ui| {
        let galley = ui.painter().layout_no_wrap(label, egui::FontId::new(12.5, mono()), TEXT);
        let (rect, response) = ui.allocate_exact_size(Vec2::new(galley.size().x + 92.0, 30.0), Sense::click());
        let hover = ease(ui, response.id.with("hover"), response.hovered());
        ui.painter().rect(rect, CornerRadius::same(15), blend(CARD_HOVER, Color32::from_gray(40), hover), Stroke::new(1.0, blend(LINE_STRONG, MUTED, hover)), StrokeKind::Inside);
        let dot = Pos2::new(rect.min.x + 16.0, rect.center().y);
        ui.painter().circle_filled(dot, 7.0, Color32::from_rgb(r, g, b));
        ui.painter().circle_stroke(dot, 7.0, Stroke::new(1.0, LINE_STRONG));
        ui.painter().galley(Pos2::new(rect.min.x + 31.0, rect.center().y - galley.size().y / 2.0), galley, TEXT);
        // a divider and a plus: this opens a picker
        let sep = rect.max.x - 34.0;
        ui.painter().line_segment([Pos2::new(sep, rect.min.y + 7.0), Pos2::new(sep, rect.max.y - 7.0)], Stroke::new(1.0, LINE_STRONG));
        let pc = Pos2::new(rect.max.x - 17.0, rect.center().y);
        let arm = Stroke::new(1.6, blend(MUTED, WHITE, hover));
        ui.painter().line_segment([Pos2::new(pc.x - 5.0, pc.y), Pos2::new(pc.x + 5.0, pc.y)], arm);
        ui.painter().line_segment([Pos2::new(pc.x, pc.y - 5.0), Pos2::new(pc.x, pc.y + 5.0)], arm);
        egui::Popup::from_toggle_button_response(&response).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).frame(egui::Frame::popup(ui.style()).inner_margin(egui::Margin::same(10))).show(|ui| {
            let mono = name.to_lowercase().contains("monochrome");
            ui.set_min_width(250.0);
            ui.set_max_width(250.0);
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 12.0);
            // what is being edited: hue, saturation and lightness (kept while dragging, so a grey does not lose its hue)
            let shown = format!("#{r:02x}{g:02x}{b:02x}");
            let state_id = egui::Id::new(("picker-state", name, hint));
            let (mut h, mut sat, mut l) = match ui.data_mut(|d| d.get_temp::<(String, f32, f32, f32)>(state_id)) {
                Some((k, h, s, l)) if k == shown => (h, s, l),
                _ => lucent::hex_to_hsl(&shown).unwrap_or((0.07, 0.4, 0.6)),
            };
            if mono {
                l = 0.47;
            }

            // the field: drag the dot. Across is the hue; for an accent, up and down is how light it is
            let (field, drag) = ui.allocate_exact_size(Vec2::splat(ui.available_width()), Sense::click_and_drag());
            let mut moved = false;
            if let (true, Some(p)) = (drag.is_pointer_button_down_on() || drag.clicked(), ui.input(|i| i.pointer.interact_pos())) {
                h = ((p.x - field.min.x - 4.0) / (field.width() - 8.0)).clamp(0.0, 1.0);
                let up = ((p.y - field.min.y - 4.0) / (field.height() - 8.0)).clamp(0.0, 1.0);
                if mono {
                    sat = (1.0 - up) * 0.4; // up is more tint
                } else {
                    l = 0.92 - up * 0.80;
                }
                moved = true;
            }
            let painter = ui.painter();
            painter.rect_filled(field, CornerRadius::same(3), Color32::from_gray(16));
            let step = 10.0;
            let (cols, rows) = (((field.width() - 8.0) / step) as i32, ((field.height() - 8.0) / step) as i32);
            for i in 0..=cols {
                for j in 0..=rows {
                    let (fx, fy) = (i as f32 / cols as f32, j as f32 / rows.max(1) as f32);
                    let [cr, cg, cb] = lucent::rgb_of(&lucent::hsl_to_hex(fx, if mono { (1.0 - fy) * 0.4 } else { sat.max(0.12) }, if mono { 0.5 } else { 0.92 - fy * 0.80 })).unwrap_or([128; 3]);
                    painter.circle_filled(Pos2::new(field.min.x + 4.0 + i as f32 * (field.width() - 8.0) / cols as f32, field.min.y + 4.0 + j as f32 * (field.height() - 8.0) / rows.max(1) as f32), 1.5, Color32::from_rgb(cr, cg, cb).gamma_multiply(0.55));
                }
            }
            let at = Pos2::new(field.min.x + 4.0 + h.clamp(0.0, 1.0) * (field.width() - 8.0), if mono { field.min.y + 4.0 + (1.0 - (sat / 0.4).clamp(0.0, 1.0)) * (field.height() - 8.0) } else { field.min.y + 4.0 + ((0.92 - l) / 0.80).clamp(0.0, 1.0) * (field.height() - 8.0) });
            let [cr, cg, cb] = lucent::rgb_of(&lucent::hsl_to_hex(h, sat, l)).unwrap_or([128; 3]);
            painter.circle_filled(at, 9.0, Color32::from_rgb(cr, cg, cb));
            painter.circle_stroke(at, 9.0, Stroke::new(2.0, WHITE));

            let mut amount = (sat * if mono { 250.0 } else { 100.0 }).round() as i64;

            // the presets: a strip with arrows to the sides, five at a time and the knob for how strong it is
            let page_id = state_id.with("page");
            let pages = options.len().div_ceil(5);
            let mut page = ui.data_mut(|d| d.get_temp::<usize>(page_id)).unwrap_or(0).min(pages - 1);
            ui.horizontal(|ui| {
                ui.set_min_height(40.0);
                ui.spacing_mut().item_spacing.x = 8.0;
                let arrow = |ui: &mut Ui, left: bool, live: bool| -> bool {
                    let (rect, response) = ui.allocate_exact_size(Vec2::new(14.0, 26.0), Sense::click());
                    let c = rect.center();
                    let d = if left { -1.0 } else { 1.0 };
                    let col = if !live { Color32::from_gray(50) } else if response.hovered() { WHITE } else { MUTED };
                    ui.painter().add(egui::Shape::line(vec![Pos2::new(c.x - 2.5 * d, c.y - 5.0), Pos2::new(c.x + 2.5 * d, c.y), Pos2::new(c.x - 2.5 * d, c.y + 5.0)], Stroke::new(1.6, col)));
                    live && response.clicked()
                };
                if arrow(ui, true, page > 0) {
                    page -= 1;
                }
                for (label, value, [r, g, b]) in options.iter().skip(page * 5).take(5) {
                    let (rect, response) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
                    let chosen = value.eq_ignore_ascii_case(current);
                    ui.painter().circle_filled(rect.center(), 11.0, Color32::from_rgb(*r, *g, *b));
                    ui.painter().circle_stroke(rect.center(), 11.0, Stroke::new(if chosen { 2.2 } else { 1.0 }, if chosen { WHITE } else if response.hovered() { MUTED } else { LINE_STRONG }));
                    if value.is_empty() {
                        ui.painter().circle_filled(rect.center(), 2.2, if (*r as u32 + *g as u32 + *b as u32) > 380 { BG } else { WHITE });
                    }
                    if response.on_hover_text(*label).clicked() {
                        crate::sound::play(crate::sound::Sound::Click);
                        picked = Some(value.clone());
                    }
                }
                let shown = options.len().saturating_sub(page * 5).min(5);
                for _ in shown..5 {
                    ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
                }
                if arrow(ui, false, page + 1 < pages) {
                    page += 1;
                }
                // the knob: drag up or down to turn it (how strong the colour is)
                let (dial, turn) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::drag());
                if turn.dragged() {
                    amount = (amount as f32 - turn.drag_delta().y * 0.8).clamp(0.0, 100.0).round() as i64;
                }
                let t = amount as f32 / 100.0;
                let angle = (-225.0 + 270.0 * t).to_radians();
                for i in 0..=10 {
                    let a = (-225.0f32 + 27.0 * i as f32).to_radians();
                    let on = i as f32 / 10.0 <= t + 0.001;
                    ui.painter().circle_filled(dial.center() + Vec2::new(a.cos(), a.sin()) * 18.5, 1.2, if on { TEXT } else { Color32::from_gray(60) });
                }
                ui.painter().circle(dial.center(), 12.5, Color32::from_gray(20), Stroke::new(1.0, if turn.hovered() || turn.dragged() { MUTED } else { LINE_STRONG }));
                ui.painter().line_segment([dial.center() + Vec2::new(angle.cos(), angle.sin()) * 5.0, dial.center() + Vec2::new(angle.cos(), angle.sin()) * 10.0], Stroke::new(2.0, WHITE));
                turn.on_hover_text(format!("{} {amount} %", if mono { "Tint" } else { "Softness" }));
            });
            let new_sat = amount as f32 / if mono { 250.0 } else { 100.0 };
            if (new_sat - sat).abs() > 0.0005 && !moved {
                sat = new_sat;
                moved = true;
            }
            ui.data_mut(|d| d.insert_temp(page_id, page));
            if moved {
                let out = lucent::hsl_to_hex(h, sat, l);
                ui.data_mut(|d| d.insert_temp(state_id, (out.clone(), h, sat, l)));
                picked = Some(out);
            }
        });
    });
    picked
}

/// A small label and its control on one line, for inside the pickers.
fn row_inline<R>(ui: &mut Ui, label: &str, control: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        small(ui, label);
        control(ui)
    })
    .inner
}

/// The first page of presets: the accent itself ("" is the default) and four shades of it.
fn variants(first: &'static str, pal: &lucent::Palette) -> Vec<(&'static str, String, [u8; 3])> {
    let shade = |n: &str| pal.shade(n).unwrap_or([128; 3]);
    let hex = |c: [u8; 3]| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
    vec![
        (first, String::new(), shade("acc")),
        ("Lighter", hex(shade("l2")), shade("l2")),
        ("Light", hex(shade("l1")), shade("l1")),
        ("Dark", hex(shade("d1")), shade("d1")),
        ("Darker", hex(shade("d2")), shade("d2")),
    ]
}

/// The header of a program: its own icon, a small line above, its name big, whether it is on, and an arrow.
fn program_header(ui: &mut Ui, id: &str, name: &str, tag: String, on: bool, open: bool) -> bool {
    section_header(ui, Mark::Program(id), name, &tag, Some(on), open)
}

/// While the theme is off: the grey every program uses, how bright and how tinted it is. A program can have a grey of its own below.
fn mono_card(app: &mut App, ui: &mut Ui) {
    let open_id = egui::Id::new("mono-settings-open");
    register_section(ui, open_id, "Monochrome theme", true);
    let mut open = ui.data_mut(|d| d.get_temp::<bool>(open_id)).unwrap_or(true);
    if section_header(ui, Mark::Program("mono"), "Monochrome theme", "Settings", Some(true), open) {
        open = !open;
        ui.data_mut(|d| d.insert_temp(open_id, open));
    }
    if open {
        card(ui, "", |ui| {
            let get = |app: &App, key: &str, default: i64| app.staged.get(key).parse::<i64>().unwrap_or(default);
            let mut bright = get(app, "mono_brightness", 50);
            row(ui, "Brightness", "How light the greys are. 50 % is as it was", |ui| slider(ui, &mut bright, 0, 100, " %"));
            if bright != get(app, "mono_brightness", 50) {
                app.staged.set("mono_brightness", &bright.to_string());
            }
            let mut tint = get(app, "mono_tint", 100);
            row(ui, "Smoothness", "How much of the colour of the tone below is kept. 0 % is plain grey", |ui| slider(ui, &mut tint, 0, 100, " %"));
            if tint != get(app, "mono_tint", 100) {
                app.staged.set("mono_tint", &tint.to_string());
            }
            let options: Vec<(&str, String, [u8; 3])> = lucent::TONES.iter().map(|(n, c)| (*n, if *n == "Grey" { String::new() } else { c.to_string() }, lucent::rgb_of(c).unwrap_or([128; 3]))).collect();
            let current = app.staged.get("mono_tone");
            if let Some(v) = swatch_row(ui, "Monochrome", "The grey every program uses while the theme is off", &current, &options) {
                app.staged.set("mono_tone", &v);
            }
        });
    }
    ui.add_space(8.0);
}
