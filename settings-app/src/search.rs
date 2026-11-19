//! The search pill above the page pill: type a setting's name, pick it, and the page opens with that setting lit up.

use crate::{theme::*, App, Page, PAGES};
use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Id, Key, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

pub struct Entry {
    pub name: String,
    pub hint: String,
    pub page: Page,
    pub widget: Option<String>,
}

#[derive(Default)]
pub struct Search {
    pub query: String,
    selected: usize,
    crawled: bool,
}

type Registry = Vec<(usize, String, String)>;

/// A page's rows and cards tell the search their names as they are drawn.
pub fn register(ui: &Ui, name: &str, hint: &str) {
    let page: usize = ui.ctx().data(|d| d.get_temp(Id::new("reg-page"))).unwrap_or(usize::MAX);
    if page == usize::MAX || name.is_empty() {
        return;
    }
    ui.ctx().data_mut(|d| {
        let list = d.get_temp_mut_or_default::<Registry>(Id::new("row-registry"));
        if !list.iter().any(|(p, n, _)| *p == page && n.eq_ignore_ascii_case(name)) {
            list.push((page, name.to_string(), hint.to_string()));
        }
    });
}

pub fn set_page(ctx: &egui::Context, page: usize) {
    ctx.data_mut(|d| d.insert_temp(Id::new("reg-page"), page));
}

fn page_index(page: Page) -> usize {
    PAGES.iter().position(|p| p.0 == page).unwrap_or(0)
}

/// Draw each page once, out of sight, so that every setting is known to the search before anyone has visited its page.
fn crawl(app: &mut App, ui: &mut Ui) {
    let area = Rect::from_min_size(Pos2::new(-8000.0, -8000.0), Vec2::new(900.0, 4000.0));
    ui.ctx().data_mut(|d| d.insert_temp(Id::new("crawling"), true));
    for page in [Page::Bar, Page::Widgets, Page::Komorebi, Page::Tacky, Page::Templates] {
        set_page(ui.ctx(), page_index(page));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(area).invisible());
        match page {
            Page::Bar => crate::bar::show(app, &mut child),
            Page::Widgets => crate::widgets::show(app, &mut child),
            Page::Komorebi => crate::komorebi::show(app, &mut child),
            Page::Tacky => crate::tacky::show(app, &mut child),
            Page::Templates => crate::templates::show(app, &mut child),
            _ => {}
        }
    }
    set_page(ui.ctx(), page_index(app.page));
    ui.ctx().data_mut(|d| d.insert_temp(Id::new("crawling"), false));
}

fn entries(app: &App, ctx: &egui::Context) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    let mut add = |name: String, hint: String, page: Page, widget: Option<String>| {
        if !out.iter().any(|e| e.page == page && e.widget == widget && e.name.eq_ignore_ascii_case(&name)) {
            out.push(Entry { name, hint, page, widget });
        }
    };
    for (page, _, name, hint) in PAGES.iter() {
        add(name.to_string(), "page".to_string(), *page, None);
    }
    for (page, name) in [(Page::Home, "Scroll hotkeys"), (Page::Home, "Komorebi switch"), (Page::Wallpaper, "Lock screen"), (Page::About, "Versions"), (Page::About, "Sounds"), (Page::About, "Interface sounds"), (Page::About, "Volume"), (Page::About, "Files")] {
        add(name.to_string(), "setting".into(), page, None);
    }
    for w in app.cfg.keys(&["widgets"]) {
        add(w.replace('_', " "), "widget".into(), Page::Widgets, Some(w.clone()));
        for k in app.cfg.keys(&["widgets", &w, "options"]) {
            add(crate::widgets::nice(&k), format!("{} option", w.replace('_', " ")), Page::Widgets, Some(w.clone()));
        }
    }
    let reg: Registry = ctx.data(|d| d.get_temp(Id::new("row-registry"))).unwrap_or_default();
    for (page, name, hint) in reg {
        if let Some((p, ..)) = PAGES.get(page) {
            add(name, if hint.is_empty() { "setting".into() } else { hint }, *p, None);
        }
    }
    out
}

