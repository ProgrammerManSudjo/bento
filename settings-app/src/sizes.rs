//! Label and icon size for the whole bar: a small block at the end of styles.css that YASB reloads by itself. 0 leaves the sizes the stylesheet has.

use std::path::Path;

const START: &str = "/* yasb-settings: sizes */";
const END: &str = "/* end yasb-settings */";

/// (label size, icon size) from the block, 0 where there is none.
pub fn read(css: &Path) -> (i64, i64) {
    let text = std::fs::read_to_string(css).unwrap_or_default();
    let Some(a) = text.find(START) else { return (0, 0) };
    let block = &text[a..];
    let find = |selector: &str| -> i64 {
        block.find(selector).and_then(|i| {
            let rest = &block[i..];
            let at = rest.find("font-size:")? + "font-size:".len();
            rest[at..].trim_start().split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
        }).unwrap_or(0)
    };
    (find(".yasb-bar .label"), find(".yasb-bar .icon"))
}

pub fn write(css: &Path, label: i64, icon: i64) -> Result<(), String> {
    let text = std::fs::read_to_string(css).map_err(|e| e.to_string())?;
    let crlf = text.contains("\r\n");
    let mut base = match (text.find(START), text.find(END)) {
        (Some(a), Some(b)) if b > a => format!("{}{}", &text[..a], text[b + END.len()..].trim_start_matches(['\r', '\n'])),
        _ => text,
    };
    base = base.trim_end().to_string();
    if label > 0 || icon > 0 {
        base.push_str(&format!("\n\n{START}\n"));
        if label > 0 {
            base.push_str(&format!(".yasb-bar .label {{ font-size: {label}px; }}\n"));
        }
        if icon > 0 {
            base.push_str(&format!(".yasb-bar .icon {{ font-size: {icon}px; }}\n"));
        }
        base.push_str(END);
    }
    base.push('\n');
    let out = if crlf { base.replace("\r\n", "\n").replace('\n', "\r\n") } else { base };
    std::fs::write(css, out).map_err(|e| e.to_string())
}

// --- one widget's own sizes ----------------------------------------------------------------------------------------------------------------

const WSTART: &str = "/* yasb-settings: widget sizes */";
const WEND: &str = "/* end yasb-settings: widget sizes */";

/// The style class YASB gives a widget, from its type in config.yaml ("yasb.clock.ClockWidget" is "clock-widget").
pub fn class_for(kind: &str) -> Option<&'static str> {
    Some(match kind.rsplit('.').next()? {
        "ClockWidget" => "clock-widget",
        "WorkspaceWidget" => "komorebi-workspaces",
        "ActiveLayoutWidget" => "komorebi-active-layout",
        "KomorebiControlWidget" => "komorebi-control-widget",
        "PowerMenuWidget" => "power-menu-widget",
        "SystrayWidget" => "systray",
        "TaskbarWidget" => "taskbar-widget",
        "HomeWidget" => "home-widget",
        "WallpapersWidget" => "wallpapers-widget",
        "NotesWidget" => "notes-widget",
        "WhkdWidget" => "whkd-widget",
        "ActiveWindowWidget" => "active-window-widget",
        "CavaWidget" => "cava-widget",
        _ => return None,
    })
}

/// The lines of the widget sizes block.
fn block_lines(text: &str) -> Vec<String> {
    match (text.find(WSTART), text.find(WEND)) {
        (Some(a), Some(b)) if b > a => text[a + WSTART.len()..b].lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect(),
        _ => Vec::new(),
    }
}

/// (label size, icon size) of one widget, 0 where it has none.
pub fn read_widget(css: &Path, name: &str) -> (i64, i64) {
    let text = std::fs::read_to_string(css).unwrap_or_default();
    let lines = block_lines(&text);
    let find = |what: &str| -> i64 {
        let tag = format!("/* {name} {what} */");
        lines.iter().find(|l| l.starts_with(&tag)).and_then(|l| {
            let at = l.find("font-size:")? + "font-size:".len();
            l[at..].trim_start().split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
        }).unwrap_or(0)
    };
    (find("label"), find("icon"))
}

pub fn write_widget(css: &Path, name: &str, class: &str, label: i64, icon: i64) -> Result<(), String> {
    let text = std::fs::read_to_string(css).map_err(|e| e.to_string())?;
    let crlf = text.contains("\r\n");
    let mut lines = block_lines(&text);
    lines.retain(|l| !l.starts_with(&format!("/* {name} label */")) && !l.starts_with(&format!("/* {name} icon */")));
    if label > 0 {
        lines.push(format!("/* {name} label */ .yasb-bar .{class} .label {{ font-size: {label}px; }}"));
    }
    if icon > 0 {
        lines.push(format!("/* {name} icon */ .yasb-bar .{class} .icon {{ font-size: {icon}px; }}"));
    }
    let mut base = match (text.find(WSTART), text.find(WEND)) {
        (Some(a), Some(b)) if b > a => format!("{}{}", &text[..a], text[b + WEND.len()..].trim_start_matches(['\r', '\n'])),
        _ => text,
    };
    base = base.trim_end().to_string();
    if !lines.is_empty() {
        base.push_str(&format!("\n\n{WSTART}\n{}\n{WEND}", lines.join("\n")));
    }
    base.push('\n');
    let out = if crlf { base.replace("\r\n", "\n").replace('\n', "\r\n") } else { base };
    std::fs::write(css, out).map_err(|e| e.to_string())
}
