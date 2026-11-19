//! Small line icons for the sidebar, drawn straight with the painter (no icon font needed).

use eframe::egui::{Color32, CornerRadius, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};

#[derive(Clone, Copy, PartialEq)]
pub enum Icon {
    Home,
    Bar,
    Widgets,
    Komorebi,
    Border,
    Image,
    Theme,
    About,
    Collapse,
    Expand,
}

/// Draw `icon` centred on `c`, about 20 px across.
pub fn paint(p: &Painter, icon: Icon, c: Pos2, colour: Color32) {
    let line = Stroke::new(1.6, colour);
    match icon {
        Icon::Home => {
            // a house: roof and walls
            let (l, r, top, bottom) = (c.x - 9.0, c.x + 9.0, c.y - 9.0, c.y + 8.0);
            p.line_segment([Pos2::new(l - 1.0, c.y - 1.0), Pos2::new(c.x, top)], line);
            p.line_segment([Pos2::new(c.x, top), Pos2::new(r + 1.0, c.y - 1.0)], line);
            p.line_segment([Pos2::new(l + 2.0, c.y - 2.0), Pos2::new(l + 2.0, bottom)], line);
            p.line_segment([Pos2::new(r - 2.0, c.y - 2.0), Pos2::new(r - 2.0, bottom)], line);
            p.line_segment([Pos2::new(l + 2.0, bottom), Pos2::new(r - 2.0, bottom)], line);
            p.circle_filled(Pos2::new(c.x, c.y + 3.0), 1.5, colour);
        }
        Icon::Bar => {
            // a rounded bar with three dots in it: left, centre, right
            let r = Rect::from_center_size(c, Vec2::new(22.0, 12.0));
            p.rect_stroke(r, CornerRadius::same(6), line, StrokeKind::Inside);
            for dx in [-6.0, 0.0, 6.0] {
                p.circle_filled(Pos2::new(c.x + dx, c.y), 1.7, colour);
            }
        }
        Icon::Widgets => {
            // four tiles
            for (dx, dy) in [(-5.0, -5.0), (5.0, -5.0), (-5.0, 5.0), (5.0, 5.0)] {
                let r = Rect::from_center_size(Pos2::new(c.x + dx, c.y + dy), Vec2::splat(8.0));
                p.rect_stroke(r, CornerRadius::same(2), line, StrokeKind::Inside);
            }
        }
        Icon::Komorebi => {
            // a tiled window: one big pane and two small ones
            let r = Rect::from_center_size(c, Vec2::new(20.0, 16.0));
            p.rect_stroke(r, CornerRadius::same(3), line, StrokeKind::Inside);
            p.line_segment([Pos2::new(c.x, r.min.y), Pos2::new(c.x, r.max.y)], line);
            p.line_segment([Pos2::new(c.x, c.y), Pos2::new(r.max.x, c.y)], line);
        }
        Icon::Border => {
            // a window with a thick outline
            let r = Rect::from_center_size(c, Vec2::new(21.0, 17.0));
            p.rect_stroke(r, CornerRadius::same(5), Stroke::new(2.4, colour), StrokeKind::Inside);
            p.rect_stroke(r.shrink(5.0), CornerRadius::same(2), Stroke::new(1.2, colour.gamma_multiply(0.6)), StrokeKind::Inside);
        }
        Icon::Image => {
            // a picture: frame, sun and hills
            let r = Rect::from_center_size(c, Vec2::new(20.0, 16.0));
            p.rect_stroke(r, CornerRadius::same(3), line, StrokeKind::Inside);
            p.circle_filled(Pos2::new(r.min.x + 6.0, r.min.y + 5.0), 1.6, colour);
            p.line_segment([Pos2::new(r.min.x + 1.5, r.max.y - 2.0), Pos2::new(r.min.x + 8.0, r.max.y - 8.0)], line);
            p.line_segment([Pos2::new(r.min.x + 8.0, r.max.y - 8.0), Pos2::new(r.max.x - 1.5, r.max.y - 2.0)], line);
        }
        Icon::Theme => {
            // two overlapping rings: colours mixing
            p.circle_stroke(Pos2::new(c.x - 3.5, c.y - 1.5), 6.5, line);
            p.circle_stroke(Pos2::new(c.x + 3.5, c.y - 1.5), 6.5, line);
            p.circle_stroke(Pos2::new(c.x, c.y + 4.0), 6.5, line);
        }
        Icon::About => {
            p.circle_stroke(c, 9.0, line);
            p.circle_filled(Pos2::new(c.x, c.y - 4.0), 1.3, colour);
            p.line_segment([Pos2::new(c.x, c.y - 0.5), Pos2::new(c.x, c.y + 5.0)], line);
        }
        Icon::Collapse | Icon::Expand => {
            let s = if icon == Icon::Collapse { -1.0 } else { 1.0 };
            for dx in [-3.0, 3.0] {
                p.line_segment([Pos2::new(c.x + dx - s * 2.5, c.y - 5.0), Pos2::new(c.x + dx + s * 2.5, c.y)], line);
                p.line_segment([Pos2::new(c.x + dx + s * 2.5, c.y), Pos2::new(c.x + dx - s * 2.5, c.y + 5.0)], line);
            }
        }
    }
}

