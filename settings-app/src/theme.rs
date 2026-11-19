//! The look: black, white and greys; Geist for text, Geist Mono for headings and small print. Plus the few controls everything is built from.

use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};
use std::sync::Arc;

pub const BG: Color32 = Color32::from_rgb(0, 0, 0);
pub const CARD: Color32 = Color32::from_rgb(10, 10, 10);
pub const CARD_HOVER: Color32 = Color32::from_rgb(20, 20, 20);
pub const LINE: Color32 = Color32::from_rgb(38, 38, 38);
pub const LINE_STRONG: Color32 = Color32::from_rgb(70, 70, 70);
pub const TEXT: Color32 = Color32::from_rgb(242, 242, 242);
pub const MUTED: Color32 = Color32::from_rgb(138, 138, 138);
pub const DIM: Color32 = Color32::from_rgb(85, 85, 85);
pub const WHITE: Color32 = Color32::from_rgb(255, 255, 255);

// While the Lucent theme is on, the window borrows the accent: its "white" things (buttons, switches, the glowing border, the dots) take a
// light, soft version of it. 0 = off.
static TINT: [std::sync::atomic::AtomicU32; 3] = [std::sync::atomic::AtomicU32::new(0), std::sync::atomic::AtomicU32::new(0), std::sync::atomic::AtomicU32::new(0)];

pub fn set_tint(t: Option<[[u8; 3]; 3]>) {
    for (slot, c) in TINT.iter().zip(t.unwrap_or([[0; 3]; 3])) {
        slot.store(if t.is_some() { 0xFF00_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32 } else { 0 }, std::sync::atomic::Ordering::Relaxed);
    }
}

fn tinted(i: usize) -> Option<Color32> {
    let v = TINT[i].load(std::sync::atomic::Ordering::Relaxed);
    (v != 0).then(|| Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// The white of buttons and switches.
pub fn lit() -> Color32 {
    tinted(0).unwrap_or(WHITE)
}

/// The colour of the moving light on the borders and of the dots.
pub fn glow() -> Color32 {
    tinted(1).unwrap_or(WHITE)
}

/// A colour that runs from the light tone (f = 0) to the accent (f = 1): the dots of the clock.
pub fn gradient(f: f32) -> Color32 {
    match (tinted(0), tinted(2)) {
        (Some(a), Some(b)) => blend(a, b, f.clamp(0.0, 1.0)),
        _ => WHITE,
    }
}


/// Blend two colours (0 = a, 1 = b).
pub fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()), f(a.a(), b.a()))
}

/// 0..1 that eases towards 1 while `on` (and back), so hovers fade in and out instead of switching.
pub fn ease(ui: &Ui, id: egui::Id, on: bool) -> f32 {
    ui.ctx().animate_bool_with_time(id, on, 0.14)
}

/// 0..1 while the pointer button is held on the widget, easing in fast and out a little slower (the "press" of a haptic feel).
pub fn press(ui: &Ui, response: &Response) -> f32 {
    let on = response.is_pointer_button_down_on();
    ui.ctx().animate_bool_with_time(response.id.with("press"), on, if on { 0.06 } else { 0.18 })
}

pub fn mono() -> FontFamily {
    FontFamily::Name("mono".into())
}

