//! Zen browser: yasb-chrome.css (browser UI) and yasb-content.css (about: pages) in every profile's chrome folder.

use crate::lucent::Palette;
use regex::Regex;
use serde_json::Value;
use std::{fs, path::{Path, PathBuf}};

const BUTTON_IDS: [&str; 8] = ["PanelUI-menu-button", "unified-extensions-button", "reload-button", "stop-button", "back-button",
    "forward-button", "downloads-button", "nav-bar-overflow-button"];
const AREAS: [&str; 4] = ["#zen-sidebar-top-buttons", "#zen-sidebar-bottom-buttons", "#nav-bar", "#TabsToolbar"];
const CUSTOMIZE_PAGE: [&str; 6] = ["#customization-palette toolbarbutton .toolbarbutton-icon", "#customization-palette .toolbarbutton-icon",
    "toolbarpaletteitem .toolbarbutton-icon", "#customization-palette toolbarbutton image", "#customization-palette toolbarpaletteitem image",
    "#customization-palette .toolbarbutton-badge-stack > image"];

fn write_if_changed(path: &Path, text: &str) -> Result<(), String> {
    if fs::read_to_string(path).is_ok_and(|old| old == text) {
        return Ok(());
    }
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn table(v: &Value, key: &str) -> Vec<(String, String)> {
    v[key].as_object().into_iter().flatten().map(|(k, e)| (k.clone(), e.as_str().unwrap_or("").to_string())).collect()
}

/// Returns how many profiles were written.
pub fn write(pal: &Palette) -> Result<usize, String> {
    let def: Value = serde_json::from_str(include_str!("../assets/lucent/zen.json")).map_err(|e| e.to_string())?;
    // the icon colour: the light accent
    let icon = pal.colour("l2")?;
    let pal = pal.with("icon", &icon);
    let colours = |key: &str| -> Result<Vec<(String, String)>, String> {
        table(&def, key).into_iter().map(|(k, e)| pal.colour(&e).map(|c| (k.clone(), c)).map_err(|m| format!("{k}: {m}"))).collect()
    };
    let mut vars = colours("vars")?;
    vars.extend(colours("icon_vars")?);
    let list = |v: &[(String, String)]| v.iter().map(|(k, c)| format!("    {k}: {c} !important;")).collect::<Vec<_>>().join("\n");

    // chrome: root variables, then the icon rules
    let mut chrome = format!(":root {{\n{}\n}}\n", list(&vars));
    let top = AREAS[0];
    let mut btn: Vec<String> = BUTTON_IDS.iter().map(|i| format!("{top} #{i}")).collect();
    btn.extend(AREAS.iter().map(|a| format!("{a} toolbarbutton:not(.bookmark-item)")));
    let mut img: Vec<String> = BUTTON_IDS.iter().map(|i| format!("{top} #{i} .toolbarbutton-icon")).collect();
    img.extend(AREAS.iter().map(|a| format!("{a} toolbarbutton .toolbarbutton-icon")));
    img.extend([".urlbar-icon", ".menu-iconic-icon", ".menu-icon", "#identity-icon", "#tracking-protection-icon", "#star-button"].map(String::from));
    let mut filt: Vec<String> = ["#zen-sidebar-top-buttons", "#nav-bar"].iter().flat_map(|a| BUTTON_IDS.iter().map(move |i| format!("{a} #{i} .toolbarbutton-icon"))).collect();
    filt.push("#zen-sidebar-bottom-buttons toolbarbutton .toolbarbutton-icon".into());
    // Zen draws menu icons through the Context Menu Icons mod: recolouring these here made them vanish
    img.retain(|s| s != ".menu-iconic-icon" && s != ".menu-icon");
    btn.push("#customization-palette toolbarbutton:not(.bookmark-item)".into());
    img.extend(CUSTOMIZE_PAGE.map(String::from));
    filt.extend(CUSTOMIZE_PAGE.map(String::from));
    let tpl = include_str!("../assets/lucent/browser-icons.css").to_string();
    let fill = [("icon", icon.clone()), ("attention", pal.colour("l1")?), ("button_selectors", btn.join(",\n")),
                ("image_selectors", img.join(",\n")), ("filter_selectors", filt.join(",\n")), ("filter", css_filter(&icon))];
    chrome.push_str(&fill.iter().fold(tpl, |t, (k, v)| t.replace(&format!("{{{{{k}}}}}"), v)));
    // Essentials: visible tiles
    chrome.push_str(&include_str!("../assets/lucent/zen-essentials.css").replace("{{tile}}", &pal.colour("t .22")?).replace("{{tile_selected}}", &pal.colour("t .35")?));

    let content = include_str!("../assets/lucent/browser-content.css").replace("{{variables}}", &list(&colours("content_vars")?));

    let root = PathBuf::from(std::env::var_os("APPDATA").unwrap_or_default()).join("zen").join("Profiles");
    let mut profiles: Vec<PathBuf> = fs::read_dir(&root).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    profiles.sort_by_key(|p| p.to_string_lossy().to_lowercase());
    if profiles.is_empty() {
        return Err("no Zen profile found".into());
    }
    for p in &profiles {
        let dir = p.join("chrome");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let mut full = chrome.clone();
        full.push_str(&mod_icons(&dir, &icon)); // the icons that Sine mods draw
        write_if_changed(&dir.join("yasb-chrome.css"), &full)?;
        write_if_changed(&dir.join("yasb-content.css"), &content)?;
        for (user_file, ours) in [("userChrome.css", "yasb-chrome.css"), ("userContent.css", "yasb-content.css")] {
            if !dir.join(user_file).exists() {
                fs::write(dir.join(user_file), format!("@import url(\"{ours}\");\n")).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(profiles.len())
}

// --- icons the Sine mods draw with context-fill ---------------------------------------------------------------------------------------------

/// Python's urllib.parse.quote: everything except letters, digits, _.-~ and `safe` becomes %XX.
fn quote(text: &str, safe: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"_.-~".contains(&b) || safe.as_bytes().contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn css_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            css_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "css") {
            out.push(p);
        }
    }
}

/// `selector { <one declaration> }` rules: find the declaration, then look back for its selector.
fn block_rules(text: &str, declaration: &Regex) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for m in declaration.captures_iter(text) {
        let whole = m.get(0).unwrap();
        let Some(open) = text[..whole.start()].rfind('{') else { continue };
        if !text[open + 1..whole.start()].trim().is_empty() {
            continue; // the declaration must be alone in its block
        }
        let start = text[..open].rfind(|c| c == '}' || c == '{').map(|i| i + 1).unwrap_or(0);
        let selector = &text[start..open];
        if !selector.trim().is_empty() {
            found.push((selector.to_string(), m[1].to_string()));
        }
    }
    found
}

fn mod_icons(chrome_dir: &Path, colour: &str) -> String {
    let mods = chrome_dir.join("sine-mods");
    if !mods.is_dir() {
        return String::new();
    }
    let mut files = Vec::new();
    css_files(&mods, &mut files);
    // sorted like Python sorts paths on Windows: part by part, ignoring case
    files.sort_by_key(|p| p.components().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).collect::<Vec<_>>());

    let mut out: Vec<String> = Vec::new();
    let mut seen: Vec<(Vec<String>, String)> = Vec::new();

    let list_rule = Regex::new(r#"list-style-image:\s*url\(["']?([^"')]+)["']?\)\s*!important;?\s*\}"#).unwrap();
    let id_only = Regex::new(r"^#[\w-]+$").unwrap();
    let wanted = Regex::new(r"button|compact|chevron|panelmenu").unwrap();
    let unwanted = Regex::new(r"(?i)^#(context|PanelUI|zen-site-data|pageAction|protections|identity)").unwrap();
    let hex = colour.trim_start_matches('#');
    for css in files.iter().filter(|p| !p.components().any(|c| c.as_os_str() == "ZenUI")) {
        let Ok(text) = fs::read_to_string(css) else { continue };
        for (sel, url) in block_rules(&text, &list_rule) {
            let url = if url.starts_with("data:image/svg+xml,") {
                url
            } else {
                let path = css.parent().unwrap().join(&url);
                if !url.to_lowercase().ends_with(".svg") || !path.is_file() {
                    continue;
                }
                let Ok(svg) = fs::read_to_string(&path) else { continue };
                format!("data:image/svg+xml,{}", quote(&svg, "/=: '\""))
            };
            if !url.contains("context-fill") {
                continue;
            }
            let fixed = url.replace("context-fill-opacity", "1").replace("context-fill", &format!("%23{hex}")).replace('"', "'");
            let sels: Vec<String> = sel.split(',').map(|s| (s, s.trim())).filter(|(raw, s)| id_only.is_match(s) && wanted.is_match(raw) && !unwanted.is_match(s))
                .map(|(_, s)| s.to_string()).collect();
            let ident = (sels.clone(), fixed.clone());
            if !sels.is_empty() && !seen.contains(&ident) {
                seen.push(ident);
                out.push(format!("{} {{\n    list-style-image: url(\"{fixed}\") !important;\n}}\n", sels.iter().map(|s| format!(":root {s}")).collect::<Vec<_>>().join(",\n")));
            }
        }
    }

    // Context Menu Icons: every menu item gets its icon as --menu-image: url("x.svg"), which the browser draws black on a dark menu
    let menu_rule = Regex::new(r#"--menu-image:\s*url\(["']?([^"')]+\.svg)["']?\)\s*;?\s*\}"#).unwrap();
    for css in files.iter().filter(|p| p.parent().is_some_and(|d| d.file_name().is_some_and(|n| n == "FluentUI"))) {
        let Ok(text) = fs::read_to_string(css) else { continue };
        for (sel, url) in block_rules(&text, &menu_rule) {
            let path = css.parent().unwrap().join(&url);
            let Ok(svg) = fs::read_to_string(&path) else { continue };
            let svg = svg.replace("context-fill-opacity", "1").replace("context-fill", colour);
            let data = format!("data:image/svg+xml,{}", quote(&svg, "/=: "));
            let sels: Vec<String> = sel.split(',').map(str::trim).filter(|s| !s.is_empty() && !s.contains("::")).map(String::from).collect();
            let ident = (sels.clone(), data.clone());
            if !sels.is_empty() && !seen.contains(&ident) {
                seen.push(ident);
                out.push(format!("{} {{\n    --menu-image: url(\"{data}\") !important;\n}}\n", sels.iter().map(|s| format!(":root {s}")).collect::<Vec<_>>().join(",\n")));
            }
        }
    }
    if out.is_empty() {
        String::new()
    } else {
        format!("\n/* Icons that Sine mods draw with context-fill: the colour is written into the picture (zen_mod_icons) */\n{}", out.join("\n"))
    }
}

// --- a filter that paints every icon one exact colour ----------------------------------------------------------------------------------------

/// An SVG colour matrix whose output is the colour whatever goes in (the icon's shape, its alpha, is kept): exact, and nothing to search for.
fn css_filter(hex: &str) -> String {
    let [r, g, b] = [1, 3, 5].map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0) as f32 / 255.0);
    let svg = format!("<svg xmlns='http://www.w3.org/2000/svg'><filter id='c' color-interpolation-filters='sRGB'><feColorMatrix type='matrix' values='0 0 0 0 {r:.4} 0 0 0 0 {g:.4} 0 0 0 0 {b:.4} 0 0 0 1 0'/></filter></svg>");
    format!("url(\"data:image/svg+xml,{}#c\")", quote(&svg, "/=: '"))
}
