//! Komorebi: its window-tiling settings in komorebi.json (spacing, borders, animation, and each workspace's name and layout).
//! Saved at once; "Apply to komorebi" tells the running komorebi to load the file.

use crate::{theme::*, App};
use eframe::egui::{self, Ui};
use serde_json::Value;
use std::{fs, path::PathBuf, time::SystemTime};

pub struct Json {
    pub path: PathBuf,
    doc: Value,
    stamp: Option<SystemTime>,
    pub error: Option<String>,
    pub dirty: bool,
}

impl Json {
    pub fn load(path: PathBuf) -> Json {
        let text = fs::read_to_string(&path).unwrap_or_default();
        let (doc, error) = match serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) {
            Ok(v) => (v, None),
            Err(e) => (Value::Null, Some(format!("komorebi.json: {e}"))),
        };
        let stamp = fs::metadata(&path).and_then(|m| m.modified()).ok();
        Json { path, doc, stamp, error, dirty: false }
    }

    pub fn reload_if_changed(&mut self) {
        if self.dirty {
            return;
        }
        let now = fs::metadata(&self.path).and_then(|m| m.modified()).ok();
        if now != self.stamp {
            *self = Json::load(self.path.clone());
        }
    }

    fn at(&self, pointer: &str) -> Option<&Value> {
        self.doc.pointer(pointer)
    }

    fn set(&mut self, pointer: &str, v: Value) -> Result<(), String> {
        let slot = self.doc.pointer_mut(pointer).ok_or_else(|| format!("{pointer} is not in komorebi.json"))?;
        *slot = v;
        self.save()
    }

    /// Add a missing key to an object, then set it.
    fn put(&mut self, parent: &str, key: &str, v: Value) -> Result<(), String> {
        let obj = self.doc.pointer_mut(parent).and_then(Value::as_object_mut).ok_or_else(|| format!("{parent} is not in komorebi.json"))?;
        obj.insert(key.to_string(), v);
        self.save()
    }

    fn save(&mut self) -> Result<(), String> {
        self.dirty = true; // written to the file by apply()
        Ok(())
    }

    /// Write the waiting edits to the file.
    pub fn apply(&mut self) -> Result<(), String> {
        let text = serde_json::to_string_pretty(&self.doc).map_err(|e| e.to_string())?;
        let r = fs::write(&self.path, text).map_err(|e| e.to_string());
        self.stamp = fs::metadata(&self.path).and_then(|m| m.modified()).ok();
        if r.is_ok() {
            self.dirty = false;
        }
        r
    }

    pub fn discard(&mut self) {
        *self = Json::load(self.path.clone());
    }

    pub fn workspaces(&self) -> usize {
        self.at("/monitors/0/workspaces").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0)
    }

    fn bool(&self, p: &str, default: bool) -> bool {
        self.at(p).and_then(Value::as_bool).unwrap_or(default)
    }

    fn int(&self, p: &str, default: i64) -> i64 {
        self.at(p).and_then(Value::as_i64).unwrap_or(default)
    }

    fn string(&self, p: &str) -> String {
        self.at(p).and_then(Value::as_str).unwrap_or("").to_string()
    }
}

fn toggle_row(app: &mut App, ui: &mut Ui, name: &str, hint: &str, parent: &str, key: &str) {
    let p = format!("{parent}/{key}");
    let mut v = app.komorebi.bool(&p, false);
    if row(ui, name, hint, |ui| toggle(ui, &mut v)).changed() {
        let r = app.komorebi.put(parent, key, Value::Bool(v));
        app.done(r);
    }
}

fn slider_row(app: &mut App, ui: &mut Ui, name: &str, hint: &str, parent: &str, key: &str, min: i64, max: i64, suffix: &str) {
    let p = format!("{parent}/{key}");
    let mut v = app.komorebi.int(&p, 0);
    let was = v;
    let response = row(ui, name, hint, |ui| slider(ui, &mut v, min, max, suffix));
    if v != was && !response.dragged() {
        let r = app.komorebi.put(parent, key, Value::from(v));
        app.done(r);
    }
}

fn choice_row(app: &mut App, ui: &mut Ui, name: &str, hint: &str, parent: &str, key: &str, options: &[&str]) {
    let p = format!("{parent}/{key}");
    let mut v = app.komorebi.string(&p);
    if row(ui, name, hint, |ui| segmented(ui, options, &mut v)) {
        let r = app.komorebi.put(parent, key, Value::String(v));
        app.done(r);
    }
}

