//! Two things that run next to the bar: komorebi (the tiling window manager) and the scroll-hotkeys script. Their state is looked at every few
//! seconds in the background, and they can be started and stopped from the Home page.

use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Where things live is worked out on this machine (nothing here names a user): the scroll script sits next to the komorebi config,
/// the programs are looked for on PATH and then in the usual places.
fn home() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default())
}

fn env_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var_os(name).unwrap_or_default())
}

fn on_path(exe: &str) -> Option<std::path::PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?).map(|d| d.join(exe)).find(|p| p.is_file())
}

fn first_existing(list: &[std::path::PathBuf]) -> Option<std::path::PathBuf> {
    list.iter().find(|p| p.is_file()).cloned()
}

fn scoop_dir() -> std::path::PathBuf {
    std::env::var_os("SCOOP").map(std::path::PathBuf::from).unwrap_or_else(|| home().join("scoop"))
}

fn scroll_script() -> std::path::PathBuf {
    home().join(".config").join("komorebi").join("komorebi-scroll.ahk")
}

fn autohotkey() -> std::path::PathBuf {
    on_path("AutoHotkey64.exe")
        .or_else(|| first_existing(&[
            env_path("LOCALAPPDATA").join(r"Programs\AutoHotkey\v2\AutoHotkey64.exe"),
            env_path("ProgramFiles").join(r"AutoHotkey\v2\AutoHotkey64.exe"),
            scoop_dir().join(r"apps\autohotkey\current\v2\AutoHotkey64.exe"),
            scoop_dir().join(r"apps\autohotkey\current\AutoHotkey64.exe"),
        ]))
        .unwrap_or_else(|| "AutoHotkey64.exe".into())
}

fn komorebic() -> std::path::PathBuf {
    on_path("komorebic-no-console.exe")
        .or_else(|| first_existing(&[
            env_path("ProgramFiles").join(r"komorebi\bin\komorebic-no-console.exe"),
            scoop_dir().join(r"apps\komorebi\current\komorebic-no-console.exe"),
        ]))
        .unwrap_or_else(|| "komorebic-no-console.exe".into())
}
const NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Copy, Default)]
pub struct Status {
    pub komorebi: bool,
    pub scroll: bool,
    /// when you flip a switch the answer shown is yours until this moment, so it does not jump back while the program is still starting
    hold_until: Option<Instant>,
}

pub type Shared = Arc<Mutex<Status>>;

fn powershell(script: &str) -> String {
    Command::new("powershell").args(["-NoProfile", "-Command", script]).creation_flags(NO_WINDOW).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}

/// "1 0": is komorebi running, is the scroll script running.
fn look() -> (bool, bool) {
    let out = powershell(
        "$k = [bool](Get-Process komorebi -ErrorAction SilentlyContinue); \
         $s = [bool](Get-CimInstance Win32_Process -Filter \"Name like 'AutoHotkey%'\" | Where-Object { $_.CommandLine -match 'komorebi-scroll' }); \
         '{0} {1}' -f [int]$k, [int]$s",
    );
    let mut parts = out.split_whitespace();
    (parts.next() == Some("1"), parts.next() == Some("1"))
}

/// Start looking every few seconds. `repaint` wakes the window when the answer changed.
pub fn watch(repaint: impl Fn() + Send + 'static) -> Shared {
    let shared: Shared = Arc::new(Mutex::new(Status::default()));
    let mine = shared.clone();
    std::thread::spawn(move || loop {
        let (komorebi, scroll) = look();
        {
            let mut s = mine.lock().unwrap();
            if s.hold_until.is_none_or(|t| Instant::now() > t) {
                let changed = s.komorebi != komorebi || s.scroll != scroll;
                s.komorebi = komorebi;
                s.scroll = scroll;
                s.hold_until = None;
                if changed {
                    repaint();
                }
            }
        }
        std::thread::sleep(Duration::from_millis(3000));
    });
    shared
}

/// Show `on` at once and keep showing it for a few seconds while the program really starts or stops.
pub fn remember(shared: &Shared, komorebi: Option<bool>, scroll: Option<bool>) {
    let mut s = shared.lock().unwrap();
    if let Some(k) = komorebi {
        s.komorebi = k;
    }
    if let Some(v) = scroll {
        s.scroll = v;
    }
    s.hold_until = Some(Instant::now() + Duration::from_secs(6));
}

pub fn set_komorebi(on: bool) -> String {
    let program = komorebic();
    let args: &[&str] = if on { &["start", "--whkd"] } else { &["stop", "--whkd"] };
    let ok = Command::new(program).args(args).creation_flags(NO_WINDOW).output().map(|o| o.status.success()).unwrap_or(false);
    match (on, ok) {
        (true, true) => "Komorebi started".into(),
        (false, true) => "Komorebi stopped".into(),
        _ => "Could not reach komorebic".into(),
    }
}

pub fn set_scroll(on: bool) -> String {
    // remembered: the logon script (stagger-startup.ps1) starts the scroll script again at the next login when this is 1
    crate::wallpaper::write_setting("scroll_hotkeys", if on { "1" } else { "0" });
    if on {
        // a normal start (not "no window"): AutoHotkey is a window program, and the script hides its own window
        let started = Command::new(autohotkey()).arg(scroll_script()).spawn().is_ok();
        return if started { "Scroll hotkeys on".into() } else { "Could not start the scroll script".into() };
    }
    powershell("Get-CimInstance Win32_Process -Filter \"Name like 'AutoHotkey%'\" | Where-Object { $_.CommandLine -match 'komorebi-scroll' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }");
    "Scroll hotkeys off".into()
}