/// How well one typed word fits some text (None: not at all). A piece of the text scores best, then the letters in order with gaps
/// ("wlpr" finds "wallpaper"), then one slip in a longer word ("wallpapre").
fn fit(word: &str, text: &str) -> Option<i32> {
    if let Some(at) = text.find(word) {
        let starts_word = at == 0 || !text[..at].chars().last().is_some_and(char::is_alphanumeric);
        return Some(100 - at.min(40) as i32 + if starts_word { 30 } else { 0 });
    }
    // the letters in order
    let mut chars = text.chars().enumerate();
    let (mut score, mut last, mut found) = (60i32, None::<usize>, 0);
    for w in word.chars() {
        let (i, _) = chars.by_ref().find(|(_, c)| *c == w)?;
        if let Some(l) = last {
            score -= (i - l - 1).min(6) as i32 * 3;
        }
        last = Some(i);
        found += 1;
    }
    if found == word.chars().count() && word.chars().count() >= 2 {
        return Some(score.max(10));
    }
    None
}

/// One slip (a letter wrong, missing or extra) between two short words.
fn near(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    let (mut i, mut j, mut slips) = (0, 0, 0);
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            i += 1;
            j += 1;
            continue;
        }
        slips += 1;
        if slips > 1 {
            return false;
        }
        if a.len() > b.len() { i += 1 } else if a.len() < b.len() { j += 1 } else { i += 1; j += 1 }
    }
    slips + (a.len() - i) + (b.len() - j) <= 1
}

fn matches<'a>(all: &'a [Entry], query: &str) -> Vec<&'a Entry> {
    let words: Vec<String> = query.to_lowercase().split_whitespace().map(String::from).collect();
    if words.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(i32, &Entry)> = all.iter().filter_map(|e| {
        let (n, h) = (e.name.to_lowercase(), e.hint.to_lowercase());
        let mut total = 0;
        for w in &words {
            // the name counts double; a slip in a word of the name still finds it
            let best = fit(w, &n).map(|v| v * 2)
                .or_else(|| fit(w, &h))
                .or_else(|| (w.chars().count() >= 4 && n.split(|c: char| !c.is_alphanumeric()).any(|part| near(w, part))).then_some(40))?;
            total += best;
        }
        if e.hint.starts_with("page") { total += 5; }
        Some((-total, e))
    }).collect();
    scored.sort_by_key(|(s, _)| *s);
    scored.into_iter().map(|(_, e)| e).take(40).collect()
}

