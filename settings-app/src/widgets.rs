//! Widgets: the options of each widget in config.yaml, shown by the kind of value they hold.

use crate::{theme::*, App};
use eframe::egui::{self, Ui};
use yaml_rust2::Yaml;

pub fn nice(key: &str) -> String {
    let mut s = key.replace('_', " ");
    if let Some(first) = s.get(..1) {
        s = first.to_uppercase() + &s[1..];
    }
    s
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let widgets = app.cfg.keys(&["widgets"]);
    if widgets.is_empty() {
        text(ui, "config.yaml has no widgets.", 14.0, MUTED);
        return;
    }
    sizes(app, ui);
    let total = widgets.len();
    for (index, w) in widgets.iter().enumerate() {
        let kind = app.cfg.string(&["widgets", w, "type"]);
        let on_bars: Vec<String> = app.cfg.keys(&["bars"]).into_iter().filter(|bar| ["left", "center", "right"].iter().any(|side| app.cfg.list(&["bars", bar, "widgets", side]).contains(w))).collect();
        let used = if on_bars.is_empty() { "not on a bar".to_string() } else { on_bars.join(", ") };
        let id = egui::Id::new(("widget-open", w));
        register_section(ui, id, &w.replace('_', " "), false);
        let stored = ui.data_mut(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        let open = stored || *w == app.widget;
        let tag = format!("{:02} / {:02}  -  {}  -  {used}", index + 1, total, kind.rsplit('.').next().unwrap_or(&kind));
        if section_header(ui, Mark::Page(crate::icons::Icon::Widgets), &w.replace('_', " "), &tag, Some(!on_bars.is_empty()), open) {
            ui.data_mut(|d| d.insert_temp(id, !open));
            if open && *w == app.widget {
                app.widget = String::new();
            }
        }
        if !open {
            ui.add_space(8.0);
            continue;
        }
        widget_sizes(app, ui, w, &kind);
        let base = vec!["widgets".to_string(), w.clone(), "options".to_string()];
        let keys = {
            let p: Vec<&str> = base.iter().map(String::as_str).collect();
            app.cfg.keys(&p)
        };
        if keys.is_empty() {
            small(ui, "This widget has no options in config.yaml.");
        } else {
            ui.add_space(6.0);
            editor(app, ui, &base, "");
        }
        ui.add_space(8.0);
    }
}

/// All the keys under `base`: scalars first as rows of one card, then each nested mapping as a card of its own.
fn editor(app: &mut App, ui: &mut Ui, base: &[String], heading_text: &str) {
    let p: Vec<&str> = base.iter().map(String::as_str).collect();
    let keys = app.cfg.keys(&p);
    let (mut scalars, mut nested, mut lists) = (vec![], vec![], vec![]);
    for k in keys {
        let mut q = p.clone();
        q.push(&k);
        match app.cfg.at(&q) {
            Yaml::Hash(_) => nested.push(k),
            Yaml::Array(_) => lists.push(k),
            _ => scalars.push(k),
        }
    }
    if !scalars.is_empty() || !lists.is_empty() {
        card(ui, heading_text, |ui| {
            for k in &scalars {
                scalar(app, ui, base, k);
            }
            for k in &lists {
                list(app, ui, base, k);
            }
        });
    }
    for k in nested {
        let mut inner = base.to_vec();
        inner.push(k.clone());
        let title_text = if heading_text.is_empty() { nice(&k) } else { format!("{heading_text} / {}", nice(&k)) };
        editor(app, ui, &inner, &title_text);
    }
}

fn scalar(app: &mut App, ui: &mut Ui, base: &[String], key: &str) {
    let mut full = base.to_vec();
    full.push(key.to_string());
    let p: Vec<&str> = full.iter().map(String::as_str).collect();
    let label = nice(key);
    // what YASB's documentation says this option does
    let wtype = base.get(1).map(|name| app.cfg.string(&["widgets", name, "type"])).unwrap_or_default();
    let help = crate::help::lookup(&wtype, key);
    let hint = help.map(|h| h.hint()).unwrap_or_default();
    match app.cfg.at(&p).clone() {
        Yaml::Boolean(_) => app.yaml_toggle(ui, &label, &hint, &p),
        Yaml::Integer(n) if help.is_some_and(|h| h.is_flag()) && (n == 0 || n == 1) => {
            let mut on = n == 1;
            if row(ui, &label, &hint, |ui| toggle(ui, &mut on)).changed() {
                let r = app.cfg.set_int(&p, on as i64);
                app.done(r);
            }
        }
        Yaml::Integer(_) | Yaml::Real(_) => number(app, ui, &label, &hint, &p),
        Yaml::String(_) => {
            let options = help.map(|h| h.choices()).unwrap_or_default();
            if options.len() >= 2 {
                choice(app, ui, &label, &hint, &p, &options);
            } else if key.contains("label") || key.contains("icon") {
                let hint = if hint.is_empty() { "Icons as \\uXXXX codes, like in config.yaml (a glyph typed or pasted is converted)".to_string() } else { format!("{hint}  Icons as \\uXXXX codes.") };
                app.yaml_glyph(ui, &label, &hint, &p, 260.0);
            } else {
                app.yaml_text(ui, &label, &hint, &p, 260.0);
            }
        }
        _ => app.yaml_text(ui, &label, &hint, &p, 260.0),
    }
}

/// One of a fixed set of words: a few sit beside the name, more go on a line of their own.
fn choice(app: &mut App, ui: &mut Ui, label: &str, hint: &str, p: &[&str], options: &[String]) {
    let words: Vec<&str> = options.iter().map(String::as_str).collect();
    if words.len() <= 3 {
        app.yaml_choice(ui, label, hint, p, &words);
        return;
    }
    let mut v = app.cfg.string(p);
    if row_below(ui, label, hint, |ui| choices(ui, &words, &mut v)) {
        let r = app.cfg.set_text(p, &v);
        app.done(r);
    }
}

/// A number typed as text: kept as a number in the file.
fn number(app: &mut App, ui: &mut Ui, label: &str, hint: &str, p: &[&str]) {
    let id = egui::Id::new(("num", p.join(".")));
    let current = app.cfg.string(p);
    let mut v: String = ui.data_mut(|d| d.get_temp::<(String, String)>(id)).filter(|(stored, _)| *stored == current).map(|(_, e)| e).unwrap_or_else(|| current.clone());
    let committed = row(ui, label, hint, |ui| input(ui, &mut v, 120.0));
    ui.data_mut(|d| d.insert_temp(id, (current.clone(), v.clone())));
    if committed && v != current {
        if v.trim().parse::<f64>().is_ok() {
            let r = app.cfg.set(p, v.trim());
            app.done(r);
        } else {
            app.say("That is not a number");
        }
    }
}

/// A list: strings are edited as comma-separated text, a list of mappings is only counted (it is edited in the file).
fn list(app: &mut App, ui: &mut Ui, base: &[String], key: &str) {
    let mut full = base.to_vec();
    full.push(key.to_string());
    let p: Vec<&str> = full.iter().map(String::as_str).collect();
    let label = nice(key);
    let all_text = app.cfg.at(&p).as_vec().is_some_and(|v| v.iter().all(|x| matches!(x, Yaml::String(_) | Yaml::Integer(_))));
    if !all_text {
        let n = app.cfg.at(&p).as_vec().map(|v| v.len()).unwrap_or(0);
        row(ui, &label, "A list of entries: edit it in config.yaml", |ui| small(ui, format!("{n} entries")));
        return;
    }
    let items = app.cfg.list(&p);
    let wtype = base.get(1).map(|name| app.cfg.string(&["widgets", name, "type"])).unwrap_or_default();
    let note = match crate::help::lookup(&wtype, key) {
        Some(h) => format!("{}  Press x to remove, type to add.", h.hint()),
        None => "Press x to remove, type to add".to_string(),
    };
    if let Some(new) = row_below(ui, &label, &note, |ui| chips(ui, &p.join("."), &items, "add...")) {
        let r = app.cfg.set_list(&p, &new);
        app.done(r);
    }
}

/// Label and icon size for the whole bar (0 keeps the stylesheet's own sizes). Written to styles.css when you let go of the slider.
fn sizes(app: &mut App, ui: &mut Ui) {
    let css = app.cfg.path.with_file_name("styles.css");
    let id = egui::Id::new("bar-sizes");
    let (mut label, mut icon): (i64, i64) = ui.data_mut(|d| d.get_temp(id)).unwrap_or_else(|| crate::sizes::read(&css));
    let mut released = false;
    card(ui, "Sizes", |ui| {
        let r1 = row(ui, "Label size", "Text on the bar. 0 keeps the sizes in styles.css", |ui| slider(ui, &mut label, 0, 32, " px"));
        let r2 = row(ui, "Icon size", "Icons on the bar. 0 keeps the sizes in styles.css", |ui| slider(ui, &mut icon, 0, 40, " px"));
        released = r1.drag_stopped() || r2.drag_stopped() || ((r1.changed() || r2.changed()) && !r1.dragged() && !r2.dragged());
    });
    ui.data_mut(|d| d.insert_temp(id, (label, icon)));
    if released {
        match crate::sizes::write(&css, label, icon) {
            Ok(()) => app.say("Sizes saved: the bar reloads them"),
            Err(e) => app.say(&format!("Could not change that: {e}")),
        }
    }
}

/// Label and icon size of one widget (0 keeps the bar's sizes). Written to styles.css when you let go of the slider.
fn widget_sizes(app: &mut App, ui: &mut Ui, name: &str, kind: &str) {
    let Some(class) = crate::sizes::class_for(kind) else { return };
    let css = app.cfg.path.with_file_name("styles.css");
    let id = egui::Id::new(("widget-sizes", name));
    let (mut label, mut icon): (i64, i64) = ui.data_mut(|d| d.get_temp(id)).unwrap_or_else(|| crate::sizes::read_widget(&css, name));
    let mut released = false;
    card(ui, "", |ui| {
        let r1 = row(ui, "Label size", "Text of this widget on the bar. 0 keeps the bar's size", |ui| slider(ui, &mut label, 0, 32, " px"));
        let r2 = row(ui, "Icon size", "Icons of this widget. 0 keeps the bar's size", |ui| slider(ui, &mut icon, 0, 40, " px"));
        released = r1.drag_stopped() || r2.drag_stopped() || ((r1.changed() || r2.changed()) && !r1.dragged() && !r2.dragged());
    });
    ui.data_mut(|d| d.insert_temp(id, (label, icon)));
    if released {
        match crate::sizes::write_widget(&css, name, class, label, icon) {
            Ok(()) => app.say("Sizes saved: the bar reloads them"),
            Err(e) => app.say(&format!("Could not change that: {e}")),
        }
    }
}