pub fn heading() -> FontFamily {
    FontFamily::Name("heading".into())
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let add = |fonts: &mut FontDefinitions, name: &str, bytes: &'static [u8]| {
        fonts.font_data.insert(name.into(), Arc::new(FontData::from_static(bytes)));
    };
    add(&mut fonts, "Geist", include_bytes!("../assets/fonts/Geist-VariableFont_wght.ttf"));
    add(&mut fonts, "GeistMono", include_bytes!("../assets/fonts/GeistMonoNerdFont-Regular.otf"));
    add(&mut fonts, "GeistMonoSemi", include_bytes!("../assets/fonts/GeistMonoNerdFont-SemiBold.otf"));
    fonts.families.insert(FontFamily::Proportional, vec!["Geist".into(), "GeistMono".into()]);
    fonts.families.insert(FontFamily::Monospace, vec!["GeistMono".into()]);
    fonts.families.insert(mono(), vec!["GeistMono".into()]);
    fonts.families.insert(heading(), vec!["GeistMonoSemi".into()]);
    // the icons komorebi workspaces are named with (Segoe Fluent Icons, part of Windows): shown when a name uses them
    if let Ok(bytes) = std::fs::read(std::path::Path::new(&std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into())).join("Fonts").join("SegoeIcons.ttf")) {
        fonts.font_data.insert("SegoeIcons".into(), Arc::new(FontData::from_owned(bytes)));
        for family in [FontFamily::Proportional, FontFamily::Monospace, mono(), heading()] {
            fonts.families.entry(family).or_default().push("SegoeIcons".into());
        }
    }
    ctx.set_fonts(fonts);

    ctx.global_style_mut(|style| {
        let v = &mut style.visuals;
        v.dark_mode = true;
        v.override_text_color = Some(TEXT);
        v.panel_fill = BG;
        v.window_fill = CARD;
        v.extreme_bg_color = Color32::from_rgb(5, 5, 5);
        v.faint_bg_color = CARD;
        v.selection.bg_fill = Color32::from_rgb(60, 60, 60);
        v.selection.stroke = Stroke::new(1.0, WHITE);
        v.hyperlink_color = WHITE;
        v.window_stroke = Stroke::new(1.0, LINE);
        v.popup_shadow = egui::Shadow::NONE;
        v.window_shadow = egui::Shadow::NONE;
        for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
            w.corner_radius = CornerRadius::same(10);
        }
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
        v.widgets.inactive.bg_fill = CARD;
        v.widgets.inactive.weak_bg_fill = CARD;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, LINE);
        v.widgets.hovered.bg_fill = CARD_HOVER;
        v.widgets.hovered.weak_bg_fill = CARD_HOVER;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, LINE_STRONG);
        v.widgets.active.bg_fill = CARD_HOVER;
        v.widgets.active.weak_bg_fill = CARD_HOVER;
        v.widgets.active.bg_stroke = Stroke::new(1.0, WHITE);
        v.widgets.open.bg_fill = CARD_HOVER;
        style.animation_time = 0.2;
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(14.0, 7.0);
        style.spacing.interact_size.y = 30.0;
        style.spacing.scroll.bar_width = 18.0;
        // the scroll bar is only there while the pointer is on the right edge (or dragging it)
        style.spacing.scroll.dormant_background_opacity = 0.0;
        style.spacing.scroll.dormant_handle_opacity = 0.0;
        style.spacing.scroll.active_background_opacity = 0.0;
        style.spacing.scroll.active_handle_opacity = 0.0;
        style.spacing.scroll.interact_background_opacity = 0.5;
        style.spacing.scroll.interact_handle_opacity = 1.0;
        style.spacing.scroll.floating = true;
        style.spacing.scroll.floating_width = 8.0;
        style.spacing.scroll.bar_inner_margin = 3.0;
        style.spacing.scroll.bar_outer_margin = 7.0; // a little in from the border
        style.spacing.scroll.handle_min_length = 48.0;
    });
}

pub fn text(ui: &mut Ui, s: impl Into<String>, size: f32, colour: Color32) -> Response {
    ui.label(egui::RichText::new(s.into()).font(FontId::new(size, FontFamily::Proportional)).color(colour))
}

pub fn small(ui: &mut Ui, s: impl Into<String>) -> Response {
    ui.label(egui::RichText::new(s.into()).font(FontId::new(11.5, mono())).color(MUTED))
}

pub fn title(ui: &mut Ui, s: &str, size: f32) -> Response {
    ui.label(egui::RichText::new(s.to_uppercase()).font(FontId::new(size, heading())).color(WHITE))
}

/// The mark in a section header: a program's own, or the icon of the page it is on.
pub enum Mark<'a> {
    Program(&'a str),
    Page(crate::icons::Icon),
}

