//! YASB Settings: a window for arranging the bar and setting each widget's options, plus komorebi.
//!
//!   yasb-settings            open the window (edits %USERPROFILE%\.config\yasb\config.yaml and komorebi.json)
//!   yasb-settings <file>     edit another YASB config file

mod about;
mod bar;
mod home;
mod icons;
mod komorebi;
mod layout;
mod glyph;
mod help;
mod lucent;
mod lucent_browser;
mod templates;
mod search;
mod sizes;
mod staged;
mod sound;
mod system;
mod tacky;
mod theme;
mod wallpaper;
mod widgets;
mod yaml;

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use icons::Icon;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use theme::*;
use yaml::Config;

static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn main() {
    START.get_or_init(Instant::now);
    let path = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| home().join(".config").join("yasb").join("config.yaml"));
    // the first position is a good guess (a screen at 100%); the first frame corrects it once the real scale is known
    let first = centred_on_pointer_monitor(Vec2::new(900.0, 720.0), 1.0).unwrap_or(Pos2::new(100.0, 100.0));
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_position(first)
            .with_inner_size([900.0, 720.0])
            .with_min_inner_size([640.0, 520.0])
            .with_title("YASB Settings")
            .with_decorations(false)
            .with_transparent(true),
        ..Default::default()
    };
    let _ = eframe::run_native("YASB Settings", options, Box::new(move |cc| {
        theme::install(&cc.egui_ctx);
        Ok(Box::new(App::new(path, cc.egui_ctx.clone())))
    }));
}

/// The top-left corner (in points) that puts a window of `size` in the middle of the monitor the pointer is on.
fn centred_on_pointer_monitor(size: Vec2, pixels_per_point: f32) -> Option<Pos2> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut at = POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut at) } == 0 {
        return None;
    }
    let monitor = unsafe { MonitorFromPoint(at, MONITOR_DEFAULTTONEAREST) };
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return None;
    }
    let work = info.rcWork; // the screen without the taskbar
    let (cx, cy) = ((work.left + work.right) as f32 / 2.0 / pixels_per_point, (work.top + work.bottom) as f32 / 2.0 / pixels_per_point);
    Some(Pos2::new(cx - size.x / 2.0, cy - size.y / 2.0))
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default())
}

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Home,
    Bar,
    Widgets,
    Komorebi,
    Tacky,
    Wallpaper,
    Templates,
    About,
}

const PAGES: [(Page, Icon, &str, &str); 8] = [
    (Page::Home, Icon::Home, "Home", ""),
    (Page::Bar, Icon::Bar, "Bar", "Where the bar sits, how big it is, which widgets it shows and where"),
    (Page::Widgets, Icon::Widgets, "Widgets", "The options of each widget"),
    (Page::Komorebi, Icon::Komorebi, "Komorebi", "How windows are tiled, and what each workspace is called"),
    (Page::Tacky, Icon::Border, "Tacky Borders", "The coloured outline round your windows"),
    (Page::Wallpaper, Icon::Image, "Wallpaper", "Pick the desktop wallpaper, and the lock screen if you like"),
    (Page::Templates, Icon::Theme, "Lucent", "Your accent colour in other programs (the Lucent theme)"),
    (Page::About, Icon::About, "About", "Versions and files"),
];

/// The message at the top of the window: what it says, buttons it offers (id, label), and how long it has been shown (the clock stops while the pointer is on it).
pub struct Toast {
    message: String,
    actions: Vec<(String, String)>,
    age: f32,
}

pub struct App {
    cfg: Config,
    komorebi: komorebi::Json,
    tacky: Config,
    nav_scroll: f32,
    staged: staged::Staged,
    tint_at: Instant,
    scroll_y: f32,
    to_top: bool,
    glyph_seed: u32,
    kmonitor: usize,
    versions: Arc<Mutex<Option<(String, String)>>>,
    versions_started: bool,
    page: Page,
    bar: String,
    widget: String,
    status: Option<Toast>,
    drag: Option<layout::Drag>,
    page_size: Vec2,
    wall: wallpaper::State,
    system: system::Shared,
    timed: bool,
    applied_at: Option<Instant>,
    nav_rect: Rect,
    search_rect: Rect,
    apply_rect: Rect,
    apply_t: f32,
    apply_confirming: bool,
    search: search::Search,
    centred: u8,
    shown_page: Page,
    page_since: Instant,
    job: Option<Arc<Mutex<Option<String>>>>,
}

impl App {
    fn new(path: PathBuf, ctx: egui::Context) -> App {
        sound::set_enabled(wallpaper::read_setting("sounds").is_none_or(|v| v != "0"));
        if let Some(v) = wallpaper::read_setting("sound_volume").and_then(|v| v.parse().ok()) {
            sound::set_volume(v);
        }
        let cfg = Config::load(path);
        let bar = cfg.keys(&["bars"]).into_iter().next().unwrap_or_default();
        let komorebi = komorebi::Json::load(home().join(".config").join("komorebi").join("komorebi.json"));
        let mut app = App { cfg, komorebi, nav_scroll: 0.0, staged: Default::default(), tint_at: Instant::now() - Duration::from_secs(60), scroll_y: 0.0, to_top: false, glyph_seed: 1, tacky: Config::load(home().join(".config").join("tacky-borders").join("config.yaml")), kmonitor: 0, versions: Arc::new(Mutex::new(None)), versions_started: false, page: Page::Home, bar, widget: String::new(), status: None, drag: None, page_size: Vec2::ZERO, wall: wallpaper::State::default(), system: system::watch(move || ctx.request_repaint()), timed: false, applied_at: None, nav_rect: Rect::NOTHING, search_rect: Rect::NOTHING, apply_rect: Rect::NOTHING, apply_t: 0.0, apply_confirming: false, search: Default::default(), centred: 0, shown_page: Page::Home, page_since: Instant::now(), job: None };
        // YASB_SETTINGS_PAGE=komorebi opens that page first (for screenshots)
        if let Ok(name) = std::env::var("YASB_SETTINGS_PAGE") {
            if let Some((page, ..)) = PAGES.iter().find(|(_, _, n, _)| n.eq_ignore_ascii_case(&name)) {
                app.page = *page;
            }
        }
        if let Some(e) = app.cfg.error.clone() {
            app.say(&e);
        }
        app
    }

    /// How many workspaces komorebi has on its first monitor.
    pub fn komorebi_workspaces(&self) -> usize {
        self.komorebi.workspaces()
    }

