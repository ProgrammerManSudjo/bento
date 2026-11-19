//! Home: one screen of cards in a grid, like widgets on a phone. A live clock over rippling dots, the seconds as a ring of dots,
//! a month calendar, the hours of the day as a dot matrix, and a few numbers about the setup. Black, white and grey only.

use crate::{theme::*, App};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontFamily, FontId, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::SystemInformation::{GetLocalTime, GetTickCount64};

const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const GAP: f32 = 12.0;

fn now() -> SYSTEMTIME {
    let mut t = unsafe { std::mem::zeroed::<SYSTEMTIME>() };
    unsafe { GetLocalTime(&mut t) };
    t
}

fn uptime_seconds() -> u64 {
    (unsafe { GetTickCount64() }) / 1000
}

fn uptime() -> String {
    let secs = uptime_seconds();
    let (d, h, m) = (secs / 86400, secs % 86400 / 3600, secs % 3600 / 60);
    if d > 0 { format!("{d}d {h}h") } else { format!("{h}h {m}m") }
}

fn days_in_month(year: u16, month: u16) -> u16 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn username() -> String {
    std::env::var("USERNAME").unwrap_or_else(|_| "there".into())
}

/// A rounded card; its outline brightens under the pointer.
fn tile(ui: &mut Ui, rect: Rect, id: &str) -> Painter {
    let response = ui.interact(rect, egui::Id::new(("tile", id)), Sense::hover());
    let stroke = if response.hovered() { Stroke::new(1.0, LINE_STRONG) } else { Stroke::new(1.0, LINE) };
    ui.painter().rect(rect, CornerRadius::same(26), CARD, stroke, StrokeKind::Inside);
    ui.painter_at(rect)
}

fn label(p: &Painter, rect: Rect, text: &str) {
    p.text(Pos2::new(rect.min.x + 20.0, rect.min.y + 18.0), Align2::LEFT_TOP, text, FontId::new(11.0, mono()), MUTED);
}

