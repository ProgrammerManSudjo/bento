//! About: which versions of YASB and komorebi are installed, and where the files are.

use crate::{theme::*, App};
use eframe::egui::{CornerRadius, Pos2, Rect, Sense, Ui, Vec2};
use std::path::PathBuf;
use std::process::Command;

/// First line of what `program --version` prints.
pub fn version(program: &str) -> String {
    use std::os::windows::process::CommandExt;
    Command::new(program).arg("--version").creation_flags(0x0800_0000).output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty()).unwrap_or_else(|| "not found".into())
}

fn open(path: PathBuf) {
    let _ = Command::new("notepad.exe").arg(path).spawn();
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let versions = app.versions(&ctx);
    card(ui, "Versions", |ui| match &versions {
        Some((yasb, komorebi)) => {
            row(ui, "YASB", "The bar", |ui| small(ui, yasb.clone()));
            row(ui, "komorebi", "The tiling window manager", |ui| small(ui, komorebi.clone()));
            row(ui, "YASB Settings", "This window", |ui| small(ui, format!("v{}", env!("CARGO_PKG_VERSION"))));
        }
        None => {
            // still looking: a bar that sweeps back and forth
            ui.add_space(14.0);
            small(ui, "Looking up the versions...");
            ui.add_space(8.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.0), Sense::hover());
            ui.painter().rect_filled(rect, CornerRadius::same(3), LINE);
            let t = ctx.input(|i| i.time) as f32;
            let phase = { let s = ((t * 0.8).sin() * 0.5 + 0.5).clamp(0.0, 1.0); s * s * (3.0 - 2.0 * s) }; // smoothstep: slows down at both ends
            let w = rect.width() * 0.28;
            let x = rect.min.x + (rect.width() - w) * phase;
            ui.painter().rect_filled(Rect::from_min_size(Pos2::new(x, rect.min.y), Vec2::new(w, rect.height())), CornerRadius::same(3), WHITE);
            ctx.request_repaint();
            ui.add_space(16.0);
        }
    });
    card(ui, "Sounds", |ui| {
        let mut on = crate::sound::enabled();
        if row(ui, "Interface sounds", "Very quiet ticks when you click", |ui| toggle(ui, &mut on)).changed() {
            crate::sound::set_enabled(on);
            crate::wallpaper::write_setting("sounds", if on { "1" } else { "0" });
        }
        let mut volume = crate::sound::volume() as i64;
        let was = volume;
        let response = row(ui, "Volume", "How loud the taps are", |ui| slider(ui, &mut volume, 0, 100, " %"));
        if volume != was {
            crate::sound::set_volume(volume as u32);
        }
        if response.drag_stopped() || (volume != was && !response.dragged()) {
            crate::wallpaper::write_setting("sound_volume", &crate::sound::volume().to_string());
            crate::sound::play(crate::sound::Sound::Click); // hear the new level
        }

    });
    let yasb_dir = app.cfg.path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let kdir = app.komorebi.path.clone();
    let tacky_file = app.tacky.path.clone();
    card(ui, "Files", |ui| {
        for (name, hint, path) in [
            ("config.yaml", "The bar, its widgets and options", yasb_dir.join("config.yaml")),
            ("styles.css", "How the bar looks", yasb_dir.join("styles.css")),
            ("komorebi.json", "Window tiling", kdir.clone()),
            ("tacky-borders config.yaml", "Window borders", tacky_file.clone()),
            ("settings.txt", "This app's own settings: the Lucent theme, softness, brightness and the rest", crate::wallpaper::data_dir().join("settings.txt")),
        ] {
            row(ui, name, &format!("{hint}  -  {}", path.display()), |ui| {
                if button(ui, "Open", false).clicked() {
                    open(path.clone());
                }
            });
        }
    });
}