/// The pill itself and, while there is something typed, the list of matches above it.
pub fn show(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let rect = app.search_rect;
    let box_id = Id::new("search-box");
    if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::K)) {
        ctx.memory_mut(|m| m.request_focus(box_id));
    }
    let focused = ctx.memory(|m| m.has_focus(box_id));
    // Escape cancels: it empties the box and gives up the focus. egui drops the focus itself when Escape is pressed, before this runs, so
    // "it was focused a moment ago" is what counts.
    let was_focused = ctx.data(|d| d.get_temp::<bool>(Id::new("search-was-focused"))).unwrap_or(false);
    ctx.data_mut(|d| d.insert_temp(Id::new("search-was-focused"), focused));
    if (focused || was_focused || !app.search.query.is_empty()) && ctx.input(|i| i.key_pressed(Key::Escape)) {
        app.search.query.clear();
        ctx.memory_mut(|m| m.surrender_focus(box_id));
        return;
    }
    let glow = ease(ui, box_id.with("glow"), focused);
    ui.painter().rect(rect, CornerRadius::same(20), blend(CARD_HOVER, Color32::from_gray(28), glow), Stroke::new(1.0, blend(LINE_STRONG, MUTED, glow)), StrokeKind::Inside);
    // the magnifier only appears once the pill has opened out
    let open_t = ((rect.width() - 96.0) / 56.0).clamp(0.0, 1.0);
    if open_t > 0.05 {
        let c = Pos2::new(rect.min.x + 24.0, rect.center().y - 1.0);
        let ink = blend(DIM, TEXT, glow).gamma_multiply(open_t);
        ui.painter().circle_stroke(c, 5.5, Stroke::new(1.5, ink));
        ui.painter().line_segment([c + Vec2::new(4.0, 4.0), c + Vec2::new(8.5, 8.5)], Stroke::new(1.6, ink));
    }
    let text_rect = Rect::from_min_max(Pos2::new(rect.min.x + 8.0 + 38.0 * open_t, rect.min.y + 4.0), Pos2::new(rect.max.x - 8.0 - 30.0 * open_t, rect.max.y - 4.0));
    let before = app.search.query.clone();
    ui.put(text_rect, egui::TextEdit::singleline(&mut app.search.query).id(box_id).frame(egui::Frame::NONE).hint_text(egui::RichText::new("Search").color(Color32::from_gray(200))).font(FontId::new(13.0, FontFamily::Proportional)).text_color(TEXT).desired_width(text_rect.width()).horizontal_align(egui::Align::Center).vertical_align(egui::Align::Center));
    if app.search.query != before {
        app.search.selected = 0;
        if !app.search.crawled {
            app.search.crawled = true;
            crawl(app, ui);
        }
    }
    if app.search.query.is_empty() {
        return;
    }
    let all = entries(app, &ctx);
    let found: Vec<(String, String, Page, Option<String>)> = matches(&all, &app.search.query).into_iter().map(|e| (e.name.clone(), e.hint.clone(), e.page, e.widget.clone())).collect();
    let mut key_moved = false;
    if focused {
        let down = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowDown));
        let up = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowUp));
        if down && app.search.selected + 1 < found.len() { app.search.selected += 1; key_moved = true; }
        if up && app.search.selected > 0 { app.search.selected -= 1; key_moved = true; }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            app.search.query.clear();
            ctx.memory_mut(|m| m.surrender_focus(box_id));
            return;
        }
    }
    app.search.selected = app.search.selected.min(found.len().saturating_sub(1));
    // a click anywhere else closes the search and clears it
    // the list is centred in the window and wide
    let list_width = (ctx.content_rect().width() - 80.0).clamp(340.0, 640.0);
    let list_left = ctx.content_rect().center().x - list_width / 2.0;
    let list_area = Rect::from_min_size(Pos2::new(list_left, app.nav_rect.min.y - 10.0 - ((found.len().clamp(1, 6) as f32) * 48.0 + 12.0)), Vec2::new(list_width, (found.len().clamp(1, 6) as f32) * 48.0 + 12.0));
    if ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !rect.contains(p) && !list_area.contains(p))) {
        app.search.query.clear();
        ctx.memory_mut(|m| m.surrender_focus(box_id));
        return;
    }
    let mut pick: Option<usize> = None;
    if focused && ctx.input(|i| i.key_pressed(Key::Enter)) && !found.is_empty() {
        pick = Some(app.search.selected);
    }
    // the list opens upwards from the pill
    let row_h = 48.0;
    let height = (found.len().clamp(1, 6) as f32) * row_h + 12.0;
    let list = Rect::from_min_size(Pos2::new(list_left, app.nav_rect.min.y - 10.0 - height), Vec2::new(list_width, height));
    ui.painter().rect(list, CornerRadius::same(20), CARD, Stroke::new(1.0, LINE_STRONG), StrokeKind::Inside);
    if found.is_empty() {
        ui.painter().text(list.center(), egui::Align2::CENTER_CENTER, "Nothing found", FontId::new(12.5, mono()), MUTED);
    }
    // at most six rows show; the rest are reached with the list's own scroll bar (it appears at the right edge, like the page's)
    let inner = Rect::from_min_max(Pos2::new(list.min.x, list.min.y + 6.0), Pos2::new(list.max.x - 2.0, list.max.y - 6.0));
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.set_clip_rect(inner);
        // the list's own bar is drawn here (always there, thin, a little in from the edge) so the page's hover-only bar settings do not touch it
        let jump_id = Id::new("search-scroll-jump");
        let mut area = egui::ScrollArea::vertical().id_salt("search-scroll").max_height(inner.height()).auto_shrink([false, false]).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
        if let Some(y) = ui.ctx().data_mut(|d| d.remove_temp::<f32>(jump_id)) {
            area = area.vertical_scroll_offset(y);
        }
        let output = area.show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
                for (i, (name, hint, row_page, _)) in found.iter().enumerate() {
                    let (full, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), row_h), Sense::click());
                    let r = full.shrink2(Vec2::new(6.0, 0.0));
                    if key_moved && app.search.selected == i {
                        response.scroll_to_me(None);
                    }
                    if response.hovered() {
                        app.search.selected = i;
                    }
                    if app.search.selected == i {
                        ui.painter().rect_filled(r, CornerRadius::same(14), CARD_HOVER);
                    }
                    // the page's icon in a small tile, the name with what it is under it, and which page it lives on at the right
                    let (icon, page_name) = PAGES.iter().find(|p| p.0 == *row_page).map(|p| (p.1, p.2)).unwrap_or((crate::icons::Icon::Home, ""));
                    let tile = Rect::from_center_size(Pos2::new(r.min.x + 28.0, r.center().y), Vec2::splat(34.0));
                    ui.painter().rect(tile, CornerRadius::same(11), Color32::from_gray(18), Stroke::new(1.0, LINE), StrokeKind::Inside);
                    crate::icons::paint(ui.painter(), icon, tile.center(), if app.search.selected == i { WHITE } else { TEXT });
                    let tag = ui.painter().layout_no_wrap(page_name.to_uppercase(), FontId::new(10.5, mono()), MUTED);
                    let tag_box = Rect::from_center_size(Pos2::new(r.max.x - 14.0 - tag.size().x / 2.0 - 10.0, r.center().y), Vec2::new(tag.size().x + 20.0, 24.0));
                    ui.painter().rect(tag_box, CornerRadius::same(12), Color32::TRANSPARENT, Stroke::new(1.0, LINE_STRONG), StrokeKind::Inside);
                    ui.painter().galley(Pos2::new(tag_box.min.x + 10.0, tag_box.center().y - tag.size().y / 2.0), tag, MUTED);
                    // one line: the icon and the name on the left, what it is in the middle (cut short when it does not fit), the page on the right
                    let title = ui.painter().layout_no_wrap(name.clone(), FontId::new(13.5, FontFamily::Proportional), TEXT);
                    let small = ui.painter().layout_no_wrap(hint.clone(), FontId::new(10.5, mono()), MUTED);
                    let title_x = r.min.x + 52.0;
                    let title_clip = Rect::from_min_max(Pos2::new(title_x, r.min.y), Pos2::new((tag_box.min.x - 8.0).max(title_x), r.max.y));
                    let title_width = title.size().x.min(title_clip.width() * 0.6);
                    ui.painter().with_clip_rect(Rect::from_min_max(title_clip.min, Pos2::new(title_x + title_width, r.max.y))).galley(Pos2::new(title_x, r.center().y - title.size().y / 2.0), title, TEXT);
                    let hint_x = title_x + title_width + 18.0;
                    if hint_x < tag_box.min.x - 24.0 {
                        let clip = Rect::from_min_max(Pos2::new(hint_x, r.min.y), Pos2::new(tag_box.min.x - 12.0, r.max.y));
                        ui.painter().with_clip_rect(clip).galley(Pos2::new(hint_x, r.center().y - small.size().y / 2.0), small, MUTED);
                    }
                    if response.clicked() {
                        pick = Some(i);
                    }
                }

        });
        let (total, view, offset) = (output.content_size.y, output.inner_rect.height(), output.state.offset().y);
        if total > view + 1.0 {
            let track = Rect::from_min_max(Pos2::new(list.max.x - 16.0, inner.min.y + 4.0), Pos2::new(list.max.x - 8.0, inner.max.y - 4.0));
            let handle_h = (view / total * track.height()).max(28.0);
            let room = (track.height() - handle_h).max(1.0);
            let top = track.min.y + offset / (total - view) * room;
            let hit = ui.interact(track.expand2(Vec2::new(6.0, 0.0)), Id::new("search-scroll-bar"), Sense::click_and_drag());
            if let (true, Some(p)) = (hit.dragged() || hit.clicked(), ui.input(|i| i.pointer.interact_pos())) {
                let at = ((p.y - track.min.y - handle_h / 2.0) / room).clamp(0.0, 1.0) * (total - view);
                ui.ctx().data_mut(|d| d.insert_temp(jump_id, at));
                ui.ctx().request_repaint();
            }
            ui.painter().rect_filled(Rect::from_min_max(Pos2::new(track.center().x - 1.0, track.min.y), Pos2::new(track.center().x + 1.0, track.max.y)), CornerRadius::same(1), Color32::from_gray(34));
            let w = if hit.hovered() || hit.dragged() { 3.5 } else { 2.5 };
            ui.painter().rect_filled(Rect::from_min_max(Pos2::new(track.center().x - w, top), Pos2::new(track.center().x + w, top + handle_h)), CornerRadius::same(3), blend(MUTED, WHITE, if hit.hovered() || hit.dragged() { 1.0 } else { 0.0 }));
        }
    });
    if let Some(i) = pick {
        let (name, _, page, widget) = found[i].clone();
        if app.page != page {
            crate::sound::play(crate::sound::Sound::Nav);
        }
        app.page = page;
        if let Some(w) = widget {
            app.widget = w;
        }
        let now = ctx.input(|i| i.time);
        ctx.data_mut(|d| {
            d.insert_temp(Id::new("highlight"), (name, now));
            d.remove::<(f64, Id)>(Id::new("highlight-key"));
        });
        app.search.query.clear();
        ctx.memory_mut(|m| m.surrender_focus(box_id));
    }
}