    /// (YASB's version, komorebi's version), once they are known. The first call starts looking them up in the background.
    pub fn versions(&mut self, ctx: &egui::Context) -> Option<(String, String)> {
        if !self.versions_started {
            self.versions_started = true;
            let slot = self.versions.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                *slot.lock().unwrap() = Some((about::version("yasbc"), about::version("komorebic")));
                ctx.request_repaint();
            });
        }
        self.versions.lock().unwrap().clone()
    }

    fn say(&mut self, message: &str) {
        self.status = Some(Toast { message: message.to_string(), actions: Vec::new(), age: 0.0 });
    }

    /// Report the result of a write.
    fn done(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => {}
            Err(e) => self.say(&format!("Could not change that: {e}")),
        }
    }

    fn has_changes(&self) -> bool {
        self.cfg.dirty || self.komorebi.dirty || self.tacky.dirty || !self.staged.is_empty()
    }

    /// Write the waiting changes. YASB picks up config.yaml by itself; komorebi is told to load its file.
    fn apply(&mut self, ctx: &egui::Context) {
        let mut problems = Vec::new();
        let komorebi_changed = self.komorebi.dirty;
        if self.tacky.dirty {
            if let Err(e) = self.tacky.apply() {
                problems.push(format!("Tacky Borders: {e}"));
            }
        }
        if self.cfg.dirty {
            if let Err(e) = self.cfg.apply() {
                problems.push(format!("config.yaml: {e}"));
            }
        }
        if komorebi_changed {
            if let Err(e) = self.komorebi.apply() {
                problems.push(format!("komorebi.json: {e}"));
            }
        }
        if !problems.is_empty() {
            self.say(&format!("Could not apply: {}", problems.join("; ")));
        } else if !self.staged.is_empty() {
            // the wallpaper and the Lucent theme's settings: done in the background (the programs and the accent take a moment)
            { self.applied_at = Some(Instant::now()); self.glyph_seed = self.glyph_seed.wrapping_mul(1664525).wrapping_add(1013904223); }
            let staged = std::mem::take(&mut self.staged);
            let lock = wallpaper::read_setting("lock_screen").is_some_and(|v| v == "1");
            spawn(self, ctx, "Applying...", move || staged::run(staged, komorebi_changed, lock));
        } else if komorebi_changed {
            { self.applied_at = Some(Instant::now()); self.glyph_seed = self.glyph_seed.wrapping_mul(1664525).wrapping_add(1013904223); }
            run_hidden(self, ctx, "Applying...", "komorebic", &["reload-configuration"], "Applied: the bar and komorebi use the new settings");
        } else {
            { self.applied_at = Some(Instant::now()); self.glyph_seed = self.glyph_seed.wrapping_mul(1664525).wrapping_add(1013904223); }
            self.say("Applied: the bar uses the new settings");
        }
    }

    fn discard(&mut self) {
        self.cfg.discard();
        self.komorebi.discard();
        self.tacky.discard();
        self.staged = Default::default();
        self.say("Changes discarded");
    }

    // --- rows that edit config.yaml ---------------------------------------------------------------------------------------------------

    fn yaml_toggle(&mut self, ui: &mut Ui, name: &str, hint: &str, path: &[&str]) {
        let mut v = self.cfg.bool(path, false);
        if row(ui, name, hint, |ui| toggle(ui, &mut v)).changed() {
            let r = self.cfg.set_bool(path, v);
            self.done(r);
        }
    }

    fn yaml_slider(&mut self, ui: &mut Ui, name: &str, hint: &str, path: &[&str], min: i64, max: i64, suffix: &str) {
        let mut v = self.cfg.int(path, min);
        let was = v;
        let response = row(ui, name, hint, |ui| slider(ui, &mut v, min, max, suffix));
        if v != was && !response.dragged() {
            let r = self.cfg.set_int(path, v);
            self.done(r);
        }
    }

    fn yaml_text(&mut self, ui: &mut Ui, name: &str, hint: &str, path: &[&str], width: f32) {
        let id = egui::Id::new(("yaml", path.join(".")));
        let current = self.cfg.string(path);
        let mut v: String = ui.data_mut(|d| d.get_temp::<(String, String)>(id)).filter(|(stored, _)| *stored == current).map(|(_, edited)| edited).unwrap_or_else(|| current.clone());
        let committed = row(ui, name, hint, |ui| input(ui, &mut v, width));
        ui.data_mut(|d| d.insert_temp(id, (current.clone(), v.clone())));
        if committed && v != current {
            let r = self.cfg.set_text(path, &v);
            self.done(r);
        }
    }

    /// A label or icon: shown and typed as UTF-16 escapes (`\udb81\ude33`), like in YASB's config.yaml.
    fn yaml_glyph(&mut self, ui: &mut Ui, name: &str, hint: &str, path: &[&str], width: f32) {
        let id = egui::Id::new(("yaml-glyph", path.join(".")));
        let current = glyph::escape(&self.cfg.string(path));
        let mut v: String = ui.data_mut(|d| d.get_temp::<(String, String)>(id)).filter(|(stored, _)| *stored == current).map(|(_, edited)| edited).unwrap_or_else(|| current.clone());
        let committed = row(ui, name, hint, |ui| input_without_tags(ui, &mut v, width));
        ui.data_mut(|d| d.insert_temp(id, (current.clone(), v.clone())));
        let typed = glyph::escape(&glyph::unescape(&v));
        if committed && typed != current {
            let r = self.cfg.set_text(path, &typed);
            self.done(r);
        }
    }

    fn yaml_choice(&mut self, ui: &mut Ui, name: &str, hint: &str, path: &[&str], options: &[&str]) {
        let mut v = self.cfg.string(path);
        if row(ui, name, hint, |ui| segmented(ui, options, &mut v)) {
            let r = self.cfg.set_text(path, &v);
            self.done(r);
        }
    }

    /// Messages are shown as a toast (see `toast`), not in the page.
    fn status_note(&mut self, _ui: &mut Ui) {}

    /// The latest message, as a notification pill that drops in from the top edge and leaves again ("Komorebi started", "Applied...").
    fn toast(&mut self, ui: &mut Ui, window: Rect) {
        let ctx = ui.ctx().clone();
        let Some(toast) = self.status.as_ref() else { return };
        let (message, actions, mut age) = (toast.message.clone(), toast.actions.clone(), toast.age);
        // longer when there is something to choose
        let life = if message.starts_with("Could not") { 9.0 } else { 5.6 } + if actions.is_empty() { 0.0 } else { 4.0 };
        if age > life {
            self.status = None;
            return;
        }
        ctx.request_repaint();
        let enter = (age / 0.35).min(1.0);
        let leave = ((life - age) / 0.3).min(1.0);
        let shown = (1.0 - (1.0 - enter).powi(3)) * leave; // eases in, then fades away
        let a = shown.clamp(0.0, 1.0);

        // the buttons first: how wide they are decides how wide the text may be
        let labels: Vec<_> = actions.iter().map(|(_, label)| ui.painter().layout_no_wrap(label.clone(), FontId::new(12.5, FontFamily::Proportional), BG)).collect();
        let buttons_w: f32 = labels.iter().map(|g| g.size().x + 26.0 + 8.0).sum();
        let galley = ui.painter().layout(message, FontId::new(12.5, mono()), TEXT, (window.width() - 170.0 - buttons_w).max(160.0)); // long messages wrap
        let height = galley.size().y.max(if actions.is_empty() { 0.0 } else { 28.0 }) + 18.0 + 9.0; // the bar sits inside, along the bottom
        let size = Vec2::new(30.0 + galley.size().x + if actions.is_empty() { 16.0 } else { 14.0 + buttons_w + 4.0 }, height);
        let top = window.min.y + 14.0 - (1.0 - shown) * 26.0;
        let rect = Rect::from_min_size(Pos2::new(window.center().x - size.x / 2.0, top), size);

        // the pointer on the message (or its bar) stops the clock, and keeps it from fading
        let over = rect.expand2(Vec2::new(8.0, 12.0)).contains(ctx.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::new(-1.0, -1.0)));
        if over {
            age = age.min(life - 0.4);
        } else {
            age += ctx.input(|i| i.stable_dt).min(0.1);
        }

        let p = ui.painter();
        p.rect(rect, CornerRadius::same((size.y / 2.0).min(26.0) as u8), CARD.gamma_multiply(a), Stroke::new(1.0, LINE_STRONG.gamma_multiply(a)), StrokeKind::Inside);
        p.circle_filled(Pos2::new(rect.min.x + 17.0, rect.center().y - 4.5), 3.0, lit().gamma_multiply(a));
        p.galley(Pos2::new(rect.min.x + 30.0, rect.center().y - 4.5 - galley.size().y / 2.0), galley.clone(), TEXT.gamma_multiply(a));
        // the bar under it: the time that is left
        let track = Rect::from_min_size(Pos2::new(rect.min.x + 22.0, rect.max.y - 13.0), Vec2::new(rect.width() - 44.0, 3.0));
        p.rect_filled(track, CornerRadius::same(2), LINE.gamma_multiply(a));
        let left = (1.0 - age / life).clamp(0.0, 1.0);
        p.rect_filled(Rect::from_min_size(track.min, Vec2::new(track.width() * left, 3.0)), CornerRadius::same(2), glow().gamma_multiply(a));

        let mut clicked: Option<(String, String)> = None;
        let mut x = rect.max.x - 10.0 - buttons_w + 8.0;
        for ((id, label), g) in actions.iter().zip(labels) {
            let button = Rect::from_min_size(Pos2::new(x, rect.center().y - 4.5 - 14.0), Vec2::new(g.size().x + 26.0, 28.0));
            let response = ui.interact(button, egui::Id::new(("toast-action", id.as_str())), Sense::click());
            let hover = ease(ui, response.id.with("hover"), response.hovered());
            let fill = blend(lit(), Color32::from_rgb(210, 210, 210), hover * 0.7).gamma_multiply(a);
            ui.painter().rect_filled(button, CornerRadius::same(14), fill);
            ui.painter().galley(Pos2::new(button.min.x + 13.0, button.center().y - g.size().y / 2.0), g, BG);
            if response.clicked() {
                clicked = Some((id.clone(), label.clone()));
            }
            x += button.width() + 8.0;
        }
        if let Some(t) = self.status.as_mut() {
            t.age = age;
        }
        if let Some((id, label)) = clicked {
            sound::play(sound::Sound::Click);
            let working = format!("{label}...");
            spawn(self, &ctx, &working, move || lucent::restart(&id));
        }
    }

    /// How far the Apply button has lit up (0 grey, 1 white, eased), and whether it is only confirming an apply.
    fn apply_state(&mut self, ui: &mut Ui) -> (f32, bool) {
        let ctx = ui.ctx().clone();
        let dirty = self.has_changes();
        let confirming = self.applied_at.is_some_and(|t| t.elapsed() < Duration::from_millis(2200)) && !dirty;
        let t = ctx.animate_bool_with_time(egui::Id::new("apply-pill"), dirty, 0.25);
        (1.0 - (1.0 - t).powi(3), confirming)
    }

    /// The Apply button, always in the pill: grey while there is nothing to apply, white when there is (with a small x to discard the changes).
    /// After applying it says so for a moment.
    fn apply_pill(&mut self, ui: &mut Ui, _window: Rect) {
        let ctx = ui.ctx().clone();
        let (t, confirming) = (self.apply_t, self.apply_confirming);
        let dirty = self.has_changes();
        let r = self.apply_rect;
        if confirming {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        let mut apply_now = false;
        let mut discard_now = false;
        let main = ui.interact(r, egui::Id::new("apply-button"), Sense::click());
        let hover = if dirty { ease(ui, main.id.with("hover"), main.hovered()) } else { 0.0 };
        let fill = blend(blend(CARD_HOVER, Color32::from_gray(34), if confirming { 1.0 } else { 0.0 }), blend(lit(), Color32::from_gray(225), hover * 0.7), t);
        let edge = blend(LINE_STRONG, lit(), t);
        ui.painter().rect(r, CornerRadius::same(20), fill, Stroke::new(1.0, edge), StrokeKind::Inside);
        let label = if confirming { "Applied" } else { "Apply" };
        let ink = if confirming { WHITE } else { blend(DIM, BG, t) };
        let galley = ui.painter().layout_no_wrap(label.to_string(), FontId::new(13.0, FontFamily::Proportional), ink);
        // with changes waiting, the x and the text sit together in the middle of the button; without, the text is alone in the middle
        let group_w = 24.0 + 8.0 + galley.size().x;
        let group_x = r.center().x - group_w / 2.0;
        let text_x = (r.center().x - galley.size().x / 2.0) * (1.0 - t) + (group_x + 32.0) * t;
        ui.painter().galley(Pos2::new(text_x, r.center().y - galley.size().y / 2.0), galley, ink);
        if dirty {
            ctx.request_repaint();
            if main.clicked() {
                apply_now = true;
            }
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            // the x: throw the changes away
            let xc = Pos2::new(group_x + 12.0, r.center().y);
            let xr = ui.interact(Rect::from_center_size(xc, Vec2::new(40.0, 40.0)), egui::Id::new("apply-discard"), Sense::click());
            let xh = ease(ui, xr.id.with("hover"), xr.hovered());
            ui.painter().circle_filled(xc, 12.0, BG.gamma_multiply((0.12 + 0.16 * xh) * t));
            let s = Stroke::new(1.9, BG.gamma_multiply(t));
            ui.painter().line_segment([xc + Vec2::new(-4.2, -4.2), xc + Vec2::new(4.2, 4.2)], s);
            ui.painter().line_segment([xc + Vec2::new(-4.2, 4.2), xc + Vec2::new(4.2, -4.2)], s);
            if xr.clicked() {
                discard_now = true;
            }
            xr.on_hover_text("Discard the changes");
        }
        if apply_now && !discard_now {
            self.apply(&ctx);
        }
        if discard_now {
            self.discard();
        }
    }

    /// The floating pill at the bottom: one icon per page, frosted glass. The name of the page under the pointer shows above it.
    /// Seconds since the last thing the glyphs react to: a page change or an Apply.
    fn glyph_since(&self) -> f32 {
        let page = self.page_since.elapsed().as_secs_f32();
        self.applied_at.map(|t| t.elapsed().as_secs_f32().min(page)).unwrap_or(page)
    }

    /// A round button at the bottom right that appears once the page is scrolled down, and takes you back to the top.
    fn back_to_top(&mut self, ui: &mut Ui, window: Rect) {
        let max: f32 = ui.ctx().data(|d| d.get_temp(egui::Id::new("scroll-max"))).unwrap_or(0.0);
        // while the search is open the pill grows and needs the room
        let searching = ui.ctx().memory(|m| m.has_focus(egui::Id::new("search-box"))) || !self.search.query.is_empty();
        for down in [false, true] {
            let show = !searching && self.page != Page::Home && if down { max - self.scroll_y > 240.0 } else { self.scroll_y > 240.0 };
            let id = egui::Id::new(("scroll-jump", down));
            let t = ui.ctx().animate_bool_with_time(id, show, 0.2);
            if t <= 0.01 {
                continue;
            }
            // on the pill's row: the one that goes up on the right, the one that goes down on the left, as far from the edge as the pill is from the bottom
            let gap = (window.max.y - self.nav_rect.max.y).max(14.0);
            let slide = (1.0 - t) * 10.0;
            let x = if down { window.min.x + gap + 23.0 + slide } else { window.max.x - gap - 23.0 - slide };
            let centre = Pos2::new(x, self.nav_rect.center().y);
            let rect = Rect::from_center_size(centre, Vec2::splat(46.0));
            let response = ui.interact(rect, id.with("button"), Sense::click());
            let hover = ease(ui, response.id.with("hover"), response.hovered());
            let a = t;
            ui.painter().circle(centre, 23.0, blend(CARD, CARD_HOVER, hover).gamma_multiply(a), Stroke::new(1.0, blend(LINE_STRONG, MUTED, hover).gamma_multiply(a)));
            let ink = blend(MUTED, WHITE, hover).gamma_multiply(a);
            let s = Stroke::new(1.8, ink);
            let d = if down { -1.0 } else { 1.0 }; // the arrow and its bar are mirrored
            ui.painter().line_segment([centre + Vec2::new(-6.0, 2.5 * d), centre + Vec2::new(0.0, -3.5 * d)], s);
            ui.painter().line_segment([centre + Vec2::new(0.0, -3.5 * d), centre + Vec2::new(6.0, 2.5 * d)], s);
            ui.painter().line_segment([centre + Vec2::new(-6.0, 8.0 * d), centre + Vec2::new(6.0, 8.0 * d)], Stroke::new(1.4, ink.gamma_multiply(0.6)));
            if response.clicked() {
                if down {
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("go-bottom"), true));
                } else {
                    self.to_top = true;
                }
                sound::play(sound::Sound::Nav);
            }
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
    }

    /// The one floating pill at the bottom: Home | search | the pages | About | (Apply when there are changes). The name of the page under the pointer shows above it.
    fn sidebar(&mut self, ui: &mut Ui, window: Rect) {
        let ctx = ui.ctx().clone();
        let (item, height, pad, sep) = (50.0, 62.0, 8.0, 17.0);
        // the search box is small until it is clicked, then it opens out
        let open = ctx.memory(|m| m.has_focus(egui::Id::new("search-box"))) || !self.search.query.is_empty();
        let grow = ctx.animate_bool_with_time(egui::Id::new("search-open"), open, 0.24);
        let grow = 1.0 - (1.0 - grow).powi(3);
        let search_w = 96.0 + 56.0 * grow;
        let (apply_t, apply_confirming) = self.apply_state(ui);
        self.apply_t = apply_t;
        self.apply_confirming = apply_confirming;
        // Apply is as wide as the search box, so the bar is even
        let width = pad * 2.0 + item * PAGES.len() as f32 + search_w * 2.0 + sep * 4.0;
        let pill = Rect::from_center_size(Pos2::new(window.center().x, window.max.y - 22.0 - height / 2.0), Vec2::new(width, height));
        self.nav_rect = pill;
        // where everything sits, left to right
        let mut x = pill.min.x + pad;
        let mut centres = vec![0.0f32; PAGES.len()];
        centres[0] = x + item / 2.0;
        x += item;
        let sep1 = x + sep / 2.0;
        x += sep;
        let search_left = x;
        x += search_w;
        let sep2 = x + sep / 2.0;
        x += sep;
        for (i, c) in centres.iter_mut().enumerate().take(PAGES.len() - 1).skip(1) {
            *c = x + item * (i as f32 - 0.5);
        }
        x += item * (PAGES.len() - 2) as f32;
        let sep3 = x + sep / 2.0;
        x += sep;
        self.apply_rect = Rect::from_min_size(Pos2::new(x, pill.center().y - 20.0), Vec2::new(search_w, 40.0));
        x += search_w;
        let sep4 = x + sep / 2.0;
        x += sep;
        centres[PAGES.len() - 1] = x + item / 2.0;
        self.search_rect = Rect::from_min_size(Pos2::new(search_left, pill.center().y - 20.0), Vec2::new(search_w, 40.0));
        frosted(ui.painter(), pill, 31.0);
        let since = self.glyph_since();
        if glyph_edge(ui, pill, 31.0, since, 1.0, self.glyph_seed) {
            ctx.request_repaint();
        }
        for (sx, a) in [(sep1, 1.0), (sep2, 1.0), (sep3, 1.0), (sep4, 1.0f32)] {
            if a > 0.01 {
                ui.painter().line_segment([Pos2::new(sx, pill.center().y - 13.0), Pos2::new(sx, pill.center().y + 13.0)], Stroke::new(1.0, LINE_STRONG.gamma_multiply(a)));
            }
        }
        let mut hovered_name = None;
        // the white circle slides to the page you pick
        let target = centres[PAGES.iter().position(|(p, ..)| *p == self.page).unwrap_or(0)];
        let circle_x = ctx.animate_value_with_time(egui::Id::new("nav-circle"), target, 0.24);
        ui.painter().circle_filled(Pos2::new(circle_x, pill.center().y), 20.0, lit());
        for (i, (page, icon, name, _)) in PAGES.iter().enumerate() {
            let centre = Pos2::new(centres[i], pill.center().y);
            let rect = Rect::from_center_size(centre, Vec2::splat(44.0));
            let response = ui.interact(rect, egui::Id::new(("nav", *name)), Sense::click());
            let under = (1.0 - (circle_x - centre.x).abs() / 34.0).clamp(0.0, 1.0); // how much of the white circle is behind this icon
            let hover = ease(ui, response.id.with("hover"), response.hovered());
            if hover > 0.0 && under < 1.0 {
                ui.painter().circle_filled(centre, 20.0, Color32::from_white_alpha((28.0 * hover * (1.0 - under)) as u8));
            }
            let idle = blend(Color32::from_white_alpha(165), WHITE, hover);
            icons::paint(ui.painter(), *icon, centre, blend(idle, BG, under));
            if response.hovered() {
                hovered_name = Some((*name, centre.x));
            }
            if response.clicked() && self.page != *page {
                self.page = *page;
                sound::play(sound::Sound::Nav);
            }
        }
        if let Some((name, x)) = hovered_name {
            let galley = ui.painter().layout_no_wrap(name.to_string(), FontId::new(12.5, mono()), WHITE);
            let tag = Rect::from_center_size(Pos2::new(x, pill.min.y - 22.0), galley.size() + Vec2::new(24.0, 12.0));
            frosted(ui.painter(), tag, tag.height() / 2.0);
            ui.painter().galley(Pos2::new(tag.min.x + 12.0, tag.center().y - galley.size().y / 2.0), galley, WHITE);
        }
    }

    fn content(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let (page, _, name, hint) = PAGES.iter().find(|(p, ..)| *p == self.page).copied().unwrap();
        // the result of something running in the background
        if let Some(job) = self.job.clone() {
            let result = job.lock().unwrap().take();
            match result {
                Some(m) => {
                    self.job = None;
                    self.say(&m);
                    // programs that only read their theme when they start: a button to restart them
                    let offers = lucent::take_offers();
                    if let Some(t) = self.status.as_mut() {
                        t.actions = offers;
                    }
                }
                None => ctx.request_repaint_after(Duration::from_millis(300)),
            }
        }
        if self.shown_page != self.page {
            self.shown_page = self.page;
            self.page_since = Instant::now();
            self.glyph_seed = self.glyph_seed.wrapping_mul(1664525).wrapping_add(1013904223) ^ (self.page_since.elapsed().subsec_nanos());
        }
        let fade = (self.page_since.elapsed().as_secs_f32() / 0.22).min(1.0);
        let fade = 1.0 - (1.0 - fade).powi(3); // eases out
        if fade < 1.0 {
            ctx.request_repaint();
        }
        search::set_page(&ctx, PAGES.iter().position(|p| p.0 == page).unwrap_or(0));
        ui.set_opacity(fade);
        let go_top = std::mem::take(&mut self.to_top);
        let go_bottom: bool = ctx.data_mut(|d| d.remove_temp(egui::Id::new("go-bottom"))).unwrap_or(false);
        let scrolled = egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if go_top {
                ui.scroll_to_cursor(Some(egui::Align::TOP)); // the cursor is still at the top of the page here, and the scroll is animated
            }
            egui::Frame::new().inner_margin(egui::Margin { left: 8, right: 32, top: 0, bottom: 0 }).show(ui, |ui| {
                ui.set_max_width(ui.available_width().min(980.0));
                if page == Page::Home {
                    // Home is one screen of cards: a slim row with the reload button, then the grid, and nothing after it (no scrolling)
                    ui.allocate_ui_with_layout(Vec2::new(ui.available_width(), 36.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.status_note(ui);
                    });
                    ui.add_space(6.0);
                    home::show(self, ui);
                    return;
                }
                // the page's header, a small hero: an animated mark, the name big and what the page is for, all centred
                let n = PAGES.iter().position(|p| p.0 == page).unwrap_or(0);
                let (head, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 228.0), Sense::hover());
                let cx = head.center().x;
                let logo_at = Pos2::new(cx, head.min.y + 84.0);
                // the pointer over the logo (a generous zone around it): the logo wakes up
                let zone = Rect::from_center_size(logo_at, Vec2::new(260.0, 150.0));
                let ctx2 = ui.ctx().clone();
                let hovered = ui.rect_contains_pointer(zone);
                let hover = ctx2.animate_bool_with_time(egui::Id::new("hero-hover"), hovered, 0.35);
                let dt = ctx2.input(|i| i.stable_dt).min(0.1);
                // time runs faster while it is hovered, so the animation speeds up and slows down smoothly without jumping
                let time = ctx2.data_mut(|d| {
                    let t: &mut f32 = d.get_temp_mut_or_insert_with(egui::Id::new("hero-phase"), || 0.0);
                    *t += dt * (1.0 + 2.2 * hover);
                    *t
                });
                // where the pointer is, relative to the logo (-1 to 1): the logo leans towards it
                let look = ctx2.input(|i| i.pointer.hover_pos()).map(|m| Vec2::new(((m.x - logo_at.x) / 200.0).clamp(-1.0, 1.0), ((m.y - logo_at.y) / 120.0).clamp(-1.0, 1.0))).unwrap_or(Vec2::ZERO) * hover;
                ctx2.request_repaint_after(Duration::from_millis(if hovered || hover > 0.01 { 16 } else { 33 }));
                page_logo(ui.painter(), logo_at, time, page, hover, look);
                ui.painter().text(Pos2::new(cx, head.min.y + 160.0), egui::Align2::CENTER_CENTER, name.to_uppercase(), FontId::new(38.0, heading()), WHITE);
                ui.painter().text(Pos2::new(cx, head.min.y + 200.0), egui::Align2::CENTER_CENTER, hint, FontId::new(12.0, mono()), MUTED);
                ui.add_space(14.0);
                if page != Page::Home {
                    theme::section_pills(ui, PAGES.iter().position(|p| p.0 == page).unwrap_or(0));
                }
                match page {
                    Page::Home => {}
                    Page::Bar => bar::show(self, ui),
                    Page::Widgets => widgets::show(self, ui),
                    Page::Komorebi => komorebi::show(self, ui),
                    Page::Tacky => tacky::show(self, ui),
                    Page::Wallpaper => wallpaper::show(self, ui),
                    Page::Templates => templates::show(self, ui),
                    Page::About => about::show(self, ui),
                }
                ui.add_space(53.0); // room for the pill (the last card ends about 28 px above it)
                if go_bottom {
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                }
            });
        });
        self.scroll_y = scrolled.state.offset().y;
        let max = (scrolled.content_size.y - scrolled.inner_rect.height()).max(0.0);
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("scroll-max"), max));
    }
}