fn text_row(app: &mut App, ui: &mut Ui, name: &str, hint: &str, pointer: &str, width: f32) {
    let id = egui::Id::new(("kjson", pointer));
    let current = app.komorebi.string(pointer);
    let mut v: String = ui.data_mut(|d| d.get_temp::<(String, String)>(id)).filter(|(s, _)| *s == current).map(|(_, e)| e).unwrap_or_else(|| current.clone());
    let committed = row(ui, name, hint, |ui| input(ui, &mut v, width));
    ui.data_mut(|d| d.insert_temp(id, (current.clone(), v.clone())));
    if committed && v != current {
        let r = app.komorebi.set(pointer, Value::String(v));
        app.done(r);
    }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    if let Some(e) = &app.komorebi.error {
        text(ui, e.clone(), 13.0, MUTED);
        return;
    }
    ui.add_space(6.0);

    card(ui, "Windows", |ui| {
        toggle_row(app, ui, "Mouse follows focus", "Move the pointer to the window you focus", "", "mouse_follows_focus");
        choice_row(app, ui, "Hiding windows", "How windows on other workspaces are hidden", "", "window_hiding_behaviour", &["Cloak", "Hide", "Minimize"]);
        choice_row(app, ui, "Moving to another monitor", "What happens to the window already there", "", "cross_monitor_move_behaviour", &["Insert", "Swap", "NoOp"]);
    });
    card(ui, "Spacing", |ui| {
        slider_row(app, ui, "Workspace padding", "Space between the screen edge and the windows", "", "default_workspace_padding", 0, 80, " px");
        slider_row(app, ui, "Window padding", "Space between windows", "", "default_container_padding", 0, 60, " px");
    });
    card(ui, "Border", |ui| {
        toggle_row(app, ui, "Border", "A coloured outline round the focused window", "", "border");
        slider_row(app, ui, "Width", "", "", "border_width", 0, 16, " px");
        slider_row(app, ui, "Offset", "Negative moves it inwards", "", "border_offset", -20, 20, " px");
    });
    card(ui, "Transparency", |ui| {
        toggle_row(app, ui, "Make unfocused windows see-through", "", "", "transparency");
        slider_row(app, ui, "Opacity", "255 is solid", "", "transparency_alpha", 0, 255, "");
    });
    if app.komorebi.at("/animation").is_some_and(Value::is_object) {
        card(ui, "Animation", |ui| scalars(app, ui, "/animation"));
    }
    if app.komorebi.at("/stackbar").is_some_and(Value::is_object) {
        card(ui, "Stack bar", |ui| scalars(app, ui, "/stackbar"));
    }
    workspaces(app, ui);
}

/// Every plain value of an object as a row.
fn scalars(app: &mut App, ui: &mut Ui, parent: &str) {
    let keys: Vec<(String, Value)> = app.komorebi.at(parent).and_then(Value::as_object).map(|o| o.iter().filter(|(_, v)| !v.is_object() && !v.is_array()).map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
    for (k, v) in keys {
        let name = k.replace('_', " ");
        let name = name[..1].to_uppercase() + &name[1..];
        match v {
            Value::Bool(_) => toggle_row(app, ui, &name, "", parent, &k),
            Value::Number(n) if n.is_i64() => slider_row(app, ui, &name, "", parent, &k, 0, (n.as_i64().unwrap_or(0) * 4).max(100), ""),
            Value::String(_) => text_row(app, ui, &name, "", &format!("{parent}/{k}"), 200.0),
            _ => {}
        }
    }
}

const LAYOUTS: [&str; 8] = ["BSP", "Columns", "Rows", "VerticalStack", "HorizontalStack", "UltrawideVerticalStack", "Grid", "RightMainVerticalStack"];

/// Each monitor's workspaces: the name that YASB shows, the layout, and the rules that send windows there.
fn workspaces(app: &mut App, ui: &mut Ui) {
    let monitors = app.komorebi.at("/monitors").and_then(Value::as_array).map(|a| a.len()).unwrap_or(0);
    if monitors == 0 {
        return;
    }
    ui.add_space(8.0);
    title(ui, "Workspaces", 18.0);
    small(ui, "The name is what the bar shows for the workspace (an icon is fine).");
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        for m in 0..monitors {
            if pill(ui, &format!("monitor {}", m + 1), app.kmonitor == m).clicked() {
                app.kmonitor = m;
            }
        }
    });
    let m = app.kmonitor.min(monitors - 1);
    let count = app.komorebi.at(&format!("/monitors/{m}/workspaces")).and_then(Value::as_array).map(|a| a.len()).unwrap_or(0);
    for w in 0..count {
        let base = format!("/monitors/{m}/workspaces/{w}");
        card(ui, &format!("Workspace {}", w + 1), |ui| {
            text_row(app, ui, "Name", "", &format!("{base}/name"), 160.0);
            let mut layout = app.komorebi.string(&format!("{base}/layout"));
            if layout.is_empty() {
                layout = "BSP".into();
            }
            if row_below(ui, "Layout", "How windows are tiled on this workspace", |ui| choices(ui, &LAYOUTS, &mut layout)) {
                let r = app.komorebi.put(&base, "layout", Value::String(layout));
                app.done(r);
            }
            // the apps that open here: rules, shown by the program or title they match; x removes one, typing adds one
            let key = if app.komorebi.at(&format!("{base}/workspace_rules")).is_some() { "workspace_rules" } else if app.komorebi.at(&format!("{base}/initial_workspace_rules")).is_some() { "initial_workspace_rules" } else { "workspace_rules" };
            let rules: Vec<Value> = app.komorebi.at(&format!("{base}/{key}")).and_then(Value::as_array).cloned().unwrap_or_default();
            let ids: Vec<String> = rules.iter().filter_map(|r| r.get("id").and_then(Value::as_str).map(String::from)).collect();
            let changed = row_below(ui, "Opens here", "Programs (like chrome.exe) or window titles that are sent to this workspace. Press x to remove, type to add", |ui| chips(ui, &base, &ids, "add a program..."));
            if let Some(new_ids) = changed {
                let list: Vec<Value> = new_ids.iter().map(|id| {
                    rules.iter().find(|r| r.get("id").and_then(Value::as_str) == Some(id.as_str())).cloned().unwrap_or_else(|| {
                        // a new rule: a program name matches exactly, anything else matches part of the window title
                        let is_exe = id.to_lowercase().ends_with(".exe");
                        serde_json::json!({"kind": if is_exe { "Exe" } else { "Title" }, "id": id, "matching_strategy": if is_exe { "Equals" } else { "Contains" }})
                    })
                }).collect();
                let r = app.komorebi.put(&base, key, Value::Array(list));
                app.done(r);
            }
        });
    }
}