/// The header of a section: its mark, a small line of text above the name, the name big, whether it is on (when that makes sense) and an arrow.
/// True when it was clicked (open or close the section).
pub fn section_header(ui: &mut Ui, mark: Mark, name: &str, tag: &str, status: Option<bool>, open: bool) -> bool {
    use egui::Align2;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 72.0), Sense::click());
    let hover = ease(ui, response.id.with("hover"), response.hovered());
    let on = status.unwrap_or(true);
    let held = press(ui, &response);
    let rect = rect.shrink2(Vec2::new(held * 2.0, held * 1.5));
    let p = ui.painter();
    p.rect(rect, CornerRadius::same(20), blend(CARD, CARD_HOVER, hover), Stroke::new(1.0, blend(LINE, LINE_STRONG, hover)), StrokeKind::Inside);
    let grow = hover * 3.0;
    let icon = Rect::from_center_size(Pos2::new(rect.min.x + 38.0, rect.center().y), Vec2::splat(44.0 + grow));
    p.rect(icon, CornerRadius::same(14), Color32::from_gray(18), Stroke::new(1.0, LINE), StrokeKind::Inside);
    let ink = if on { TEXT } else { MUTED };
    match mark {
        Mark::Program(id) => crate::icons::paint_program(p, id, icon.center(), ink),
        Mark::Page(i) => crate::icons::paint(p, i, icon.center(), ink),
    }
    let room = rect.width() - 150.0;
    let tag_galley = p.layout_no_wrap(tag.to_uppercase(), FontId::new(11.5, mono()), MUTED);
    let name_galley = p.layout_no_wrap(name.to_uppercase(), FontId::new(19.0, heading()), if on { WHITE } else { MUTED });
    let x = icon.max.x + 16.0 - grow + hover * 3.0; // the name slides a little to the right
    p.with_clip_rect(Rect::from_min_max(Pos2::new(x, rect.min.y), Pos2::new(x + room, rect.max.y)))
        .galley(Pos2::new(x, rect.center().y - 17.0 - tag_galley.size().y / 2.0), tag_galley, MUTED);
    p.with_clip_rect(Rect::from_min_max(Pos2::new(x, rect.min.y), Pos2::new(x + room, rect.max.y)))
        .galley(Pos2::new(x, rect.center().y + 7.0 - name_galley.size().y / 2.0), name_galley, WHITE);
    // right: on or off, and an arrow that turns when it is open
    let c = Pos2::new(rect.max.x - 28.0, rect.center().y + hover * 2.0 * if open { -1.0 } else { 1.0 }); // the arrow nudges the way it will turn
    if let Some(s) = status {
        p.text(Pos2::new(c.x - 22.0, c.y), Align2::RIGHT_CENTER, if s { "ON" } else { "OFF" }, FontId::new(11.5, mono()), if s { lit() } else { MUTED });
    }
    let d = if open { -1.0 } else { 1.0 };
    let col = blend(MUTED, WHITE, hover);
    p.line_segment([Pos2::new(c.x - 5.0, c.y - 2.5 * d), Pos2::new(c.x, c.y + 2.5 * d)], Stroke::new(1.8, col));
    p.line_segment([Pos2::new(c.x, c.y + 2.5 * d), Pos2::new(c.x + 5.0, c.y - 2.5 * d)], Stroke::new(1.8, col));
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
}

type Sections = (usize, Vec<(egui::Id, String, bool)>);

/// A section tells the pills at the top of the page that it exists: where its open/closed state lives, its name, and whether it starts open.
pub fn register_section(ui: &Ui, id: egui::Id, name: &str, default_open: bool) {
    let ctx = ui.ctx();
    if ctx.data(|d| d.get_temp::<bool>(egui::Id::new("crawling"))).unwrap_or(false) {
        return;
    }
    let page: usize = ctx.data(|d| d.get_temp(egui::Id::new("reg-page"))).unwrap_or(usize::MAX);
    ctx.data_mut(|d| {
        let (p, list) = d.get_temp_mut_or_insert_with::<Sections>(egui::Id::new("section-list"), || (page, Vec::new()));
        if *p != page {
            *p = page;
            list.clear();
        }
        if !list.iter().any(|(i, ..)| *i == id) {
            list.push((id, name.to_string(), default_open));
        }
    });
}

/// A row of tiny pills at the top of a page, one for each section: lit while it is open, a click opens or closes it. The first opens or closes all.
/// The list is what the sections registered in the last frame.
pub fn section_pills(ui: &mut Ui, page: usize) {
    let ctx = ui.ctx().clone();
    let (p, list) = ctx.data_mut(|d| d.remove_temp::<Sections>(egui::Id::new("section-list"))).unwrap_or((usize::MAX, Vec::new()));
    if p != page || list.len() < 2 {
        return;
    }
    let state: Vec<bool> = list.iter().map(|(id, _, default)| ctx.data(|d| d.get_temp::<bool>(*id)).unwrap_or(*default)).collect();
    let all_open = state.iter().all(|s| *s);
    let mut set: Vec<(egui::Id, bool)> = Vec::new();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
        if tiny_pill(ui, if all_open { "Close all" } else { "Open all" }, false, true).clicked() {
            set.extend(list.iter().map(|(id, ..)| (*id, !all_open)));
        }
        for ((id, name, _), open) in list.iter().zip(&state) {
            if tiny_pill(ui, name, *open, false).clicked() {
                set.push((*id, !open));
            }
        }
    });
    ui.add_space(12.0);
    for (id, open) in set {
        ctx.data_mut(|d| d.insert_temp(id, open));
    }
}

fn tiny_pill(ui: &mut Ui, label: &str, on: bool, outline_only: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(label.to_uppercase(), FontId::new(12.0, heading()), if on { BG } else { TEXT });
    let (rect, response) = ui.allocate_exact_size(Vec2::new(galley.size().x + 40.0, 36.0), Sense::click());
    let chosen = ease(ui, response.id.with("on"), on);
    let hover = ease(ui, response.id.with("hover"), response.hovered());
    let held = press(ui, &response);
    let rect = rect.translate(Vec2::new(0.0, -hover * 1.5)).shrink2(Vec2::new(held * 2.0, held * 1.5));
    let fill = blend(blend(BG, CARD_HOVER, hover), lit(), chosen);
    let edge = blend(if outline_only { MUTED } else { LINE_STRONG }, lit(), chosen);
    ui.painter().rect(rect, CornerRadius::same(18), fill, Stroke::new(1.0, edge), StrokeKind::Inside);
    ui.painter().galley(Pos2::new(rect.min.x + 20.0, rect.center().y - galley.size().y / 2.0), galley, TEXT);
    response
}

