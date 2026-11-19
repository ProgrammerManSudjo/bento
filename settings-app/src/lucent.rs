//! Lucent theme: the Windows accent colour, as YASB reads it (yasb_colors.css), written into the theme files of other programs.
//! It runs when you ask (the Templates page) and right after a wallpaper is picked here, and never in the background.

use crate::yaml::Config;
use regex::Regex;
use serde_json::{Map, Value};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use std::fs;

const NO_WINDOW: u32 = 0x0800_0000;

// --- the colours ---------------------------------------------------------------------------------------------------------------------------

const NAMES: [(&str, &str); 7] = [
    ("acc", "accent"), ("d1", "accent-dark1"), ("d2", "accent-dark2"), ("d3", "accent-dark3"),
    ("l1", "accent-light1"), ("l2", "accent-light2"), ("l3", "accent-light3"),
];

#[derive(Clone)]
pub struct Palette {
    pub shades: [(&'static str, [u8; 3]); 7],
    extra: Vec<(String, String)>,
    /// the plain grey palette used when the theme is off
    pub mono: bool,
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default())
}

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_default())
}

pub fn colours_css() -> PathBuf {
    home().join(".config").join("yasb").join("yasb_colors.css")
}

/// "--yasb-<name>-rgb: 12, 34, 56" -> [12, 34, 56]
fn find_rgb(css: &str, name: &str) -> Option<[u8; 3]> {
    let key = format!("--yasb-{name}-rgb:");
    let rest = &css[css.find(&key)? + key.len()..];
    let mut n = rest.split(';').next()?.split(',').map(|v| v.trim().parse::<u8>());
    Some([n.next()?.ok()?, n.next()?.ok()?, n.next()?.ok()?])
}

/// How much of Windows' saturation the theme keeps, in percent (a softer accent, like #d09074 from a stronger orange).
pub fn strength() -> i64 {
    crate::wallpaper::read_setting("lucent_strength").and_then(|v| v.parse().ok()).unwrap_or(60).clamp(0, 100)
}

/// (hue 0..1, saturation 0..1, lightness 0..1)
fn to_hsl(rgb: [u8; 3]) -> (f64, f64, f64) {
    let [r, g, b] = rgb.map(|v| v as f64 / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    if max == min {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r { ((g - b) / d).rem_euclid(6.0) } else if max == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 } / 6.0;
    (h, s, l)
}

fn from_hsl(h: f64, s: f64, l: f64) -> [u8; 3] {
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let channel = |t: f64| {
        let t = t.rem_euclid(1.0);
        let v = if t < 1.0 / 6.0 { p + (q - p) * 6.0 * t } else if t < 0.5 { q } else if t < 2.0 / 3.0 { p + (q - p) * (2.0 / 3.0 - t) * 6.0 } else { p };
        (v * 255.0).round().clamp(0.0, 255.0) as u8
    };
    [channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0)]
}

/// The colour with its saturation (HSL) multiplied by `keep`, hue and lightness unchanged.
fn soften(rgb: [u8; 3], keep: f64) -> [u8; 3] {
    let (h, s, l) = to_hsl(rgb);
    if s == 0.0 { rgb } else { from_hsl(h, s * keep, l) }
}

pub fn load() -> Palette {
    load_with(strength())
}

/// The palette at a given softness (percent), for previews of a value that is not saved yet.
pub fn load_with(softness: i64) -> Palette {
    load_for(softness, &setting("lucent_accent"))
}

/// A saved text setting ("" when it has none).
pub fn setting(key: &str) -> String {
    crate::wallpaper::read_setting(key).unwrap_or_default()
}

/// Colours to pick from for an accent: (name, "#rrggbb").
pub const PRESETS: [(&str, &str); 12] = [
    ("Orange", "#c98a6a"), ("Amber", "#c9a56b"), ("Yellow", "#bfb36e"), ("Green", "#8fae7e"), ("Teal", "#6fa89b"), ("Blue", "#7a9cc4"),
    ("Indigo", "#8a8cc4"), ("Purple", "#a68ac4"), ("Pink", "#c48aa8"), ("Red", "#c47a78"), ("Sand", "#b8a894"), ("Slate", "#8a97a6"),
];

/// Tones for a monochrome program: plain greys first, then greys with a trace of colour.
pub const TONES: [(&str, &str); 9] = [
    ("Black", "#0f0f0f"), ("Grey", "#787878"), ("Dark grey", "#5a5a5a"), ("Darker grey", "#3c3c3c"), ("Light grey", "#969696"),
    ("Warm grey", "#8a7d75"), ("Cool grey", "#75808a"), ("Violet grey", "#7d758a"), ("Green grey", "#758a7d"),
];

/// (hue, saturation, lightness), each 0 to 1, of "#rrggbb".
pub fn hex_to_hsl(hex: &str) -> Option<(f32, f32, f32)> {
    let (h, s, l) = to_hsl(parse_hex(hex)?);
    Some((h as f32, s as f32, l as f32))
}

pub fn hsl_to_hex(h: f32, s: f32, l: f32) -> String {
    hex(from_hsl(h as f64, s as f64, l as f64))
}

pub fn rgb_of(hex: &str) -> Option<[u8; 3]> {
    parse_hex(hex)
}

/// All seven shades from one accent colour (softened like Windows' own), darker and lighter by lightness.
fn from_accent(rgb: [u8; 3], softness: i64) -> Palette {
    let (h, s, l) = to_hsl(soften(rgb, softness as f64 / 100.0));
    let dark = |k: f64| from_hsl(h, s, l * k);
    let light = |k: f64| from_hsl(h, s, l + (1.0 - l) * k);
    let table = [from_hsl(h, s, l), dark(0.8), dark(0.6), dark(0.4), light(0.25), light(0.5), light(0.75)];
    let mut shades = [("", [0u8; 3]); 7];
    for (i, (short, _)) in NAMES.iter().enumerate() {
        shades[i] = (short, table[i]);
    }
    Palette { shades, extra: Vec::new(), mono: false }
}

/// A whole number setting.
pub fn number_setting(key: &str, default: i64) -> i64 {
    setting(key).parse().unwrap_or(default)
}

/// The monochrome palette. `tone` is a program's own grey ("" takes the global one); the global brightness and tint apply to every program.
pub fn mono_with(tone: &str) -> Palette {
    let tone = if parse_hex(tone).is_some() { tone.to_string() } else { setting("mono_tone") };
    let tint = number_setting("mono_tint", 100).clamp(0, 100) as f64 / 100.0;
    let bright = 0.6 + 0.8 * number_setting("mono_brightness", 50).clamp(0, 100) as f64 / 100.0; // 50 is as it was
    // the tone's own lightness (against the plain grey, 47 %) makes the greys darker or lighter
    let (h, s, own) = parse_hex(&tone).map(to_hsl).unwrap_or((0.0, 0.0, 0.47));
    // a darker tone darkens only a little: the programs' panels are tints of the accent, and they must stay apart from each other
    let ratio = own / 0.47;
    let bright = bright * if ratio < 1.0 { 0.75 + 0.25 * ratio } else { ratio.min(1.7) };
    let greys = [120u8, 90, 60, 38, 160, 200, 235];
    let mut shades = [("", [0u8; 3]); 7];
    for (i, (short, _)) in NAMES.iter().enumerate() {
        shades[i] = (short, from_hsl(h, s * tint, (greys[i] as f64 / 255.0 * bright).min(0.97)));
    }
    Palette { shades, extra: Vec::new(), mono: true }
}