/// A small monochrome mark for each program, drawn with lines like the sidebar icons.
pub fn paint_program(p: &Painter, id: &str, c: Pos2, colour: Color32) {
    let line = Stroke::new(1.7, colour);
    let pt = |x: f32, y: f32| Pos2::new(c.x + x, c.y + y);
    match id {
        "obs" => {
            // a disc with three crescents turning round it
            p.circle_stroke(c, 10.0, line);
            for k in 0..3 {
                let a = std::f32::consts::TAU * k as f32 / 3.0 - 1.57;
                let from = a - 0.6;
                let arc: Vec<Pos2> = (0..=6).map(|i| { let t = from + 1.2 * i as f32 / 6.0; pt(t.cos() * 5.0, t.sin() * 5.0) }).collect();
                p.add(eframe::egui::Shape::line(arc, Stroke::new(1.6, colour)));
            }
        }
        "spicetify" => {
            // a disc with three curved bars across it
            p.circle_stroke(c, 10.0, line);
            for (i, y) in [-4.5f32, 0.0, 4.5].iter().enumerate() {
                let half = 6.5 - i as f32 * 1.0;
                let arc: Vec<Pos2> = (0..=8).map(|k| { let x = -half + 2.0 * half * k as f32 / 8.0; pt(x, *y - (x / half * 1.6).powi(2) * -0.0 + (1.0 - (x / half).powi(2)) * -1.6) }).collect();
                p.add(eframe::egui::Shape::line(arc, Stroke::new(1.5, colour)));
            }
        }
        "mono" => {
            // a circle that is half filled
            p.circle_stroke(c, 9.0, line);
            let half: Vec<Pos2> = (0..=12).map(|i| { let a = (-90.0 + 15.0 * i as f32).to_radians(); pt(a.cos() * 9.0, a.sin() * 9.0) }).collect();
            let mut fill = half;
            fill.push(pt(0.0, 9.0));
            p.add(eframe::egui::Shape::convex_polygon(fill, colour.gamma_multiply(0.85), Stroke::NONE));
        }
        "lucent" => {
            // colours mixing: three overlapping rings
            p.circle_stroke(pt(-3.5, -3.0), 6.5, line);
            p.circle_stroke(pt(3.5, -3.0), 6.5, line);
            p.circle_stroke(pt(0.0, 3.0), 6.5, line);
        }
        "zed" => {
            // a Z
            for (a, b) in [((-7.0, -7.0), (7.0, -7.0)), ((7.0, -7.0), (-7.0, 7.0)), ((-7.0, 7.0), (7.0, 7.0))] {
                p.line_segment([pt(a.0, a.1), pt(b.0, b.1)], line);
            }
        }
        "discord" => {
            // a speech bubble with two eyes
            p.rect_stroke(Rect::from_center_size(pt(0.0, -1.0), Vec2::new(22.0, 15.0)), CornerRadius::same(6), line, StrokeKind::Inside);
            p.line_segment([pt(-4.0, 6.5), pt(-6.5, 10.0)], line);
            p.circle_filled(pt(-4.0, -1.0), 1.7, colour);
            p.circle_filled(pt(4.0, -1.0), 1.7, colour);
        }
        "zen" => {
            // a ring with a wave through it
            p.circle_stroke(c, 10.0, line);
            let wave: Vec<Pos2> = (0..=16).map(|i| pt(-9.0 + i as f32, (i as f32 * 0.55).sin() * 3.0)).collect();
            p.add(eframe::egui::Shape::line(wave, line));
        }
        "helium" => {
            // a balloon: a round top that narrows into a tail
            p.circle_stroke(pt(0.0, -3.0), 7.0, line);
            p.line_segment([pt(0.0, 4.0), pt(0.0, 10.0)], line);
            p.line_segment([pt(-2.0, 4.0), pt(2.0, 4.0)], line);
        }
        "obsidian" => {
            // a gem
            let pts = vec![pt(0.0, -10.0), pt(8.5, -3.0), pt(5.0, 9.0), pt(-5.0, 9.0), pt(-8.5, -3.0), pt(0.0, -10.0)];
            p.add(eframe::egui::Shape::line(pts, line));
            p.line_segment([pt(-8.5, -3.0), pt(8.5, -3.0)], Stroke::new(1.2, colour.gamma_multiply(0.7)));
        }
        "terminal" => {
            // a window with a prompt
            p.rect_stroke(Rect::from_center_size(c, Vec2::new(22.0, 17.0)), CornerRadius::same(3), line, StrokeKind::Inside);
            p.line_segment([pt(-6.0, -3.0), pt(-2.0, 0.0)], line);
            p.line_segment([pt(-2.0, 0.0), pt(-6.0, 3.0)], line);
            p.line_segment([pt(1.0, 3.0), pt(6.0, 3.0)], line);
        }
        "filepilot" => {
            // a folder
            p.rect_stroke(Rect::from_min_max(pt(-10.0, -5.0), pt(10.0, 8.0)), CornerRadius::same(3), line, StrokeKind::Inside);
            p.line_segment([pt(-10.0, -5.0), pt(-10.0, -8.0)], line);
            p.line_segment([pt(-10.0, -8.0), pt(-3.0, -8.0)], line);
            p.line_segment([pt(-3.0, -8.0), pt(-1.0, -5.0)], line);
        }
        "tacky" => {
            // a window with a thick outline
            p.rect_stroke(Rect::from_center_size(c, Vec2::new(21.0, 17.0)), CornerRadius::same(5), Stroke::new(2.6, colour), StrokeKind::Inside);
        }
        "rainmeter" => {
            // a drop
            p.circle_stroke(pt(0.0, 3.0), 7.0, line);
            p.line_segment([pt(-6.0, -1.0), pt(0.0, -10.0)], line);
            p.line_segment([pt(0.0, -10.0), pt(6.0, -1.0)], line);
        }
        _ => { p.circle_stroke(c, 8.0, line); }
    }
}