/// A boxed group of rows. With a heading it is a section: a header (the page's icon, the page's name, the heading) that opens and closes it.
/// Open by default, and always open while the search reads the pages. `None` when it is closed.
pub fn card<R>(ui: &mut Ui, heading_text: &str, body: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    ui.add_space(4.0);
    let ctx = ui.ctx().clone();
    let page: usize = ctx.data(|d| d.get_temp(egui::Id::new("reg-page"))).unwrap_or(usize::MAX);
    let crawling: bool = ctx.data(|d| d.get_temp(egui::Id::new("crawling"))).unwrap_or(false);
    let sectioned = !heading_text.is_empty() && page < crate::PAGES.len();
    crate::search::register(ui, heading_text, "section");
    if sectioned {
        let id = egui::Id::new(("card-open", page, heading_text));
        register_section(ui, id, heading_text, true);
        let mut open = ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(true);
        let (_, icon, page_name, _) = crate::PAGES[page];
        if section_header(ui, Mark::Page(icon), heading_text, page_name, None, open || crawling) && !crawling {
            open = !open;
            ctx.data_mut(|d| d.insert_temp(id, open));
        }
        if !open && !crawling {
            ui.add_space(8.0);
            return None;
        }
        ui.add_space(6.0);
    } else if !heading_text.is_empty() {
        ui.horizontal(|ui| {
            dots(ui);
            ui.label(egui::RichText::new(heading_text.to_uppercase()).font(FontId::new(12.5, heading())).color(MUTED));
        });
        ui.add_space(2.0);
    }
    let out = egui::Frame::new().fill(CARD).stroke(Stroke::new(1.0, LINE)).corner_radius(CornerRadius::same(18)).inner_margin(egui::Margin::symmetric(18, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let first_id = ui.id().with("first-row");
            ui.data_mut(|d| d.insert_temp(first_id, true));
            body(ui)
        });
    highlight(ui, out.response.rect, heading_text, 18);
    let corner = Pos2::new(out.response.rect.max.x - 14.0, out.response.rect.min.y + 14.0);
    ui.painter().circle_filled(corner, 2.0, LINE_STRONG);
    if sectioned {
        ui.add_space(8.0);
    }
    Some(out.inner)
}

/// Three dots that fade out: the small mark before a section title.
pub fn dots(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(20.0, 12.0), Sense::hover());
    for (i, a) in [1.0, 0.5, 0.2].into_iter().enumerate() {
        ui.painter().circle_filled(Pos2::new(rect.min.x + 3.0 + i as f32 * 7.0, rect.center().y), 2.2, WHITE.gamma_multiply(a));
    }
}

pub fn divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, LINE);
}

/// One setting: the name and a line under it on the left, the control on the right.
pub fn row<R>(ui: &mut Ui, name: &str, hint: &str, control: impl FnOnce(&mut Ui) -> R) -> R {
    let first_id = ui.id().with("first-row");
    let first = ui.data_mut(|d| d.get_temp::<bool>(first_id)).unwrap_or(true);
    ui.data_mut(|d| d.insert_temp(first_id, false));
    if !first {
        divider(ui);
    }
    ui.add_space(8.0);
    crate::search::register(ui, name, hint);
    let backdrop = ui.painter().add(egui::Shape::Noop);
    let out = ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width((ui.available_width() - 260.0).max(180.0));
            text(ui, name, 14.5, TEXT);
            if !hint.is_empty() {
                small(ui, hint);
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| control(ui)).inner
    });
    ui.add_space(8.0);
    // the row under the pointer lights up faintly
    let zone = out.response.rect.expand2(Vec2::new(8.0, 4.0));
    let over = ui.rect_contains_pointer(zone);
    let glow_t = ui.ctx().animate_bool_with_time(ui.id().with(("row-hover", name)), over, 0.18);
    if glow_t > 0.01 {
        ui.painter().set(backdrop, egui::epaint::RectShape::filled(zone, CornerRadius::same(12), WHITE.gamma_multiply(0.028 * glow_t)));
    }
    highlight(ui, zone, name, 12);
    out.inner
}

/// The last row of a card has no line under it.
pub fn last_row_fix(ui: &mut Ui) {
    let _ = ui;
}