/// What one program gets: grey when the theme is off or the program is set to monochrome, else its own accent, else the global one.
pub fn app_palette(id: &str) -> Palette {
    if !on() || setting(&format!("app_{id}_mode")) == "mono" {
        return mono_with(&setting(&format!("app_{id}_mono")));
    }
    match setting(&format!("app_{id}_accent")) {
        own if parse_hex(&own).is_some() => load_for(strength(), &own),
        _ => load(),
    }
}

pub fn load_for(softness: i64, accent: &str) -> Palette {
    if let Some(rgb) = parse_hex(accent) {
        return from_accent(rgb, softness);
    }
    let css = fs::read_to_string(colours_css()).unwrap_or_default();
    let keep = softness as f64 / 100.0;
    let mut shades = [("", [0u8; 3]); 7];
    for (i, (short, long)) in NAMES.iter().enumerate() {
        shades[i] = (short, soften(find_rgb(&css, long).unwrap_or([191, 191, 191]), keep));
    }
    Palette { shades, extra: Vec::new(), mono: false }
}

/// Without the accent: a subtle monochrome, dark greys with light text.
pub fn monochrome() -> Palette {
    // dark and quiet: the backgrounds are tints of `acc`, so a mid grey here makes near-black panels (about what a coloured accent gives), with bright text
    let greys = [120u8, 90, 60, 38, 160, 200, 235];
    let mut shades = [("", [0u8; 3]); 7];
    for (i, (short, _)) in NAMES.iter().enumerate() {
        shades[i] = (short, [greys[i]; 3]);
    }
    Palette { shades, extra: Vec::new(), mono: true }
}

fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

fn parse_hex(c: &str) -> Option<[u8; 3]> {
    let h = c.strip_prefix('#')?;
    Some([u8::from_str_radix(h.get(0..2)?, 16).ok()?, u8::from_str_radix(h.get(2..4)?, 16).ok()?, u8::from_str_radix(h.get(4..6)?, 16).ok()?])
}

impl Palette {
    /// This palette plus one more named colour, usable in expressions ("icon", "tab").
    pub fn with(&self, name: &str, colour: &str) -> Palette {
        let mut p = self.clone();
        p.extra.push((name.to_string(), colour.to_string()));
        p
    }

    /// The six terminal colours (red, green, yellow, blue, purple, cyan) at one tone: red, green and yellow keep their meaning but lean toward the
    /// accent; blue, purple and cyan are the accent and its neighbours. With a grey accent they are all grey.
    fn terminal(&self, bright: bool) -> [String; 6] {
        let (hue, sat, _) = to_hsl(self.shade("acc").unwrap_or([128; 3]));
        let vivid = (sat / 0.45).clamp(0.0, 1.0);
        let tone = if bright { 0.74 } else { 0.62 };
        let degrees = |d: f64| d / 360.0;
        // `h0` leaned toward the accent: by `share` of the gap, at most `limit` degrees
        let lean = |h0: f64, share: f64, limit: f64| {
            let gap = (hue - degrees(h0) + 1.5).rem_euclid(1.0) - 0.5;
            degrees(h0) + (gap * share).clamp(-degrees(limit), degrees(limit))
        };
        let tint = |h: f64, saturation: f64, floor: f64| hex(from_hsl(h, (saturation * vivid).max(floor), tone));
        [tint(lean(5.0, 0.5, 15.0), 0.55, 0.22), tint(lean(135.0, 0.4, 22.0), 0.45, 0.18), tint(lean(48.0, 0.6, 28.0), 0.50, 0.18),
         tint(hue, 0.50, 0.0), tint(hue - degrees(16.0), 0.40, 0.0), tint(hue + degrees(16.0), 0.42, 0.0)]
    }

    pub fn shade(&self, name: &str) -> Option<[u8; 3]> {
        self.shades.iter().find(|(n, _)| *n == name).map(|(_, c)| *c)
    }

    pub fn accent_hex(&self) -> String {
        hex(self.shade("acc").unwrap_or([191, 191, 191]))
    }

    /// A colour as written in the app definitions: "#rrggbb", a shade ("acc", "d1"-"d3", "l1"-"l3"), "t .22" (the accent over black at 22%),
    /// "gray 60", "l1 @ .12" (with opacity), "#c4655f * .3" (scaled) or "mix(a, b, .25)". The answer is "#rrggbb" or "#rrggbbaa".
    pub fn colour(&self, expr: &str) -> Result<String, String> {
        let expr = expr.trim();
        if let Some(args) = expr.strip_prefix("mix(").and_then(|s| s.strip_suffix(')')) {
            let p: Vec<&str> = args.split(',').collect();
            if p.len() != 3 {
                return Err(format!("'{expr}': mix(a, b, amount)"));
            }
            let amount: f64 = p[2].trim().parse().map_err(|_| format!("bad amount in '{expr}'"))?;
            let (a, b) = (parse_hex(&self.colour(p[0])?).unwrap_or([0; 3]), parse_hex(&self.colour(p[1])?).unwrap_or([0; 3]));
            let mixed = [0, 1, 2].map(|i| (a[i] as f64 * (1.0 - amount) + b[i] as f64 * amount).round() as u8);
            return Ok(hex(mixed));
        }
        if let Some((base, f)) = expr.split_once('*') {
            let f: f64 = f.trim().parse().map_err(|_| format!("bad number in '{expr}'"))?;
            let c = parse_hex(&self.colour(base)?).unwrap_or([0; 3]);
            return Ok(hex(c.map(|v| (v as f64 * f).round().min(255.0) as u8)));
        }
        if let Some((base, alpha)) = expr.split_once('@') {
            let a: f64 = alpha.trim().parse().map_err(|_| format!("bad opacity in '{expr}'"))?;
            return Ok(format!("{}{:02x}", self.colour(base)?, (255.0 * a).round() as u8));
        }
        if expr.starts_with('#') {
            return Ok(expr.to_lowercase());
        }
        if let Some((kind, n)) = expr.split_once(' ').filter(|(k, _)| *k == "ansi" || *k == "bright") {
            return self.terminal(kind == "bright").get(n.trim().parse::<usize>().map_err(|_| format!("bad number in '{expr}'"))?).cloned().ok_or_else(|| format!("'{expr}': 0 to 5"));
        }
        if let Some(f) = expr.strip_prefix("t ") {
            let f: f64 = f.trim().parse().map_err(|_| format!("bad number in '{expr}'"))?;
            return Ok(hex(self.shade("acc").unwrap_or([191; 3]).map(|v| (v as f64 * f).round().min(255.0) as u8)));
        }
        if let Some(n) = expr.strip_prefix("gray ") {
            let n: f64 = n.trim().parse().map_err(|_| format!("bad number in '{expr}'"))?;
            let v = (n * 2.55).round().clamp(0.0, 255.0) as u8;
            return Ok(hex([v, v, v]));
        }
        if let Some((_, c)) = self.extra.iter().find(|(n, _)| n == expr) {
            return Ok(c.clone());
        }
        self.shade(expr).map(hex).ok_or_else(|| format!("unknown colour '{expr}'"))
    }
}

