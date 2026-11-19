//! Changes that wait for the Apply button: the wallpaper you picked and the Lucent theme's switches and sliders. Nothing is written until Apply;
//! Discard throws them away. Each setter drops its entry again when the value is back to what is saved.

use crate::lucent;
use std::os::windows::process::CommandExt;

#[derive(Default, Clone)]
pub struct Staged {
    pub wallpaper: Option<String>,
    pub lucent: Option<bool>,
    pub yasb: Option<bool>,
    pub windhawk: Option<bool>,
    pub strength: Option<i64>,
    pub effect: Option<i64>,
    /// text settings that wait too: the global accent ("lucent_accent") and each program's mode, accent and monochrome tone ("app_<id>_mode" ...)
    pub other: std::collections::BTreeMap<String, String>,
}

impl Staged {
    pub fn is_empty(&self) -> bool {
        self.wallpaper.is_none() && self.lucent.is_none() && self.yasb.is_none() && self.windhawk.is_none() && self.strength.is_none() && self.effect.is_none() && self.other.is_empty()
    }

    // what the page should show: the waiting value, else the saved one
    pub fn lucent_on(&self) -> bool {
        self.lucent.unwrap_or_else(lucent::on)
    }

    pub fn capsule_on(&self, key: &str) -> bool {
        match key {
            "yasb" => self.yasb,
            _ => self.windhawk,
        }
        .unwrap_or_else(|| lucent::capsule_on(key))
    }

    pub fn strength(&self) -> i64 {
        self.strength.unwrap_or_else(lucent::strength)
    }

    pub fn effect(&self) -> i64 {
        self.effect.unwrap_or_else(lucent::effect)
    }

    pub fn get(&self, key: &str) -> String {
        self.other.get(key).cloned().unwrap_or_else(|| lucent::setting(key))
    }

    pub fn set(&mut self, key: &str, value: &str) {
        if value == lucent::setting(key) {
            self.other.remove(key);
        } else {
            self.other.insert(key.to_string(), value.to_string());
        }
    }

    pub fn set_lucent(&mut self, on: bool) {
        self.lucent = (on != lucent::on()).then_some(on);
    }

    pub fn set_capsule(&mut self, key: &str, on: bool) {
        let value = (on != lucent::capsule_on(key)).then_some(on);
        if key == "yasb" { self.yasb = value } else { self.windhawk = value }
    }

    pub fn set_strength(&mut self, v: i64) {
        self.strength = (v != lucent::strength()).then_some(v);
    }

    pub fn set_effect(&mut self, v: i64) {
        self.effect = (v != lucent::effect()).then_some(v);
    }

    /// Pick a wallpaper (or take the pick back by clicking it again). The one that is set now is not a change.
    pub fn pick_wallpaper(&mut self, path: String, current: &str) {
        let same = |a: &str, b: &str| a.replace('\\', "/") == b.replace('\\', "/");
        self.wallpaper = if self.wallpaper.as_deref().is_some_and(|p| same(p, &path)) || same(&path, current) { None } else { Some(path) };
    }
}

/// What Apply does with the waiting changes, off the window's thread. The answer is the message for the top of the window.
pub fn run(st: Staged, reload_komorebi: bool, lock_screen: bool) -> String {
    if reload_komorebi {
        let _ = std::process::Command::new("komorebic").arg("reload-configuration").creation_flags(0x0800_0000).status();
    }
    // the settings first, so everything below reads the new values
    if let Some(v) = st.lucent {
        lucent::set_on(v);
    }
    for (key, v) in [("yasb", st.yasb), ("windhawk", st.windhawk)] {
        if let Some(v) = v {
            lucent::set_capsule_on(key, v);
        }
    }
    if let Some(v) = st.strength {
        crate::wallpaper::write_setting("lucent_strength", &v.to_string());
    }
    if let Some(v) = st.effect {
        crate::wallpaper::write_setting("lucent_effect", &v.to_string());
    }
    for (k, v) in &st.other {
        crate::wallpaper::write_setting(k, v);
    }
    let (theme, yasb, windhawk) = (lucent::on(), lucent::capsule_on("yasb"), lucent::capsule_on("windhawk"));
    let mut notes: Vec<String> = Vec::new();

    if let Some(path) = &st.wallpaper {
        let before = if theme || yasb || windhawk { std::fs::read_to_string(lucent::colours_css()).unwrap_or_default() } else { String::new() };
        if !crate::wallpaper::set_wallpaper(path) {
            return "Could not set the wallpaper".into();
        }
        if lock_screen {
            if let Err(e) = crate::wallpaper::set_lock_screen(path) {
                notes.push(format!("Lock screen failed: {e}"));
            }
        }
        if theme || yasb || windhawk {
            // Windows picks its accent from the new picture and YASB writes it down: wait for that
            lucent::wait_for_new_accent(&before);
        }
    }

    // a new wallpaper or softness changes every colour; the effect only the capsules
    let recolour = st.wallpaper.is_some() || st.strength.is_some() || st.other.contains_key("lucent_accent");
    // a program's own colour settings: only that program is written again
    let mono_changed = st.other.keys().any(|k| k.starts_with("mono_"));
    let own: Vec<String> = st.other.keys().filter_map(|k| k.strip_prefix("app_").and_then(|r| r.rsplit_once('_')).map(|(id, _)| id.to_string())).collect();
    let brighter = st.effect.is_some();
    // switching the theme off puts the programs back to monochrome
    if (theme && (recolour || st.lucent == Some(true))) || st.lucent == Some(false) || mono_changed {
        let results = lucent::apply_all();
        lucent::remember(&results);
        notes.extend(lucent::problems(&results));
    } else if !own.is_empty() {
        let results = lucent::apply_some(&own);
        lucent::remember(&results);
        notes.extend(lucent::problems(&results));
    }
    let refresh = recolour || mono_changed || st.lucent.is_some();
    let (refresh_yasb, refresh_windhawk) = (refresh || st.other.contains_key("yasb_accent") || st.other.contains_key("yasb_mono"), refresh || st.other.contains_key("windhawk_accent") || st.other.contains_key("windhawk_mono"));
    if yasb && (refresh_yasb || brighter || st.yasb == Some(true)) {
        if let Err(e) = lucent::yasb_capsule(true) { notes.push(format!("Bar: {e}")); }
    } else if st.yasb == Some(false) {
        if let Err(e) = lucent::yasb_capsule(false) { notes.push(format!("Bar: {e}")); }
    }
    if windhawk && (refresh_windhawk || brighter || st.windhawk == Some(true)) {
        notes.extend(lucent::windhawk_problem(true));
    } else if st.windhawk == Some(false) {
        notes.extend(lucent::windhawk_problem(false));
    }
    if lucent::restart_pending() {
        notes.insert(0, "(apps need a restart)".into());
    }
    if notes.is_empty() {
        "Applied".into()
    } else {
        if notes[0].starts_with('(') { format!("Applied {}", notes.join(". ")) } else { format!("Applied. {}", notes.join(". ")) }
    }
}