pub fn toggle(ui: &mut Ui, on: &mut bool) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(Vec2::new(46.0, 26.0), Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
        crate::sound::play(if *on { crate::sound::Sound::On } else { crate::sound::Sound::Off });
    }
    let t = ui.ctx().animate_bool_responsive(response.id, *on);
    let hover = ease(ui, response.id.with("hover"), response.hovered());
    let held = press(ui, &response);
    let (fill, stroke, knob) = if *on { (blend(lit(), WHITE, hover * 0.25), lit(), BG) } else { (blend(BG, CARD_HOVER, hover), blend(LINE_STRONG, MUTED, hover), blend(MUTED, WHITE, hover * 0.6)) };
    ui.painter().rect(rect.shrink2(Vec2::new(held * 0.8, held * 0.8)), CornerRadius::same(13), fill, Stroke::new(1.2, stroke), StrokeKind::Inside);
    // the knob grows a little under the pointer and stretches sideways while it is pressed
    let (w, h) = (16.0 + hover * 2.0 + held * 7.0, 16.0 + hover * 2.0 - held * 1.5);
    let travel = 20.0 - held * 3.5 * if *on { 1.0 } else { 0.0 };
    let x = rect.min.x + 13.0 + t * travel - if *on { 0.0 } else { held * 0.0 } - (w - 16.0) * 0.5 * t + (w - 16.0) * 0.5 * (1.0 - t);
    let knob_rect = Rect::from_center_size(Pos2::new(x, rect.center().y), Vec2::new(w, h));
    ui.painter().rect_filled(knob_rect, CornerRadius::same((h / 2.0) as u8), knob);
    ui.painter().circle_filled(knob_rect.center(), 2.0, if *on { lit() } else { BG });
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// A whole-number slider with its value written next to it.
pub fn slider(ui: &mut Ui, value: &mut i64, min: i64, max: i64, suffix: &str) -> Response {
    ui.horizontal(|ui| {
        let (rect, mut response) = ui.allocate_exact_size(Vec2::new(170.0, 26.0), Sense::click_and_drag());
        if let Some(p) = response.interact_pointer_pos() {
            let f = ((p.x - rect.min.x - 8.0) / (rect.width() - 16.0)).clamp(0.0, 1.0) as f64;
            let mut v = (min as f64 + f * (max - min) as f64).round() as i64;
            // steps of 5 (the two ends stay reachable), unless the range is too small for that
            if max - min >= 20 {
                v = ((v as f64 / 5.0).round() as i64 * 5).clamp(min, max);
                if (v - min).abs() <= 2 { v = min; }
                if (max - v).abs() <= 2 { v = max; }
            }
            if v != *value {
                *value = v;
                response.mark_changed();
            }
        }
        let f = ((*value - min) as f32 / (max - min).max(1) as f32).clamp(0.0, 1.0);
        let (x0, x1) = (rect.min.x + 8.0, rect.max.x - 8.0);
        let x = x0 + (x1 - x0) * f;
        let hover = ease(ui, response.id.with("hover"), response.hovered() || response.dragged());
        let held = press(ui, &response);
        for k in 0..=10 {
            let tx = x0 + (x1 - x0) * k as f32 / 10.0;
            // the ticks close to the knob stand a little taller while the pointer is on the slider
            let near = (1.0 - ((tx - x).abs() / 40.0)).clamp(0.0, 1.0) * hover;
            ui.painter().line_segment([Pos2::new(tx, rect.center().y + 8.0 - near * 1.5), Pos2::new(tx, rect.center().y + 12.0 + near * 2.0)], Stroke::new(1.0 + near * 0.6, if tx <= x { blend(MUTED, WHITE, near) } else { LINE_STRONG }));
        }
        let thick = 2.0 + hover * 1.0;
        ui.painter().line_segment([Pos2::new(x0, rect.center().y), Pos2::new(x1, rect.center().y)], Stroke::new(thick, LINE_STRONG));
        ui.painter().line_segment([Pos2::new(x0, rect.center().y), Pos2::new(x, rect.center().y)], Stroke::new(thick, lit()));
        let centre = Pos2::new(x, rect.center().y);
        if held > 0.0 || hover > 0.0 {
            ui.painter().circle_filled(centre, 10.0 + 4.0 * held, lit().gamma_multiply(0.10 * hover + 0.12 * held));
        }
        ui.painter().circle(centre, 7.0 + hover * 1.5 + held * 1.5, lit(), Stroke::new(3.0, BG));
        let _ = Rect::NOTHING;
        ui.label(egui::RichText::new(format!("{}{suffix}", *value)).font(FontId::new(12.5, mono())).color(TEXT));
        response
    }).inner
}

