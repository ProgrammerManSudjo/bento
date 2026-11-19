//! Wallpaper: pick a picture from a folder to set as the desktop wallpaper, and optionally as the lock screen too. Nothing else changes.

use crate::{theme::*, App};
use eframe::egui::{self, CornerRadius, Pos2, Sense, Stroke, StrokeKind, TextureHandle, Ui, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;

#[derive(Default)]
pub struct State {
    folder: Option<PathBuf>,
    files: Vec<PathBuf>,
    thumbs: HashMap<PathBuf, TextureHandle>,
    incoming: Option<mpsc::Receiver<(PathBuf, egui::ColorImage)>>,
    loaded: bool,
    lock_screen: bool,
}

// --- a few remembered settings (the folder, the lock screen switch) --------------------------------------------------------------------

pub(crate) fn data_dir() -> PathBuf {
    let dir = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("yasb-settings");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn read_setting(key: &str) -> Option<String> {
    let text = std::fs::read_to_string(data_dir().join("settings.txt")).ok()?;
    text.lines().find_map(|l| l.strip_prefix(&format!("{key}=")).map(|v| v.trim().to_string()))
}

pub fn write_setting(key: &str, value: &str) {
    let text = std::fs::read_to_string(data_dir().join("settings.txt")).unwrap_or_default();
    let mut lines: Vec<String> = text.lines().filter(|l| !l.starts_with(&format!("{key}="))).map(String::from).collect();
    lines.push(format!("{key}={value}"));
    let _ = std::fs::write(data_dir().join("settings.txt"), lines.join("\n"));
}

// --- Windows ---------------------------------------------------------------------------------------------------------------------------

pub fn current_wallpaper() -> String {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETDESKWALLPAPER};
    let mut buf = [0u16; 520];
    unsafe { SystemParametersInfoW(SPI_GETDESKWALLPAPER, buf.len() as u32, buf.as_mut_ptr().cast(), 0) };
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end]).replace('\\', "/")
}

pub(crate) fn set_wallpaper(path: &str) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SPI_SETDESKWALLPAPER};
    let mut wide: Vec<u16> = path.encode_utf16().chain([0]).collect();
    (unsafe { SystemParametersInfoW(SPI_SETDESKWALLPAPER, 0, wide.as_mut_ptr().cast(), SPIF_UPDATEINIFILE | SPIF_SENDCHANGE) }) != 0
}

/// Windows' own LockScreen API through PowerShell (per user, no administrator rights; it also switches the lock screen from Spotlight to a picture).
pub(crate) fn set_lock_screen(path: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let dir = data_dir();
    let source = PathBuf::from(path);
    // a copy: the original may be on a drive that is not always there
    let copy = dir.join(format!("lock_screen.{}", source.extension().and_then(|e| e.to_str()).unwrap_or("jpg").to_lowercase()));
    std::fs::copy(&source, &copy).map_err(|e| e.to_string())?;
    let script = dir.join("set_lock_screen.ps1");
    std::fs::write(&script, include_str!("../assets/lock_screen.ps1")).map_err(|e| e.to_string())?;
    let out = std::process::Command::new("powershell").args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"]).arg(&script).arg(&copy)
        .creation_flags(0x0800_0000).output().map_err(|e| e.to_string())?;
    if out.status.success() && String::from_utf8_lossy(&out.stdout).contains("ok") {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(if out.stderr.is_empty() { &out.stdout } else { &out.stderr }).lines().next().unwrap_or("no answer").chars().take(110).collect())
    }
}

// --- the page --------------------------------------------------------------------------------------------------------------------------