/// Run something slow off the window's thread; what it returns is shown as the status when it is done.
pub fn spawn(app: &mut App, ctx: &egui::Context, working: &str, work: impl FnOnce() -> String + Send + 'static) {
    let slot = Arc::new(Mutex::new(None));
    let out = slot.clone();
    let ctx = ctx.clone();
    app.job = Some(slot);
    app.say(working);
    std::thread::spawn(move || {
        let message = work();
        *out.lock().unwrap() = Some(message);
        ctx.request_repaint();
    });
}

/// Run a command without a window, off the window's thread; `done` is shown when it succeeded.
pub fn run_hidden(app: &mut App, ctx: &egui::Context, working: &str, program: &str, args: &[&str], done: &str) {
    use std::os::windows::process::CommandExt;
    let slot = Arc::new(Mutex::new(None));
    let out = slot.clone();
    let (ctx, program, args, done) = (ctx.clone(), program.to_string(), args.iter().map(|s| s.to_string()).collect::<Vec<_>>(), done.to_string());
    app.job = Some(slot);
    app.say(working);
    std::thread::spawn(move || {
        let ok = std::process::Command::new(&program).args(&args).creation_flags(0x0800_0000).output().map(|o| o.status.success()).unwrap_or(false);
        *out.lock().unwrap() = Some(if ok { done } else { format!("{program} did not answer: is it running?") });
        ctx.request_repaint();
    });
}