/// Pick one of a few: a row of pills, the chosen one white.
pub fn segmented(ui: &mut Ui, options: &[&str], current: &mut String) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        // right-to-left layout reverses the order: draw backwards so it reads left to right
        for o in options.iter().rev() {
            let on = current.eq_ignore_ascii_case(o);
            if pill(ui, o, on).clicked() && !on {
                *current = o.to_string();
                changed = true;
            }
        }
    });
    changed
}

pub fn pill(ui: &mut Ui, label: &str, on: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(label.to_string(), FontId::new(12.5, mono()), if on { BG } else { TEXT });
    let size = Vec2::new(galley.size().x + 24.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let chosen = ease(ui, response.id.with("on"), on);
    let hover = ease(ui, response.id.with("hover"), response.hovered());
    let held = press(ui, &response);
    let fill = blend(blend(BG, CARD_HOVER, hover), lit(), chosen);
    let stroke = blend(blend(LINE, LINE_STRONG, hover), lit(), chosen);
    let text_colour = blend(TEXT, BG, chosen);
    // it rises a pixel under the pointer and sinks and narrows while pressed
    let shown = rect.translate(Vec2::new(0.0, -hover)).shrink2(Vec2::new(held * 1.5, held * 1.0));
    ui.painter().rect(shown, CornerRadius::same(14), fill, Stroke::new(1.0, stroke), StrokeKind::Inside);
    ui.painter().galley(Pos2::new(shown.min.x + 12.0 - held * 0.5, shown.center().y - galley.size().y / 2.0), galley, text_colour);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.clicked() {
        crate::sound::play(crate::sound::Sound::Click);
    }
    response
}

/// A button. `primary` = white.
pub fn button(ui: &mut Ui, label: &str, primary: bool) -> Response {
    let colour = if primary { BG } else { TEXT };
    let galley = ui.painter().layout_no_wrap(label.to_string(), FontId::new(13.0, FontFamily::Proportional), colour);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(galley.size().x + 28.0, 32.0), Sense::click());
    let hover = ease(ui, response.id.with("hover"), response.hovered());
    let held = press(ui, &response);
    let rect = rect.translate(Vec2::new(0.0, -hover)).shrink2(Vec2::new(held * 1.5, held * 1.0));
    let (fill, stroke) = if primary { (blend(lit(), Color32::from_rgb(214, 214, 214), hover * 0.7), lit()) } else { (blend(BG, CARD_HOVER, hover), blend(LINE_STRONG, WHITE, hover)) };
    ui.painter().rect(rect, CornerRadius::same(16), fill, Stroke::new(1.0, stroke), StrokeKind::Inside);
    ui.painter().galley(Pos2::new(rect.min.x + 14.0 - held * 0.5, rect.center().y - galley.size().y / 2.0), galley, colour);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.clicked() {
        crate::sound::play(if primary { crate::sound::Sound::Apply } else { crate::sound::Sound::Click });
    }
    response
}

/// A text box; true when the text changed (and the box lost focus or Enter was pressed, so half-typed values are never saved).
pub fn input(ui: &mut Ui, value: &mut String, width: f32) -> bool {
    let response = ui.add(egui::TextEdit::singleline(value).desired_width(width).font(FontId::new(13.0, mono())).margin(egui::Margin::symmetric(10, 6)));
    response.lost_focus() && response.changed() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
}

/// Like `input`, but markup such as `<span>` and `</span>` is kept in the text and not shown (it is drawn invisibly, a hair wide).
pub fn input_without_tags(ui: &mut Ui, value: &mut String, width: f32) -> bool {
    let font = FontId::new(13.0, mono());
    let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, wrap: f32| {
        let text = text.as_str();
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = wrap;
        let (mut rest, mut inside) = (text, false);
        while !rest.is_empty() {
            let cut = if inside { rest.find('>').map(|i| i + 1) } else { rest.find('<') }.unwrap_or(rest.len());
            let (piece, tail) = rest.split_at(cut);
            let format = if inside {
                egui::TextFormat { font_id: FontId::new(1.0, mono()), color: Color32::TRANSPARENT, ..Default::default() }
            } else {
                egui::TextFormat { font_id: font.clone(), color: TEXT, ..Default::default() }
            };
            job.append(piece, 0.0, format);
            rest = tail;
            inside = !inside;
        }
        ui.fonts_mut(|f| f.layout_job(job))
    };
    let response = ui.add(egui::TextEdit::singleline(value).desired_width(width).font(font.clone()).layouter(&mut layouter).margin(egui::Margin::symmetric(10, 6)));
    response.lost_focus() && response.changed() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
}