/// A JSON file's values: every string that starts with "=" is a colour.
fn convert(v: &Value, pal: &Palette) -> Result<Value, String> {
    Ok(match v {
        Value::String(s) => match s.strip_prefix('=') {
            Some(expr) => Value::String(pal.colour(expr).map_err(|e| format!("{s}: {e}"))?),
            None => v.clone(),
        },
        Value::Array(a) => Value::Array(a.iter().map(|x| convert(x, pal)).collect::<Result<_, _>>()?),
        Value::Object(o) => {
            let mut m = Map::new();
            for (k, x) in o {
                m.insert(k.clone(), convert(x, pal)?);
            }
            Value::Object(m)
        }
        other => other.clone(),
    })
}

// --- the programs --------------------------------------------------------------------------------------------------------------------------

pub struct AppInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub hint: &'static str,
}

pub const APPS: [AppInfo; 11] = [
    AppInfo { id: "zed", name: "Zed", hint: "The theme \"YASB Accent\" in Zed's themes folder: pick it once in Zed, it follows after that" },
    AppInfo { id: "discord", name: "Discord", hint: "A theme file for BetterDiscord, Vencord or Equicord (plain Discord can't be themed): switch it on in the client mod" },
    AppInfo { id: "zen", name: "Zen Browser", hint: "yasb-chrome.css and yasb-content.css in every profile; restart Zen to see a change" },
    AppInfo { id: "helium", name: "Helium", hint: "The theme folder .config\\yasb\\themes\\helium: load it once as an unpacked extension; restart Helium to see a change" },
    AppInfo { id: "obsidian", name: "Obsidian", hint: "A CSS snippet \"yasb-colors\" in every vault: switch it on in Settings, Appearance, CSS snippets" },
    AppInfo { id: "terminal", name: "Windows Terminal", hint: "The colour scheme \"YASB Accent\": pick it under a profile's Appearance" },
    AppInfo { id: "filepilot", name: "File Pilot", hint: "The colour scheme \"YASB Accent\". File Pilot must be closed while it is written" },
    AppInfo { id: "tacky", name: "Tacky Borders", hint: "The active window's border colour" },
    AppInfo { id: "spicetify", name: "Spotify", hint: "A \"YASB\" colour scheme in the Comfy Spicetify theme (needs Spicetify and Comfy): it is applied and Spotify restarts to show it" },
    AppInfo { id: "obs", name: "OBS Studio", hint: "A variant of the Yami theme, \"YASB Accent\": pick it once in OBS (Settings, Appearance); restart OBS to see a change" },
    AppInfo { id: "rainmeter", name: "Rainmeter", hint: "The YasbBento skin: its highlights take the accent" },
];

pub fn on() -> bool {
    crate::wallpaper::read_setting("lucent").is_some_and(|v| v == "1")
}

pub fn set_on(on: bool) {
    crate::wallpaper::write_setting("lucent", if on { "1" } else { "0" });
}

/// The programs switched on for the theme (all of them until you switch one off).
pub fn enabled(id: &str) -> bool {
    match crate::wallpaper::read_setting("lucent_apps") {
        Some(list) => list.split(',').any(|x| x == id),
        None => true,
    }
}

pub fn set_enabled(id: &str, on: bool) {
    let mut list: Vec<String> = match crate::wallpaper::read_setting("lucent_apps") {
        Some(l) => l.split(',').filter(|x| !x.is_empty()).map(String::from).collect(),
        None => APPS.iter().map(|a| a.id.to_string()).collect(),
    };
    list.retain(|x| x != id);
    if on {
        list.push(id.to_string());
    }
    crate::wallpaper::write_setting("lucent_apps", &list.join(","));
}

fn write_if_changed(path: &Path, text: &str) -> Result<bool, String> {
    if fs::read_to_string(path).is_ok_and(|old| old == text) {
        return Ok(false);
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(true)
}

fn running(exe: &str) -> bool {
    Command::new("tasklist").args(["/FI", &format!("IMAGENAME eq {exe}"), "/NH"]).creation_flags(NO_WINDOW).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains(&exe.to_lowercase())).unwrap_or(false)
}

fn zed(pal: &Palette) -> Result<String, String> {
    let root = env_path("APPDATA").join("Zed");
    if !root.exists() {
        return Err("Zed is not installed".into());
    }
    let theme: Value = serde_json::from_str(include_str!("../assets/lucent/zed.json")).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&convert(&theme, pal)?).map_err(|e| e.to_string())?;
    write_if_changed(&root.join("themes").join("yasb-colors.json"), &text)?;
    Ok("theme written".into())
}

fn discord(pal: &Palette) -> Result<String, String> {
    let def: Value = serde_json::from_str(include_str!("../assets/lucent/discord.json")).map_err(|e| e.to_string())?;
    let mut vars = String::new();
    for (k, v) in def["vars"].as_object().into_iter().flatten() {
        vars.push_str(&format!("    {k}: {} !important;\n", pal.colour(v.as_str().unwrap_or("").trim_start_matches('='))?));
    }
    let mut css = include_str!("../assets/lucent/discord.css").replace("{{variables}}", vars.trim_end());
    for (k, v) in def["values"].as_object().into_iter().flatten() {
        let raw = v.as_str().unwrap_or("");
        css = css.replace(&format!("{{{{{k}}}}}"), &pal.colour(raw).unwrap_or_else(|_| raw.to_string()));
    }
    let mut done = 0;
    for client in ["BetterDiscord", "Vencord", "Equicord"] {
        let dir = env_path("APPDATA").join(client);
        if dir.exists() {
            write_if_changed(&dir.join("themes").join("yasb-colors.theme.css"), &css)?;
            done += 1;
        }
    }
    if done == 0 { Err("no BetterDiscord, Vencord or Equicord folder found".into()) } else { Ok(format!("{done} client(s)")) }
}