/// The two circles in the top right corner: minimise and close. A sign appears in the one under the pointer.
/// A soft round glow: a radial gradient from `colour` in the middle to nothing at the edge.
fn glow_disc(p: &egui::Painter, c: Pos2, r: f32, colour: Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(c, colour);
    let n = 56;
    for i in 0..=n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(c + Vec2::new(a.cos(), a.sin()) * r, Color32::TRANSPARENT);
    }
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 2 + i);
    }
    p.add(egui::Shape::mesh(mesh));
}

/// A point on the edge of a rectangle, `t` (0 to 1) of the way round it from the top left.
fn along(rect: Rect, t: f32) -> Pos2 {
    let (w, h) = (rect.width(), rect.height());
    let mut d = t.rem_euclid(1.0) * 2.0 * (w + h);
    if d < w { return Pos2::new(rect.min.x + d, rect.min.y); }
    d -= w;
    if d < h { return Pos2::new(rect.max.x, rect.min.y + d); }
    d -= h;
    if d < w { return Pos2::new(rect.max.x - d, rect.max.y); }
    d -= w;
    Pos2::new(rect.min.x, rect.max.y - d)
}

/// The mark in the middle of each page's header: its own little animation, on a soft glow of the accent.
fn page_logo(p: &egui::Painter, c0: Pos2, t: f32, page: Page, hover: f32, look: Vec2) {
    let l = lit();
    let [r, g, b, _] = l.to_array();
    // the glow follows the pointer a little and swells; the logo itself leans less, so there is depth
    let c = c0 + look * 7.0;
    glow_disc(p, c0 + look * 16.0, 82.0 + 26.0 * hover, Color32::from_rgba_unmultiplied(r, g, b, (26.0 + 24.0 * hover) as u8));
    glow_disc(p, c0 + look * 11.0, 48.0 + 14.0 * hover, Color32::from_rgba_unmultiplied(r, g, b, (46.0 + 36.0 * hover) as u8));
    if hover > 0.01 {
        // rings that spread out from the logo, one after another
        for k in 0..3 {
            let phase = (t * 0.55 + k as f32 / 3.0).fract();
            p.circle_stroke(c, 30.0 + phase * 58.0, Stroke::new(1.3, l.gamma_multiply(hover * (1.0 - phase) * 0.55)));
        }
        // and a few sparks drifting round it
        for k in 0..7 {
            let a = t * (0.9 + 0.13 * k as f32) + k as f32 * 0.9;
            let rad = 52.0 + 9.0 * (t * 1.7 + k as f32).sin();
            p.circle_filled(c + Vec2::new(a.cos(), a.sin() * 0.7) * rad, 1.6 + 0.8 * ((t * 3.0 + k as f32).sin() * 0.5 + 0.5), WHITE.gamma_multiply(hover * 0.55));
        }
    }
    let line = |a: f32| Stroke::new(1.7, WHITE.gamma_multiply(a));
    let pulse = |speed: f32, phase: f32| 0.5 + 0.5 * (t * speed + phase).sin();
    let big = 1.0 + 0.18 * hover; // what lights up grows a little under the pointer
    match page {
        Page::Bar => {
            // a bar with three dots; the light moves along them
            p.rect_stroke(Rect::from_center_size(c, Vec2::new(70.0, 28.0)), CornerRadius::same(14), line(0.9), StrokeKind::Inside);
            let on = ((t * 1.3) as usize) % 3;
            for i in 0..3 {
                let x = c.x + (i as f32 - 1.0) * 19.0;
                if i == on {
                    p.circle_filled(Pos2::new(x, c.y), 5.0 + pulse(6.0, 0.0), l);
                } else {
                    p.circle_filled(Pos2::new(x, c.y), 3.0, WHITE.gamma_multiply(0.55));
                }
            }
        }
        Page::Widgets => {
            // four tiles; one after another they light up
            let lit_one = ((t * 1.1) as usize) % 4;
            for (i, (dx, dy)) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].into_iter().enumerate() {
                let rect = Rect::from_center_size(c + Vec2::new(dx * 15.0, dy * 15.0), Vec2::splat(25.0));
                if i == lit_one {
                    p.rect_filled(rect, CornerRadius::same(7), l.gamma_multiply(0.55 + 0.35 * pulse(5.0, 0.0)));
                }
                p.rect_stroke(rect, CornerRadius::same(7), line(0.9), StrokeKind::Inside);
            }
        }
        Page::Komorebi => {
            // a window that tiles itself in a new way every few seconds
            let frame = Rect::from_center_size(c, Vec2::new(78.0, 56.0));
            p.rect_stroke(frame, CornerRadius::same(7), line(0.95), StrokeKind::Inside);
            let layout = ((t * 0.45) as usize) % 3;
            let fade = (1.0 - ((t * 0.45).fract() - 0.8).max(0.0) * 5.0).clamp(0.0, 1.0);
            let ink = Stroke::new(1.5, l.gamma_multiply(fade));
            match layout {
                0 => {
                    p.line_segment([Pos2::new(c.x, frame.min.y), Pos2::new(c.x, frame.max.y)], ink);
                }
                1 => {
                    p.line_segment([Pos2::new(c.x - 6.0, frame.min.y), Pos2::new(c.x - 6.0, frame.max.y)], ink);
                    p.line_segment([Pos2::new(c.x - 6.0, c.y), Pos2::new(frame.max.x, c.y)], ink);
                }
                _ => {
                    p.line_segment([Pos2::new(c.x, frame.min.y), Pos2::new(c.x, frame.max.y)], ink);
                    p.line_segment([Pos2::new(frame.min.x, c.y), Pos2::new(frame.max.x, c.y)], ink);
                }
            }
        }
        Page::Tacky => {
            // a window with a bright light running round its border
            let rect = Rect::from_center_size(c, Vec2::new(70.0, 52.0));
            p.rect_stroke(rect, CornerRadius::same(14), Stroke::new(2.4, WHITE.gamma_multiply(0.35)), StrokeKind::Inside);
            for k in 0..14 {
                let tail = k as f32 * 0.012;
                p.circle_filled(along(rect.shrink(1.0), t * 0.22 - tail), 3.4 - k as f32 * 0.2, l.gamma_multiply(1.0 - k as f32 / 14.0));
            }
        }
        Page::Wallpaper => {
            // a picture with a sun drifting over hills
            let frame = Rect::from_center_size(c, Vec2::new(76.0, 56.0));
            p.rect_stroke(frame, CornerRadius::same(9), line(0.95), StrokeKind::Inside);
            let sun = Pos2::new(c.x + (t * 0.6).sin() * 20.0, c.y - 10.0 + (t * 1.2).cos() * 3.0);
            p.circle_filled(sun, 5.5, l);
            let hills = vec![Pos2::new(frame.min.x + 3.0, frame.max.y - 6.0), Pos2::new(c.x - 10.0, c.y + 4.0), Pos2::new(c.x + 4.0, frame.max.y - 12.0), Pos2::new(c.x + 14.0, c.y + 8.0), Pos2::new(frame.max.x - 3.0, frame.max.y - 6.0)];
            p.add(egui::Shape::line(hills, line(0.8)));
        }
        Page::Templates => {
            // three rings that slowly trade places, their colours mixing
            for k in 0..3 {
                let a = t * 0.45 + k as f32 * std::f32::consts::TAU / 3.0;
                let at = c + Vec2::new(a.cos(), a.sin()) * 10.0;
                p.circle_stroke(at, 17.0, Stroke::new(1.7, blend(WHITE, l, k as f32 / 2.0).gamma_multiply(0.9)));
            }
            p.circle_filled(c, 2.4, WHITE);
        }
        Page::About => {
            // an i inside a ring that breathes
            let breath = pulse(2.0, 0.0);
            p.circle_stroke(c, 25.0 + breath * 5.0, Stroke::new(1.4, l.gamma_multiply(0.7 - 0.5 * breath)));
            p.circle_stroke(c, 25.0, line(0.95));
            p.circle_filled(Pos2::new(c.x, c.y - 9.0), 2.6, WHITE);
            p.line_segment([Pos2::new(c.x, c.y - 2.0), Pos2::new(c.x, c.y + 11.0)], Stroke::new(2.4, WHITE));
        }
        Page::Home => {}
    }
}