/// A setting whose control is wide (a row of choices, a list): the name and its line on top, the control underneath using the full width.
pub fn row_below<R>(ui: &mut Ui, name: &str, hint: &str, control: impl FnOnce(&mut Ui) -> R) -> R {
    let first_id = ui.id().with("first-row");
    let first = ui.data_mut(|d| d.get_temp::<bool>(first_id)).unwrap_or(true);
    ui.data_mut(|d| d.insert_temp(first_id, false));
    if !first {
        divider(ui);
    }
    ui.add_space(8.0);
    text(ui, name, 14.5, TEXT);
    if !hint.is_empty() {
        small(ui, hint);
    }
    ui.add_space(6.0);
    let out = control(ui);
    ui.add_space(8.0);
    out
}

/// Pills to pick one of many, wrapping onto more lines when they do not fit. True when the choice changed.
pub fn choices(ui: &mut Ui, options: &[&str], current: &mut String) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
        for o in options {
            let on = current.eq_ignore_ascii_case(o);
            if pill(ui, o, on).clicked() && !on {
                *current = o.to_string();
                changed = true;
            }
        }
    });
    changed
}

/// A list you can edit: every item is a pill with a small x to remove it, and a box to type a new one (Enter adds it).
/// Returns the new list when something changed.
pub fn chips(ui: &mut Ui, salt: &str, items: &[String], placeholder: &str) -> Option<Vec<String>> {
    let mut result: Option<Vec<String>> = None;
    let id = ui.id().with(("chips", salt));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
        for (i, item) in items.iter().enumerate() {
            let galley = ui.painter().layout_no_wrap(item.clone(), FontId::new(12.5, mono()), TEXT);
            let (rect, response) = ui.allocate_exact_size(Vec2::new(galley.size().x + 44.0, 28.0), Sense::click());
            let x_rect = Rect::from_center_size(Pos2::new(rect.max.x - 15.0, rect.center().y), Vec2::splat(20.0));
            let over_x = ui.input(|inp| inp.pointer.hover_pos()).is_some_and(|p| x_rect.contains(p));
            let (fill, stroke) = if response.hovered() { (CARD_HOVER, LINE_STRONG) } else { (BG, LINE) };
            ui.painter().rect(rect, CornerRadius::same(14), fill, Stroke::new(1.0, stroke), StrokeKind::Inside);
            ui.painter().galley(Pos2::new(rect.min.x + 12.0, rect.center().y - galley.size().y / 2.0), galley, TEXT);
            if over_x {
                ui.painter().circle_filled(x_rect.center(), 9.0, Color32::from_white_alpha(36));
            }
            let c = x_rect.center();
            let s = Stroke::new(1.4, if over_x { WHITE } else { MUTED });
            ui.painter().line_segment([Pos2::new(c.x - 3.0, c.y - 3.0), Pos2::new(c.x + 3.0, c.y + 3.0)], s);
            ui.painter().line_segment([Pos2::new(c.x - 3.0, c.y + 3.0), Pos2::new(c.x + 3.0, c.y - 3.0)], s);
            if response.clicked() && over_x {
                let mut v = items.to_vec();
                v.remove(i);
                result = Some(v);
            }
        }
        // the box for a new one
        let mut buffer: String = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
        let edit = ui.add(egui::TextEdit::singleline(&mut buffer).hint_text(placeholder).desired_width(150.0).font(FontId::new(12.5, mono())).margin(egui::Margin::symmetric(10, 5)));
        let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if enter && !buffer.trim().is_empty() {
            let mut v = items.to_vec();
            v.push(buffer.trim().to_string());
            result = Some(v);
            buffer.clear();
            edit.request_focus();
        }
        ui.data_mut(|d| d.insert_temp(id, buffer));
    });
    result
}

/// The point `s` px along the outline of a rounded rectangle, starting at the middle of its top edge going clockwise.
fn along_outline(rect: Rect, radius: f32, s: f32) -> Pos2 {
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let (w, h) = (rect.width() - 2.0 * r, rect.height() - 2.0 * r);
    let arc = std::f32::consts::FRAC_PI_2 * r;
    let total = 2.0 * (w + h) + 4.0 * arc;
    let mut s = (s + w / 2.0).rem_euclid(total); // begin mid-top: shift to the top-left corner's end
    let corner = |c: Pos2, a0: f32, s: f32| {
        let a = a0 + s / r;
        c + Vec2::new(a.cos(), a.sin()) * r
    };
    let pi = std::f32::consts::PI;
    if s < w { return Pos2::new(rect.min.x + r + s, rect.min.y); }
    s -= w;
    if s < arc { return corner(Pos2::new(rect.max.x - r, rect.min.y + r), -pi / 2.0, s); }
    s -= arc;
    if s < h { return Pos2::new(rect.max.x, rect.min.y + r + s); }
    s -= h;
    if s < arc { return corner(Pos2::new(rect.max.x - r, rect.max.y - r), 0.0, s); }
    s -= arc;
    if s < w { return Pos2::new(rect.max.x - r - s, rect.max.y); }
    s -= w;
    if s < arc { return corner(Pos2::new(rect.min.x + r, rect.max.y - r), pi / 2.0, s); }
    s -= arc;
    if s < h { return Pos2::new(rect.min.x, rect.max.y - r - s); }
    s -= h;
    corner(Pos2::new(rect.min.x + r, rect.min.y + r), pi, s)
}