/// The folder the pictures come from: the one you chose here, else the one the wallpapers widget uses.
fn folder(app: &App) -> Option<PathBuf> {
    if let Some(f) = read_setting("folder").filter(|f| !f.is_empty()) {
        return Some(PathBuf::from(f));
    }
    let from_yaml = app.cfg.string(&["widgets", "wallpapers", "options", "image_path"]);
    if !from_yaml.is_empty() { Some(PathBuf::from(from_yaml)) } else { app.cfg.list(&["widgets", "wallpapers", "options", "image_path"]).into_iter().next().map(PathBuf::from) }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    if !app.wall.loaded {
        app.wall.loaded = true;
        app.wall.lock_screen = read_setting("lock_screen").is_some_and(|v| v == "1");
    }
    // a new folder (or the first time): list its pictures and load their thumbnails in the background
    let wanted = folder(app);
    if wanted != app.wall.folder {
        app.wall.folder = wanted.clone();
        app.wall.files.clear();
        app.wall.thumbs.clear();
        app.wall.incoming = None;
        if let Some(dir) = &wanted {
            let mut files: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path())
                .filter(|p| p.extension().and_then(|e| e.to_str()).is_some_and(|e| matches!(e.to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "bmp" | "webp"))).collect();
            files.sort();
            let (tx, rx) = mpsc::channel();
            let list = files.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                for p in list {
                    if let Ok(img) = image::open(&p) {
                        let t = img.resize_to_fill(360, 203, image::imageops::FilterType::Triangle).to_rgba8(); // cropped to the tile, never stretched
                        let size = [t.width() as usize, t.height() as usize];
                        if tx.send((p, egui::ColorImage::from_rgba_unmultiplied(size, t.as_raw()))).is_err() {
                            return;
                        }
                        ctx.request_repaint();
                    }
                }
            });
            app.wall.files = files;
            app.wall.incoming = Some(rx);
        }
    }
    if let Some(rx) = &app.wall.incoming {
        while let Ok((p, img)) = rx.try_recv() {
            let tex = ctx.load_texture(p.to_string_lossy(), img, egui::TextureOptions::LINEAR);
            app.wall.thumbs.insert(p, tex);
        }
    }

    card(ui, "", |ui| {
        let shown = wanted.as_ref().map(|f| f.display().to_string()).unwrap_or_else(|| "no folder chosen".into());
        row(ui, "Folder", &shown, |ui| {
            if button(ui, "Choose...", false).clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    let dir = dir.to_string_lossy().replace('\\', "/");
                    write_setting("folder", &dir);
                    if app.cfg.at(&["widgets", "wallpapers", "options", "image_path"]).is_badvalue() {
                        // there is no wallpapers widget to keep in step
                    } else {
                        let r = app.cfg.set_text(&["widgets", "wallpapers", "options", "image_path"], &dir);
                        app.done(r);
                    }
                }
            }
        });
        let mut lock = app.wall.lock_screen;
        if row(ui, "Also set the lock screen", "The picture you click becomes the lock screen too", |ui| toggle(ui, &mut lock)).changed() {
            app.wall.lock_screen = lock;
            write_setting("lock_screen", if lock { "1" } else { "0" });
        }
    });
    ui.add_space(8.0);

    let current = current_wallpaper();
    let files = app.wall.files.clone();
    if files.is_empty() {
        small(ui, "There are no pictures in this folder.");
        return;
    }
    // whole rows of pictures that use the full width: as many columns as fit, then the tiles are made exactly wide enough to fill the row
    let gap = 12.0;
    let avail = ui.available_width();
    let columns = (((avail + gap) / (210.0 + gap)).floor() as usize).max(1);
    let tile_w = ((avail - gap * (columns - 1) as f32) / columns as f32).floor();
    let tile = Vec2::new(tile_w, (tile_w * 9.0 / 16.0).floor());
    let mut chosen = None;
    for row_of_files in files.chunks(columns) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(gap);
            for p in row_of_files {
                let (rect, response) = ui.allocate_exact_size(tile, Sense::click());
                let on = p.to_string_lossy().replace('\\', "/") == current;
                let waiting = app.staged.wallpaper.as_deref().is_some_and(|s| s.replace('\\', "/") == p.to_string_lossy().replace('\\', "/"));
                match app.wall.thumbs.get(p) {
                    Some(t) => {
                        egui::Image::new(t).corner_radius(CornerRadius::same(16)).paint_at(ui, rect);
                    }
                    None => {
                        ui.painter().rect_filled(rect, CornerRadius::same(16), CARD_HOVER);
                    }
                }
                let stroke = if on { Stroke::new(2.5, lit()) } else if waiting { Stroke::new(2.5, glow()) } else if response.hovered() { Stroke::new(1.5, Color32w::LINE_STRONG) } else { Stroke::new(1.0, LINE) };
                ui.painter().rect_stroke(rect, CornerRadius::same(16), stroke, StrokeKind::Inside);
                if on {
                    ui.painter().circle_filled(Pos2::new(rect.max.x - 16.0, rect.min.y + 16.0), 6.0, lit());
                } else if waiting {
                    // picked, waiting for Apply: a ring
                    ui.painter().circle_stroke(Pos2::new(rect.max.x - 16.0, rect.min.y + 16.0), 6.0, Stroke::new(2.0, glow()));
                }
                if response.clicked() && !on {
                    chosen = Some(p.clone());
                }
            }
        });
    }
    // a click picks it; it is set when you press Apply
    if let Some(p) = chosen {
        app.staged.pick_wallpaper(p.to_string_lossy().to_string(), &current);
    }
}

/// (the outline colour of a hovered tile)
struct Color32w;
impl Color32w {
    const LINE_STRONG: egui::Color32 = crate::theme::LINE_STRONG;
}