/// A tile with a name, a big On / Off, a hint and a switch; a click anywhere on it flips it. True when it was clicked.
pub fn switch_tile(ui: &mut Ui, rect: Rect, name: &str, hint: &str, on: bool) -> bool {
    let ctx = ui.ctx().clone();
    let p = tile(ui, rect, name);
    label(&p, rect, name);
    p.text(Pos2::new(rect.min.x + 20.0, rect.max.y - 18.0), Align2::LEFT_BOTTOM, if on { "On" } else { "Off" }, FontId::new(44.0, heading()), if on { lit() } else { MUTED });
    p.text(Pos2::new(rect.max.x - 20.0, rect.max.y - 24.0), Align2::RIGHT_BOTTOM, hint, FontId::new(11.5, mono()), DIM);
    // the switch
    let sw = Rect::from_center_size(Pos2::new(rect.max.x - 20.0 - 30.0, rect.min.y + 34.0), Vec2::new(60.0, 32.0));
    p.rect(sw, CornerRadius::same(16), if on { lit() } else { BG }, Stroke::new(1.2, if on { lit() } else { LINE_STRONG }), StrokeKind::Inside);
    let knob_x = ctx.animate_value_with_time(egui::Id::new(("switch", name)), if on { sw.max.x - 16.0 } else { sw.min.x + 16.0 }, 0.12);
    p.circle_filled(Pos2::new(knob_x, sw.center().y), 11.0, if on { BG } else { MUTED });
    let click = ui.interact(rect, egui::Id::new(("switch-click", name)), Sense::click());
    if click.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    click.clicked()
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    ctx.request_repaint(); // animation: every frame, so the motion is smooth
    let t = now();
    let time = ctx.input(|i| i.time) as f32;

    // the whole grid is one screen: three columns, three rows
    let width = ui.available_width();
    let height = (app.page_size.y - 34.0 - 6.0 - 78.0).max(400.0);
    let (area, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let col_w = (width - GAP * 2.0) / 3.0;
    let (h1, h2) = (height * 0.33, height * 0.355);
    let h3 = height - h1 - h2 - GAP * 2.0;
    let col = |i: usize| area.min.x + i as f32 * (col_w + GAP);
    let rows = [area.min.y, area.min.y + h1 + GAP, area.min.y + h1 + h2 + GAP * 2.0];
    let cell = |c: usize, r: usize, spans: usize| Rect::from_min_size(Pos2::new(col(c), rows[r]), Vec2::new(col_w * spans as f32 + GAP * (spans - 1) as f32, [h1, h2, h3][r]));

    clock_card(ui, cell(0, 0, 2), &t, time);
    seconds_card(ui, cell(2, 0, 1), &t, time);
    calendar_card(ui, cell(0, 1, 1), &t);
    hours_card(ui, cell(1, 1, 1), &t, time);
    uptime_card(ui, cell(2, 1, 1), time);

    // two switches side by side: the scroll hotkeys and komorebi. A click anywhere on a tile flips it.
    let status = *app.system.lock().unwrap();
    let switches = cell(0, 2, 3);
    let w = (switches.width() - GAP * 2.0) / 3.0;
    for (i, (name, hint, on)) in [("SCROLL HOTKEYS", "Alt or Win + wheel", status.scroll), ("KOMOREBI", "Window tiling", status.komorebi), ("LUCENT THEME", "Accent in your apps", app.staged.lucent_on())].into_iter().enumerate() {
        let rect = Rect::from_min_size(Pos2::new(switches.min.x + i as f32 * (w + GAP), switches.min.y), Vec2::new(w, switches.height()));
        if switch_tile(ui, rect, name, hint, on) {
            let now_on = !on;
            if i == 2 {
                // waits for Apply, like the other settings
                crate::sound::play(if now_on { crate::sound::Sound::On } else { crate::sound::Sound::Off });
                app.staged.set_lucent(now_on);
            } else if i == 0 {
                crate::sound::play(if now_on { crate::sound::Sound::On } else { crate::sound::Sound::Off });
                crate::system::remember(&app.system, None, Some(now_on));
                crate::spawn(app, &ctx, if now_on { "Starting the scroll hotkeys..." } else { "Stopping the scroll hotkeys..." }, move || crate::system::set_scroll(now_on));
            } else {
                crate::sound::play(if now_on { crate::sound::Sound::On } else { crate::sound::Sound::Off });
                crate::system::remember(&app.system, Some(now_on), None);
                crate::spawn(app, &ctx, if now_on { "Starting komorebi..." } else { "Stopping komorebi..." }, move || crate::system::set_komorebi(now_on));
            }
        }
    }

}

fn clock_card(ui: &mut Ui, rect: Rect, t: &SYSTEMTIME, time: f32) {
    let p = tile(ui, rect, "clock");
    // dots that swell and fade as a wave rolls out from a drifting point
    let pitch = 16.0;
    let origin = Pos2::new(rect.max.x - 90.0 + (time * 0.35).sin() * 50.0, rect.center().y + (time * 0.27).cos() * 30.0);
    for cx in 0..(rect.width() / pitch) as i32 {
        for cy in 0..(rect.height() / pitch) as i32 {
            let at = Pos2::new(rect.min.x + 10.0 + cx as f32 * pitch + pitch / 2.0, rect.min.y + 8.0 + cy as f32 * pitch + pitch / 2.0);
            if !rect.shrink(10.0).contains(at) {
                continue;
            }
            let d = at.distance(origin);
            let wave = (d * 0.045 - time * 1.6).sin() * 0.5 + 0.5;
            let fade = (1.0 - d / (rect.width() * 0.85)).clamp(0.1, 1.0);
            p.circle_filled(at, 0.9 + 1.8 * wave * fade, gradient((at.x - rect.min.x) / rect.width()).gamma_multiply((0.04 + 0.38 * wave * fade).clamp(0.04, 0.45)));
        }
    }
    let greeting = match t.wHour {
        5..=11 => "GOOD MORNING",
        12..=17 => "GOOD AFTERNOON",
        18..=21 => "GOOD EVENING",
        _ => "GOOD NIGHT",
    };
    p.text(Pos2::new(rect.min.x + 26.0, rect.min.y + 22.0), Align2::LEFT_TOP, format!("{greeting}, {}", username().to_uppercase()), FontId::new(11.5, mono()), MUTED);
    let big = (rect.height() * 0.36).min(96.0);
    p.text(Pos2::new(rect.min.x + 22.0, rect.center().y - 4.0), Align2::LEFT_CENTER, format!("{:02}:{:02}", t.wHour, t.wMinute), FontId::new(big, heading()), WHITE);
    p.text(Pos2::new(rect.min.x + 26.0, rect.max.y - 24.0), Align2::LEFT_BOTTOM, format!("{}, {} {}", DAYS[t.wDayOfWeek as usize % 7], t.wDay, MONTHS[(t.wMonth as usize).clamp(1, 12) - 1]), FontId::new(18.0, FontFamily::Proportional), TEXT);
}

fn seconds_card(ui: &mut Ui, rect: Rect, t: &SYSTEMTIME, time: f32) {
    let p = tile(ui, rect, "seconds");
    label(&p, rect, "SECONDS");
    let centre = Pos2::new(rect.center().x, rect.center().y + 10.0);
    let radius = (rect.width().min(rect.height() - 40.0) / 2.0 - 16.0).max(30.0);
    for i in 0..60u16 {
        let angle = (i as f32 / 60.0) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let at = Pos2::new(centre.x + angle.cos() * radius, centre.y + angle.sin() * radius);
        let (r, a) = if i == t.wSecond { (4.2 + (time * 6.0).sin() * 0.8, 1.0) } else if i < t.wSecond { (2.2, 0.85) } else { (1.5, 0.2) };
        p.circle_filled(at, r, glow().gamma_multiply(a));
    }
    p.text(centre, Align2::CENTER_CENTER, format!("{:02}", t.wSecond), FontId::new(radius * 0.7, heading()), WHITE);
}

fn calendar_card(ui: &mut Ui, rect: Rect, t: &SYSTEMTIME) {
    let p = tile(ui, rect, "calendar");
    label(&p, rect, &format!("{} {}", MONTHS[(t.wMonth as usize).clamp(1, 12) - 1].to_uppercase(), t.wYear));
    let first = (((t.wDayOfWeek as i32 + 6) % 7) - (t.wDay as i32 - 1)).rem_euclid(7); // Monday = 0
    let total = days_in_month(t.wYear, t.wMonth) as i32;
    let weeks = (first + total + 6) / 7; // 4 to 6 lines of days
    let area = Rect::from_min_max(Pos2::new(rect.min.x + 14.0, rect.min.y + 46.0), Pos2::new(rect.max.x - 14.0, rect.max.y - 14.0));
    // the letters and the day numbers share one even grid; it is centred in what is left of the card
    let header_gap = 5.0;
    let rows = weeks as f32 + 1.0;
    let ch = ((area.height() - header_gap) / rows).clamp(14.0, 26.0);
    let cw = (area.width() / 7.0).min(ch * 2.1);
    let x0 = area.center().x - cw * 3.5;
    let y0 = area.min.y + ((area.height() - (ch * rows + header_gap)) / 2.0).max(0.0);
    let radius = ch.min(cw) * 0.46;
    for (i, d) in ["M", "T", "W", "T", "F", "S", "S"].iter().enumerate() {
        p.text(Pos2::new(x0 + (i as f32 + 0.5) * cw, y0 + ch * 0.5), Align2::CENTER_CENTER, *d, FontId::new(10.0, mono()), DIM);
    }
    for day in 1..=total {
        let slot = first + day - 1;
        let (c, r) = (slot % 7, slot / 7 + 1);
        let centre = Pos2::new(x0 + (c as f32 + 0.5) * cw, y0 + header_gap + (r as f32 + 0.5) * ch);
        if day == t.wDay as i32 {
            p.circle_filled(centre, radius, WHITE);
            p.text(centre, Align2::CENTER_CENTER, day.to_string(), FontId::new(11.5, heading()), BG);
        } else {
            p.text(centre, Align2::CENTER_CENTER, day.to_string(), FontId::new(11.5, mono()), if day < t.wDay as i32 { MUTED } else { DIM });
        }
    }
}

fn hours_card(ui: &mut Ui, rect: Rect, t: &SYSTEMTIME, time: f32) {
    let p = tile(ui, rect, "hours");
    label(&p, rect, "HOURS");
    let grid = Rect::from_min_max(Pos2::new(rect.min.x + 24.0, rect.min.y + 46.0), Pos2::new(rect.max.x - 24.0, rect.max.y - 40.0));
    let (cw, ch) = (grid.width() / 6.0, grid.height() / 4.0);
    for h in 0..24u16 {
        let centre = Pos2::new(grid.min.x + ((h % 6) as f32 + 0.5) * cw, grid.min.y + ((h / 6) as f32 + 0.5) * ch);
        let (r, a) = if h == t.wHour { (5.5 + (time * 3.0).sin() * 0.9, 1.0) } else if h < t.wHour { (3.6, 0.8) } else { (2.6, 0.2) };
        p.circle_filled(centre, r, glow().gamma_multiply(a));
    }
    p.text(Pos2::new(rect.min.x + 20.0, rect.max.y - 14.0), Align2::LEFT_BOTTOM, format!("{} of 24", t.wHour), FontId::new(12.0, mono()), MUTED);
}

fn uptime_card(ui: &mut Ui, rect: Rect, time: f32) {
    let p = tile(ui, rect, "uptime");
    label(&p, rect, "UPTIME");
    p.text(Pos2::new(rect.min.x + 20.0, rect.center().y + 4.0), Align2::LEFT_CENTER, uptime(), FontId::new((rect.height() * 0.30).min(40.0), heading()), WHITE);
    // a row of dots, one per hour up to a day, then it starts again
    let hours = (uptime_seconds() / 3600) as usize;
    let n = 24;
    for i in 0..n {
        let x = rect.min.x + 22.0 + i as f32 * ((rect.width() - 44.0) / (n - 1) as f32);
        let lit = i < hours % 24 || hours >= 24;
        let breathing = i == hours % 24 && hours < 24;
        let (r, a) = if breathing { (3.4 + (time * 3.0).sin() * 0.6, 1.0) } else if lit { (2.2, 0.8) } else { (1.6, 0.2) };
        p.circle_filled(Pos2::new(x, rect.max.y - 22.0), r, glow().gamma_multiply(a));
    }
    let _ = Color32::BLACK;
}