/// The config with the scheme in its "Colors" list: an existing one is replaced where it stands, otherwise it is added at the top. (text, was new)
fn filepilot_update(text: &str, name: &str, scheme: &[(String, String)]) -> Result<(String, bool), String> {
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let list = Regex::new(r#""Colors"\s*:\s*\["#).unwrap().find(text)
        .ok_or("FPilot-Config.json has no \"Colors\" list yet: in File Pilot open Settings (Ctrl+,), hover a colour scheme and press its fork icon, then try again")?;
    let mut rows = vec![format!("\t\t\"{name}\":"), "\t\t{".to_string()];
    for (i, (k, v)) in scheme.iter().enumerate() {
        rows.push(format!("\t\t\t\"{k}\": \"{v}\"{}", if i + 1 < scheme.len() { "," } else { "" }));
    }
    rows.push("\t\t}".to_string());
    let block = rows.join(nl);
    let after = &text[list.end()..];
    if let Some(old) = Regex::new(&format!(r#"[ \t]*"{}"\s*:\s*\{{"#, regex::escape(name))).unwrap().find(after) {
        let start = list.end() + old.start();
        let bytes = text.as_bytes();
        let (mut depth, mut i) = (0i32, list.end() + old.end() - 1);
        while i < bytes.len() {
            depth += match bytes[i] { b'{' => 1, b'}' => -1, _ => 0 };
            if depth == 0 {
                break;
            }
            i += 1;
        }
        return Ok((format!("{}{}{}", &text[..start], block, &text[i + 1..]), false));
    }
    let empty = after.trim_start().starts_with(']');
    Ok((format!("{}{nl}{block}{}{after}", &text[..list.end()], if empty { "" } else { "," }), true))
}

fn filepilot(pal: &Palette) -> Result<String, String> {
    let def: Value = serde_json::from_str(include_str!("../assets/lucent/filepilot.json")).map_err(|e| e.to_string())?;
    let name = def["name"].as_str().unwrap_or("YASB Accent").to_string();
    let mut scheme = Vec::new();
    for (k, e) in def["colors"].as_object().into_iter().flatten() {
        let c = pal.colour(e.as_str().unwrap_or("").trim_start_matches('=')).map_err(|m| format!("{k}: {m}"))?;
        scheme.push((k.clone(), c.trim_start_matches('#').to_uppercase()));
    }
    let mut done = 0;
    for dir in [env_path("LOCALAPPDATA").join("Voidstar").join("FilePilot"), env_path("APPDATA").join("Voidstar").join("FilePilot")] {
        let path = dir.join("FPilot-Config.json");
        let Ok(raw) = fs::read(&path) else { continue };
        if running("FilePilot.exe") || running("FPilot.exe") {
            theme_when_closed(pal.clone());
            offer_if_running("filepilot");
            return Err("File Pilot needs a restart".into());
        }
        let bom = raw.starts_with(&[0xef, 0xbb, 0xbf]);
        let text = String::from_utf8_lossy(if bom { &raw[3..] } else { &raw }).into_owned();
        let (mut new, created) = filepilot_update(&text, &name, &scheme)?;
        if created {  // the first time it is also the selected scheme (afterwards the one you pick stays yours)
            new = Regex::new(r#"("ColorScheme"\s*:\s*)"[^"]*""#).unwrap().replacen(&new, 1, format!("${{1}}\"{name}\"")).into_owned();
            new = Regex::new(r#"("SystemColorScheme"\s*:\s*)true"#).unwrap().replacen(&new, 1, "${1}false").into_owned();
        }
        if new != text {
            let mut out = if bom { vec![0xef, 0xbb, 0xbf] } else { vec![] };
            out.extend(new.as_bytes());
            fs::write(&path, out).map_err(|e| e.to_string())?;
        }
        done += 1;
    }
    if done == 0 { Err("File Pilot's FPilot-Config.json was not found".into()) } else { Ok("scheme written".into()) }
}

fn tacky(pal: &Palette) -> Result<String, String> {
    let path = home().join(".config").join("tacky-borders").join("config.yaml");
    if !path.exists() {
        return Err("Tacky Borders has no config.yaml".into());
    }
    let mut cfg = Config::load(path);
    if let Some(e) = &cfg.error {
        return Err(e.clone());
    }
    // a spiral's gradient (picked on the Tacky Borders page) is the user's: only a plain colour follows the theme
    if !matches!(cfg.at(&["global", "active_color"]), yaml_rust2::Yaml::Hash(_)) {
        cfg.set_text(&["global", "active_color"], &pal.accent_hex())?;
    }
    cfg.set_text(&["global", "inactive_color"], &pal.colour("mix(d2, gray 8, .55)")?)?;
    cfg.apply()?;
    Ok("border colours set".into())
}

/// Obsidian: a CSS snippet in every vault (the vault folders are listed in obsidian.json).
fn obsidian(pal: &Palette) -> Result<String, String> {
    let list = env_path("APPDATA").join("obsidian").join("obsidian.json");
    let json: Value = serde_json::from_str(&fs::read_to_string(&list).map_err(|_| "Obsidian is not installed".to_string())?).map_err(|e| e.to_string())?;
    let def: Value = serde_json::from_str(include_str!("../assets/lucent/obsidian.json")).map_err(|e| e.to_string())?;
    let mut vars = String::new();
    for (k, e) in def["vars"].as_object().into_iter().flatten() {
        // !important: a theme such as Baseline builds its greys from its own variables, and its settings plugin writes them late
        vars.push_str(&format!("    {k}: {} !important;\n", pal.colour(e.as_str().unwrap_or(""))?));
    }
    let css = include_str!("../assets/lucent/obsidian.css").replace("{{variables}}", vars.trim_end());
    let mut done = 0;
    for vault in json["vaults"].as_object().into_iter().flatten().filter_map(|(_, v)| v["path"].as_str()) {
        let root = PathBuf::from(vault);
        if root.join(".obsidian").is_dir() {
            write_if_changed(&root.join(".obsidian").join("snippets").join("yasb-colors.css"), &css)?;
            done += 1;
        }
    }
    if done == 0 { Err("no Obsidian vault found".into()) } else { Ok(format!("{done} vault(s)")) }
}

/// OBS Studio: a variant of its Yami theme. Only Yami's colour variables are replaced (no widget rules), so Yami keeps drawing everything and
/// OBS updates do not break it. The id stays the same, so a theme chosen once keeps following the accent.
fn obs(pal: &Palette) -> Result<String, String> {
    let dir = env_path("APPDATA").join("obs-studio");
    if !dir.is_dir() {
        return Err("OBS Studio is not installed".into());
    }
    let def: Value = serde_json::from_str(include_str!("../assets/lucent/obs.json")).map_err(|e| e.to_string())?;
    let mut vars = String::new();
    for (k, e) in def["vars"].as_object().into_iter().flatten() {
        vars.push_str(&format!("    {k}: {};\n", pal.colour(e.as_str().unwrap_or(""))?.chars().take(7).collect::<String>()));
    }
    let text = format!("@OBSThemeMeta {{\n    name: 'YASB Accent';\n    id: 'com.obsproject.Yami.YASBAccent';\n    extends: 'com.obsproject.Yami';\n    author: 'YASB Settings';\n    dark: 'true';\n}}\n\n@OBSThemeVars {{\n{vars}}}\n");
    let changed = write_if_changed(&dir.join("themes").join("Yami_YASB_Accent.ovt"), &text)?;
    if changed {
        offer_if_running("obs");
    }
    Ok("theme written".into())
}

/// Spotify through Spicetify, the way Noctalia does it: the Comfy theme stays as it is and gets one more colour scheme, "YASB", in its color.ini,
/// built from the Lucent palette. Comfy is made the current theme with that scheme and applied. Spotify has to restart for it: when it is open the
/// "Restart apps" button does that, otherwise it is applied right away.
fn spicetify(pal: &Palette) -> Result<String, String> {
    let root = env_path("APPDATA").join("spicetify");
    if !root.is_dir() {
        return Err("Spicetify is not installed".into());
    }
    let comfy = root.join("Themes").join("Comfy").join("color.ini");
    let existing = fs::read_to_string(&comfy).map_err(|_| "the Comfy theme is not installed (add it in the Spicetify Marketplace)".to_string())?;
    let def: Value = serde_json::from_str(include_str!("../assets/lucent/spicetify.json")).map_err(|e| e.to_string())?;
    let mut section = String::from("[YASB]\n");
    for (key, expr) in def["colors"].as_object().into_iter().flatten() {
        let c = pal.colour(expr.as_str().unwrap_or(""))?;
        section.push_str(&format!("{key:<19}= {}\n", c.trim_start_matches('#').chars().take(6).collect::<String>().to_uppercase()));
    }
    // the file again without an old [YASB] section, with the new one at the end
    let nl = if existing.contains("\r\n") { "\r\n" } else { "\n" };
    let mut kept: Vec<&str> = Vec::new();
    let mut skipping = false;
    for line in existing.lines() {
        if line.trim() == "[YASB]" {
            skipping = true;
        } else if line.trim_start().starts_with('[') {
            skipping = false;
        }
        if !skipping {
            kept.push(line);
        }
    }
    let mut text = kept.join(nl).trim_end().to_string();
    text.push_str(nl);
    text.push_str(nl);
    text.push_str(&section.replace('\n', nl));
    let mut changed = write_if_changed(&comfy, &text)?;
    // Spicetify's own settings: Comfy with this scheme must be the current ones
    let current = |key: &str| -> String {
        fs::read_to_string(root.join("config-xpui.ini")).unwrap_or_default().lines()
            .find_map(|l| l.strip_prefix(key).and_then(|r| r.trim_start().strip_prefix('=')).map(|v| v.trim().to_string())).unwrap_or_default()
    };
    if current("current_theme") != "Comfy" || current("color_scheme") != "YASB" {
        let ok = Command::new("spicetify").args(["config", "current_theme", "Comfy", "color_scheme", "YASB", "inject_css", "1", "replace_colors", "1", "overwrite_assets", "1", "inject_theme_js", "1"]).creation_flags(NO_WINDOW).output().map(|o| o.status.success()).unwrap_or(false);
        if !ok {
            return Err("could not run spicetify (is it on the PATH?)".into());
        }
        changed = true;
    }
    if !changed {
        return Ok("up to date".into());
    }
    if main_process("spicetify").is_some() {
        offer_if_running("spicetify");
        return Err("Spotify needs a restart".into());
    }
    let ok = Command::new("spicetify").args(["apply", "-n"]).creation_flags(NO_WINDOW).output().map(|o| o.status.success()).unwrap_or(false);
    if ok { Ok("applied".into()) } else { Err("spicetify apply did not finish".into()) }
}

/// Windows Terminal: a settings fragment with the scheme "YASB Accent" (Terminal finds fragments by itself).
fn terminal(pal: &Palette) -> Result<String, String> {
    let local = env_path("LOCALAPPDATA");
    let installed = ["Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe", "Packages/Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe", "Microsoft/Windows Terminal"].iter().any(|d| local.join(d).exists());
    if !installed {
        return Err("Windows Terminal is not installed".into());
    }
    let scheme: Value = serde_json::from_str(include_str!("../assets/lucent/wt.json")).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&convert_plain(&scheme, pal)?).map_err(|e| e.to_string())?;
    write_if_changed(&local.join("Microsoft").join("Windows Terminal").join("Fragments").join("yasb").join("yasb-accent.json"), &text)?;
    Ok("scheme written".into())
}

/// Like `convert`, for the definitions whose colours are plain expressions (no leading "=") and whose "name" stays as it is.
fn convert_plain(v: &Value, pal: &Palette) -> Result<Value, String> {
    Ok(match v {
        Value::String(s) => Value::String(pal.colour(s).unwrap_or_else(|_| s.clone())),
        Value::Array(a) => Value::Array(a.iter().map(|x| convert_plain(x, pal)).collect::<Result<_, _>>()?),
        Value::Object(o) => Value::Object(o.iter().map(|(k, x)| if k == "name" { Ok((k.clone(), x.clone())) } else { convert_plain(x, pal).map(|c| (k.clone(), c)) }).collect::<Result<Map<_, _>, String>>()?),
        other => other.clone(),
    })
}

/// Helium (and other Chromium browsers): a theme extension, a folder with a manifest you load once. The version changes with the colours so the browser notices.
fn helium(pal: &Palette) -> Result<String, String> {
    use std::hash::{Hash, Hasher};
    let def: Value = serde_json::from_str(include_str!("../assets/lucent/helium.json")).map_err(|e| e.to_string())?;
    let pal = pal.with("address", &pal.colour("mix(l1, gray 20, .45)")?);
    let mut colours: Map<String, Value> = Map::new();
    for key in ["colors", "mode_black"] {
        for (k, e) in def[key].as_object().into_iter().flatten() {
            let c = pal.colour(e.as_str().unwrap_or(""))?;
            let rgb = parse_hex(&c).unwrap_or([0; 3]);
            colours.insert(k.clone(), Value::from(rgb.to_vec()));
        }
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    colours.iter().for_each(|(k, v)| (k, v.to_string()).hash(&mut hasher));
    let manifest = serde_json::json!({
        "manifest_version": 3, "name": "YASB Accent", "version": format!("1.{}.0", hasher.finish() & 0xFFFF),
        "description": "The Windows accent colour, written by YASB Settings (Lucent theme)",
        "theme": { "colors": colours, "tints": { "buttons": [-1, -1, -1] } },
    });
    let dir = home().join(".config").join("yasb").join("themes").join("helium");
    let changed = write_if_changed(&dir.join("manifest.json"), &serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?)?;
    write_if_changed(&dir.join("README.txt"), include_str!("../assets/lucent/helium-readme.txt"))?;
    if changed {
        let _ = fs::remove_file(dir.join("Cached Theme.pak")); // the browser builds the theme again from the new manifest
    }
    Ok("theme written".into())
}

fn rainmeter_exe() -> Option<PathBuf> {
    [env_path("ProgramFiles").join(r"Rainmeter\Rainmeter.exe"), home().join(r"scoop\apps\rainmeter\current\Rainmeter.exe")].into_iter().find(|p| p.is_file())
}

fn rainmeter(pal: &Palette) -> Result<String, String> {
    let skin = home().join("Documents").join("Rainmeter").join("Skins").join("YasbBento");
    if !skin.exists() {
        return Err("the YasbBento skin is not installed".into());
    }
    let [r, g, b] = if pal.mono { [255, 255, 255] } else { pal.shade("acc").unwrap_or([255, 255, 255]) };
    // the grain behind the clock: the accent, scaled so that its brightest channel is about 140 (a bright accent does not make bright grain)
    let k = 140.0 / (r.max(g).max(b).max(1) as f32);
    let [gr, gg, gb] = [r, g, b].map(|v| (v as f32 * k.min(1.4)).round().min(255.0) as u8);
    let text = format!("[Variables]\nAccent={r},{g},{b}\nGrain={gr},{gg},{gb}\n");
    let changed = write_if_changed(&skin.join("@Resources").join("Accent.inc"), &text)?;
    if changed {
        if let Some(exe) = rainmeter_exe() {
            let _ = Command::new(exe).args(["!Refresh", "YasbBento"]).creation_flags(NO_WINDOW).output();
        }
    }
    Ok("accent written".into())
}

pub fn apply_app(id: &str, pal: &Palette) -> Result<String, String> {
    match id {
        "zed" => zed(pal),
        "discord" => discord(pal),
        "filepilot" => filepilot(pal),
        "tacky" => tacky(pal),
        "zen" => crate::lucent_browser::write(pal).map(|n| {
            offer_if_running("zen");
            format!("{n} profile(s)")
        }),
        "helium" => helium(pal).map(|m| {
            offer_if_running("helium");
            m
        }),
        "obsidian" => obsidian(pal),
        "terminal" => terminal(pal),
        "rainmeter" => rainmeter(pal),
        "spicetify" => spicetify(pal),
        "obs" => obs(pal),
        _ => Err("unknown program".into()),
    }
}

/// Every switched-on program, one result each: (id, Ok(what was done) or Err(why not)).
pub fn apply_all() -> Vec<(&'static str, Result<String, String>)> {
    // theme on: the accent; off: back to the greys
    APPS.iter().filter(|a| enabled(a.id)).map(|a| (a.id, apply_app(a.id, &app_palette(a.id)))).collect()
}

/// A few words for the toast: how many programs took the colour, and the first reason when one did not.
pub fn summary(results: &[(&'static str, Result<String, String>)]) -> String {
    let ok = results.iter().filter(|(_, r)| r.is_ok()).count();
    match results.iter().find_map(|(id, r)| r.as_ref().err().map(|e| (id, e))) {
        Some((id, e)) => {
            let name = APPS.iter().find(|a| a.id == *id).map(|a| a.name).unwrap_or(id);
            let short: String = e.chars().take(140).collect();
            format!("Theme: {ok} updated. {name}: {short}")
        }
        None if !on() => format!("Back to monochrome in {ok} programs"),
        None => format!("Theme applied to {ok} programs"),
    }
}

/// After a wallpaper is set Windows works out its accent colour and YASB writes yasb_colors.css: wait for that file to change (a few seconds at most).
pub fn wait_for_new_accent(before: &str) {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(6) {
        if fs::read_to_string(colours_css()).is_ok_and(|now| now != before) {
            std::thread::sleep(Duration::from_millis(300));
            return;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn last_results() -> &'static std::sync::Mutex<Vec<(String, String)>> {
    static LAST: std::sync::OnceLock<std::sync::Mutex<Vec<(String, String)>>> = std::sync::OnceLock::new();
    LAST.get_or_init(Default::default)
}

/// Keep what the last run did, per program, for the Templates page.
pub fn remember(results: &[(&'static str, Result<String, String>)]) {
    let mut last = last_results().lock().unwrap();
    for (id, r) in results {
        let text = match r {
            Ok(done) => done.clone(),
            Err(e) => e.chars().take(60).collect(),
        };
        last.retain(|(i, _)| i != id);
        last.push((id.to_string(), text));
    }
}

pub fn last(id: &str) -> Option<String> {
    last_results().lock().unwrap().iter().find(|(i, _)| i == id).map(|(_, t)| t.clone())
}

/// File Pilot rewrites its config when it closes, so a theme written while it runs is lost. This waits (in this app only, at most half an hour)
/// until it has been closed, then writes the scheme.
fn theme_when_closed(pal: Palette) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static WAITING: AtomicBool = AtomicBool::new(false);
    if WAITING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(1800) {
            std::thread::sleep(Duration::from_secs(3));
            if !running("FilePilot.exe") && !running("FPilot.exe") {
                let result = filepilot(&pal);
                remember(&[("filepilot", result)]);
                break;
            }
        }
        WAITING.store(false, Ordering::SeqCst);
    });
}

/// The colours the settings window itself borrows while the theme is on: (the near-white, the glow of the borders, the accent). None when it is off.
pub fn tint() -> Option<[[u8; 3]; 3]> {
    if !on() {
        return None;
    }
    let pal = load();
    let (l3, l1, acc) = (pal.shade("l3")?, pal.shade("l1")?, pal.shade("acc")?);
    let glow = [0, 1, 2].map(|i| ((l3[i] as u16 + l1[i] as u16 + acc[i] as u16) / 3) as u8);
    Some([l3, glow, acc])
}

// --- the dark capsules: the bar's and Windhawk's taskbar buttons ------------------------------------------------------------------------

/// The three greys the capsules use: resting (#141414), the active one (#1a1a1a) and the clock (#292929).
pub struct Capsule {
    pub rest: String,
    pub active: String,
    pub clock: String,
}

pub fn capsule_on(which: &str) -> bool {
    crate::wallpaper::read_setting(&format!("lucent_{which}")).is_some_and(|v| v == "1")
}

pub fn set_capsule_on(which: &str, on: bool) {
    crate::wallpaper::write_setting(&format!("lucent_{which}"), if on { "1" } else { "0" });
}

/// Just as dark as before, with the (softened) accent in it: the accent's hue and saturation at the same lightness as the greys.
/// With a grey accent that is exactly the greys. `tinted` false gives the plain greys.
pub fn capsule(which: &str, tinted: bool) -> Capsule {
    capsule_with(which, tinted, strength(), effect())
}

/// The palette a capsule target ("yasb" or "windhawk") uses while the theme is on: its own accent when it has one, else the global accent.
fn target_palette(which: &str, softness: i64) -> Palette {
    let own = setting(&format!("{which}_accent"));
    if parse_hex(&own).is_some() { load_for(softness, &own) } else { load_with(softness) }
}

pub fn capsule_with(which: &str, tinted: bool, softness: i64, effect: i64) -> Capsule {
    // a lighter or darker global accent than Windows' own makes the capsules lighter or darker too (1 when it is Windows' own)
    let (h, s, shade) = if tinted {
        if on() {
            let (h, s, l) = to_hsl(target_palette(which, softness).shade("acc").unwrap_or([128; 3]));
            let (_, _, l_win) = to_hsl(load_for(softness, "").shade("acc").unwrap_or([128; 3]));
            (h, s, (l / l_win.max(0.05)).clamp(0.5, 2.2))
        } else {
            // the theme is off: the capsules follow the monochrome theme's tone, tint and brightness (or their own tone)
            let own_tone = setting(&format!("{which}_mono"));
            let tone = if parse_hex(&own_tone).is_some() { own_tone.clone() } else { setting("mono_tone") };
            let (h, s, _) = to_hsl(mono_with(&own_tone).shade("acc").unwrap_or([128; 3]));
            let tone_l = parse_hex(&tone).map(|c| to_hsl(c).2).unwrap_or(0.47);
            (h, s, (0.6 + 0.8 * number_setting("mono_brightness", 50).clamp(0, 100) as f64 / 100.0) * (tone_l / 0.47).clamp(0.2, 2.0))
        }
    } else {
        (0.0, 0.0, 1.0)
    };
    let lift = if tinted { brighten_by(effect) * shade } else { 1.0 };
    let tone = |v: u8| hex(from_hsl(h, s, (v as f64 / 255.0 * lift).min(1.0)));
    // the active one (the focused app, the active workspace) stands out more when tinted: the same tone as the bar's active workspace
    let active = if tinted { hex(from_hsl(h, s, (0.215 * lift).min(1.0))) } else { tone(0x1a) };
    Capsule { rest: tone(0x14), active, clock: tone(0x29) }
}

/// The "lucent effect", 1 to 100 percent: how far the capsules are brightened. 1 is as bright as they are now, 100 is 35 percent brighter: never drastic.
pub fn effect() -> i64 {
    crate::wallpaper::read_setting("lucent_effect").and_then(|v| v.parse().ok()).unwrap_or(35).clamp(1, 100)
}

fn brighten() -> f64 {
    brighten_by(effect())
}

fn brighten_by(effect: i64) -> f64 {
    1.0 + 0.35 * (effect.clamp(1, 100) - 1) as f64 / 99.0
}

const CAPSULE_START: &str = "/* yasb-settings: capsule */";
const CAPSULE_END: &str = "/* end yasb-settings: capsule */";

/// The bar: a block at the end of styles.css gives the tiles the capsule colour (the tiles are the ones themes/yasb_mono.css paints #141414).
/// Off removes the block again.
pub fn yasb_capsule(on: bool) -> Result<String, String> {
    let dir = home().join(".config").join("yasb");
    let css_path = dir.join("styles.css");
    let text = fs::read_to_string(&css_path).map_err(|e| format!("styles.css: {e}"))?;
    let crlf = text.contains("\r\n");
    let mut base = match (text.find(CAPSULE_START), text.find(CAPSULE_END)) {
        (Some(a), Some(b)) if b > a => format!("{}{}", &text[..a], text[b + CAPSULE_END.len()..].trim_start_matches(['\r', '\n'])),
        _ => text.clone(),
    };
    base = base.trim_end().to_string();
    if on {
        let mono = fs::read_to_string(dir.join("themes").join("yasb_mono.css")).map_err(|e| format!("yasb_mono.css: {e}"))?;
        let selectors = Regex::new(r"(?s)([^{}]*?)\{\s*background-color:\s*#141414;\s*\}").unwrap().captures(&mono)
            .map(|c| Regex::new(r"(?s)/\*.*?\*/").unwrap().replace_all(&c[1], "").trim().to_string()).filter(|s| !s.is_empty())
            .ok_or("the capsule rule (background-color: #141414) is not in themes/yasb_mono.css")?;
        base.push_str(&format!("\n\n{CAPSULE_START}\n{}\n{CAPSULE_END}", bar_block(&selectors)));
    }
    base.push('\n');
    let out = if crlf { base.replace("\r\n", "\n").replace('\n', "\r\n") } else { base };
    if out != text {
        fs::write(&css_path, out).map_err(|e| e.to_string())?;
    }
    Ok(if on { "Bar capsules take the accent".into() } else { "Bar capsules are grey again".into() })
}

/// The floating menus, and the highlighted row in them.
const POPUP_HOVERS: [&str; 6] = [".home-menu .menu-item:hover", ".komorebi-layout-menu .menu-item:hover", ".context-menu .menu-item:hover", ".power-menu-popup .button.hover",
    ".power-menu-compact .button.hover", ".komorebi-control-menu .button:hover"];

/// What the bar gets while the capsules take the accent: the capsule colour, the text and icons in a tint of it (the bar's text colour is a variable),
/// and the workspace and taskbar highlights in the tint too, the populated workspaces included.
fn bar_block(capsule_selectors: &str) -> String {
    let pal = if on() { target_palette("yasb", strength()) } else { mono_with(&setting("yasb_mono")) };
    let (h, s, _) = to_hsl(pal.shade("acc").unwrap_or([128; 3]));
    let lift = brighten();
    let caps = capsule("yasb", true);
    let text = parse_hex(&pal.colour("mix(l3, #ffffff, .3)").unwrap_or_else(|_| "#f1f1f1".into())).unwrap_or([241; 3]);
    let text_on_active = parse_hex(&pal.colour("mix(l3, #ffffff, .6)").unwrap_or_else(|_| "#ffffff".into())).unwrap_or([255; 3]);
    let l1 = pal.shade("l1").unwrap_or([200; 3]);
    let tone = |light: f64| hex(from_hsl(h, s, (light * lift).min(1.0)));
    let rgb = |c: [u8; 3]| format!("{}, {}, {}", c[0], c[1], c[2]);
    // on the dark greys of the monochrome theme the workspace and taskbar buttons need more of their light tint to stand apart from the capsule
    let boost = if on() { 1.0 } else { 2.4 };
    let tint = |a: f64| format!("rgba({}, {:.2})", rgb(l1), (a * boost).min(0.6));
    // the floating menus: a dark tone of the accent too (they were plain black), and their highlighted rows follow the accent
    let acc_rgb = rgb(pal.shade("acc").unwrap_or([128; 3]));
    let hovers = POPUP_HOVERS.join(",\n");
    // the accent colours the bar's styles use (the clock's big date, selected days, highlights): they are grey by default
    let mut shade_vars = String::new();
    for (short, css) in [("acc", "accent"), ("d1", "accent-dark1"), ("d2", "accent-dark2"), ("l1", "accent-light1"), ("l2", "accent-light2")] {
        let c = pal.shade(short).unwrap_or([128; 3]);
        shade_vars.push_str(&format!("    --yasb-{css}: rgb({});\n    --yasb-{css}-rgb: {};\n", rgb(c), rgb(c)));
    }
    format!(
        "{hovers} {{\n    background-color: rgba({acc_rgb}, 0.30);\n}}\n\
:root {{\n{shade_vars}    --yasb-accent-dark3: {};\n    --yasb-accent-dark3-rgb: {};\n    --yasb-accent-light3: {};\n    --yasb-accent-light3-rgb: {};\n}}\n\
{capsule_selectors} {{\n    background-color: {};\n}}\n\
.yasb-bar .komorebi-workspaces .ws-btn {{ background-color: {}; }}\n\
.yasb-bar .komorebi-workspaces .ws-btn:hover {{ background-color: {}; }}\n\
.yasb-bar .komorebi-workspaces .ws-btn.populated {{ background-color: {}; }}\n\
.yasb-bar .komorebi-workspaces .ws-btn.populated:hover {{ background-color: {}; }}\n\
.yasb-bar .komorebi-workspaces .ws-btn.active {{ background-color: {}; }}\n\
.yasb-bar .taskbar-widget .app-container.running {{ background-color: {}; }}\n\
.yasb-bar .taskbar-widget .app-container:hover {{ background-color: {}; }}\n\
.yasb-bar .taskbar-widget .app-container.foreground,\n.yasb-bar .taskbar-widget .app-container.foreground:hover {{ background-color: {}; }}",
        hex(text), rgb(text), hex(text_on_active), rgb(text_on_active),
        caps.rest,
        tint(0.05), tint(0.10), tint(0.10), tint(0.18), tone(0.215),
        tint(0.05), tint(0.11), tone(0.19),
    )
}

/// Windhawk keeps its mod settings where only administrators can write (HKLM), so this runs a short script elevated: Windows asks you to allow it
/// each time. The script only takes the three colours (checked) and sets the capsule colours in the taskbar, start menu and notification center styler mods.
pub fn windhawk_capsule(on: bool) -> String {
    let cli = env_path("ProgramFiles").join("Windhawk").join("windhawk-cli.exe");
    if !cli.is_file() {
        return "Windhawk is not installed".into();
    }
    let c = capsule("windhawk", on);
    let data = home().join("AppData").join("Local").join("yasb-settings");
    let _ = fs::create_dir_all(&data);
    let json = format!("{{\"rest\":\"{}\",\"active\":\"{}\",\"clock\":\"{}\"}}", c.rest, c.active, c.clock);
    let script = data.join("windhawk-apply.ps1");
    let result = data.join("windhawk-result.txt");
    if fs::write(data.join("windhawk.json"), json).is_err() || fs::write(&script, include_str!("../assets/lucent/windhawk-apply.ps1")).is_err() {
        return "Could not write the Windhawk colours".into();
    }
    let _ = fs::remove_file(&result);
    let launch = format!(
        "Start-Process powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-WindowStyle','Hidden','-File','{}'",
        script.display()
    );
    let started = Command::new("powershell").args(["-NoProfile", "-Command", &launch]).creation_flags(NO_WINDOW).output();
    if started.map(|o| !o.status.success()).unwrap_or(true) {
        return "Windhawk was not changed: the administrator prompt was declined".into();
    }
    match fs::read_to_string(&result) {
        Ok(r) if r.trim().starts_with("ok") => if on { "Taskbar buttons take the accent".into() } else { "Taskbar buttons are grey again".into() },
        Ok(r) => format!("Windhawk: {}", r.trim()),
        Err(_) => "Windhawk did not answer".into(),
    }
}

/// Only these programs (when they are switched on): what a change to one program's own colours writes.
pub fn apply_some(ids: &[String]) -> Vec<(&'static str, Result<String, String>)> {
    APPS.iter().filter(|a| enabled(a.id) && ids.iter().any(|i| i == a.id)).map(|a| (a.id, apply_app(a.id, &app_palette(a.id)))).collect()
}

/// A program that is open needs a restart to show what was just written.
pub fn restart_pending() -> bool {
    !offers().lock().unwrap().is_empty()
}

/// What did not work, one short line per program (nothing when all went well).
pub fn problems(results: &[(&'static str, Result<String, String>)]) -> Vec<String> {
    results
        .iter()
        .filter_map(|(id, r)| r.as_ref().err().filter(|e| !e.ends_with("needs a restart")).map(|e| {
            let name = APPS.iter().find(|a| a.id == *id).map(|a| a.name).unwrap_or(id);
            if e.starts_with(name) { e.chars().take(80).collect() } else { format!("{name}: {}", e.chars().take(80).collect::<String>()) }
        }))
        .collect()
}

/// Windhawk's result as a problem line, or nothing when the message is not a failure.
pub fn windhawk_problem(on: bool) -> Option<String> {
    let m = windhawk_capsule(on);
    let l = m.to_lowercase();
    if l.contains("fail") || l.contains("could not") || l.contains("not ") || l.contains("cancel") { Some(m) } else { None }
}

/// After a wallpaper: the capsules that are switched on follow the new accent. A few words for the toast ("" when none is on).
pub fn apply_capsules() -> String {
    let mut notes: Vec<String> = Vec::new();
    if capsule_on("yasb") {
        notes.push(yasb_capsule(true).unwrap_or_else(|e| format!("Bar capsules: {e}")));
    }
    if capsule_on("windhawk") {
        notes.push(windhawk_capsule(true));
    }
    notes.join(". ")
}

// --- programs that read their theme only when they start ------------------------------------------------------------------------------------

fn offers() -> &'static std::sync::Mutex<Vec<&'static str>> {
    static OFFERS: std::sync::OnceLock<std::sync::Mutex<Vec<&'static str>>> = std::sync::OnceLock::new();
    OFFERS.get_or_init(Default::default)
}

/// The main process of a program (its pid and its exe), when it is open.
fn main_process(id: &str) -> Option<(String, String)> {
    let filter = match id {
        "zen" => "$_.ProcessName -eq 'zen' -and $_.MainWindowHandle -ne 0",
        "helium" => "$_.MainWindowHandle -ne 0 -and ($_.Path -like '*\\Helium\\*' -or $_.ProcessName -like '*helium*')",
        "spicetify" => "$_.ProcessName -eq 'Spotify' -and $_.MainWindowHandle -ne 0",
        "obs" => "$_.ProcessName -eq 'obs64' -and $_.MainWindowHandle -ne 0",
        "filepilot" => "($_.ProcessName -eq 'FPilot' -or $_.ProcessName -eq 'FilePilot') -and $_.MainWindowHandle -ne 0",
        _ => return None,
    };
    let script = format!("Get-Process | Where-Object {{ {filter} }} | Select-Object -First 1 | ForEach-Object {{ '{{0}}|{{1}}' -f $_.Id, $_.Path }}");
    let out = Command::new("powershell").args(["-NoProfile", "-Command", &script]).creation_flags(NO_WINDOW).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    text.split_once('|').map(|(a, b)| (a.to_string(), b.to_string()))
}

/// Only a program that is open needs a restart to show the new colours (otherwise it reads them when it starts).
fn offer_if_running(id: &'static str) {
    if main_process(id).is_some() {
        let mut list = offers().lock().unwrap();
        if !list.contains(&id) {
            list.push(id);
        }
    }
}

fn to_restart() -> &'static std::sync::Mutex<Vec<&'static str>> {
    static LIST: std::sync::OnceLock<std::sync::Mutex<Vec<&'static str>>> = std::sync::OnceLock::new();
    LIST.get_or_init(Default::default)
}

/// The button for the message after an apply: one "Restart apps" for every program that needs it (id, label), or none.
pub fn take_offers() -> Vec<(String, String)> {
    let found: Vec<&'static str> = offers().lock().unwrap().drain(..).collect();
    if found.is_empty() {
        return Vec::new();
    }
    *to_restart().lock().unwrap() = found;
    vec![("apps".to_string(), "Restart apps".to_string())]
}

fn alive(pid: &str) -> bool {
    Command::new("tasklist").args(["/FI", &format!("PID eq {pid}"), "/NH"]).creation_flags(NO_WINDOW).output().map(|o| String::from_utf8_lossy(&o.stdout).contains(pid)).unwrap_or(false)
}

/// Close the program the polite way (it saves its tabs), wait for it, and open it again. Helium is asked to bring its tabs back.
fn restart_one(id: &str) -> String {
    let name = match id { "zen" => "Zen", "filepilot" => "File Pilot", "spicetify" => "Spotify", "obs" => "OBS Studio", _ => "Helium" };
    let Some((pid, path)) = main_process(id) else {
        return format!("{name} is not open: it reads the new colours when it starts");
    };
    let _ = Command::new("taskkill").args(if id == "spicetify" { vec!["/T", "/PID", &pid] } else { vec!["/PID", &pid] }).creation_flags(NO_WINDOW).output();
    let start = Instant::now();
    while alive(&pid) && start.elapsed() < Duration::from_secs(15) {
        std::thread::sleep(Duration::from_millis(250));
    }
    if alive(&pid) {
        return format!("{name} did not close (it may be asking something): close it yourself, then open it again");
    }
    std::thread::sleep(Duration::from_millis(900));
    // File Pilot rewrites its settings when it closes: now that it has, the scheme can be written
    let mut written = String::new();
    if id == "filepilot" {
        let pal = app_palette("filepilot");
        if let Err(e) = filepilot(&pal) {
            written = format!(" ({e})");
        }
    }
    if id == "spicetify" {
        // Spicetify writes the theme into Spotify and starts it again
        let ok = Command::new("spicetify").arg("apply").creation_flags(NO_WINDOW).output().map(|o| o.status.success()).unwrap_or(false);
        return if ok { "Spotify restarted".into() } else { "Spotify closed but spicetify apply did not finish: run it yourself".into() };
    }
    let args: &[&str] = if id == "helium" { &["--restore-last-session"] } else { &[] };
    let opened = Command::new(&path).args(args).current_dir(Path::new(&path).parent().unwrap_or(Path::new("."))).creation_flags(0x0000_0008 | NO_WINDOW).spawn().is_ok();
    if opened { format!("{name} restarted{written}") } else { format!("{name} closed but could not be started again: open it yourself") }
}

/// The "Restart apps" button: each program that was waiting for a restart, one after the other.
pub fn restart(id: &str) -> String {
    if id != "apps" {
        return restart_one(id);
    }
    let list: Vec<&'static str> = to_restart().lock().unwrap().drain(..).collect();
    let names: Vec<&str> = list.iter().map(|id| match *id { "zen" => "Zen", "filepilot" => "File Pilot", "spicetify" => "Spotify", "obs" => "OBS Studio", _ => "Helium" }).collect();
    let results: Vec<String> = list.iter().map(|id| restart_one(id)).collect();
    // all went well: one short line; otherwise what each one said
    if results.iter().all(|r| r.ends_with("restarted")) {
        format!("Restarted {}", names.join(", "))
    } else {
        results.join(". ")
    }
}