/// The small bento mark: one tall tile beside two short ones.
fn bento_mark(p: &egui::Painter, c: Pos2, colour: Color32) {
    let stroke = Stroke::new(1.5, colour);
    p.rect_stroke(Rect::from_min_size(Pos2::new(c.x - 9.0, c.y - 9.0), Vec2::new(10.0, 18.0)), CornerRadius::same(3), stroke, StrokeKind::Inside);
    p.rect_filled(Rect::from_min_size(Pos2::new(c.x + 3.0, c.y - 9.0), Vec2::new(6.0, 8.0)), CornerRadius::same(2), colour);
    p.rect_stroke(Rect::from_min_size(Pos2::new(c.x + 3.0, c.y + 1.0), Vec2::new(6.0, 8.0)), CornerRadius::same(2), stroke, StrokeKind::Inside);
}

/// Height of the title bar band above the page (the window buttons sit in the middle of it).
const TITLE_BAR: f32 = 56.0;

fn window_controls(ui: &mut Ui, ctx: &egui::Context, window: Rect) {
    let mut x = window.max.x - 34.0;
    for (i, command) in [egui::ViewportCommand::Close, egui::ViewportCommand::Minimized(true)].into_iter().enumerate() {
        let centre = Pos2::new(x, window.min.y + TITLE_BAR / 2.0);
        let rect = Rect::from_center_size(centre, Vec2::splat(38.0));
        let response = ui.interact(rect, egui::Id::new(("window-button", i)), Sense::click());
        let t = ui.ctx().animate_bool_with_time(response.id, response.hovered(), 0.15);
        let r = 11.0 + 1.5 * t;
        let p = ui.painter();
        if i == 0 {
            p.circle_filled(centre, r, blend(MUTED, WHITE, t));
            let k = 3.6;
            let s = Stroke::new(1.6, BG.gamma_multiply(t));
            p.line_segment([centre + Vec2::new(-k, -k), centre + Vec2::new(k, k)], s);
            p.line_segment([centre + Vec2::new(-k, k), centre + Vec2::new(k, -k)], s);
        } else {
            p.circle_stroke(centre, r, Stroke::new(1.8, blend(MUTED, WHITE, t)));
            p.line_segment([centre + Vec2::new(-3.8, 0.0), centre + Vec2::new(3.8, 0.0)], Stroke::new(1.6, WHITE.gamma_multiply(t)));
        }
        if response.clicked() {
            ctx.send_viewport_cmd(command);
        }
        x -= 40.0;
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // transparent: the rounded shape below is the window
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if std::env::var("YASB_SETTINGS_TIMING").is_ok() && !self.timed {
            self.timed = true;
            let _ = std::fs::write(std::env::temp_dir().join("yasb-settings-timing.txt"), format!("first frame after {} ms", START.get().map(|s| s.elapsed().as_millis()).unwrap_or(0)));
        }
        // the colours this window borrows from the theme (looked up again now and then: the accent can change)
        if self.tint_at.elapsed() > Duration::from_millis(1500) {
            self.tint_at = Instant::now();
            theme::set_tint(lucent::tint());
        }
        self.cfg.reload_if_changed(); // something else (YASB, an editor) changed the file
        self.komorebi.reload_if_changed();
        self.tacky.reload_if_changed();
        ctx.request_repaint_after(Duration::from_millis(1000));
        // centre the window on the monitor the pointer was on (twice: the second time the real window size and scale are surely known)
        if self.centred < 2 {
            if let Some(outer) = ctx.input(|i| i.viewport().outer_rect) {
                if let Some(at) = centred_on_pointer_monitor(outer.size(), ctx.pixels_per_point()) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(at));
                }
                self.centred += 1;
            }
        }
        // the wheel over the pill turns the pages (and does not scroll the page under it)
        if ctx.input(|i| i.pointer.hover_pos()).is_some_and(|p| self.nav_rect.expand(6.0).contains(p) && !self.search_rect.expand(4.0).contains(p)) {
            let d = ctx.input_mut(|i| {
                let d = i.smooth_scroll_delta.y;
                i.smooth_scroll_delta = Vec2::ZERO;
                d
            });
            self.nav_scroll += d;
            if self.page_since.elapsed().as_millis() < 350 {
                self.nav_scroll = 0.0; // one step per flick of the wheel
            } else if self.nav_scroll.abs() >= 30.0 {
                let at = PAGES.iter().position(|(p, ..)| *p == self.page).unwrap_or(0) as i32;
                let to = (at + if self.nav_scroll < 0.0 { 1 } else { -1 }).clamp(0, PAGES.len() as i32 - 1) as usize;
                if PAGES[to].0 != self.page {
                    self.page = PAGES[to].0;
                    sound::play(sound::Sound::Nav);
                }
                self.nav_scroll = 0.0;
            }
        } else {
            self.nav_scroll = 0.0;
        }
        let area = ctx.content_rect();
        let window = area.shrink(10.0); // the black window floats 10 px inside Windows' own outline
        let round = if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) { 0 } else { 22 };
        ui.painter().rect_filled(window, CornerRadius::same(round), BG);
        // a faint grid of lines over the whole window: it shows in the gaps between the cards
        {
            let clip = window.shrink(12.0);
            let s = Stroke::new(1.0, Color32::from_gray(20));
            let mut x = clip.min.x;
            while x <= clip.max.x {
                ui.painter().line_segment([Pos2::new(x, clip.min.y), Pos2::new(x, clip.max.y)], s);
                x += 32.0;
            }
            let mut y = clip.min.y;
            while y <= clip.max.y {
                ui.painter().line_segment([Pos2::new(clip.min.x, y), Pos2::new(clip.max.x, y)], s);
                y += 32.0;
            }
        }
        // the page, to the right of the pill
        let page = Rect::from_min_max(Pos2::new(window.min.x + 26.0, window.min.y + TITLE_BAR), Pos2::new(window.max.x, window.max.y - 40.0)); // right to the edge, so the scroll bar shows there too; 40 above (the controls) and 40 below
        ui.scope_builder(egui::UiBuilder::new().max_rect(page), |ui| {
            ui.set_clip_rect(page);
            self.page_size = page.size();
            self.content(ui);
        });
        // the title bar has its own band: the page's label and BENTO (not on Home), and a faint line under it once the page has scrolled beneath
        let line = (self.scroll_y / 24.0).clamp(0.0, 1.0);
        if line > 0.0 && self.page != Page::Home {
            let y = window.min.y + TITLE_BAR - 4.0;
            ui.painter().line_segment([Pos2::new(window.min.x + 18.0, y), Pos2::new(window.max.x - 18.0, y)], Stroke::new(1.0, LINE_STRONG.gamma_multiply(line)));
        }
        if self.page == Page::Home {
            // on Home the title bar only carries the logo, at the left
            let mid = window.min.y + TITLE_BAR / 2.0;
            let zone = Rect::from_min_size(Pos2::new(window.min.x + 20.0, window.min.y + 8.0), Vec2::new(150.0, TITLE_BAR - 16.0));
            let hover = ease(ui, egui::Id::new("title-home-logo"), ui.rect_contains_pointer(zone));
            bento_mark(ui.painter(), Pos2::new(window.min.x + 44.0 + hover * 3.0, mid), blend(Color32::from_gray(225), WHITE, hover));
            ui.painter().text(Pos2::new(window.min.x + 66.0 + hover * 3.0, mid), egui::Align2::LEFT_CENTER, "BENTO", FontId::new(15.0, heading()), blend(Color32::from_gray(225), WHITE, hover));
        }
        let edge = if self.page == Page::Home { 0.0 } else { 1.0 };
        if edge > 0.0 {
            // and the title bar says where you are (left: the page, like its header) and what this is (middle, between two bento marks)
            let mid = window.min.y + TITLE_BAR / 2.0;
            let (number, name) = PAGES.iter().position(|p| p.0 == self.page).map(|i| (i, PAGES[i].2)).unwrap_or((1, "")); // Home is not counted: the Bar is 01
            // the label on the left brightens and slides when the pointer is on it; the logo in the middle opens its marks outwards
            let left_zone = Rect::from_min_size(Pos2::new(window.min.x + 20.0, window.min.y + 8.0), Vec2::new(260.0, TITLE_BAR - 16.0));
            let left_hover = ease(ui, egui::Id::new("title-left"), ui.rect_contains_pointer(left_zone));
            let tag = ui.painter().layout_no_wrap(format!("{:02} / {:02}", number, PAGES.len() - 1), FontId::new(11.5, mono()), blend(MUTED, WHITE, left_hover));
            let tag_width = tag.size().x;
            let slide = left_hover * 4.0;
            ui.painter().galley(Pos2::new(window.min.x + 34.0 + slide, mid - tag.size().y / 2.0), tag, MUTED);
            ui.painter().text(Pos2::new(window.min.x + 34.0 + tag_width + 14.0 + slide, mid), egui::Align2::LEFT_CENTER, name.to_uppercase(), FontId::new(15.0, heading()), blend(Color32::from_gray(225), WHITE, left_hover));
            let galley = ui.painter().layout_no_wrap("BENTO".to_string(), FontId::new(15.0, heading()), WHITE);
            let cx = window.center().x;
            let half = galley.size().x / 2.0;
            let logo_zone = Rect::from_center_size(Pos2::new(cx, mid), Vec2::new(galley.size().x + 120.0, TITLE_BAR - 12.0));
            let logo_hover = ease(ui, egui::Id::new("title-logo"), ui.rect_contains_pointer(logo_zone));
            ui.painter().galley(Pos2::new(cx - half, mid - galley.size().y / 2.0), galley, WHITE);
            let spread = 24.0 + logo_hover * 6.0;
            let ink = blend(MUTED, WHITE, logo_hover);
            bento_mark(ui.painter(), Pos2::new(cx - half - spread, mid), ink);
            bento_mark(ui.painter(), Pos2::new(cx + half + spread, mid), ink);
        }
        self.sidebar(ui, window);
        self.apply_pill(ui, window);
        self.toast(ui, window);
        self.back_to_top(ui, window);
        search::show(self, ui);
        ui.painter().rect_stroke(window.shrink(0.5), CornerRadius::same(round), Stroke::new(1.0, LINE_STRONG), StrokeKind::Inside);
        if glyph_edge(ui, window.shrink(0.5), round as f32, self.glyph_since(), 0.9, self.glyph_seed) {
            ctx.request_repaint();
        }
        // drag the window by its top strip
        let strip = Rect::from_min_max(area.min, Pos2::new(area.max.x - 130.0, area.min.y + 40.0));
        if ui.interact(strip, egui::Id::new("drag-strip"), Sense::click_and_drag()).drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        window_controls(ui, &ctx, window);
        let _ = Color32::BLACK;
    }
}

/// The sidebar pill and its name tag: the same fill and edge as the cards.
pub fn frosted(p: &egui::Painter, rect: Rect, radius: f32) {
    p.rect(rect, CornerRadius::same(radius as u8), CARD, Stroke::new(1.0, LINE), StrokeKind::Inside);
}

/// The width of a `button` with this label.
fn button_width(ui: &Ui, label: &str) -> f32 {
    ui.painter().layout_no_wrap(label.to_string(), FontId::new(13.0, FontFamily::Proportional), TEXT).size().x + 28.0
}
