//! Layout: the bar drawn as a bar. Drag a widget to where it should sit (left, centre or right), drag it out to remove it,
//! drag one from the shelf below to add it.

use crate::{theme::*, App};
use eframe::egui::{self, CornerRadius, FontFamily, FontId, Order, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

const SIDES: [(&str, &str); 3] = [("left", "LEFT"), ("center", "CENTRE"), ("right", "RIGHT")];

/// What is being dragged: the widget, and the side it came from (None = from the shelf).
#[derive(Clone)]
pub struct Drag {
    name: String,
    from: Option<usize>,
}

fn chip_width(ui: &Ui, name: &str) -> f32 {
    ui.painter().layout_no_wrap(name.to_string(), FontId::new(12.5, crate::theme::mono()), TEXT).size().x + 26.0
}

fn paint_chip(ui: &Ui, rect: Rect, name: &str, fill: egui::Color32, stroke: egui::Color32, colour: egui::Color32) {
    ui.painter().rect(rect, CornerRadius::same(14), fill, Stroke::new(1.0, stroke), StrokeKind::Inside);
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, name, FontId::new(12.5, crate::theme::mono()), colour);
}

pub fn show(app: &mut App, ui: &mut Ui, bar: &str) {
    let ctx = ui.ctx().clone();
    let mut lists: Vec<Vec<String>> = SIDES.iter().map(|(s, _)| app.cfg.list(&["bars", bar, "widgets", s])).collect();
    let all = app.cfg.keys(&["widgets"]);
    let pointer = ctx.pointer_interact_pos();
    let dragging = app.drag.clone();

    small(ui, "Drag a widget to move it. Drop it on another side, or between two widgets, to place it.");
    ui.add_space(8.0);

    // the bar
    let width = ui.available_width();
    let zone_w = width / 3.0;
    // chips flow onto new lines inside their zone: the bar grows to fit the busiest zone
    let flow = |ui: &Ui, names: &[String]| -> Vec<(usize, f32)> {
        let (mut row, mut x, mut out) = (0usize, 0.0f32, Vec::new());
        for n in names {
            let w = chip_width(ui, n);
            if x > 0.0 && x + w > zone_w - 28.0 {
                row += 1;
                x = 0.0;
            }
            out.push((row, x));
            x += w + 6.0;
        }
        out
    };
    let flows: Vec<Vec<(usize, f32)>> = lists.iter().map(|l| flow(ui, l)).collect();
    let rows = flows.iter().map(|f| f.last().map(|(r, _)| r + 1).unwrap_or(1)).max().unwrap_or(1);
    let (track, _) = ui.allocate_exact_size(Vec2::new(width, 46.0 + rows as f32 * 38.0), Sense::hover());
    let zone = |i: usize| Rect::from_min_size(Pos2::new(track.min.x + zone_w * i as f32, track.min.y), Vec2::new(zone_w, track.height()));
    ui.painter().rect_filled(track, CornerRadius::same(22), BG);
    let over = pointer.and_then(|p| track.contains(p).then(|| (((p.x - track.min.x) / zone_w) as usize).min(2)));
    if dragging.is_some() {
        if let Some(i) = over {
            ui.painter().rect_filled(zone(i).shrink(3.0), CornerRadius::same(18), CARD_HOVER);
        }
    }
    ui.painter().rect_stroke(track, CornerRadius::same(22), Stroke::new(1.0, LINE_STRONG), StrokeKind::Inside);
    for i in 1..3 {
        let x = track.min.x + zone_w * i as f32;
        ui.painter().line_segment([Pos2::new(x, track.min.y + 14.0), Pos2::new(x, track.max.y - 14.0)], Stroke::new(1.0, LINE));
    }

    // the chips, remembered with their places so a drop can find its position
    let mut placed: Vec<(usize, String, Rect)> = Vec::new();
    for (i, (_, label)) in SIDES.iter().enumerate() {
        let z = zone(i);
        ui.painter().text(Pos2::new(z.min.x + 18.0, z.min.y + 14.0), egui::Align2::LEFT_CENTER, label, FontId::new(10.5, crate::theme::mono()), DIM);
        for (n, name) in lists[i].iter().enumerate() {
            let (line, x) = flows[i][n];
            let w = chip_width(ui, name);
            let rect = Rect::from_min_size(Pos2::new(z.min.x + 14.0 + x, z.min.y + 34.0 + line as f32 * 38.0), Vec2::new(w, 30.0));
            let response = ui.interact(rect, egui::Id::new(("chip", i, name.as_str())), Sense::click_and_drag());
            let lifted = dragging.as_ref().is_some_and(|d| d.name == *name);
            let (fill, stroke, colour) = if lifted { (BG, LINE, DIM) } else if response.hovered() { (CARD_HOVER, WHITE, WHITE) } else { (CARD, LINE_STRONG, TEXT) };
            paint_chip(ui, rect, name, fill, stroke, colour);
            if response.drag_started() {
                app.drag = Some(Drag { name: name.clone(), from: Some(i) });
            }
            placed.push((i, name.clone(), rect));
        }
        if lists[i].is_empty() {
            ui.painter().text(Pos2::new(z.center().x, z.min.y + 50.0), egui::Align2::CENTER_CENTER, "empty", FontId::new(12.0, crate::theme::mono()), DIM);
        }
    }

    // the shelf: widgets that are not on this bar
    ui.add_space(14.0);
    small(ui, "NOT ON THIS BAR  -  drag one up into the bar");
    ui.add_space(4.0);
    let on_bar: Vec<&String> = lists.iter().flatten().collect();
    let shelf: Vec<String> = all.iter().filter(|w| !on_bar.contains(w)).cloned().collect();
    ui.horizontal_wrapped(|ui| {
        for name in &shelf {
            let (rect, response) = ui.allocate_exact_size(Vec2::new(chip_width(ui, name), 30.0), Sense::click_and_drag());
            let lifted = dragging.as_ref().is_some_and(|d| d.name == *name);
            let (fill, stroke, colour) = if lifted { (BG, LINE, DIM) } else if response.hovered() { (CARD_HOVER, WHITE, WHITE) } else { (BG, LINE_STRONG, MUTED) };
            paint_chip(ui, rect, name, fill, stroke, colour);
            if response.drag_started() {
                app.drag = Some(Drag { name: name.clone(), from: None });
            }
        }
        if shelf.is_empty() {
            small(ui, "Every widget is on this bar.");
        }
    });

    // while dragging: the widget follows the pointer
    if let (Some(d), Some(p)) = (&app.drag, pointer) {
        ctx.request_repaint();
        let painter = ctx.layer_painter(egui::LayerId::new(Order::Tooltip, egui::Id::new("drag-ghost")));
        let rect = Rect::from_center_size(p, Vec2::new(chip_width(ui, &d.name), 30.0));
        painter.rect(rect, CornerRadius::same(14), WHITE, Stroke::NONE, StrokeKind::Inside);
        painter.text(rect.center(), egui::Align2::CENTER_CENTER, &d.name, FontId::new(12.5, FontFamily::Name("mono".into())), BG);
        if d.from.is_some() && over.is_none() {
            ui.add_space(8.0);
            small(ui, "Let go outside the bar to take it off");
        }
    }

    // the drop
    if app.drag.is_some() && ctx.input(|i| i.pointer.any_released()) {
        let d = app.drag.take().unwrap();
        let mut changed = vec![false; 3];
        if let Some(from) = d.from {
            if over.is_none() && pointer.is_some() {
                lists[from].retain(|n| *n != d.name); // let go outside the bar: take it off
                changed[from] = true;
            }
        }
        if let (Some(side), Some(p)) = (over, pointer) {
            let before = placed.iter().filter(|(s, n, r)| *s == side && *n != d.name && r.center().x < p.x).count();
            if let Some(from) = d.from {
                lists[from].retain(|n| *n != d.name);
                changed[from] = true;
            }
            let at = before.min(lists[side].len());
            lists[side].insert(at, d.name.clone());
            changed[side] = true;
        }
        for (i, (side, _)) in SIDES.iter().enumerate() {
            if changed[i] {
                let r = app.cfg.set_list(&["bars", bar, "widgets", side], &lists[i]);
                app.done(r);
            }
        }
    }
}