/// Nothing Phone (1) glyph style: the edge is a ring of separate light strips, faintly lit. When something happens (`since` = seconds since then)
/// a pulse runs round both sides from the top and the strips flare up one after another and fade. Returns true while it is still moving.
pub fn glyph_edge(ui: &Ui, rect: Rect, radius: f32, since: f32, strength: f32, seed: u32) -> bool {
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let total = 2.0 * (rect.width() - 2.0 * r + rect.height() - 2.0 * r) + 2.0 * std::f32::consts::PI * r;
    let (strip, gap) = (34.0, 9.0);
    let n = (total / (strip + gap)).round().max(4.0) as usize;
    let pitch = total / n as f32;
    // after the reaction has faded the glyphs idle: a slow, softer pulse every few seconds and a faint breathing
    let time = ui.input(|i| i.time) as f32;
    let reacting = since < 1.6;
    let cycle = if reacting { seed } else { (time / 4.5).floor() as u32 };
    let pattern = if reacting { (seed >> 5) % 5 } else { (cycle * 3 + 1) % 5 }; // from the top, bottom, round clockwise, round the other way, twinkle
    let (since, strength) = if reacting {
        (since, strength)
    } else {
        let c = time.rem_euclid(4.5);
        if c < 1.6 { (c, strength * 0.5) } else { (9.0, 0.0) }
    };
    let breath = 0.03 * (time * 0.9).sin();
    let head = since / 1.1 * 1.25 - 0.1; // where the pulse is, 0 = top middle, 1 = bottom middle
    for i in 0..n {
        let mid = (i as f32 + 0.5) / n as f32; // 0..1 round the whole outline
        let from_top = mid.min(1.0 - mid) * 2.0; // distance from the top middle along either side
        let d = match pattern { 1 => 1.0 - from_top, 2 => mid, 3 => 1.0 - mid, _ => from_top };
        let lit = if pattern == 4 {
            let h = ((i as u32 * 2654435761u32).wrapping_add(cycle.wrapping_mul(40503)) >> 8) as f32 / 16777216.0 % 1.0;
            (-((since / 1.6 - (0.1 + 0.8 * h)) * 7.0).powi(2)).exp()
        } else if d < head { (-(head - d) * 5.0).exp() } else { (-((d - head) * 18.0).powi(2)).exp() };
        let lit = if since > 1.6 { 0.0 } else { lit };
        let alpha = 0.10 + breath + 0.9 * lit * strength;
        let width = 1.6 + 1.4 * lit;
        let s0 = i as f32 * pitch + (pitch - strip) / 2.0;
        let pts: Vec<Pos2> = (0..=6).map(|k| along_outline(rect, radius, s0 + strip * k as f32 / 6.0)).collect();
        ui.painter().add(egui::Shape::line(pts, Stroke::new(width, glow().gamma_multiply(alpha.min(1.0)))));
    }
    if since < 1.6 {
        ui.ctx().request_repaint();
    } else {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(60));
    }
    since < 1.6
}

/// A setting found by the search: it is lit with a soft pulsing outline for a couple of seconds, and scrolled into view.
pub fn highlight(ui: &Ui, rect: Rect, name: &str, radius: u8) {
    let ctx = ui.ctx();
    let Some((target, start)) = ctx.data(|d| d.get_temp::<(String, f64)>(egui::Id::new("highlight"))) else { return };
    let age = (ctx.input(|i| i.time) - start) as f32;
    if age > 2.8 || !name.eq_ignore_ascii_case(&target) {
        return;
    }
    // only the first row with that name
    let key = ui.id().with(name);
    let owner = ctx.data_mut(|d| {
        let slot = d.get_temp_mut_or_insert_with::<(f64, egui::Id)>(egui::Id::new("highlight-key"), || (start, key));
        if slot.0 != start {
            *slot = (start, key);
        }
        slot.1
    });
    if owner != key {
        return;
    }
    if age < 0.9 {
        ui.scroll_to_rect(rect.expand(40.0), Some(egui::Align::Center));
    }
    let fade = if age < 1.8 { 1.0 } else { 1.0 - (age - 1.8) };
    let pulse = 0.55 + 0.45 * (age * 6.0).sin();
    ui.painter().rect(rect, CornerRadius::same(radius), Color32::from_white_alpha((14.0 * fade) as u8), Stroke::new(1.5, glow().gamma_multiply(pulse * fade)), StrokeKind::Outside);
    ctx.request_repaint();
}
